use super::frame::{
    ContinuationDecoder, ContinuationEncoder, DataEncoder, HeadersDecoder, HeadersEncoder,
    PushPromiseEncoder,
};
use super::hpack::{DecodeInstructions, FieldRep, Indices};
use crate::common::COLON;
use crate::{ReadByte, WriteByte};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::io::Error;
use std::sync::Arc;

///A trait for getting a stream identifier.
pub trait H2StreamIdentifier {
    ///Returns a stream identifier.
    fn next(&mut self) -> u32;
}

///A builder for getting a stream identifier.
pub struct H2StreamIdentifierBuilder(Arc<u32>);

impl H2StreamIdentifier for H2StreamIdentifierBuilder {
    fn next(&mut self) -> u32 {
        if let Some(i) = Arc::get_mut(&mut self.0) {
            let n = *i;
            *i = n + 1;
            n
        } else {
            0
        }
    }
}

impl H2StreamIdentifierBuilder {
    pub fn new() -> Self {
        Self(Arc::new(1))
    }
}

///A trait for several frame output.
pub trait H2DistributeEncoder {
    ///Exports a headers frame.
    fn headers(&mut self, o: HeadersEncoder);

    ///Exports a continuation frame.
    fn continuation(&mut self, o: ContinuationEncoder);

    ///Exports a push_promise frame.
    fn push_promise(&mut self, o: PushPromiseEncoder);

    ///Exports a data frame.
    fn data(&mut self, o: DataEncoder);
}

impl H2DistributeEncoder for Vec<Vec<u8>> {
    fn headers(&mut self, o: HeadersEncoder) {
        let mut v = Vec::new();
        o.encode(&mut v);
        self.push(v);
    }

    fn continuation(&mut self, o: ContinuationEncoder) {
        let mut v = Vec::new();
        o.encode(&mut v);
        self.push(v);
    }

    fn push_promise(&mut self, o: PushPromiseEncoder) {
        let mut v = Vec::new();
        o.encode(&mut v);
        self.push(v);
    }

    fn data(&mut self, o: DataEncoder) {
        let mut v = Vec::new();
        o.encode(&mut v);
        self.push(v);
    }
}

enum HeadersContinuation {
    Headers(HeadersEncoder),
    Continuation(ContinuationEncoder),
    PushPromise(PushPromiseEncoder),
    None,
    Unused,
}

impl WriteByte for HeadersContinuation {
    fn surplus_mut(&self) -> usize {
        match self {
            Self::Headers(h) => {
                let v = h.field_block_fragment();
                v.capacity() - v.len()
            }
            Self::Continuation(c) => {
                let v = c.field_block_fragment();
                v.capacity() - v.len()
            }
            Self::PushPromise(p) => {
                let v = p.field_block_fragment();
                v.capacity() - v.len()
            }
            Self::None | Self::Unused => 0,
        }
    }

    fn put(&mut self, o: u8) -> Option<Error> {
        match self {
            Self::Headers(h) => h.field_block_fragment_mut().put(o),
            Self::Continuation(c) => c.field_block_fragment_mut().put(o),
            Self::PushPromise(p) => p.field_block_fragment_mut().put(o),
            Self::None | Self::Unused => None,
        }
    }

    fn put_repeat(&mut self, cnt: usize, o: u8) -> Option<Error> {
        match self {
            Self::Headers(h) => h.field_block_fragment_mut().put_repeat(cnt, o),
            Self::Continuation(c) => c.field_block_fragment_mut().put_repeat(cnt, o),
            Self::PushPromise(p) => p.field_block_fragment_mut().put_repeat(cnt, o),
            Self::None | Self::Unused => None,
        }
    }

    fn put_all(&mut self, buf: &[u8]) -> Option<Error> {
        match self {
            Self::Headers(h) => h.field_block_fragment_mut().put_all(buf),
            Self::Continuation(c) => c.field_block_fragment_mut().put_all(buf),
            Self::PushPromise(p) => p.field_block_fragment_mut().put_all(buf),
            Self::None | Self::Unused => None,
        }
    }
}

impl HeadersContinuation {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Self::None)
    }
}

///A helper to build one HEADERS/PUSH_PROMISE frame, followed by zero or more CONTINUATION frames.
///
///If the length of buffer exceeds its capacity, the buffer will export.
///
///Call the flush method after encoding all fields.
#[derive(CopyGetters, Setters)]
pub struct H2EncodeFieldsHelper<'a, T, U>
where
    T: H2StreamIdentifier,
    U: H2DistributeEncoder,
{
    #[getset(get_copy = "pub", set = "pub")]
    fields_capacity: u32,
    #[getset(get_copy = "pub", set = "pub")]
    continuation_capacity: u32,
    push_promise: bool,
    stream_identifier: &'a mut T,
    output: &'a mut U,
    buffer: HeadersContinuation,
}

