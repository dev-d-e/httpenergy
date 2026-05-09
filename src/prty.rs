pub use crate::common::*;
use crate::io::*;
use derive_more::{Debug, Deref, DerefMut};
pub(crate) use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::collections::HashMap;

///Represents HTTP fields and body.
#[derive(CopyGetters, Debug, Default, Deref, DerefMut, Getters, MutGetters, Setters)]
pub struct Entity {
    #[deref]
    #[deref_mut]
    headers: Fields,
    #[debug("{}", body.len())]
    #[getset(get = "pub", get_mut = "pub")]
    body: Vec<u8>,
    #[getset(get_copy = "pub", set = "pub(crate)")]
    err: bool,
}

impl Entity {
    ///Exports headers and body.
    pub(crate) fn export(&self, o: &mut dyn PutU8) {
        for (k, v) in self.headers.iter() {
            o.put_exact(k.as_bytes());
            o.put_u8(COLON);
            v.export(o);
            o.put_u8(CR);
            o.put_u8(LF);
        }
        o.put_u8(CR);
        o.put_u8(LF);
        if self.body.len() > 0 {
            o.put_exact(&self.body);
        }
    }
}

///Represents HTTP fields.
#[derive(Debug, Default, Deref, DerefMut)]
#[debug("{_0:?}")]
pub struct Fields(HashMap<FieldName, FieldValues>);

impl Fields {
    ///Add a field.
    pub fn add_field(&mut self, k: impl Into<FieldName>, v: impl Into<FieldValue>) {
        let k = k.into();
        // let v = v.into();
        if let Some(o) = self.0.get_mut(&k) {
            o.push(v);
        } else {
            self.0.insert(k, v.into().into());
        }
    }
}

///Represents some field value.
#[derive(Debug, Getters, MutGetters)]
#[debug("{}", self)]
pub struct FieldValues {
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    one: FieldValue,
    rest: Vec<FieldValue>,
}

impl std::fmt::Display for FieldValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(s) = self.one.as_str() {
            write!(f, "[{:?}", s)?;
        }
        for v in self.rest.iter() {
            write!(f, ", ")?;
            if let Ok(s) = v.as_str() {
                write!(f, "{:?}", s)?;
            }
        }
        write!(f, "]")
    }
}

impl FieldValues {
    ///Returns the number of values.
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    ///Returns true if the rest contains value.
    pub fn has_rest(&self) -> bool {
        self.rest.len() > 0
    }

    ///Appends a value to the back of a collection.
    pub fn push(&mut self, o: impl Into<FieldValue>) {
        self.rest.push(o.into());
    }

    ///Exports field values.
    pub(crate) fn export(&self, o: &mut dyn PutU8) {
        o.put_exact(self.one.as_bytes());
        for r in self.rest.iter() {
            o.put_u8(SPACE);
            o.put_exact(r.as_bytes());
        }
    }
}

impl<T: Into<FieldValue>> From<T> for FieldValues {
    ///Creates.
    fn from(o: T) -> Self {
        Self {
            one: o.into(),
            rest: Vec::new(),
        }
    }
}

///Represents field name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[debug("{:?}",self.as_str().unwrap_or(""))]
#[repr(transparent)]
pub struct FieldName(Fref);

impl FieldName {
    ///Returns a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    ///Returns a string slice.
    pub fn as_str(&self) -> Result<&str, &[u8]> {
        self.0.as_str()
    }

    ///Returns the number of byte.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    ///Returns true if no byte.
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    ///Creates with a `Vec<u8>`. Maybe from() is is better.
    pub fn owned(o: Vec<u8>) -> Self {
        Self(Fref::Owned(o))
    }

    pub(crate) fn is_pseudo(&self) -> bool {
        self.as_bytes().starts_with(b":")
    }
}

impl From<&'static [u8]> for FieldName {
    fn from(o: &'static [u8]) -> Self {
        Self(Fref::Ref(o))
    }
}

impl From<Vec<u8>> for FieldName {
    fn from(o: Vec<u8>) -> Self {
        to_field_name(&o).unwrap_or_else(|| Self(Fref::Owned(o)))
    }
}

impl From<String> for FieldName {
    fn from(o: String) -> Self {
        o.into_bytes().into()
    }
}

impl From<&str> for FieldName {
    fn from(o: &str) -> Self {
        let o = o.as_bytes();
        to_field_name(o).unwrap_or_else(|| Self(Fref::Owned(o.to_vec())))
    }
}

