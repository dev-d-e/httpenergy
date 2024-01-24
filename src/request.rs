use crate::common::*;
use std::collections::HashMap;

///Parse bytes to `RequestUnits`.
pub fn new_request_units(bytes: &Vec<u8>) -> RequestUnits {
    let mut units = RequestUnits::new();
    units.build(bytes);
    units
}

///Parse bytes to `Request`.
pub fn to_request(bytes: Vec<u8>) -> Request {
    let mut request = Request::new(bytes);
    request.build();
    request
}

///Parse bytes to `Request`.
pub fn string_to_request(str: String) -> Request {
    to_request(str.into_bytes())
}

///Pack sth into `Request`.
pub fn pack_request(method: &str, target: &str) -> Request {
    let mut request = Request::new_pack();
    request.set_method(method);
    request.set_target(target);
    request.set_version(VERSION);
    request
}

macro_rules! units_header_body {
    () => {
        fn build(&mut self, buf: &[u8]) {
            let mut context = match self.build_context.take() {
                Some(context) => context,
                None => return,
            };

            accept_context(&mut context, buf);

            self.from_context(&mut context, buf);

            context.search_header_name = None;
            if context.suspend {
                context.suspend = false;
                if !context.finish {
                    self.build_context.replace(context);
                }
            }
        }

        fn header_value_index(&mut self, name: &str, buf: &[u8]) -> Option<(usize, usize)> {
            if !self.headers.contains_key(name) {
                if let Some(context) = &mut self.build_context {
                    context.search_header_name = Some(name.to_string());
                    self.build(buf);
                }
            }
            self.headers.get(name).copied()
        }

        ///Returns a reference to header value.
        pub fn header_value<'a>(&mut self, name: &str, buf: &'a [u8]) -> Option<&'a [u8]> {
            let (index0, index1) = self.header_value_index(name, buf)?;
            return Some(&buf[index0..index1]);
        }

        ///Returns a header value `Vec`.
        pub fn header_value_vec(&mut self, name: &str, buf: &[u8]) -> Vec<u8> {
            match self.header_value(name, buf) {
                Some(value) => Vec::from(value),
                None => Vec::new(),
            }
        }

        ///Returns a header value `String`.
        pub fn header_value_string(&mut self, name: &str, buf: &[u8]) -> String {
            match self.header_value(name, buf) {
                Some(value) => into_str(value),
                None => String::new(),
            }
        }

        ///Returns a index to body.
        pub fn body(&mut self, buf: &[u8]) -> usize {
            if self.body == 0 {
                self.build(buf);
            }
            self.body
        }

        ///Returns true if the building is finished.
        pub fn is_finish(&self) -> bool {
            self.build_context.is_none()
        }

        ///Returns true if format is wrong.
        pub fn is_err(&self) -> bool {
            self.err
        }
    };
}

///Represents units of an HTTP request.
#[derive(Debug)]
pub struct RequestUnits {
    method: String,
    target: String,
    version: String,
    headers: HashMap<String, (usize, usize)>,
    body: usize,
    err: bool,
    build_context: Option<BuildContext>,
}

impl RequestUnits {
    fn new() -> Self {
        RequestUnits {
            method: String::new(),
            target: String::new(),
            version: String::new(),
            headers: HashMap::new(),
            body: 0,
            err: false,
            build_context: Some(BuildContext::new()),
        }
    }