impl<'a, T, U> WriteByte for H2EncodeFieldsHelper<'a, T, U>
where
    T: H2StreamIdentifier,
    U: H2DistributeEncoder,
{
    fn surplus_mut(&self) -> usize {
        usize::MAX
    }

    fn put(&mut self, o: u8) -> Option<Error> {
        if self.buffer.surplus_mut() > 0 {
            self.buffer.put(o)
        } else {
            self.flush();
            self.next();
            self.put(o)
        }
    }

    fn put_repeat(&mut self, cnt: usize, o: u8) -> Option<Error> {
        let n = self.buffer.surplus_mut();
        if n >= cnt {
            return self.buffer.put_repeat(cnt, o);
        } else {
            self.buffer.put_repeat(n, o);
            self.flush();
            self.next();
            self.put_repeat(cnt - n, o)
        }
    }

    fn put_all(&mut self, mut buf: &[u8]) -> Option<Error> {
        let n = self.buffer.surplus_mut();
        if n > 0 {
            if n >= buf.len() {
                return self.buffer.put_all(buf);
            } else {
                self.buffer.put_all(&buf[..n]);
                buf = &buf[n..]
            }
        }
        self.flush();
        self.next();
        self.put_all(buf)
    }
}

impl<'a, T, U> H2EncodeFieldsHelper<'a, T, U>
where
    T: H2StreamIdentifier,
    U: H2DistributeEncoder,
{
    ///Creates a helper to build one HEADERS frame, followed by zero or more CONTINUATION frames.
    pub fn new(stream_identifier: &'a mut T, output: &'a mut U) -> Self {
        Self {
            fields_capacity: 0,
            continuation_capacity: 0,
            push_promise: false,
            stream_identifier,
            output,
            buffer: HeadersContinuation::Unused,
        }
    }

    ///Creates a helper to build one PUSH_PROMISE frame, followed by zero or more CONTINUATION frames.
    pub fn new_push_promise(stream_identifier: &'a mut T, output: &'a mut U) -> Self {
        Self {
            fields_capacity: 0,
            continuation_capacity: 0,
            push_promise: true,
            stream_identifier,
            output,
            buffer: HeadersContinuation::Unused,
        }
    }

    fn next(&mut self) {
        match self.buffer {
            HeadersContinuation::Headers(_) => {}
            HeadersContinuation::Continuation(_) => {}
            HeadersContinuation::PushPromise(_) => {}
            HeadersContinuation::None => {
                let c = ContinuationEncoder::new(
                    self.stream_identifier.next(),
                    self.continuation_capacity as usize,
                );
                self.buffer = HeadersContinuation::Continuation(c);
            }
            HeadersContinuation::Unused => {
                if self.push_promise {
                    let p = PushPromiseEncoder::new(
                        self.stream_identifier.next(),
                        self.fields_capacity as usize,
                    );
                    self.buffer = HeadersContinuation::PushPromise(p);
                } else {
                    let h = HeadersEncoder::new(
                        self.stream_identifier.next(),
                        self.fields_capacity as usize,
                    );
                    self.buffer = HeadersContinuation::Headers(h);
                }
            }
        }
    }

    ///Flushes the buffer to ensure that the frame reach their destination.
    pub fn flush(&mut self) {
        match self.buffer.take() {
            HeadersContinuation::Headers(h) => {
                self.output.headers(h);
            }
            HeadersContinuation::Continuation(c) => {
                self.output.continuation(c);
            }
            HeadersContinuation::PushPromise(p) => {
                self.output.push_promise(p);
            }
            _ => {}
        }
    }

    ///Encodes a field into the buffer.
    pub fn field(&mut self, a: FieldRep) {
        self.next();
        a.encode(self);
    }

    ///Encodes fields vec into the buffer.
    pub fn fields(&mut self, vec: Vec<FieldRep>) {
        self.next();
        for o in vec {
            o.encode(self);
        }
    }

    ///Encodes pseudo-header fields vec and fields vec into the buffer.
    pub fn pseudo_and_fields(&mut self, pseudo: Vec<FieldRep>, fields: Vec<FieldRep>) {
        self.next();
        for o in pseudo {
            o.encode(self);
        }
        for o in fields {
            o.encode(self);
        }
    }
}

///A helper to build one or more DATA frames.
///
///If the length of buffer exceeds its capacity, the buffer will export.
///
///Call the flush method after encoding all bytes.
#[derive(CopyGetters, Setters)]
pub struct H2ContentHelper<'a, T, U>
where
    T: H2StreamIdentifier,
    U: H2DistributeEncoder,
{
    #[getset(get_copy = "pub", set = "pub")]
    data_capacity: u32,
    stream_identifier: &'a mut T,
    output: &'a mut U,
    buffer: Option<DataEncoder>,
}

