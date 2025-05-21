use crate::common::*;
use crate::{Entity, WriteByte};
use getset::{Getters, MutGetters, Setters};
use std::ops::{Deref, DerefMut};

///Represents an HTTP/1.1 response.
#[derive(Getters, Setters)]
pub struct H1Response {
    #[getset(get = "pub", set = "pub")]
    version: String,
    #[getset(get = "pub", set = "pub")]
    status_code: String,
    #[getset(get = "pub", set = "pub")]
    reason: Vec<u8>,
    headers_body: Entity,
}

impl Deref for H1Response {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.headers_body
    }
}

impl DerefMut for H1Response {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers_body
    }
}

impl std::fmt::Debug for H1Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H1Response")
            .field("version", &self.version)
            .field("status_code", &self.status_code)
            .field("reason len", &self.reason.len())
            .field("headers", self.headers_body.headers())
            .field("body len", &self.headers_body.body().len())
            .field("err", &self.headers_body.err())
            .finish()
    }
}

impl H1Response {
    ///Creates.
    pub fn new() -> Self {
        Self {
            version: VERSION.to_string(),
            status_code: String::new(),
            reason: Vec::new(),
            headers_body: Entity::new(),
        }
    }

    ///Creates with status code.
    pub fn with_status_code(status_code: &str) -> H1Response {
        Self {
            version: VERSION.to_string(),
            status_code: status_code.to_string(),
            reason: Vec::new(),
            headers_body: Entity::new(),
        }
    }

    ///Exports an HTTP/1.1 message.
    pub fn export(&self, writer: &mut impl WriteByte) {
        writer.put_all(self.version.as_bytes());
        writer.put(SPACE);
        writer.put_all(self.status_code.as_bytes());
        writer.put(SPACE);
        writer.put_all(&self.reason);
        writer.put(CR);
        writer.put(LF);
        self.headers_body.export(writer);
    }
}

///Represents units of an HTTP/1.1 response.
pub struct H1ResponseUnits {
    ptr: *const u8,
    len: usize,
    build_context: BuildContext,
}

impl H1ResponseUnits {
    ///Creates with bytes.
    pub fn new(s: &[u8]) -> Self {
        let mut o = Self {
            ptr: s.as_ptr(),
            len: s.len(),
            build_context: BuildContext::new(),
        };
        o.build();
        o
    }

    units_header_body!();

    ///Returns a slice to version value.
    pub fn version(&self) -> &[u8] {
        &self.build_context.version_vec
    }

    ///Returns a slice to status-code.
    pub fn status_code(&self) -> &[u8] {
        &self.build_context.status_code_vec
    }

    ///Returns a slice to reason-phrase.
    pub fn reason(&self) -> &[u8] {
        &self.build_context.reason
    }

    ///Copies bytes from self to response.
    pub fn copy_to_response(mut self, response: &mut H1Response) {
        if !self.is_finish() {
            self.build();
        }

        if response.version.is_empty() {
            response.set_version(into_str(self.version()));
        }
        if response.status_code.is_empty() {
            response.set_status_code(into_str(self.status_code()));
        }
        if response.reason.is_empty() {
            response.set_reason(self.reason().to_vec());
        }

        let buf = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };

        let m = response.headers_body.headers_mut();
        //The header value does not include leading or trailing whitespace.
        for (a, b, c) in self.build_context.headers.drain(..) {
            let (b, c) = trim_whitespace(buf, b, c);
            if let Some(s) = buf.get(b..c) {
                m.add_field(vec_to_str(a), s.to_vec());
            }
        }

        if let Some(body) = self.body() {
            response.headers_body.body_mut().extend_from_slice(body);
        }

        response.headers_body.set_err(self.is_err());
    }
}

///Represents a response decoder. Hold response bytes.
#[derive(Getters, MutGetters)]
pub struct H1ResponseDecoder {
    buffer: Vec<u8>,
    #[getset(get = "pub", get_mut = "pub")]
    units: H1ResponseUnits,
    #[getset(get = "pub")]
    response: H1Response,
}

impl H1ResponseDecoder {
    ///Creates with bytes.
    pub fn new(buffer: Vec<u8>) -> Self {
        let units = H1ResponseUnits::new(&buffer);
        let mut o = Self {
            buffer,
            units,
            response: H1Response::new(),
        };
        o.units.set_slice(&o.buffer);
        o
    }

