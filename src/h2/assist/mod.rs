use super::frame::*;
use super::hpack::*;
use super::*;
use derive_more::Debug;
use std::collections::VecDeque;

macro_rules! frame_helper {
    ($a:ident, $b:ident, $c:ty, $d:literal) => {
        #[doc = concat!("A helper to build one ", $d, " frame, followed by zero or more CONTINUATION frames.")]
        pub struct $a {
            stream_identifier: u32,
            $b: usize,
            continuation_capacity: usize,
            rst: $c,
            o: Option<Continuation>,
            rst_vec: Vec<Continuation>,
            start: bool,
        }

        impl $a {
            ///Creates.
            pub fn new(stream_identifier: u32, $b: usize, continuation_capacity: usize) -> Self {
                Self {
                    stream_identifier,
                    $b,
                    continuation_capacity,
                    rst: <$c>::new(stream_identifier, $b),
                    o: None,
                    rst_vec: Vec::new(),
                    start: false,
                }
            }

            ///Takes frames out of self.
            pub fn take(mut self) -> ($c, Vec<Continuation>) {
                self.flush();
                (self.rst, self.rst_vec)
            }

            ///Exports frames into [`PutU8`].
            pub fn export(self, o: &mut dyn PutU8) {
                let (e, v) = self.take();
                e.export(o);
                for r in v {
                    r.export(o);
                }
            }

            fn buffer(&mut self) -> &mut FiniteVec {
                if self.start {
                    self.o
                        .get_or_insert_with(|| {
                            Continuation::new(
                                self.stream_identifier,
                                self.continuation_capacity,
                            )
                        })
                        .field_block_fragment_mut()
                } else {
                    self.rst.field_block_fragment_mut()
                }
            }

            fn flush(&mut self) {
                if self.start {
                    if let Some(o) = self.o.take() {
                        self.rst_vec.push(o);
                    }
                } else {
                    self.start = true;
                }
            }
        }
    }
}

macro_rules! helper_impl {
    ($a:ty) => {
        impl PutU8 for $a {
            fn blank(&self) -> usize {
                usize::MAX
            }

            fn put_u8(&mut self, o: u8) -> bool {
                let v = self.buffer();
                if v.blank() > 0 {
                    v.put_u8(o)
                } else {
                    self.flush();
                    self.put_u8(o)
                }
            }

            fn put_exact(&mut self, mut o: &[u8]) -> bool {
                let v = self.buffer();
                let n = v.blank();
                if n >= o.len() {
                    v.put_exact(o)
                } else {
                    if n > 0 {
                        v.put_exact(&o[..n]);
                        o = &o[n..];
                    }
                    self.flush();
                    self.put_exact(o)
                }
            }

            fn put_repeat(&mut self, n: usize, o: u8) -> bool {
                let v = self.buffer();
                let s = v.blank();
                if s >= n {
                    v.put_repeat(n, o)
                } else {
                    if s > 0 {
                        v.put_repeat(s, o);
                    }
                    self.flush();
                    self.put_repeat(n - s, o)
                }
            }
        }
    };
}

frame_helper!(HeadersHelper, headers_capacity, Headers, "HEADERS");

helper_impl!(HeadersHelper);

frame_helper!(
    PushPromiseHelper,
    push_promise_capacity,
    PushPromise,
    "PUSH_PROMISE"
);

helper_impl!(PushPromiseHelper);

///A helper to build one or more DATA frames.
pub struct DataHelper {
    stream_identifier: u32,
    capacity: usize,
    o: Option<Data>,
    rst: Vec<Data>,
}

impl DataHelper {
    ///Creates.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            capacity,
            o: None,
            rst: Vec::new(),
        }
    }

    ///Takes frames out of self.
    pub fn take(mut self) -> Vec<Data> {
        self.flush();
        self.rst
    }

    ///Exports frames into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let v = self.take();
        for r in v {
            r.export(o);
        }
    }

    fn buffer(&mut self) -> &mut FiniteVec {
        self.o
            .get_or_insert_with(|| Data::new(self.stream_identifier, self.capacity))
            .data_mut()
    }

    fn flush(&mut self) {
        if let Some(o) = self.o.take() {
            self.rst.push(o);
        }
    }
}

helper_impl!(DataHelper);

