use super::prty::*;
use super::qpack::FieldLineRepresentations;
use crate::h2::frame::FrameError;
use crate::WriteByte;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::collections::HashSet;
use std::io::Error;

const DATA_FRAME_TYPE: u8 = 0x00;
const HEADERS_FRAME_TYPE: u8 = 0x01;
const CANCEL_PUSH_FRAME_TYPE: u8 = 0x03;
const SETTINGS_FRAME_TYPE: u8 = 0x04;
const PUSH_PROMISE_FRAME_TYPE: u8 = 0x05;
const GOAWAY_FRAME_TYPE: u8 = 0x07;
const MAX_PUSH_ID_FRAME_TYPE: u8 = 0x0d;

#[inline(always)]
fn fill_header(frame_type: u8, length: usize, writer: &mut impl WriteByte) {
    writer.put(frame_type);
    encode_u64(length as u64, writer)
}

///A builder which encodes data into DATA frame.
#[derive(Getters, MutGetters)]
pub struct DataEncoder {
    #[getset(get = "pub", get_mut = "pub")]
    data: Vec<u8>,
}

impl std::fmt::Debug for DataEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataEncoder")
            .field("data len", &self.data.len())
            .finish()
    }
}

impl DataEncoder {
    ///Creates with capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            data: Vec::with_capacity(capacity as usize),
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        fill_header(DATA_FRAME_TYPE, self.data.len(), writer);
        writer.put_all(&self.data)
    }
}

///A builder which encodes field section into HEADERS frame.
#[derive(Getters, MutGetters)]
pub struct HeadersEncoder {
    #[getset(get = "pub", get_mut = "pub")]
    encoded_field_section: Vec<u8>,
}

impl std::fmt::Debug for HeadersEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadersEncoder")
            .field(
                "encoded_field_section len",
                &self.encoded_field_section.len(),
            )
            .finish()
    }
}

impl HeadersEncoder {
    ///Creates with capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            encoded_field_section: Vec::with_capacity(capacity as usize),
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        fill_header(HEADERS_FRAME_TYPE, self.encoded_field_section.len(), writer);
        writer.put_all(&self.encoded_field_section)
    }
}

///A builder which encodes info into CANCEL_PUSH frame.
#[derive(CopyGetters, Setters)]
pub struct CancelPushEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
}

impl std::fmt::Debug for CancelPushEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelPushEncoder")
            .field("push_id", &self.push_id)
            .finish()
    }
}

impl CancelPushEncoder {
    ///Creates.
    pub fn new(push_id: u64) -> Self {
        Self { push_id }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let push_id = u64_to_var(self.push_id);
        fill_header(CANCEL_PUSH_FRAME_TYPE, push_id.len(), writer);
        writer.put_all(&push_id)
    }
}

///A builder which encodes info into SETTINGS frame.
#[derive(Getters, MutGetters)]
pub struct SettingsEncoder {
    #[getset(get = "pub", get_mut = "pub")]
    setting: Vec<u8>,
}

impl std::fmt::Debug for SettingsEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsEncoder")
            .field("setting len", &self.setting.len())
            .finish()
    }
}

impl SettingsEncoder {
    ///Creates with capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            setting: Vec::with_capacity(capacity as usize),
        }
    }

    ///Appends identifier and value to the back of buffer.
    pub fn push(&mut self, identifier: u64, value: u64) -> bool {
        let vec = u64_2_to_var(identifier, value);
        if self.setting.surplus_mut() < vec.len() {
            false
        } else {
            self.setting.put_all(&vec);
            true
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        fill_header(SETTINGS_FRAME_TYPE, self.setting.len(), writer);
        writer.put_all(&self.setting)
    }
}

///A builder which encodes info into PUSH_PROMISE frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct PushPromiseEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
    #[getset(get = "pub", get_mut = "pub")]
    encoded_field_section: Vec<u8>,
}

