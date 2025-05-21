use crate::common::*;
use crate::{Entity, WriteByte};
use getset::{Getters, MutGetters, Setters};
use std::ops::{Deref, DerefMut};

///Represents an HTTP/1.1 request.
#[derive(Getters, Setters)]
pub struct H1Request {
    #[getset(get = "pub", set = "pub")]
    method: String,
    #[getset(get = "pub", set = "pub")]
    target: String,
    #[getset(get = "pub", set = "pub")]
    version: String,
    headers_body: Entity,
}

impl Deref for H1Request {
    type Target = Entity;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.headers_body
    }
}

impl DerefMut for H1Request {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers_body
    }
}

impl std::fmt::Debug for H1Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("H1Request")
            .field("method", &self.method)
            .field("target", &self.target)
            .field("version", &self.version)
            .field("headers", self.headers_body.headers())
            .field("body len", &self.headers_body.body().len())
            .field("err", &self.headers_body.err())
            .finish()
    }
}

impl H1Request {
    ///Creates.
    pub fn new() -> Self {
        Self {
            method: String::new(),
            target: String::new(),
            version: VERSION.to_string(),
            headers_body: Entity::new(),
        }
    }

    ///Creates with method and target.
    pub fn with_method_target(method: &str, target: &str) -> Self {
        Self {
            method: method.to_string(),
            target: target.to_string(),
            version: VERSION.to_string(),
            headers_body: Entity::new(),
        }
    }

    ///Exports an HTTP/1.1 message.
    pub fn export(&self, writer: &mut impl WriteByte) {
        writer.put_all(self.method.as_bytes());
        writer.put(SPACE);
        writer.put_all(self.target.as_bytes());
        writer.put(SPACE);
        writer.put_all(self.version.as_bytes());
        writer.put(CR);
        writer.put(LF);
        self.headers_body.export(writer);
    }
}

macro_rules! units_header_body {
    () => {
        pub fn set_slice(&mut self, s: &[u8]) {
            self.ptr = s.as_ptr();
            let n = s.len();
            if n > self.len {
                self.len = n;
            }
        }

        fn build(&mut self) {
            if self.is_finish() {
                return;
            }
            let buf = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
            accept_context(&mut self.build_context, buf);
        }

        fn header_value_index(&mut self, name: &[u8]) -> Option<(usize, usize)> {
            self.build_context.find_header(name).or_else(|| {
                self.build_context.set_search(name.into());
                self.build();
                self.build_context.find_header(name)
            })
        }

        ///Returns a reference to header value.
        pub fn header_value(&mut self, name: &str) -> Option<&[u8]> {
            self.header_value_index(name.as_bytes())
                .map(|r| {
                    let buf = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
                    let r = trim_whitespace(buf, r.0, r.1);
                    buf.get(r.0..r.1)
                })
                .flatten()
        }

        ///Returns a header value `Vec`.
        pub fn header_value_vec(&mut self, name: &str) -> Vec<u8> {
            self.header_value(name)
                .map(|r| r.to_vec())
                .unwrap_or(Vec::new())
        }

        ///Returns a header value `String`.
        pub fn header_value_string(&mut self, name: &str) -> String {
            self.header_value(name)
                .map(|r| into_str(r))
                .unwrap_or(String::new())
        }

        ///Returns a index to body.
        pub fn position(&mut self) -> usize {
            if self.build_context.body == 0 {
                self.build();
            }
            self.build_context.body
        }

        ///Returns a reference to body.
        pub fn body(&mut self) -> Option<&[u8]> {
            let buf = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
            buf.get(self.position()..)
        }

        ///Returns true if the building is finished.
        pub fn is_finish(&self) -> bool {
            self.build_context.finish
        }

        ///Returns true if format is wrong.
        pub fn is_err(&self) -> bool {
            !self.build_context.err.is_empty()
        }
    };
}

///Represents units of an HTTP/1.1 request.
pub struct H1RequestUnits {
    ptr: *const u8,
    len: usize,
    build_context: BuildContext,
}

impl H1RequestUnits {
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

    ///Returns a slice to method value.
    pub fn method(&self) -> &[u8] {
        &self.build_context.method_vec
    }

    ///Returns a slice to target value.
    pub fn target(&self) -> &[u8] {
        &self.build_context.target_vec
    }

    ///Returns a slice to version value.
    pub fn version(&self) -> &[u8] {
        &self.build_context.version_vec
    }

    ///Copies bytes from self to request.
    pub fn copy_to_request(mut self, request: &mut H1Request) {
        if !self.is_finish() {
            self.build();
        }

        if request.method.is_empty() {
            request.set_method(into_str(self.method()));
        }
        if request.target.is_empty() {
            request.set_target(into_str(self.target()));
        }
        if request.version.is_empty() {
            request.set_version(into_str(self.version()));
        }

        let buf = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };

        let m = request.headers_body.headers_mut();
        //The header value does not include leading or trailing whitespace.
        for (a, b, c) in self.build_context.headers.drain(..) {
            let (b, c) = trim_whitespace(buf, b, c);
            if let Some(s) = buf.get(b..c) {
                m.add_field(vec_to_str(a), s.to_vec());
            }
        }

        if let Some(body) = self.body() {
            request.headers_body.body_mut().extend_from_slice(body);
        }

        request.headers_body.set_err(self.is_err());
    }
}

///Represents a request decoder. Hold request bytes.
#[derive(Getters, MutGetters)]
pub struct H1RequestDecoder {
    buffer: Vec<u8>,
    #[getset(get = "pub", get_mut = "pub")]
    units: H1RequestUnits,
    #[getset(get = "pub")]
    request: H1Request,
}

