use crate::common::*;
use std::collections::HashMap;
use std::ptr;

///Represents units of an HTTP response.
#[derive(Debug)]
pub struct ResponseUnits {
    origin: *const [u8],
    version: String,
    _version_index: (usize, usize),
    status_code: String,
    _status_code_index: (usize, usize),
    reason: String,
    _reason_index: (usize, usize),
    headers: HashMap<String, (usize, usize)>,
    _headers_index: Vec<(usize, usize, usize, usize)>,
    body: usize,
    err: bool,
}

impl ResponseUnits {
    fn new(origin: &[u8]) -> Self {
        ResponseUnits {
            origin,
            version: String::new(),
            _version_index: (0, 0),
            status_code: String::new(),
            _status_code_index: (0, 0),
            reason: String::new(),
            _reason_index: (0, 0),
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

    ///Returns a slice to version value.
    pub fn version(&mut self) -> &str {
        if self.version.is_empty() {
            let index = self._version_index;
            let s = self.origin();
            self.version = slice_index_into_str(s, index.0, index.1);
        }
        &self.version
    }

    ///Returns a slice to status-code.
    pub fn status_code(&mut self) -> &str {
        if self.status_code.is_empty() {
            let index = self._status_code_index;
            let s = self.origin();
            self.status_code = slice_index_into_str(s, index.0, index.1);
        }
        &self.status_code
    }

    ///Returns a slice to reason-phrase.
    pub fn reason(&mut self) -> &str {
        if self.reason.is_empty() {
            let index = self._reason_index;
            let s = self.origin();
            self.reason = slice_index_into_str(s, index.0, index.1);
        }
        &self.reason
    }

    units_header_body!();
}

///Hold an HTTP response bytes and `ResponseUnits`.
#[derive(Debug)]
pub struct Response(ResponseUnits, Vec<u8>);

impl Response {
    fn new(bytes: Vec<u8>) -> Self {
        Response(ResponseUnits::new(&bytes), bytes)
    }

    ///Returns a mutable reference to `ResponseUnits`.
    pub fn units(&mut self) -> &mut ResponseUnits {
        &mut self.0
    }

    ///Returns a reference to bytes.
    pub fn as_bytes(&self) -> &Vec<u8> {
        &self.1
    }

    ///Returns bytes. This consumes the `Response`.
    pub fn into_bytes(self) -> Vec<u8> {
        self.1
    }
}

///Builder can parse response bytes to `ResponseUnits` or `Response`.
pub struct ResponseBuilder {
    parser: Parser,
}

impl ResponseBuilder {
    ///Creates a new `ResponseBuilder`.
    pub fn new() -> Self {
        ResponseBuilder {
            parser: Parser::new(),
        }
    }

    ///Parse bytes to `ResponseUnits`.
    pub fn from_bytes(&mut self, bytes: &[u8]) -> ResponseUnits {
        let mut units = ResponseUnits::new(bytes);
        self.parser.accept_units(&mut units);
        units.err = self.parser.err;
        units
    }

    ///Parse bytes to `Response`.
    pub fn from_vec(&mut self, bytes: Vec<u8>) -> Response {
        let mut r = Response::new(bytes);
        self.parser.accept_units(&mut r.0);
        r
    }

    ///Parse bytes to `Response`.
    pub fn from_string(&mut self, str: String) -> Response {
        self.from_vec(str.into_bytes())
    }
}

struct Parser {
    current_function: fn(&mut Parser),
    post_separator_function: fn(&mut Parser),
    b: u8,
    n: usize,
    header_index: (usize, usize, usize),
    units: *mut ResponseUnits,
    err: bool,
}

impl Parser {
    fn new() -> Self {
        Parser {
            current_function: Self::version_first,
            post_separator_function: Self::version_first,
            b: 0,
            n: 0,
            header_index: (0, 0, 0),
            units: ptr::null_mut(),
            err: false,
        }
    }

    fn accept_units(&mut self, units: &mut ResponseUnits) {
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

    fn units(&mut self) -> &mut ResponseUnits {
        unsafe {
            match self.units.as_mut() {
                Some(r) => r,
                None => {
                    self.units = &mut ResponseUnits::new(b"");
                    &mut *self.units
                }
            }
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
            self.post_separator_function = Self::status_code_first;
            self.current_function = Self::space;
            self.space();
        }
    }

    space_cr_lf!();

    fn status_code_first(&mut self) {
        let b = self.b;
        if b.is_ascii_digit() {
            self.units()._status_code_index.0 = self.n;
            self.current_function = Self::status_code_tail;
        } else {
            self.err = true;
        }
    }

    fn status_code_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_digit() {
        } else {
            self.units()._status_code_index.1 = self.n;
            self.post_separator_function = Self::reason_first;
            self.current_function = Self::space;
            self.space();
        }
    }

    fn reason_first(&mut self) {
        let b = self.b;
        self.units()._reason_index.0 = self.n;
        self.current_function = Self::reason_tail;
        if is_crlf(b) {
            self.reason_tail();
        }
    }

    fn reason_tail(&mut self) {
        let b = self.b;
        if is_crlf(b) {
            self.units()._reason_index.1 = self.n;
            self.post_separator_function = Self::header_name_first;
            self.current_function = Self::cr;
            self.cr();
        }
    }

    parse_headers_body!(units);
}