macro_rules! fields_vec {
    ($a:ident, $b:ty) => {
        ///A helper to add some fields, differentiate pseudo-header fields.
        pub fn $a(v: Vec<(FieldName, FieldValue)>, r: &mut $b) {
            for (name, value) in v {
                if name.is_empty() {
                    return;
                }
                if name.is_pseudo() {
                    r.set_pseudo(name.as_bytes(), value);
                } else {
                    r.add_field(name, value);
                }
            }
        }
    };
}

fields_vec!(add_fields_to_request, H2Request);

fields_vec!(add_fields_to_response, H2Response);

///This struct is used for test, maybe not meet the requirements.
#[derive(Debug, CopyGetters)]
pub struct DynamicTable {
    #[getset(get_copy = "pub")]
    capacity: usize,
    inner: VecDeque<(FieldName, FieldValue)>,
}

impl Default for DynamicTable {
    fn default() -> Self {
        Self::new(4096)
    }
}

impl DynamicTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            inner: VecDeque::new(),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn set_capacity(&mut self, n: usize) {
        self.capacity = n;
        if n == 0 {
            self.clear();
        } else {
            self.eviction();
        }
    }

    pub fn size(&self) -> usize {
        let mut i = 0;
        for (a, b) in &self.inner {
            i += a.len() + b.len() + 32;
        }
        i
    }

    pub fn eviction(&mut self) {
        while self.size() > self.capacity {
            self.inner.pop_back();
        }
    }

    pub fn add(&mut self, name: FieldName, value: FieldValue) {
        self.inner.push_front((name, value));
        self.eviction();
    }

    pub fn get_entry(&self, n: usize) -> Option<&(FieldName, FieldValue)> {
        match static_table_get_entry(n) {
            Ok(o) => Some(o),
            Err(e) => e.and_then(|n| self.inner.get(n - 1)),
        }
    }

    pub fn get_name(&self, n: usize) -> Option<&FieldName> {
        match static_table_get_name(n) {
            Ok(o) => Some(o),
            Err(e) => e.and_then(|n| self.inner.get(n - 1).map(|(o, _)| o)),
        }
    }
}

///This function is used for test, maybe not meet the requirements.
pub fn update_dynamic_table_to_vec(
    v: Vec<HfriResult>,
    t: &mut DynamicTable,
) -> Vec<(FieldName, FieldValue)> {
    let mut r = Vec::new();
    for o in v {
        if let Some((name, value)) = update_dynamic_table(o, t) {
            r.push((name, value));
        }
    }
    r
}

///This function is used for test, maybe not meet the requirements.
pub fn update_dynamic_table(
    o: HfriResult,
    t: &mut DynamicTable,
) -> Option<(FieldName, FieldValue)> {
    match o {
        HfriResult::Indexed(n) => t.get_entry(n).cloned(),
        HfriResult::IncrementalIndexingIndexedName(n, value) => {
            t.get_name(n).cloned().map(|name| {
                t.add(name.clone(), value.clone());
                (name, value)
            })
        }
        HfriResult::IncrementalIndexingNewName(name, value) => {
            t.add(name.clone(), value.clone());
            Some((name, value))
        }
        HfriResult::WithoutIndexingIndexedName(n, value) => {
            t.get_name(n).map(|name| (name.clone(), value))
        }
        HfriResult::WithoutIndexingNewName(name, value) => Some((name, value)),
        HfriResult::NeverIndexedIndexedName(n, value) => {
            t.get_name(n).map(|name| (name.clone(), value))
        }
        HfriResult::NeverIndexedNewName(name, value) => Some((name, value)),
        HfriResult::DynamicTableSizeUpdate(n) => {
            t.set_capacity(n);
            None
        }
    }
}

