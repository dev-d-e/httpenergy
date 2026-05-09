/*!
A module for HTTP/3 format.

This module provides two types([`H3Request`] and [`H3Response`]) for working with request and response abstractly.

Within each stream, the basic unit of HTTP/3 communication is a frame. Each frame type serves a different purpose.
This module contains functions for working with frames, field section compression and decompression,
and several assist types and traits for helping to use these functions.

*/
mod assist;
pub mod frame;
mod prty;
pub mod qpack;

use crate::io::*;
use crate::prty::*;
pub use assist::*;
use derive_more::{Debug, Deref, DerefMut};

///The ":method" pseudo-header field.
pub const PSEUDO_METHOD: &[u8] = b":method";
///The ":scheme" pseudo-header field.
pub const PSEUDO_SCHEME: &[u8] = b":scheme";
///The ":authority" pseudo-header field.
pub const PSEUDO_AUTHORITY: &[u8] = b":authority";
///The ":path" pseudo-header field.
pub const PSEUDO_PATH: &[u8] = b":path";
///The ":status" pseudo-header field.
pub const PSEUDO_STATUS: &[u8] = b":status";

///Represents an HTTP/3 request.
#[derive(Debug, Default, Deref, DerefMut, Getters, MutGetters)]
pub struct H3Request {
    #[getset(get = "pub", get_mut = "pub")]
    method: FieldValue,
    #[getset(get = "pub", get_mut = "pub")]
    scheme: Option<FieldValue>,
    #[getset(get = "pub", get_mut = "pub")]
    authority: Option<FieldValue>,
    #[getset(get = "pub", get_mut = "pub")]
    path: Option<FieldValue>,
    #[deref]
    #[deref_mut]
    headers_body: Entity,
}

impl H3Request {
    ///Creates.
    pub fn new(method: impl Into<FieldValue>) -> Self {
        Self {
            method: method.into(),
            scheme: None,
            authority: None,
            path: None,
            headers_body: Default::default(),
        }
    }

    ///Sets scheme.
    pub fn set_scheme(&mut self, value: Option<impl Into<FieldValue>>) {
        self.scheme = value.map(|o| o.into());
    }

    ///Sets authority.
    pub fn set_authority(&mut self, value: Option<impl Into<FieldValue>>) {
        self.authority = value.map(|o| o.into());
    }

    ///Sets path.
    pub fn set_path(&mut self, value: Option<impl Into<FieldValue>>) {
        self.path = value.map(|o| o.into());
    }

    ///Sets a pseudo-header field.
    pub fn set_pseudo(&mut self, name: &[u8], value: impl Into<FieldValue>) {
        let value = value.into();
        match name {
            PSEUDO_METHOD => {
                self.method = value;
            }
            PSEUDO_SCHEME => {
                self.scheme.replace(value);
            }
            PSEUDO_AUTHORITY => {
                self.authority.replace(value);
            }
            PSEUDO_PATH => {
                self.path.replace(value);
            }
            _ => {}
        }
    }

    ///Returns a static table index value of ":method".
    pub fn indexed_method(&self) -> IndexRef<'_> {
        let o = self.method.as_bytes();
        match o {
            b"CONNECT" => IndexRef::StaticBoth(15),
            b"DELETE" => IndexRef::StaticBoth(16),
            b"GET" => IndexRef::StaticBoth(17),
            b"HEAD" => IndexRef::StaticBoth(18),
            b"OPTIONS" => IndexRef::StaticBoth(19),
            b"POST" => IndexRef::StaticBoth(20),
            b"PUT" => IndexRef::StaticBoth(21),
            _ => IndexRef::StaticOne(15, o),
        }
    }

    ///Returns a static table index value of ":scheme" or None.
    pub fn indexed_scheme(&self) -> Option<IndexRef<'_>> {
        self.scheme.as_ref().map(|scheme| {
            let o = scheme.as_bytes();
            match o {
                b"http" => IndexRef::StaticBoth(22),
                b"https" => IndexRef::StaticBoth(23),
                _ => IndexRef::StaticOne(22, o),
            }
        })
    }

    ///Returns a static table index value of ":authority" or None.
    pub fn indexed_authority(&self) -> Option<IndexRef<'_>> {
        self.authority.as_ref().map(|authority| {
            if authority.is_empty() {
                IndexRef::StaticBoth(0)
            } else {
                IndexRef::StaticOne(0, authority.as_bytes())
            }
        })
    }

    ///Returns a static table index value of ":path" or None.
    pub fn indexed_path(&self) -> Option<IndexRef<'_>> {
        self.path.as_ref().map(|path| {
            let o = path.as_bytes();
            match o {
                b"/" => IndexRef::StaticBoth(1),
                _ => IndexRef::StaticOne(1, o),
            }
        })
    }
}

///Represents an HTTP/3 response.
#[derive(Debug, Default, Deref, DerefMut, Getters, MutGetters)]
pub struct H3Response {
    #[getset(get = "pub", get_mut = "pub")]
    status: FieldValue,
    #[deref]
    #[deref_mut]
    headers_body: Entity,
}

impl H3Response {
    ///Creates.
    pub fn new(status: impl Into<FieldValue>) -> Self {
        Self {
            status: status.into(),
            headers_body: Default::default(),
        }
    }

    ///Sets a pseudo-header field.
    pub fn set_pseudo(&mut self, name: &[u8], value: impl Into<FieldValue>) {
        match name {
            PSEUDO_STATUS => {
                self.status = value.into();
            }
            _ => {}
        }
    }

    ///Returns a static table index value of ":status".
    pub fn indexed_status(&self) -> IndexRef<'_> {
        let o = self.status.as_bytes();
        match o {
            b"103" => IndexRef::StaticBoth(24),
            b"200" => IndexRef::StaticBoth(25),
            b"304" => IndexRef::StaticBoth(26),
            b"404" => IndexRef::StaticBoth(27),
            b"503" => IndexRef::StaticBoth(28),
            _ => IndexRef::StaticOne(24, o),
        }
    }
}

///Represents reference to an existing table entry.
#[repr(u8)]
pub enum IndexRef<'a> {
    ///Identifies an entry(name-value pair) in the static table.
    StaticBoth(usize),
    ///Identifies a name in the static table.
    StaticOne(usize, &'a [u8]),
    ///Identifies an entry(name-value pair) in the dynamic table.
    DynamicBoth(usize),
    ///Identifies a name in the dynamic table.
    DynamicOne(usize, &'a [u8]),
}
