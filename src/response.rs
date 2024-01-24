use crate::common::*;
use std::collections::HashMap;

///Parse bytes to `ResponseUnits`.
pub fn new_response_units(bytes: &Vec<u8>) -> ResponseUnits {
    let mut units = ResponseUnits::new();
    units.build(bytes);
    units
}

///Parse bytes to `Response`.
pub fn to_response(bytes: Vec<u8>) -> Response {
    let mut response = Response::new(bytes);
    response.build();
    response
}

///Parse bytes to `Response`.
pub fn string_to_response(str: String) -> Response {
    to_response(str.into_bytes())
}

///Pack sth into `Response`.
pub fn pack_response(status_code: &str) -> Response {
    let mut response = Response::new_pack();
    response.set_status_code(status_code);
    response.set_version(VERSION);
    response
}

///Represents units of an HTTP response.
#[derive(Debug)]
pub struct ResponseUnits {
    version: String,
    status_code: String,
    reason: Vec<u8>,
    headers: HashMap<String, (usize, usize)>,
    body: usize,
    err: bool,
    build_context: Option<BuildContext>,
}

impl ResponseUnits {
    fn new() -> Self {
        ResponseUnits {
            version: String::new(),
            status_code: String::new(),
            reason: Vec::new(),
            headers: HashMap::new(),
            body: 0,
            err: false,
            build_context: Some(BuildContext::new()),
        }
    }

    fn from_context(&mut self, context: &mut BuildContext, buf: &[u8]) {
        if self.version.is_empty() {
            self.version = to_str(context.version_vec.drain(..));
        }
        if self.status_code.is_empty() {
            self.status_code = to_str(context.status_code_vec.drain(..));
        }

        //The header value does not include leading or trailing whitespace.
        let headers = &mut context.headers;
        for h in headers.drain(..) {
            let v = trim_whitespace(buf, h.1, h.2);
            self.headers.insert(h.0, v);
        }

        self.body = context.body;
        self.err = context.err;
    }

    ///Returns a slice to version value.
    pub fn version(&mut self) -> &str {
        &self.version
    }

    ///Returns a slice to status-code.
    pub fn status_code(&mut self) -> &str {
        &self.status_code
    }

    ///Returns a slice to reason-phrase.
    pub fn reason(&mut self) -> &Vec<u8> {
        &self.reason
    }

    units_header_body!();
}

///Represents an HTTP response. Hold response bytes.
#[derive(Debug)]
pub struct Response {
    version: String,
    status_code: String,
    reason: Vec<u8>,
    headers: HashMap<String, Vec<u8>>,
    body: Vec<u8>,
    err: bool,
    bytes: Vec<u8>,
    build_context: Option<BuildContext>,
}

impl Response {
    fn new(bytes: Vec<u8>) -> Self {
        Response {
            version: String::new(),
            status_code: String::new(),
            reason: Vec::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            err: false,
            bytes,
            build_context: Some(BuildContext::new()),
        }
    }

    fn new_pack() -> Self {
        Response {
            version: String::new(),
            status_code: String::new(),
            reason: Vec::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            err: false,
            bytes: Vec::new(),
            build_context: None,
        }
    }

    fn from_context(&mut self, context: &mut BuildContext) {
        if self.version.is_empty() {
            self.version = to_str(context.version_vec.drain(..));
        }
        if self.status_code.is_empty() {
            self.status_code = to_str(context.status_code_vec.drain(..));
        }
        if self.reason.is_empty() {
            self.reason = context.reason.drain(..).collect();
        }

        //The header value does not include leading or trailing whitespace.
        let headers = &mut context.headers;
        let mut n = 0;
        for h in headers.iter_mut() {
            if n < h.2 {
                n = h.2;
            }
            (h.1, h.2) = trim_whitespace(&self.bytes, h.1, h.2);
        }

        if n > 0 {
            let mut vec: Vec<u8> = self.bytes.drain(..n).collect();
            while let Some(h) = headers.pop() {
                let v = vec.split_off(h.2);
                drop(v);
                let v = vec.split_off(h.1);
                self.headers.insert(h.0, v);
            }
            drop(vec);
        }

        if context.body > 0 && context.body >= n {
            n = context.body - n;
            self.body = self.bytes.split_off(n);
            self.bytes.clear();
        }

        self.err = context.err;
    }

    ///Set version value.
    pub fn set_version(&mut self, version: &str) {
        self.version.clear();
        self.version.push_str(version);
    }

    ///Returns a slice to version value.
    pub fn version(&mut self) -> &str {
        &self.version
    }

    ///Set status-code.
    pub fn set_status_code(&mut self, status_code: &str) {
        self.status_code.clear();
        self.status_code.push_str(status_code);
    }

    ///Returns a slice to status-code.
    pub fn status_code(&mut self) -> &str {
        &self.status_code
    }

    ///Set reason-phrase.
    pub fn set_reason(&mut self, reason: &[u8]) {
        self.reason.clear();
        self.reason.extend_from_slice(reason);
    }

    ///Returns a slice to reason-phrase.
    pub fn reason(&mut self) -> &Vec<u8> {
        &self.reason
    }

    ///Pack bytes.
    pub fn pack(&mut self) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.extend_from_slice(self.version.as_bytes());
        vec.push(SPACE);
        vec.extend_from_slice(self.status_code.as_bytes());
        vec.push(SPACE);
        vec.extend_from_slice(&self.reason);
        vec.push(CR);
        vec.push(LF);
        self.pack_headers_body(&mut vec);
        vec
    }

    header_body!();
}

#[derive(Debug)]
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
    headers: Vec<(String, usize, usize)>,
    body: usize,
    search_header_name: Option<String>,
    suspend: bool,
    finish: bool,
    err: bool,
}

impl BuildContext {
    fn new() -> Self {
        BuildContext {
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
            err: false,
        }
    }
}

parse_context!(BuildContext);

fn version_first(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphanumeric() {
        context.version_vec.push(b);
        context.current_function = version_tail;
    } else {
        context.err = true;
    }
}

fn version_tail(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphanumeric() || b == DOT {
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
        context.err = true;
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