impl H1RequestDecoder {
    ///Creates with bytes.
    pub fn new(buffer: Vec<u8>) -> Self {
        let units = H1RequestUnits::new(&buffer);
        let mut o = Self {
            buffer,
            units,
            request: H1Request::new(),
        };
        o.units.set_slice(&o.buffer);
        o
    }

    ///Splits bytes from self to request.
    pub fn to_request(mut self) -> H1Request {
        if !self.units.is_finish() {
            self.units.build();
        }

        if self.request.method.is_empty() {
            self.request.set_method(into_str(self.units.method()));
        }
        if self.request.target.is_empty() {
            self.request.set_target(into_str(self.units.target()));
        }
        if self.request.version.is_empty() {
            self.request.set_version(into_str(self.units.version()));
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
            *self.request.headers_body.body_mut() = self.buffer.split_off(body);
        }

        self.buffer.truncate(n);

        let headers = &mut self.units.build_context.headers;
        let m = self.request.headers_body.headers_mut();
        while let Some((a, b, c)) = headers.pop() {
            let (b, c) = trim_whitespace(&self.buffer, b, c);
            self.buffer.truncate(c);
            let v = self.buffer.split_off(b);
            m.add_field(vec_to_str(a), v);
        }

        self.request.headers_body.set_err(self.units.is_err());

        self.request
    }

    ///Copies bytes from self to request.
    pub fn copy_to_request(mut self) -> (H1Request, Vec<u8>) {
        if !self.units.is_finish() {
            self.units.build();
        }
        self.units.copy_to_request(&mut self.request);
        (self.request, self.buffer)
    }
}

struct BuildContext {
    current_function: fn(&mut BuildContext),
    post_separator_function: fn(&mut BuildContext),
    b: u8,
    n: usize,
    method_vec: Vec<u8>,
    target_vec: Vec<u8>,
    version_vec: Vec<u8>,
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
            current_function: method_first,
            post_separator_function: method_first,
            b: 0,
            n: 0,
            method_vec: Vec::new(),
            target_vec: Vec::new(),
            version_vec: Vec::new(),
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

macro_rules! parse_context {
    ($context: ty) => {
        fn accept_context(context: &mut $context, buf: &[u8]) {
            let n0 = context.n;
            let n1 = buf.len();
            for i in n0..n1 {
                accept(context, buf[i]);
                if context.suspend {
                    break;
                }
            }
            context.reset();
        }

        fn accept(context: &mut $context, b: u8) {
            context.b = b;
            (context.current_function)(context);
            context.n += 1;
        }
    };
}

macro_rules! space_cr_lf {
    ($context: ty) => {
        fn space(context: &mut $context) {
            let b = context.b;
            if b == SPACE {
                context.current_function = context.post_separator_function;
            } else {
                context.err.push(context.n);
            }
        }

        fn cr(context: &mut $context) {
            let b = context.b;
            if b == CR {
                context.current_function = lf;
            } else {
                context.err.push(context.n);
            }
        }

        fn lf(context: &mut $context) {
            let b = context.b;
            if b == LF {
                context.current_function = context.post_separator_function;
            } else if b == CR {
                context.err.push(context.n);
            } else {
                context.current_function = cr;
                context.err.push(context.n);
            }
        }
    };
}

macro_rules! parse_headers_body {
    ($context: ty) => {
        fn header_name_first(context: &mut $context) {
            let b = context.b;
            if b.is_ascii_alphabetic() {
                context.header_name.push(b);
                context.current_function = header_name_tail;
            } else if b == CR {
                context.post_separator_function = body_first;
                context.current_function = cr;
                cr(context);
            } else {
                context.err.push(context.n);
            }
        }

        fn header_name_tail(context: &mut $context) {
            let b = context.b;
            if b.is_ascii_alphanumeric() || b == HYPHEN {
                context.header_name.push(b);
            } else {
                context.post_separator_function = header_value_first;
                context.current_function = colon;
                colon(context);
            }
        }

        fn colon(context: &mut $context) {
            let b = context.b;
            if b == COLON {
                context.current_function = context.post_separator_function;
            } else {
                context.err.push(context.n);
            }
        }

        fn header_value_first(context: &mut $context) {
            let b = context.b;
            context.header_value_index = context.n;
            context.current_function = header_value_tail;
            if is_crlf(b) {
                header_value_tail(context);
            }
        }

        fn header_value_tail(context: &mut $context) {
            let b = context.b;
            if is_crlf(b) {
                let name = std::mem::take(&mut context.header_name);
                if let Some(s) = &context.search_header_name {
                    if s == &name {
                        context.suspend = true;
                    }
                }
                let index = context.header_value_index;
                context.headers.push((name, index, context.n));
                context.post_separator_function = header_name_first;
                context.current_function = cr;
                cr(context);
            }
        }

        fn body_first(context: &mut $context) {
            context.body = context.n;
            context.finish = true;
            context.current_function = body_tail;
        }

        fn body_tail(_context: &mut $context) {}
    };
}

parse_context!(BuildContext);

fn method_first(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphabetic() {
        context.method_vec.push(b);
        context.current_function = method_tail;
    } else {
        context.err.push(context.n);
    }
}

fn method_tail(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphabetic() {
        context.method_vec.push(b);
    } else {
        context.post_separator_function = target_first;
        context.current_function = space;
        space(context);
    }
}

space_cr_lf!(BuildContext);

fn target_first(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_graphic() {
        context.target_vec.push(b);
        context.current_function = target_tail;
    } else {
        context.err.push(context.n);
    }
}

fn target_tail(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_graphic() {
        context.target_vec.push(b);
    } else {
        context.post_separator_function = version_first;
        context.current_function = space;
        space(context);
    }
}

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
        context.post_separator_function = header_name_first;
        context.current_function = cr;
        cr(context);
        context.suspend = true;
    }
}

parse_headers_body!(BuildContext);