impl<'a, T, U> H2ContentHelper<'a, T, U>
where
    T: H2StreamIdentifier,
    U: H2DistributeEncoder,
{
    ///Creates a helper to build DATA frames.
    pub fn new(stream_identifier: &'a mut T, output: &'a mut U) -> Self {
        Self {
            data_capacity: 0,
            stream_identifier,
            output,
            buffer: None,
        }
    }

    fn next(&mut self) -> &mut DataEncoder {
        self.buffer.get_or_insert_with(|| {
            DataEncoder::new(self.stream_identifier.next(), self.data_capacity as usize)
        })
    }

    ///Flushes the buffer to ensure that the frame reach their destination.
    pub fn flush(&mut self) {
        if let Some(buf) = self.buffer.take() {
            self.output.data(buf);
        }
    }

    ///Encodes a byte slice into the buffer.
    pub fn byte_slice(&mut self, o: &[u8]) {
        let buf = self.next();
        let n = buf.data_mut().surplus_mut();
        if n >= o.len() {
            buf.data_mut().put_all(o);
        } else {
            buf.data_mut().put_all(&o[0..n]);
            self.flush();
            self.byte_slice(&o[n..]);
        }
    }

    ///Encodes a `ReadByte` into the buffer.
    pub fn read_byte(&mut self, r: &mut impl ReadByte) {
        while let Some(o) = r.fetch_some() {
            self.byte_slice(o);
        }
    }
}

///A trait for name-value pairs output.
pub trait H2DistributeFields {
    ///Exports a name-value pair.
    fn next(&mut self, name: Vec<u8>, value: Vec<u8>) {
        if let Some(i) = name.first() {
            if *i == COLON {
                return self.next_pseudo(name, value);
            }
        }
        self.next_field(name, value);
    }

    ///Exports a pseudo-header field.
    fn next_pseudo(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///Exports a field.
    fn next_field(&mut self, name: Vec<u8>, value: Vec<u8>);
}

impl H2DistributeFields for Vec<(Vec<u8>, Vec<u8>)> {
    fn next_pseudo(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.push((name, value))
    }

    fn next_field(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.push((name, value))
    }
}

///A helper to decode field section.
///
///It will export name-value pairs.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct H2DecodeFieldsHelper<'a, T, U>
where
    T: Indices,
    U: H2DistributeFields,
{
    #[getset(get = "pub", get_mut = "pub")]
    index: &'a mut T,
    output: &'a mut U,
}

impl<'a, T, U> DecodeInstructions for H2DecodeFieldsHelper<'a, T, U>
where
    T: Indices,
    U: H2DistributeFields,
{
    fn indexed(&mut self, n: usize) {
        if let Some((name, value)) = self.index.get_entry(n) {
            self.output.next(name.to_vec(), value.to_vec());
        }
    }

    fn incremental_indexing_indexed_name(&mut self, n: usize, value: Vec<u8>) {
        if let Some(name) = self.index.get_name(n) {
            self.output.next(name.to_vec(), value.clone());
            self.index.add(name.to_vec(), value);
        }
    }

    fn incremental_indexing_new_name(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.output.next(name.clone(), value.clone());
        self.index.add(name, value);
    }

    fn without_indexing_indexed_name(&mut self, n: usize, value: Vec<u8>) {
        if let Some(name) = self.index.get_name(n) {
            self.output.next(name.to_vec(), value);
        }
    }

    fn without_indexing_new_name(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.output.next(name, value);
    }

    fn never_indexed_indexed_name(&mut self, n: usize, value: Vec<u8>) {
        if let Some(name) = self.index.get_name(n) {
            self.output.next(name.to_vec(), value);
        }
    }

    fn never_indexed_new_name(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.output.next(name, value);
    }

    fn dynamic_table_size_update(&mut self, n: usize) {
        self.index.size_update(n);
    }
}

impl<'a, T, U> H2DecodeFieldsHelper<'a, T, U>
where
    T: Indices,
    U: H2DistributeFields,
{
    ///Creates a helper to decode field section.
    pub fn new(index: &'a mut T, output: &'a mut U) -> Self {
        Self { index, output }
    }

    ///Decodes a byte slice into the output.
    pub fn byte_slice(&mut self, buffer: &[u8]) {
        super::hpack::decode(buffer, self)
    }

    ///Decodes one `HeadersDecoder`, zero or more `ContinuationDecoder` into the output.
    pub fn headers_cont(&mut self, o: HeadersDecoder, vec: Vec<ContinuationDecoder>) {
        o.decode_fields(self);
        for i in vec {
            i.decode_fields(self);
        }
    }

    ///Decodes one `HeadersDecoder`.
    pub fn headers(&mut self, o: HeadersDecoder) {
        o.decode_fields(self);
    }
}