#[inline]
fn handle_index_ref(r: IndexRef, o: &mut dyn PutU8) {
    match r {
        IndexRef::Both(i) => build_indexed(i, o),
        IndexRef::One(i, v) => build_without_indexing_indexed_name(i, v.into(), o),
    }
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_request_pseudo_header_fields(r: &H2Request, o: &mut dyn PutU8) {
    handle_index_ref(r.indexed_method(), o);
    if let Some(v) = r.indexed_scheme() {
        handle_index_ref(v, o)
    }
    if let Some(v) = r.indexed_authority() {
        handle_index_ref(v, o)
    }
    if let Some(v) = r.indexed_path() {
        handle_index_ref(v, o)
    }
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_response_pseudo_header_fields(r: &H2Response, o: &mut dyn PutU8) {
    handle_index_ref(r.indexed_status(), o)
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_fields(r: &Entity, o: &mut dyn PutU8) {
    for (k, v) in r.iter() {
        build_incremental_indexing_new_name(k.as_bytes().into(), v.one().as_bytes().into(), o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request() {
        let a: &[u8] = &[
            0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70,
            0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d,
        ];
        let mut t = DynamicTable::default();
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut req = H2Request::default();
            add_fields_to_request(update_dynamic_table_to_vec(v, &mut t), &mut req);
        }
        assert_eq!(t.size(), 57);

        let a: &[u8] = &[
            0x82, 0x86, 0x84, 0xbe, 0x58, 0x08, 0x6e, 0x6f, 0x2d, 0x63, 0x61, 0x63, 0x68, 0x65,
        ];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut req = H2Request::default();
            add_fields_to_request(update_dynamic_table_to_vec(v, &mut t), &mut req);
        }
        assert_eq!(t.size(), 110);

        let a: &[u8] = &[
            0x82, 0x87, 0x85, 0xbf, 0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x6b,
            0x65, 0x79, 0x0c, 0x63, 0x75, 0x73, 0x74, 0x6f, 0x6d, 0x2d, 0x76, 0x61, 0x6c, 0x75,
            0x65,
        ];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut req = H2Request::default();
            add_fields_to_request(update_dynamic_table_to_vec(v, &mut t), &mut req);
            println!("{:?}", req);
        }
        assert_eq!(t.size(), 164);

        let a: &[u8] = &[
            0x82, 0x86, 0x84, 0x41, 0x8c, 0xf1, 0xe3, 0xc2, 0xe5, 0xf2, 0x3a, 0x6b, 0xa0, 0xab,
            0x90, 0xf4, 0xff,
        ];
        let mut t = DynamicTable::default();
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut req = H2Request::default();
            add_fields_to_request(update_dynamic_table_to_vec(v, &mut t), &mut req);
        }
        assert_eq!(t.size(), 57);

        let a: &[u8] = &[
            0x82, 0x86, 0x84, 0xbe, 0x58, 0x86, 0xa8, 0xeb, 0x10, 0x64, 0x9c, 0xbf,
        ];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut req = H2Request::default();
            add_fields_to_request(update_dynamic_table_to_vec(v, &mut t), &mut req);
        }
        assert_eq!(t.size(), 110);

        let a: &[u8] = &[
            0x82, 0x87, 0x85, 0xbf, 0x40, 0x88, 0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xa9, 0x7d, 0x7f,
            0x89, 0x25, 0xa8, 0x49, 0xe9, 0x5b, 0xb8, 0xe8, 0xb4, 0xbf,
        ];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut req = H2Request::default();
            add_fields_to_request(update_dynamic_table_to_vec(v, &mut t), &mut req);
            println!("{:?}", req);
        }
        assert_eq!(t.size(), 164);
    }

    #[test]
    fn response() {
        let a: &[u8] = &[
            0x48, 0x03, 0x33, 0x30, 0x32, 0x58, 0x07, 0x70, 0x72, 0x69, 0x76, 0x61, 0x74, 0x65,
            0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c, 0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74, 0x20,
            0x32, 0x30, 0x31, 0x33, 0x20, 0x32, 0x30, 0x3a, 0x31, 0x33, 0x3a, 0x32, 0x31, 0x20,
            0x47, 0x4d, 0x54, 0x6e, 0x17, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x77,
            0x77, 0x77, 0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d,
        ];
        let mut t = DynamicTable::default();
        t.set_capacity(256);
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut rsp = H2Response::default();
            add_fields_to_response(update_dynamic_table_to_vec(v, &mut t), &mut rsp);
        }
        assert_eq!(t.size(), 222);

        let a: &[u8] = &[0x48, 0x03, 0x33, 0x30, 0x37, 0xc1, 0xc0, 0xbf];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut rsp = H2Response::default();
            add_fields_to_response(update_dynamic_table_to_vec(v, &mut t), &mut rsp);
        }
        assert_eq!(t.size(), 222);

        let a: &[u8] = &[
            0x88, 0xc1, 0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c, 0x20, 0x32, 0x31, 0x20, 0x4f, 0x63,
            0x74, 0x20, 0x32, 0x30, 0x31, 0x33, 0x20, 0x32, 0x30, 0x3a, 0x31, 0x33, 0x3a, 0x32,
            0x32, 0x20, 0x47, 0x4d, 0x54, 0xc0, 0x5a, 0x04, 0x67, 0x7a, 0x69, 0x70, 0x77, 0x38,
            0x66, 0x6f, 0x6f, 0x3d, 0x41, 0x53, 0x44, 0x4a, 0x4b, 0x48, 0x51, 0x4b, 0x42, 0x5a,
            0x58, 0x4f, 0x51, 0x57, 0x45, 0x4f, 0x50, 0x49, 0x55, 0x41, 0x58, 0x51, 0x57, 0x45,
            0x4f, 0x49, 0x55, 0x3b, 0x20, 0x6d, 0x61, 0x78, 0x2d, 0x61, 0x67, 0x65, 0x3d, 0x33,
            0x36, 0x30, 0x30, 0x3b, 0x20, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x3d, 0x31,
        ];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut rsp = H2Response::default();
            add_fields_to_response(update_dynamic_table_to_vec(v, &mut t), &mut rsp);
            println!("{:?}", rsp);
        }
        assert_eq!(t.size(), 215);

        let a: &[u8] = &[
            0x48, 0x82, 0x64, 0x02, 0x58, 0x85, 0xae, 0xc3, 0x77, 0x1a, 0x4b, 0x61, 0x96, 0xd0,
            0x7a, 0xbe, 0x94, 0x10, 0x54, 0xd4, 0x44, 0xa8, 0x20, 0x05, 0x95, 0x04, 0x0b, 0x81,
            0x66, 0xe0, 0x82, 0xa6, 0x2d, 0x1b, 0xff, 0x6e, 0x91, 0x9d, 0x29, 0xad, 0x17, 0x18,
            0x63, 0xc7, 0x8f, 0x0b, 0x97, 0xc8, 0xe9, 0xae, 0x82, 0xae, 0x43, 0xd3,
        ];
        let mut t = DynamicTable::default();
        t.set_capacity(256);
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut rsp = H2Response::default();
            add_fields_to_response(update_dynamic_table_to_vec(v, &mut t), &mut rsp);
        }
        assert_eq!(t.size(), 222);

        let a: &[u8] = &[0x48, 0x83, 0x64, 0x0e, 0xff, 0xc1, 0xc0, 0xbf];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut rsp = H2Response::default();
            add_fields_to_response(update_dynamic_table_to_vec(v, &mut t), &mut rsp);
        }
        assert_eq!(t.size(), 222);

        let a: &[u8] = &[
            0x88, 0xc1, 0x61, 0x96, 0xd0, 0x7a, 0xbe, 0x94, 0x10, 0x54, 0xd4, 0x44, 0xa8, 0x20,
            0x05, 0x95, 0x04, 0x0b, 0x81, 0x66, 0xe0, 0x84, 0xa6, 0x2d, 0x1b, 0xff, 0xc0, 0x5a,
            0x83, 0x9b, 0xd9, 0xab, 0x77, 0xad, 0x94, 0xe7, 0x82, 0x1d, 0xd7, 0xf2, 0xe6, 0xc7,
            0xb3, 0x35, 0xdf, 0xdf, 0xcd, 0x5b, 0x39, 0x60, 0xd5, 0xaf, 0x27, 0x08, 0x7f, 0x36,
            0x72, 0xc1, 0xab, 0x27, 0x0f, 0xb5, 0x29, 0x1f, 0x95, 0x87, 0x31, 0x60, 0x65, 0xc0,
            0x03, 0xed, 0x4e, 0xe5, 0xb1, 0x06, 0x3d, 0x50, 0x07,
        ];
        if let Ok(v) = get_hfris_to_vec(&mut a.into_get()) {
            let mut rsp = H2Response::default();
            add_fields_to_response(update_dynamic_table_to_vec(v, &mut t), &mut rsp);
            println!("{:?}", rsp);
        }
        assert_eq!(t.size(), 215);
    }
}
