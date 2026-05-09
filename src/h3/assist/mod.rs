use super::frame::*;
use super::qpack::*;
use super::*;
use derive_more::Debug;
use std::collections::VecDeque;

///A helper to build one or more DATA frames.
pub struct DataHelper {
    capacity: u64,
    o: Option<Data>,
    rst: Vec<Data>,
}

impl DataHelper {
    ///Creates.
    pub fn new(capacity: u64) -> Self {
        Self {
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

    ///Builds frames into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let v = self.take();
        for r in v {
            r.export(o);
        }
    }

    fn buffer(&mut self) -> &mut FiniteVec {
        self.o.get_or_insert_with(|| Data::new(self.capacity))
    }

    fn flush(&mut self) {
        if let Some(o) = self.o.take() {
            self.rst.push(o);
        }
    }
}

impl PutU8 for DataHelper {
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

fields_vec!(add_fields_to_request, H3Request);

fields_vec!(add_fields_to_response, H3Response);

///This struct is used for test, maybe not meet the requirements.
#[derive(Debug, CopyGetters)]
pub struct DynamicTable {
    #[getset(get_copy = "pub")]
    capacity: usize,
    #[getset(get_copy = "pub")]
    absolute: usize,
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
            absolute: 0,
            inner: VecDeque::new(),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn set_capacity(&mut self, n: usize) {
        self.capacity = n;
        self.eviction();
    }

    pub fn size(&self) -> usize {
        let mut s = 0;
        for (a, b) in &self.inner {
            s += a.len() + b.len() + 32;
        }
        s
    }

    pub fn eviction(&mut self) {
        while self.size() > self.capacity {
            self.inner.pop_back();
        }
    }

    pub fn add(&mut self, name: FieldName, value: FieldValue) {
        self.inner.push_front((name, value));
        self.absolute += 1;
        self.eviction();
    }

    pub fn get_entry(&self, n: usize) -> Option<&(FieldName, FieldValue)> {
        self.inner.get(n)
    }

    pub fn get_name(&self, n: usize) -> Option<&FieldName> {
        self.get_entry(n).map(|(name, _)| name)
    }
}

///This function is used for test, maybe not meet the requirements.
pub fn update_dynamic_table(o: EinstResult, t: &mut DynamicTable) {
    match o {
        EinstResult::SetDynamicTableCapacity(n) => t.set_capacity(n),
        EinstResult::InsertWithNameReference { t_bit, n, value } => {
            if t_bit {
                if let Some(name) = static_table_get_name(n) {
                    t.add(name.clone(), value);
                }
            } else {
                if let Some(name) = t.get_name(n) {
                    t.add(name.clone(), value);
                }
            }
        }
        EinstResult::InsertWithLiteralName { name, value } => {
            t.add(name, value);
        }
        EinstResult::Duplicate(n) => {
            if let Some((name, value)) = t.get_entry(n) {
                t.add(name.clone(), value.clone());
            }
        }
    }
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_field_line_representations_to_vec(
    v: Vec<FsectionResult>,
    t: &mut DynamicTable,
) -> Vec<(FieldName, FieldValue)> {
    let mut r = Vec::new();
    for o in v {
        if let Some((name, value)) = handle_field_line_representations(o, t) {
            r.push((name, value));
        }
    }
    r
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_field_line_representations(
    o: FsectionResult,
    t: &mut DynamicTable,
) -> Option<(FieldName, FieldValue)> {
    match o {
        FsectionResult::Prefix {
            required_insert_count: _,
            s_bit: _,
            delta_base: _,
        } => None,
        FsectionResult::IndexedFieldLine { t_bit, n } => {
            if t_bit {
                static_table_get_entry(n).cloned()
            } else {
                t.get_entry(n).cloned()
            }
        }
        FsectionResult::IndexedFieldLineWithPostBaseIndex(_) => None,
        FsectionResult::LiteralFieldLineWithNameReference {
            n_bit: _,
            t_bit,
            n,
            value,
        } => {
            if t_bit {
                static_table_get_name(n).map(|name| (name.clone(), value))
            } else {
                t.get_name(n).map(|name| (name.clone(), value))
            }
        }
        FsectionResult::LiteralFieldLineWithPostBaseNameReference {
            n_bit: _,
            n: _,
            value: _,
        } => None,
        FsectionResult::LiteralFieldLineWithLiteralName {
            n_bit: _,
            name,
            value,
        } => Some((name, value)),
    }
}

#[inline]
fn handle_index_ref(r: IndexRef, o: &mut dyn PutU8) {
    match r {
        IndexRef::StaticBoth(i) => build_indexed_field_line(true, i, o),
        IndexRef::StaticOne(i, v) => {
            build_literal_field_line_with_name_reference(true, true, i, v.into(), o)
        }
        IndexRef::DynamicBoth(i) => build_indexed_field_line(false, i, o),
        IndexRef::DynamicOne(i, v) => {
            build_literal_field_line_with_name_reference(true, false, i, v.into(), o)
        }
    }
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_request_pseudo_header_fields(r: &H3Request, o: &mut dyn PutU8) {
    build_prefix(0, true, 0, o);
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
pub fn handle_response_pseudo_header_fields(r: &H3Response, o: &mut dyn PutU8) {
    build_prefix(0, true, 0, o);
    handle_index_ref(r.indexed_status(), o)
}

///This function is used for test, maybe not meet the requirements.
pub fn handle_fields(r: &Entity, o: &mut dyn PutU8) {
    for (k, v) in r.iter() {
        build_literal_field_line_with_literal_name(
            true,
            k.as_bytes().into(),
            v.one().as_bytes().into(),
            o,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instructions() {
        let a: &[u8] = &[
            0x00, 0x00, 0x51, 0x0b, 0x2f, 0x69, 0x6e, 0x64, 0x65, 0x78, 0x2e, 0x68, 0x74, 0x6d,
            0x6c,
        ];
        let _ = get_fsection(|o| println!("test: {:?}", o), &mut a.into_get());

        let a: &[u8] = &[
            0x3f, 0xbd, 0x01, 0xc0, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70,
            0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0xc1, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70, 0x6c,
            0x65, 0x2f, 0x70, 0x61, 0x74, 0x68,
        ];
        let mut t = DynamicTable::default();
        if let Ok(v) = get_einsts_to_vec(&mut a.into_get()) {
            for o in v {
                update_dynamic_table(o, &mut t);
            }
        }
        println!("DynamicTable: {:?}", t);
        assert_eq!(t.size(), 106);
    }
}
