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

use crate::h2::hpack::IndexResult;
use crate::Entity;
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

///Represents an HTTP/3 request.
#[derive(Getters, MutGetters)]
pub struct H3Request {
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

impl Deref for H3Request {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.headers_body
    }
}

impl DerefMut for H3Request {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers_body
    }
}

impl std::fmt::Debug for H3Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H3Request")
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

impl H3Request {
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
            "CONNECT" => IndexResult::Both(15),
            "DELETE" => IndexResult::Both(16),
            "GET" => IndexResult::Both(17),
            "HEAD" => IndexResult::Both(18),
            "OPTIONS" => IndexResult::Both(19),
            "POST" => IndexResult::Both(20),
            "PUT" => IndexResult::Both(21),
            _ => IndexResult::One(15, self.method.as_bytes()),
        }
    }

    ///Returns a static table index value of ":scheme" or None if scheme is None.
    pub fn indexed_scheme(&self) -> IndexResult<'_> {
        if let Some(scheme) = &self.scheme {
            match scheme.as_str() {
                "http" => IndexResult::Both(22),
                "https" => IndexResult::Both(23),
                _ => IndexResult::One(22, scheme.as_bytes()),
            }
        } else {
            IndexResult::None
        }
    }

    ///Returns a static table index value of ":authority" or None if authority is None.
    pub fn indexed_authority(&self) -> IndexResult<'_> {
        if let Some(authority) = &self.authority {
            if authority.is_empty() {
                IndexResult::Both(0)
            } else {
                IndexResult::One(0, authority.as_bytes())
            }
        } else {
            IndexResult::None
        }
    }

    ///Returns a static table index value of ":path" or None if path is None.
    pub fn indexed_path(&self) -> IndexResult<'_> {
        if let Some(path) = &self.path {
            match path.as_str() {
                "/" => IndexResult::Both(1),
                _ => IndexResult::One(1, path.as_bytes()),
            }
        } else {
            IndexResult::None
        }
    }
}

///Represents an HTTP/3 response.
#[derive(Getters, MutGetters)]
pub struct H3Response {
    #[getset(get = "pub", get_mut = "pub")]
    status: String,
    headers_body: Entity,
}

impl Deref for H3Response {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.headers_body
    }
}

impl DerefMut for H3Response {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers_body
    }
}

impl std::fmt::Debug for H3Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H3Response")
            .field("status", &self.status)
            .field("headers", self.headers_body.headers())
            .field("body len", &self.headers_body.body().len())
            .field("err", &self.headers_body.err())
            .finish()
    }
}

impl H3Response {
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
            "103" => IndexResult::Both(24),
            "200" => IndexResult::Both(25),
            "304" => IndexResult::Both(26),
            "404" => IndexResult::Both(27),
            "503" => IndexResult::Both(28),
            _ => IndexResult::One(24, self.status.as_bytes()),
        }
    }
}