    ///Splits bytes from self to response.
    pub fn to_response(mut self) -> H1Response {
        if !self.units.is_finish() {
            self.units.build();
        }

        if self.response.version.is_empty() {
            self.response.set_version(into_str(self.units.version()));
        }
        if self.response.status_code.is_empty() {
            self.response
                .set_status_code(into_str(self.units.status_code()));
        }
        if self.response.reason.is_empty() {
            self.response.set_reason(self.units.reason().to_vec());
        }

        //The header value does not include leading or trailing whitespace.
        let headers = &mut self.units.build_context.headers;
        let mut n = 0;
        headers.iter().for_each(|h| {
            if n < h.2 {
                n = h.2;
            }
        });

        let body = self.units.position();
        if body > 0 && body >= n {
            *self.response.headers_body.body_mut() = self.buffer.split_off(body);
        }

        self.buffer.truncate(n);

        let headers = &mut self.units.build_context.headers;
        let m = self.response.headers_body.headers_mut();
        while let Some((a, b, c)) = headers.pop() {
            let (b, c) = trim_whitespace(&self.buffer, b, c);
            self.buffer.truncate(c);
            let v = self.buffer.split_off(b);
            m.add_field(vec_to_str(a), v);
        }

        self.response.headers_body.set_err(self.units.is_err());

        self.response
    }

    ///Copies bytes from self to response.
    pub fn copy_to_response(mut self) -> (H1Response, Vec<u8>) {
        if !self.units.is_finish() {
            self.units.build();
        }
        self.units.copy_to_response(&mut self.response);
        (self.response, self.buffer)
    }
}

struct BuildContext {
    current_function: fn(&mut BuildContext),
    post_separator_function: fn(&mut BuildContext),
    b: u8,
    n: usize,
    version_vec: Vec<u8>,
    status_code_vec: Vec<u8>,
    reason: Vec<u8>,
    header_name: Vec<u8>,
    header_value_index: usize,
    headers: Vec<(Vec<u8>, usize, usize)>,
    body: usize,
    search_header_name: Option<Vec<u8>>,
    suspend: bool,
    finish: bool,
    err: Vec<usize>,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            current_function: version_first,
            post_separator_function: version_first,
            b: 0,
            n: 0,
            version_vec: Vec::new(),
            status_code_vec: Vec::new(),
            reason: Vec::new(),
            header_name: Vec::new(),
            header_value_index: 0,
            headers: Vec::new(),
            body: 0,
            search_header_name: None,
            suspend: false,
            finish: false,
            err: Vec::new(),
        }
    }

    fn set_search(&mut self, name: Vec<u8>) {
        self.search_header_name.replace(name);
    }

    fn reset(&mut self) {
        self.search_header_name.take();
        if self.suspend {
            self.suspend = false;
        }
    }

    fn find_header(&mut self, k: &[u8]) -> Option<(usize, usize)> {
        self.headers.iter().find(|a| a.0 == k).map(|r| (r.1, r.2))
    }
}

parse_context!(BuildContext);

fn version_first(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphanumeric() {
        context.version_vec.push(b);
        context.current_function = version_tail;
    } else {
        context.err.push(context.n);
    }
}

fn version_tail(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphanumeric() || b == DOT || b == SLASH {
        context.version_vec.push(b);
    } else {
        context.post_separator_function = status_code_first;
        context.current_function = space;
        space(context);
    }
}

space_cr_lf!(BuildContext);

fn status_code_first(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_digit() {
        context.status_code_vec.push(b);
        context.current_function = status_code_tail;
    } else {
        context.err.push(context.n);
    }
}

fn status_code_tail(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_digit() {
        context.status_code_vec.push(b);
    } else {
        context.post_separator_function = reason_first;
        context.current_function = space;
        space(context);
    }
}

fn reason_first(context: &mut BuildContext) {
    let b = context.b;
    context.reason.push(b);
    context.current_function = reason_tail;
    if is_crlf(b) {
        reason_tail(context);
    }
}

fn reason_tail(context: &mut BuildContext) {
    let b = context.b;
    if is_crlf(b) {
        context.post_separator_function = header_name_first;
        context.current_function = cr;
        cr(context);
        context.suspend = true;
    } else {
        context.reason.push(b);
    }
}

parse_headers_body!(BuildContext);