impl std::fmt::Debug for PushPromiseEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushPromiseEncoder")
            .field("push_id", &self.push_id)
            .field(
                "encoded_field_section len",
                &self.encoded_field_section.len(),
            )
            .finish()
    }
}

impl PushPromiseEncoder {
    ///Creates with capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            push_id: 0,
            encoded_field_section: Vec::with_capacity(capacity as usize),
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let push_id = u64_to_var(self.push_id);
        fill_header(
            PUSH_PROMISE_FRAME_TYPE,
            push_id.len() + self.encoded_field_section.len(),
            writer,
        );
        writer.put_all(&push_id);
        writer.put_all(&self.encoded_field_section)
    }
}

///A builder which encodes info into GOAWAY frame.
#[derive(CopyGetters, Setters)]
pub struct GoawayEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
}

impl std::fmt::Debug for GoawayEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoawayEncoder")
            .field("push_id", &self.push_id)
            .finish()
    }
}

impl GoawayEncoder {
    ///Creates.
    pub fn new(push_id: u64) -> Self {
        Self { push_id }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let push_id = u64_to_var(self.push_id);
        fill_header(GOAWAY_FRAME_TYPE, push_id.len(), writer);
        writer.put_all(&push_id)
    }
}

///A builder which encodes info into MAX_PUSH_ID frame.
#[derive(CopyGetters, Setters)]
pub struct MaxPushIdEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
}

impl std::fmt::Debug for MaxPushIdEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaxPushIdEncoder")
            .field("push_id", &self.push_id)
            .finish()
    }
}

impl MaxPushIdEncoder {
    ///Creates.
    pub fn new(push_id: u64) -> Self {
        Self { push_id }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let push_id = u64_to_var(self.push_id);
        fill_header(MAX_PUSH_ID_FRAME_TYPE, push_id.len(), writer);
        writer.put_all(&push_id)
    }
}

#[inline(always)]
fn check_length(length: u64, n: usize, err: &mut HashSet<FrameError>) {
    let n = n as u64;
    if length == n {
    } else if length > n {
        err.insert(FrameError::LengthShortage);
    } else {
        err.insert(FrameError::LengthExcess);
    };
}

