macro_rules! units_header_body {
    () => {
        //The header value does not include leading or trailing whitespace.
        fn header_value_index(&mut self, name: &str) -> Option<(usize, usize)> {
            match self.headers.get(name) {
                Some(index) => Some(*index),
                None => {
                    let s = unsafe { self.origin.as_ref()? };
                    let fi = &mut self._headers_index;
                    while let Some(n) = fi.pop() {
                        let header_name = slice_index_into_str(s, n.0, n.1);
                        let (index0, index1) = trim_whitespace(s, n.2, n.3);
                        if header_name == name {
                            self.headers.insert(header_name, (index0, index1));
                            return Some((index0, index1));
                        }
                        self.headers.insert(header_name, (index0, index1));
                    }
                    None
                }
            }
        }

        ///Returns a reference to header value.
        pub fn header_value(&mut self, name: &str) -> Option<&[u8]> {
            let (index0, index1) = self.header_value_index(name)?;
            let s = unsafe { self.origin.as_ref()? };
            return Some(&s[index0..index1]);
        }

        ///Returns a header value `Vec`.
        pub fn header_value_vec(&mut self, name: &str) -> Vec<u8> {
            match self.header_value(name) {
                Some(s) => Vec::from(s),
                None => Vec::new(),
            }
        }

        ///Returns a header value `String`.
        pub fn header_value_string(&mut self, name: &str) -> String {
            match self.header_value(name) {
                Some(s) => into_str(s),
                None => String::new(),
            }
        }

        ///Returns a reference to body.
        pub fn body(&mut self) -> Option<&[u8]> {
            let index = self.body;
            if index > 0 {
                let s = self.origin();
                if s.len() > index {
                    return Some(&s[index..]);
                }
            }
            None
        }

        ///Returns true if format is wrong.
        pub fn is_err(&self) -> bool {
            self.err
        }
    };
}

macro_rules! space_cr_lf {
    () => {
        fn space(&mut self) {
            let b = self.b;
            if b == SPACE {
                self.current_function = self.post_separator_function;
            } else {
                self.err = true;
            }
        }

        fn cr(&mut self) {
            let b = self.b;
            if b == CR {
                self.current_function = Self::lf;
            } else {
                self.err = true;
            }
        }

        fn lf(&mut self) {
            let b = self.b;
            if b == LF {
                self.current_function = self.post_separator_function;
            } else if b == CR {
                self.err = true;
            } else {
                self.current_function = Self::cr;
                self.err = true;
            }
        }
    };
}

macro_rules! parse_headers_body {
    ($units:ident) => {
        fn header_name_first(&mut self) {
            let b = self.b;
            if b.is_ascii_alphabetic() {
                self.header_index.0 = self.n;
                self.current_function = Self::header_name_tail;
            } else if b == CR {
                self.post_separator_function = Self::body_first;
                self.current_function = Self::cr;
                self.cr();
            } else {
                self.err = true;
            }
        }

        fn header_name_tail(&mut self) {
            let b = self.b;
            if b.is_ascii_alphanumeric() || b == HYPHEN {
            } else {
                self.header_index.1 = self.n;
                self.post_separator_function = Self::header_value_first;
                self.current_function = Self::colon;
                self.colon();
            }
        }

        fn colon(&mut self) {
            let b = self.b;
            if b == COLON {
                self.current_function = self.post_separator_function;
            } else {
                self.err = true;
            }
        }

        fn header_value_first(&mut self) {
            let b = self.b;
            self.header_index.2 = self.n;
            self.current_function = Self::header_value_tail;
            if is_crlf(b) {
                self.header_value_tail();
            }
        }

        fn header_value_tail(&mut self) {
            let b = self.b;
            if is_crlf(b) {
                let index0 = self.header_index.0;
                let index1 = self.header_index.1;
                let index2 = self.header_index.2;
                let index3 = self.n;
                self.$units().header_index(index0, index1, index2, index3);
                self.post_separator_function = Self::header_name_first;
                self.current_function = Self::cr;
                self.cr();
            }
        }

        fn body_first(&mut self) {
            self.$units().body = self.n;
            self.current_function = Self::body_tail;
        }

        fn body_tail(&mut self) {}
    };
}

use crate::common::*;
use std::collections::HashMap;
use std::ptr;

///Represents units of an HTTP request.
#[derive(Debug)]
pub struct RequestUnits {
    origin: *const [u8],
    method: String,
    _method_index: (usize, usize),
    target: String,
    _target_index: (usize, usize),
    version: String,
    _version_index: (usize, usize),
    headers: HashMap<String, (usize, usize)>,
    _headers_index: Vec<(usize, usize, usize, usize)>,
    body: usize,
    err: bool,
}

impl RequestUnits {
    fn new(origin: &[u8]) -> Self {
        RequestUnits {
            origin,
            method: String::new(),
            _method_index: (0, 0),
            target: String::new(),
            _target_index: (0, 0),
            version: String::new(),
            _version_index: (0, 0),
            headers: HashMap::new(),
            _headers_index: Vec::new(),
            body: 0,
            err: false,
        }
    }