macro_rules! partial_eq {
    ($t:ty) => {
        impl<const N: usize> PartialEq<[u8; N]> for $t {
            fn eq(&self, other: &[u8; N]) -> bool {
                self.as_bytes() == other
            }
        }

        impl<const N: usize> PartialEq<&[u8; N]> for $t {
            fn eq(&self, other: &&[u8; N]) -> bool {
                self.as_bytes() == *other
            }
        }

        impl<const N: usize> PartialEq<$t> for [u8; N] {
            fn eq(&self, other: &$t) -> bool {
                self.as_slice() == other.as_bytes()
            }
        }

        impl<const N: usize> PartialEq<$t> for &[u8; N] {
            fn eq(&self, other: &$t) -> bool {
                self.as_slice() == other.as_bytes()
            }
        }

        impl PartialEq<[u8]> for $t {
            fn eq(&self, other: &[u8]) -> bool {
                self.as_bytes() == other
            }
        }

        impl PartialEq<&[u8]> for $t {
            fn eq(&self, other: &&[u8]) -> bool {
                self.as_bytes() == *other
            }
        }

        impl PartialEq<$t> for &[u8] {
            fn eq(&self, other: &$t) -> bool {
                *self == other.as_bytes()
            }
        }

        impl PartialEq<&$t> for &[u8] {
            fn eq(&self, other: &&$t) -> bool {
                *self == other.as_bytes()
            }
        }

        impl PartialEq<str> for $t {
            fn eq(&self, other: &str) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }
        impl PartialEq<&str> for $t {
            fn eq(&self, other: &&str) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }

        impl PartialEq<$t> for str {
            fn eq(&self, other: &$t) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }
    };
}

partial_eq!(FieldName);

///Represents field value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[debug("{:?}",self.as_str().unwrap_or(""))]
#[repr(transparent)]
pub struct FieldValue(Fref);

impl Default for FieldValue {
    fn default() -> Self {
        Self(Fref::Owned(Vec::new()))
    }
}

impl FieldValue {
    ///Returns a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    ///Returns a string slice.
    pub fn as_str(&self) -> Result<&str, &[u8]> {
        self.0.as_str()
    }

    ///Returns the number of byte.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    ///Returns true if no byte.
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    ///Creates with a `Vec<u8>`. Maybe from() is is better.
    pub fn owned(o: Vec<u8>) -> Self {
        Self(Fref::Owned(o))
    }
}

impl From<&'static [u8]> for FieldValue {
    fn from(o: &'static [u8]) -> Self {
        Self(Fref::Ref(o))
    }
}

impl From<Vec<u8>> for FieldValue {
    fn from(o: Vec<u8>) -> Self {
        to_field_value(&o).unwrap_or_else(|| Self(Fref::Owned(o)))
    }
}

impl From<String> for FieldValue {
    fn from(o: String) -> Self {
        o.into_bytes().into()
    }
}

impl From<&str> for FieldValue {
    fn from(o: &str) -> Self {
        let o = o.as_bytes();
        to_field_value(o).unwrap_or_else(|| Self(Fref::Owned(o.to_vec())))
    }
}

partial_eq!(FieldValue);

#[derive(Clone, Debug, Eq, Hash)]
#[repr(u8)]
enum Fref {
    Ref(&'static [u8]),
    Owned(Vec<u8>),
}

impl PartialEq for Fref {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Fref {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Ref(o) => o,
            Self::Owned(o) => o.as_slice(),
        }
    }

    fn as_str(&self) -> Result<&str, &[u8]> {
        let o = self.as_bytes();
        str::from_utf8(o).map_err(|_| o)
    }
}

///Represents bytes to string literal.
///
///A string literal is encoded as a sequence of octets, either by directly encoding the string literal’s octets or by using a Huffman code.
#[derive(Deref)]
pub struct OctetsRef<'a>(#[deref] &'a [u8], bool);

impl<'a> From<&'a [u8]> for OctetsRef<'a> {
    ///Creates, and sets the bytes will be huffman encoded.
    fn from(s: &'a [u8]) -> Self {
        Self(s, true)
    }
}

impl<'a> OctetsRef<'a> {
    ///Creates with whether or not the bytes will be huffman encoded.
    pub fn new(s: &'a [u8], o: bool) -> Self {
        Self(s, o)
    }

    ///Whether or not the bytes will be huffman encoded.
    pub fn huffman(&self) -> bool {
        self.1
    }

    ///Sets whether or not the bytes will be huffman encoded.
    pub fn set_huffman(&mut self, o: bool) {
        self.1 = o;
    }
}