    fn from_context(&mut self, context: &mut BuildContext, buf: &[u8]) {
        if self.method.is_empty() {
            self.method = to_str(context.method_vec.drain(..));
        }
        if self.target.is_empty() {
            self.target = to_str(context.target_vec.drain(..));
        }
        if self.version.is_empty() {
            self.version = to_str(context.version_vec.drain(..));
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

    ///Returns a slice to method value.
    pub fn method(&mut self) -> &str {
        &self.method
    }

    ///Returns a slice to target value.
    pub fn target(&mut self) -> &str {
        &self.target
    }

    ///Returns a slice to version value.
    pub fn version(&mut self) -> &str {
        &self.version
    }

    units_header_body!();
}

macro_rules! header_body {
    () => {
        fn build(&mut self) {
            let mut context = match self.build_context.take() {
                Some(context) => context,
                None => return,
            };

            let buf = &self.bytes;
            accept_context(&mut context, buf);

            self.from_context(&mut context);

            context.search_header_name = None;
            if context.suspend {
                context.suspend = false;
                if !context.finish {
                    self.build_context.replace(context);
                }
            }
        }

        ///Set header field. If the headers did have this header name present, the value is updated.
        pub fn set_header(&mut self, name: String, value: Vec<u8>) {
            self.headers.insert(name, value);
        }

        ///Returns a reference to header value.
        pub fn header_value(&mut self, name: &str) -> Option<&Vec<u8>> {
            if !self.headers.contains_key(name) {
                if let Some(context) = &mut self.build_context {
                    context.search_header_name = Some(name.to_string());
                    self.build();
                }
            }
            self.headers.get(name)
        }

        ///Returns a header value `String`.
        pub fn header_value_string(&mut self, name: &str) -> String {
            match self.header_value(name) {
                Some(value) => into_str(value),
                None => String::new(),
            }
        }

        ///Set body.
        pub fn set_body(&mut self, body: Vec<u8>) {
            self.body = body;
        }

        ///Returns a reference to body.
        pub fn body(&mut self) -> &Vec<u8> {
            if self.body.is_empty() {
                self.build();
            }
            &self.body
        }

        ///Returns true if format is wrong.
        pub fn is_err(&self) -> bool {
            self.err
        }

        ///Pack headers and body.
        pub fn pack_headers_body(&mut self, vec: &mut Vec<u8>) {
            for (k, v) in self.headers.iter() {
                vec.extend_from_slice(k.as_bytes());
                vec.push(COLON);
                vec.extend_from_slice(v);
                vec.push(CR);
                vec.push(LF);
            }
            vec.push(CR);
            vec.push(LF);
            if self.body.len() > 0 {
                vec.extend_from_slice(&self.body);
            }
        }
    };
}

///Represents an HTTP request. Hold request bytes.
#[derive(Debug)]
pub struct Request {
    method: String,
    target: String,
    version: String,
    headers: HashMap<String, Vec<u8>>,
    body: Vec<u8>,
    err: bool,
    bytes: Vec<u8>,
    build_context: Option<BuildContext>,
}

impl Request {
    fn new(bytes: Vec<u8>) -> Self {
        Request {
            method: String::new(),
            target: String::new(),
            version: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            err: false,
            bytes,
            build_context: Some(BuildContext::new()),
        }
    }

    fn new_pack() -> Self {
        Request {
            method: String::new(),
            target: String::new(),
            version: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            err: false,
            bytes: Vec::new(),
            build_context: None,
        }
    }

    fn from_context(&mut self, context: &mut BuildContext) {
        if self.method.is_empty() {
            self.method = to_str(context.method_vec.drain(..));
        }
        if self.target.is_empty() {
            self.target = to_str(context.target_vec.drain(..));
        }
        if self.version.is_empty() {
            self.version = to_str(context.version_vec.drain(..));
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

    ///Set method value.
    pub fn set_method(&mut self, method: &str) {
        self.method.clear();
        self.method.push_str(method);
    }

    ///Returns a slice to method value.
    pub fn method(&mut self) -> &str {
        &self.method
    }

    ///Set target value.
    pub fn set_target(&mut self, target: &str) {
        self.target.clear();
        self.target.push_str(target);
    }

    ///Returns a slice to target value.
    pub fn target(&mut self) -> &str {
        &self.target
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

    ///Pack bytes.
    pub fn pack(&mut self) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.extend_from_slice(self.method.as_bytes());
        vec.push(SPACE);
        vec.extend_from_slice(self.target.as_bytes());
        vec.push(SPACE);
        vec.extend_from_slice(self.version.as_bytes());
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
    method_vec: Vec<u8>,
    target_vec: Vec<u8>,
    version_vec: Vec<u8>,
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
            err: false,
        }
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
                context.err = true;
            }
        }

        fn cr(context: &mut $context) {
            let b = context.b;
            if b == CR {
                context.current_function = lf;
            } else {
                context.err = true;
            }
        }

        fn lf(context: &mut $context) {
            let b = context.b;
            if b == LF {
                context.current_function = context.post_separator_function;
            } else if b == CR {
                context.err = true;
            } else {
                context.current_function = cr;
                context.err = true;
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
                context.err = true;
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
                context.err = true;
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
                let name = to_str(context.header_name.drain(..));
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
        context.err = true;
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
        context.err = true;
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
        context.err = true;
    }
}

fn version_tail(context: &mut BuildContext) {
    let b = context.b;
    if b.is_ascii_alphanumeric() || b == DOT {
        context.version_vec.push(b);
    } else {
        context.post_separator_function = header_name_first;
        context.current_function = cr;
        cr(context);
        context.suspend = true;
    }
}

parse_headers_body!(BuildContext);
