use crate::{common::*, WriteByte};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

///Represents field value.
pub struct FieldValue(Vec<u8>, Vec<Vec<u8>>);

impl std::fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entry(&into_str(&self.0))
            .entries(self.1.iter().map(|v| into_str(v)))
            .finish()
    }
}

impl FieldValue {
    ///Creates with bytes.
    pub fn new(o: Vec<u8>) -> Self {
        Self(o, Vec::new())
    }

    ///Returns a reference to first value.
    pub fn one(&self) -> &[u8] {
        &self.0
    }

    ///Returns a mutable reference to first value.
    pub fn one_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    ///Returns true if the rest contains value.
    pub fn has_rest(&self) -> bool {
        self.1.len() > 0
    }

    ///Returns a reference to the rest values.
    pub fn rest(&self) -> &Vec<Vec<u8>> {
        &self.1
    }

    ///Returns a mutable reference to the rest values.
    pub fn rest_mut(&mut self) -> &mut Vec<Vec<u8>> {
        &mut self.1
    }

    ///Appends a value to the back of a collection.
    pub fn push(&mut self, o: Vec<u8>) {
        self.1.push(o);
    }

    ///Returns an iterator over self.
    pub fn iter(&self) -> FieldValueIter {
        FieldValueIter(self, 0)
    }

    ///Converts self to a String, with each value separated by a comma.
    pub fn to_string(&self) -> String {
        let mut s = into_str(&self.0);
        self.1.iter().for_each(|v| {
            s.push(COMMA as char);
            s.push_str(&into_str(v))
        });
        s
    }
}

///Immutable `FieldValue` iterator.
pub struct FieldValueIter<'a>(&'a FieldValue, usize);

impl<'a> Iterator for FieldValueIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.1;
        if n == 0 {
            self.1 += 1;
            Some(self.0.one())
        } else {
            let v = self.0.rest();
            if n <= v.len() {
                self.1 += 1;
                Some(&v[n - 1])
            } else {
                None
            }
        }
    }
}

///Represents HTTP fields.
pub struct Fields(HashMap<String, FieldValue>);

impl Deref for Fields {
    type Target = HashMap<String, FieldValue>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Fields {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Debug for Fields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.0.iter()).finish()
    }
}

impl Fields {
    ///Creates.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    ///Add a field.
    pub fn add_field(&mut self, k: String, v: Vec<u8>) {
        if let Some(o) = self.0.get_mut(&k) {
            o.push(v);
        } else {
            self.0.insert(k, FieldValue::new(v));
        }
    }
}

///Represents HTTP fields and body.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct Entity {
    #[getset(get = "pub", get_mut = "pub")]
    headers: Fields,
    #[getset(get = "pub", get_mut = "pub")]
    body: Vec<u8>,
    #[getset(get_copy = "pub", set = "pub(crate)")]
    err: bool,
}

impl Entity {
    pub(crate) fn new() -> Self {
        Self {
            headers: Fields::new(),
            body: Vec::new(),
            err: false,
        }
    }

    ///Exports headers and body.
    pub(crate) fn export(&self, writer: &mut impl WriteByte) {
        for (k, v) in self.headers.iter() {
            for o in v.iter() {
                writer.put_all(k.as_bytes());
                writer.put(COLON);
                writer.put_all(o);
                writer.put(CR);
                writer.put(LF);
            }
        }
        writer.put(CR);
        writer.put(LF);
        if self.body.len() > 0 {
            writer.put_all(&self.body);
        }
    }
}
