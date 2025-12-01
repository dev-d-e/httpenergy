/*!
A module for HTTP/2 format.

This module provides two types([`H2Request`] and [`H2Response`]) for working with request and response abstractly.

The basic protocol unit in HTTP/2 is a frame. Each frame type serves a different purpose.
This module contains functions for working with frames, field section compression and decompression,
and several assist types and traits for helping to use these functions.

# Examples
```
use httpenergy::h2::frame::FrameDecoder;
use httpenergy::h2::hpack::IndexingTables;
use httpenergy::h2::*;

let mut r = H2Request::with_method("POST");
r.scheme_mut().replace("https".to_string());
r.authority_mut().replace("example.org".to_string());
r.path_mut().replace("/resource".to_string());

let a = r.headers_mut();
a.add_field("content-type".to_string(), "image/jpeg".into());
a.add_field("host".to_string(), "example.org".into());
a.add_field("content-length".to_string(), "123".into());

//Converts request into HEADERS frame.
//You can use `H2EncodeFieldsHelper` or `HeadersEncoder` directly.
let p = r.pseudo_rep();
let h = r.headers_rep();

let mut stream_builder = H2StreamIdentifierBuilder::new();
let mut out = Vec::<Vec<u8>>::new();
let mut helper = H2EncodeFieldsHelper::new(&mut stream_builder, &mut out);
helper.pseudo_and_fields(p, h);
helper.flush();
let s = &out[0];

//Converts HEADERS frame into request.
//You can use `H2DecodeFieldsHelper` or `HeadersDecoder` directly.
let mut index = IndexingTables::new();
let mut req = H2Request::new();
match FrameDecoder::decode(s) {
    FrameDecoder::Headers(o) => {
        let mut helper = H2DecodeFieldsHelper::new(&mut index, &mut req);
        helper.headers(o);
    }
    _ => {}
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

use self::hpack::{FieldRep, IndexResult};
use crate::common::*;
use crate::Entity;
use crate::OctetsRef;
pub use assist::*;
use getset::{Getters, MutGetters};
use std::ops::{Deref, DerefMut};

///The ":method" pseudo-header field.
pub const PSEUDO_METHOD: &str = ":method";
///The ":scheme" pseudo-header field.
pub const PSEUDO_SCHEME: &str = ":scheme";
///The ":authority" pseudo-header field.
pub const PSEUDO_AUTHORITY: &str = ":authority";
///The ":path" pseudo-header field.
pub const PSEUDO_PATH: &str = ":path";
///The ":status" pseudo-header field.
pub const PSEUDO_STATUS: &str = ":status";

///Represents an HTTP/2 request.
#[derive(Getters, MutGetters)]
pub struct H2Request {
    #[getset(get = "pub", get_mut = "pub")]
    method: String,
    #[getset(get = "pub", get_mut = "pub")]
    scheme: Option<String>,
    #[getset(get = "pub", get_mut = "pub")]
    authority: Option<String>,
    #[getset(get = "pub", get_mut = "pub")]
    path: Option<String>,
    headers_body: Entity,
}

impl Deref for H2Request {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.headers_body
    }
}

impl DerefMut for H2Request {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers_body
    }
}

impl std::fmt::Debug for H2Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H2Request")
            .field("method", &self.method)
            .field("scheme", &self.scheme)
            .field("authority", &self.authority)
            .field("path", &self.path)
            .field("headers", self.headers_body.headers())
            .field("body len", &self.headers_body.body().len())
            .field("err", &self.headers_body.err())
            .finish()
    }
}

impl H2DistributeFields for H2Request {
    fn next_pseudo(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.set_pseudo(&vec_to_str(name), vec_to_str(value));
    }

    fn next_field(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.headers_mut().add_field(vec_to_str(name), value);
    }
}

impl H2Request {
    ///Creates.
    pub fn new() -> Self {
        Self {
            method: String::new(),
            scheme: None,
            authority: None,
            path: None,
            headers_body: Entity::new(),
        }
    }

    ///Creates.
    pub fn with_method(method: &str) -> Self {
        Self {
            method: method.to_string(),
            scheme: None,
            authority: None,
            path: None,
            headers_body: Entity::new(),
        }
    }

    ///Sets a pseudo-header field.
    pub fn set_pseudo(&mut self, name: &str, value: String) {
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
    pub fn indexed_method(&self) -> IndexResult<'_> {
        match self.method.as_str() {
            "GET" => IndexResult::Both(2),
            "POST" => IndexResult::Both(3),
            _ => IndexResult::One(2, self.method.as_bytes()),
        }
    }

    ///Returns a static table index value of ":scheme" or None if scheme is None.
    pub fn indexed_scheme(&self) -> IndexResult<'_> {
        if let Some(scheme) = &self.scheme {
            match scheme.as_str() {
                "http" => IndexResult::Both(6),
                "https" => IndexResult::Both(7),
                _ => IndexResult::One(6, scheme.as_bytes()),
            }
        } else {
            IndexResult::None
        }
    }

    ///Returns a static table index value of ":authority" or None if authority is None.
    pub fn indexed_authority(&self) -> IndexResult<'_> {
        if let Some(authority) = &self.authority {
            if authority.is_empty() {
                IndexResult::Both(1)
            } else {
                IndexResult::One(1, authority.as_bytes())
            }
        } else {
            IndexResult::None
        }
    }

    ///Returns a static table index value of ":path" or None if path is None.
    pub fn indexed_path(&self) -> IndexResult<'_> {
        if let Some(path) = &self.path {
            match path.as_str() {
                "/" => IndexResult::Both(4),
                "/index.html" => IndexResult::Both(5),
                _ => IndexResult::One(4, path.as_bytes()),
            }
        } else {
            IndexResult::None
        }
    }

    ///Converts fields to `FieldRep` vec.
    ///
    ///This function is used for test, maybe not meet your requirements.
    pub fn pseudo_rep(&self) -> Vec<FieldRep<'_>> {
        let mut vec = Vec::new();
        index_to_rep(self.indexed_method(), &mut vec);
        index_to_rep(self.indexed_scheme(), &mut vec);
        index_to_rep(self.indexed_authority(), &mut vec);
        index_to_rep(self.indexed_path(), &mut vec);
        vec
    }

    ///Converts fields to `FieldRep` vec.
    ///
    ///This function is used for test, maybe not meet your requirements.
    pub fn headers_rep(&self) -> Vec<FieldRep<'_>> {
        let mut vec = Vec::new();
        for (k, v) in self.headers().iter() {
            vec.push(FieldRep::IncrementalIndexingNewName(
                OctetsRef::new(k.as_bytes()),
                OctetsRef::new(v.one()),
            ));
        }
        vec
    }
}

#[inline]
fn index_to_rep<'a>(r: IndexResult<'a>, vec: &mut Vec<FieldRep<'a>>) {
    match r {
        IndexResult::Both(i) => {
            vec.push(FieldRep::Indexed(i));
        }
        IndexResult::One(i, o) => {
            vec.push(FieldRep::WithoutIndexingIndexedName(i, OctetsRef::new(o)));
        }
        IndexResult::None => {}
    }
}

///Represents an HTTP/2 response.
#[derive(Getters, MutGetters)]
pub struct H2Response {
    #[getset(get = "pub", get_mut = "pub")]
    status: String,
    headers_body: Entity,
}

impl Deref for H2Response {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.headers_body
    }
}

impl DerefMut for H2Response {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers_body
    }
}

impl std::fmt::Debug for H2Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H2Response")
            .field("status", &self.status)
            .field("headers", self.headers_body.headers())
            .field("body len", &self.headers_body.body().len())
            .field("err", &self.headers_body.err())
            .finish()
    }
}

impl H2DistributeFields for H2Response {
    fn next_pseudo(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.set_pseudo(&vec_to_str(name), vec_to_str(value));
    }

    fn next_field(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.headers_mut().add_field(vec_to_str(name), value);
    }
}

impl H2Response {
    ///Creates.
    pub fn new(status: &str) -> Self {
        Self {
            status: status.to_string(),
            headers_body: Entity::new(),
        }
    }

    ///Sets a pseudo-header field.
    pub fn set_pseudo(&mut self, name: &str, value: String) {
        match name {
            PSEUDO_STATUS => {
                self.status = value;
            }
            _ => {}
        }
    }

    ///Returns a static table index value of ":status".
    pub fn indexed_status(&self) -> IndexResult<'_> {
        match self.status.as_str() {
            "200" => IndexResult::Both(8),
            "204" => IndexResult::Both(9),
            "206" => IndexResult::Both(10),
            "304" => IndexResult::Both(11),
            "400" => IndexResult::Both(12),
            "404" => IndexResult::Both(13),
            "500" => IndexResult::Both(14),
            _ => IndexResult::One(8, self.status.as_bytes()),
        }
    }

    ///Converts fields to `FieldRep` vec.
    ///
    ///This function is used for test, maybe not meet your requirements.
    pub fn pseudo_rep(&self) -> Vec<FieldRep<'_>> {
        let mut vec = Vec::new();
        index_to_rep(self.indexed_status(), &mut vec);
        vec
    }

    ///Converts fields to `FieldRep` vec.
    ///
    ///This function is used for test, maybe not meet your requirements.
    pub fn headers_rep(&self) -> Vec<FieldRep<'_>> {
        let mut vec = Vec::new();
        for (k, v) in self.headers().iter() {
            vec.push(FieldRep::IncrementalIndexingNewName(
                OctetsRef::new(k.as_bytes()),
                OctetsRef::new(v.one()),
            ));
        }
        vec
    }
}
