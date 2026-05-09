/*!
A module for HTTP/2 format.

This module provides two types([`H2Request`] and [`H2Response`]) for working with request and response abstractly.

The basic protocol unit in HTTP/2 is a frame. Each frame type serves a different purpose.
This module contains functions for working with frames, field section compression and decompression,
and several assist types and traits for helping to use these functions.

# Examples
```
use httpenergy::h2::frame::*;
use httpenergy::h2::hpack::*;
use httpenergy::h2::*;
use httpenergy::*;

let mut r = H2Request::new("POST");
r.set_scheme(Some("https"));
r.set_authority(Some("example.org"));
r.set_path(Some("/resource"));
r.add_field("content-type", "image/jpeg");
r.add_field("host", "example.org");
r.add_field("content-length", "123");

//Converts request into HEADERS frame.
//You can use `HeadersHelper` or `Headers` directly.
let mut helper = HeadersHelper::new(1, 100, 100);
handle_request_pseudo_header_fields(&r, &mut helper);
handle_fields(&r, &mut helper);
let mut s = Vec::new();
helper.export(&mut s);
assert!(s.len() > 0);

//Converts HEADERS frame into request.
let mut t = DynamicTable::default();
let mut req = H2Request::default();
let mut g = s.into_get();
if let Ok(rst) = get_frame(&mut g) {
  match rst {
    FrameResult::Headers(o) => {
        if let Some(mut f) = o.field_block_fragment(&mut g) {
            if let Ok(v) = get_hfris_to_vec(f.as_mut()) {
                let v = update_dynamic_table_to_vec(v, &mut t);
                add_fields_to_request(v, &mut req);
            }
        }
    }
    _ => {}
  }
}

assert_eq!(r.method(), req.method());
assert_eq!(r.scheme(), req.scheme());
assert_eq!(r.authority(), req.authority());
assert_eq!(r.path(), req.path());
```
*/

mod assist;
pub mod frame;
pub mod hpack;
pub(crate) mod huffman;
pub(crate) mod prty;

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

///Represents an HTTP/2 request.
#[derive(Debug, Default, Deref, DerefMut, Getters, MutGetters)]
pub struct H2Request {
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

impl H2Request {
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
        match self.method.as_bytes() {
            b"GET" => IndexRef::Both(2),
            b"POST" => IndexRef::Both(3),
            _ => IndexRef::One(2, self.method.as_bytes()),
        }
    }

    ///Returns a static table index value of ":scheme" or None.
    pub fn indexed_scheme(&self) -> Option<IndexRef<'_>> {
        self.scheme.as_ref().map(|scheme| match scheme.as_bytes() {
            b"http" => IndexRef::Both(6),
            b"https" => IndexRef::Both(7),
            _ => IndexRef::One(6, scheme.as_bytes()),
        })
    }

    ///Returns a static table index value of ":authority" or None.
    pub fn indexed_authority(&self) -> Option<IndexRef<'_>> {
        self.authority.as_ref().map(|authority| {
            if authority.is_empty() {
                IndexRef::Both(1)
            } else {
                IndexRef::One(1, authority.as_bytes())
            }
        })
    }

    ///Returns a static table index value of ":path" or None.
    pub fn indexed_path(&self) -> Option<IndexRef<'_>> {
        self.path.as_ref().map(|path| match path.as_bytes() {
            b"/" => IndexRef::Both(4),
            b"/index.html" => IndexRef::Both(5),
            _ => IndexRef::One(4, path.as_bytes()),
        })
    }
}

///Represents an HTTP/2 response.
#[derive(Debug, Default, Deref, DerefMut, Getters, MutGetters)]
pub struct H2Response {
    #[getset(get = "pub", get_mut = "pub")]
    status: FieldValue,
    #[deref]
    #[deref_mut]
    headers_body: Entity,
}

impl H2Response {
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
        match self.status.as_bytes() {
            b"200" => IndexRef::Both(8),
            b"204" => IndexRef::Both(9),
            b"206" => IndexRef::Both(10),
            b"304" => IndexRef::Both(11),
            b"400" => IndexRef::Both(12),
            b"404" => IndexRef::Both(13),
            b"500" => IndexRef::Both(14),
            _ => IndexRef::One(8, self.status.as_bytes()),
        }
    }
}

///Represents reference to an existing table entry.
#[repr(u8)]
pub enum IndexRef<'a> {
    ///Identifies an entry(name-value pair) in either the static table or the dynamic table.
    Both(usize),
    ///Identifies a name in either the static table or the dynamic table.
    One(usize, &'a [u8]),
}