///Frame decoder.
pub enum FrameDecoder<'a> {
    Data(DataDecoder<'a>),
    Headers(HeadersDecoder<'a>),
    CancelPush(CancelPushDecoder<'a>),
    Settings(SettingsDecoder<'a>),
    PushPromise(PushPromiseDecoder<'a>),
    Goaway(GoawayDecoder<'a>),
    MaxPushId(MaxPushIdDecoder<'a>),
    Invalid(FrameError),
}

impl<'a> FrameDecoder<'a> {
    ///Returns a decoder depend on the frame type, or error.
    pub fn decode(buf: &'a [u8]) -> Self {
        if buf.len() >= 2 {
            match buf[0] {
                DATA_FRAME_TYPE => Self::Data(DataDecoder::decode(buf)),
                HEADERS_FRAME_TYPE => Self::Headers(HeadersDecoder::decode(buf)),
                CANCEL_PUSH_FRAME_TYPE => Self::CancelPush(CancelPushDecoder::decode(buf)),
                SETTINGS_FRAME_TYPE => Self::Settings(SettingsDecoder::decode(buf)),
                PUSH_PROMISE_FRAME_TYPE => Self::PushPromise(PushPromiseDecoder::decode(buf)),
                GOAWAY_FRAME_TYPE => Self::Goaway(GoawayDecoder::decode(buf)),
                MAX_PUSH_ID_FRAME_TYPE => Self::MaxPushId(MaxPushIdDecoder::decode(buf)),
                _ => Self::Invalid(FrameError::InvalidFrameType),
            }
        } else {
            Self::Invalid(FrameError::LengthShortage)
        }
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct DataDecoder<'a> {
    length: u64,
    #[getset(skip)]
    data: usize,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for DataDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataDecoder")
            .field("length", &self.length)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> DataDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let o_len = o.len();
        let mut err = HashSet::new();
        check_length(length, o_len, &mut err);

        Self {
            length,
            data: v.len() - o_len,
            buffer: v,
            err,
        }
    }

    ///Returns data.
    pub fn data(&self) -> Option<&[u8]> {
        self.buffer.get(self.data..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct HeadersDecoder<'a> {
    length: u64,
    #[getset(skip)]
    encoded_field_section: usize,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for HeadersDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadersDecoder")
            .field("length", &self.length)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> HeadersDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let o_len = o.len();
        let mut err = HashSet::new();
        check_length(length, o_len, &mut err);

        Self {
            length,
            encoded_field_section: v.len() - o_len,
            buffer: v,
            err,
        }
    }

    ///Returns encoded field section.
    pub fn encoded_field_section(&self) -> Option<&[u8]> {
        self.buffer.get(self.encoded_field_section..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }

    ///Decodes encoded field section.
    ///
    ///You need an implementation of `FieldLineRepresentations`.
    pub fn decode_fields(&self, ins: &mut impl FieldLineRepresentations) {
        if let Some(o) = self.encoded_field_section() {
            super::qpack::decode_field(o, ins)
        }
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct CancelPushDecoder<'a> {
    length: u64,
    push_id: u64,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for CancelPushDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelPushDecoder")
            .field("length", &self.length)
            .field("push_id", &self.push_id)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> CancelPushDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let mut err = HashSet::new();
        check_length(length, o.len(), &mut err);

        let push_id = decode_var(&mut o);

        Self {
            length,
            push_id,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct SettingsDecoder<'a> {
    length: u64,
    #[getset(skip)]
    setting: usize,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for SettingsDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsDecoder")
            .field("length", &self.length)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> SettingsDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let o_len = o.len();
        let mut err = HashSet::new();
        check_length(length, o_len, &mut err);

        Self {
            length,
            setting: v.len() - o_len,
            buffer: v,
            err,
        }
    }

    ///Returns setting.
    pub fn setting(&self) -> Option<&[u8]> {
        self.buffer.get(self.setting..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct PushPromiseDecoder<'a> {
    length: u64,
    push_id: u64,
    #[getset(skip)]
    encoded_field_section: usize,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for PushPromiseDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushPromiseDecoder")
            .field("length", &self.length)
            .field("push_id", &self.push_id)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> PushPromiseDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let mut err = HashSet::new();
        check_length(length, o.len(), &mut err);

        let push_id = decode_var(&mut o);

        Self {
            length,
            push_id,
            encoded_field_section: v.len() - o.len(),
            buffer: v,
            err,
        }
    }

    ///Returns encoded field section.
    pub fn encoded_field_section(&self) -> Option<&[u8]> {
        self.buffer.get(self.encoded_field_section..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }

    ///Decodes encoded field section.
    ///
    ///You need an implementation of `FieldLineRepresentations`.
    pub fn decode_fields(&self, ins: &mut impl FieldLineRepresentations) {
        if let Some(o) = self.encoded_field_section() {
            super::qpack::decode_field(o, ins)
        }
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct GoawayDecoder<'a> {
    length: u64,
    ///Stream ID/Push ID
    push_id: u64,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for GoawayDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoawayDecoder")
            .field("length", &self.length)
            .field("push_id", &self.push_id)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> GoawayDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let mut err = HashSet::new();
        check_length(length, o.len(), &mut err);

        let push_id = decode_var(&mut o);

        Self {
            length,
            push_id,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct MaxPushIdDecoder<'a> {
    length: u64,
    push_id: u64,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for MaxPushIdDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaxPushIdDecoder")
            .field("length", &self.length)
            .field("push_id", &self.push_id)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> MaxPushIdDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let mut o = &v[1..];
        let length = decode_var(&mut o);
        let mut err = HashSet::new();
        check_length(length, o.len(), &mut err);

        let push_id = decode_var(&mut o);

        Self {
            length,
            push_id,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}