    fn origin(&mut self) -> &[u8] {
        unsafe {
            match self.origin.as_ref() {
                Some(o) => o,
                None => {
                    self.origin = b"";
                    &*self.origin
                }
            }
        }
    }

    fn header_index(&mut self, index0: usize, index1: usize, index2: usize, index3: usize) {
        self._headers_index.push((index0, index1, index2, index3));
    }

    ///Returns a slice to method value.
    pub fn method(&mut self) -> &str {
        if self.method.is_empty() {
            let index = self._method_index;
            let s = self.origin();
            self.method = slice_index_into_str(s, index.0, index.1);
        }
        &self.method
    }

    ///Returns a slice to target value.
    pub fn target(&mut self) -> &str {
        if self.target.is_empty() {
            let index = self._target_index;
            let s = self.origin();
            self.target = slice_index_into_str(s, index.0, index.1);
        }
        &self.target
    }

    ///Returns a slice to version value.
    pub fn version(&mut self) -> &str {
        if self.version.is_empty() {
            let index = self._version_index;
            let s = self.origin();
            self.version = slice_index_into_str(s, index.0, index.1);
        }
        &self.version
    }

    units_header_body!();
}

///Hold an HTTP request bytes and `RequestUnits`.
#[derive(Debug)]
pub struct Request(RequestUnits, Vec<u8>);

impl Request {
    fn new(bytes: Vec<u8>) -> Self {
        Request(RequestUnits::new(&bytes), bytes)
    }

    ///Returns a mutable reference to `RequestUnits`.
    pub fn units(&mut self) -> &mut RequestUnits {
        &mut self.0
    }

    ///Returns a reference to bytes.
    pub fn as_bytes(&self) -> &Vec<u8> {
        &self.1
    }

    ///Returns bytes. This consumes the `Request`.
    pub fn into_bytes(self) -> Vec<u8> {
        self.1
    }
}

///Builder can parse request bytes to `RequestUnits` or `Request`.
pub struct RequestBuilder {
    parser: Parser,
}

impl RequestBuilder {
    ///Creates a new `RequestBuilder`.
    pub fn new() -> Self {
        RequestBuilder {
            parser: Parser::new(),
        }
    }

    ///Parse bytes to `RequestUnits`.
    pub fn from_bytes(&mut self, bytes: &[u8]) -> RequestUnits {
        let mut units = RequestUnits::new(bytes);
        self.parser.accept_units(&mut units);
        units.err = self.parser.err;
        units
    }

    ///Parse bytes to `Request`.
    pub fn from_vec(&mut self, bytes: Vec<u8>) -> Request {
        let mut r = Request::new(bytes);
        self.parser.accept_units(&mut r.0);
        r
    }

    ///Parse bytes to `Request`.
    pub fn from_string(&mut self, str: String) -> Request {
        self.from_vec(str.into_bytes())
    }
}

struct Parser {
    current_function: fn(&mut Parser),
    post_separator_function: fn(&mut Parser),
    b: u8,
    n: usize,
    header_index: (usize, usize, usize),
    units: *mut RequestUnits,
    err: bool,
}

impl Parser {
    fn new() -> Self {
        Parser {
            current_function: Self::method_first,
            post_separator_function: Self::method_first,
            b: 0,
            n: 0,
            header_index: (0, 0, 0),
            units: ptr::null_mut(),
            err: false,
        }
    }

    fn accept_units(&mut self, units: &mut RequestUnits) {
        self.units = units;
        let o = units.origin();
        for i in o {
            self.accept(*i);
        }
    }

    fn accept(&mut self, b: u8) {
        self.b = b;
        (self.current_function)(self);
        self.n += 1;
    }

    fn units(&mut self) -> &mut RequestUnits {
        unsafe {
            match self.units.as_mut() {
                Some(r) => r,
                None => {
                    self.units = &mut RequestUnits::new(b"");
                    &mut *self.units
                }
            }
        }
    }

    fn method_first(&mut self) {
        let b = self.b;
        if b.is_ascii_alphabetic() {
            self.units()._method_index.0 = self.n;
            self.current_function = Self::method_tail;
        } else {
            self.err = true;
        }
    }

    fn method_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_alphabetic() {
        } else {
            self.units()._method_index.1 = self.n;
            self.post_separator_function = Self::target_first;
            self.current_function = Self::space;
            self.space();
        }
    }

    space_cr_lf!();

    fn target_first(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
            self.units()._target_index.0 = self.n;
            self.current_function = Self::target_tail;
        } else {
            self.err = true;
        }
    }

    fn target_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
        } else {
            self.units()._target_index.1 = self.n;
            self.post_separator_function = Self::version_first;
            self.current_function = Self::space;
            self.space();
        }
    }

    fn version_first(&mut self) {
        let b = self.b;
        if b.is_ascii_alphanumeric() {
            self.units()._version_index.0 = self.n;
            self.current_function = Self::version_tail;
        } else {
            self.err = true;
        }
    }

    fn version_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_alphanumeric() || b == DOT {
        } else {
            self.units()._version_index.1 = self.n;
            self.post_separator_function = Self::header_name_first;
            self.current_function = Self::cr;
            self.cr();
        }
    }

    parse_headers_body!(units);
}
