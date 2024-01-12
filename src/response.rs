use crate::slice_index_into_str;
use std::collections::HashMap;
use std::ptr;

///Represents an HTTP response.
#[derive(Debug)]
pub struct Response {
    origin: *const [u8],
    version: String,
    _version_index: (usize, usize),
    status_code: String,
    _status_code_index: (usize, usize),
    reason: String,
    _reason_index: (usize, usize),
    fields: HashMap<String, Vec<u8>>,
    _fields_index: Vec<(usize, usize, usize, usize)>,
    body: usize,
}

impl Response {
    fn new(origin: &[u8]) -> Self {
        Response {
            origin,
            version: String::new(),
            _version_index: (0, 0),
            status_code: String::new(),
            _status_code_index: (0, 0),
            reason: String::new(),
            _reason_index: (0, 0),
            fields: HashMap::new(),
            _fields_index: Vec::new(),
            body: 0,
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

    fn field_index(&mut self, index: (usize, usize, usize, usize)) {
        self._fields_index.push(index);
    }

    pub fn version(&mut self) -> &String {
        if self.version.is_empty() {
            let index = self._version_index;
            let s = self.origin();
            self.version = slice_index_into_str(s, index);
        }
        &self.version
    }

    pub fn status_code(&mut self) -> &String {
        if self.status_code.is_empty() {
            let index = self._status_code_index;
            let s = self.origin();
            self.status_code = slice_index_into_str(s, index);
        }
        &self.status_code
    }

    pub fn reason(&mut self) -> &String {
        if self.reason.is_empty() {
            let index = self._reason_index;
            let s = self.origin();
            self.reason = slice_index_into_str(s, index);
        }
        &self.reason
    }

    pub fn field(&mut self, k: &str) -> Option<&Vec<u8>> {
        if !self.fields.contains_key(k) {
            let s = unsafe { self.origin.as_ref()? };
            let fi = &mut self._fields_index;
            let f = &mut self.fields;
            while let Some(n) = fi.pop() {
                let t = slice_index_into_str(s, (n.0, n.1));
                let mut v = Vec::new();
                v.extend_from_slice(&s[n.2..n.3]);
                if t == k {
                    f.insert(t, v);
                    break;
                } else {
                    f.insert(t, v);
                }
            }
        }
        return self.fields.get(k);
    }
}

///Builder can parse response bytes to `Response`.
pub struct ResponseBuilder {
    parser: Parser,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            parser: Parser::new(),
        }
    }

    pub fn from_bytes(&mut self, bytes: &[u8]) -> Response {
        let mut response = Response::new(bytes);
        self.parser.accept_response(&mut response);
        response
    }
}

struct Parser {
    current_function: fn(&mut Parser),
    b: u8,
    n: usize,
    field_index: (usize, usize, usize),
    response: *mut Response,
}

impl Parser {
    fn new() -> Self {
        Parser {
            current_function: Self::version_first,
            b: 0,
            n: 0,
            field_index: (0, 0, 0),
            response: ptr::null_mut(),
        }
    }

    fn accept_response(&mut self, response: &mut Response) {
        self.response = response;
        let o = response.origin();
        for i in o {
            self.accept(*i);
        }
    }

    fn accept(&mut self, b: u8) {
        self.b = b;
        (self.current_function)(self);
        self.n += 1;
    }

    fn response(&mut self) -> &mut Response {
        unsafe {
            match self.response.as_mut() {
                Some(r) => r,
                None => {
                    self.response = &mut Response::new(b"");
                    &mut *self.response
                }
            }
        }
    }

    fn version_first(&mut self) {
        let b = self.b;
        if b == b'\r' || b == b'\n' {
        } else {
            if b.is_ascii_alphanumeric() {
                self.response()._version_index.0 = self.n;
                self.current_function = Self::version_tail;
            } else {
            }
        }
    }

    fn version_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_alphanumeric() || b == b'.' {
        } else {
            self.response()._version_index.1 = self.n;
            if b == b' ' {
                self.current_function = Self::status_code_first;
                return;
            }
        }
    }

    fn status_code_first(&mut self) {
        let b = self.b;
        if b.is_ascii_digit() {
            self.response()._status_code_index.0 = self.n;
            self.current_function = Self::status_code_tail;
        } else {
        }
    }

    fn status_code_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_digit() {
        } else {
            self.response()._status_code_index.1 = self.n;
            if b == b' ' {
                self.current_function = Self::reason_first;
                return;
            }
        }
    }

    fn reason_first(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
            self.response()._reason_index.0 = self.n;
            self.current_function = Self::reason_tail;
        } else {
        }
    }

    fn reason_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
        } else {
            self.response()._reason_index.1 = self.n;
            self.current_function = Self::cr_behind_reason;
            self.cr_behind_reason();
        }
    }

    fn cr_behind_reason(&mut self) {
        let b = self.b;
        if b == b'\r' {
            self.current_function = Self::lf_behind_reason;
        } else if b == b'\n' {
        }
    }

    fn lf_behind_reason(&mut self) {
        let b = self.b;
        if b == b'\n' {
            self.current_function = Self::field_name_first;
        } else if b == b'\r' {
        }
    }

    fn field_name_first(&mut self) {
        let b = self.b;
        if b.is_ascii_alphabetic() {
            self.field_index.0 = self.n;
            self.current_function = Self::field_name_tail;
        } else {
        }
    }

    fn field_name_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_alphanumeric() || b == b'-' {
        } else if b == b':' {
            self.field_index.1 = self.n;
            self.current_function = Self::field_value_first;
        } else {
        }
    }

    fn field_value_first(&mut self) {
        let b = self.b;
        if b.is_ascii_whitespace() {
        } else if b.is_ascii_graphic() {
            self.field_index.2 = self.n;
            self.current_function = Self::field_value_tail;
        } else {
        }
    }

    fn field_value_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
        } else if b.is_ascii_whitespace() || b == b'\r' {
            let f = (
                self.field_index.0,
                self.field_index.1,
                self.field_index.2,
                self.n,
            );
            self.response().field_index(f);
            if b == b'\r' {
                self.current_function = Self::cr_behind_field;
                return;
            }
        } else {
        }
    }

    fn cr_behind_field(&mut self) {
        let b = self.b;
        if b == b'\r' {
            self.current_function = Self::lf_behind_field;
        } else if b == b'\n' {
        }
    }

    fn lf_behind_field(&mut self) {
        let b = self.b;
        if b == b'\n' {
            self.current_function = Self::body_first;
        } else if b == b'\r' {
        } else {
        }
    }

    fn body_first(&mut self) {
        self.response().body = self.n;
        self.current_function = Self::body_tail;
    }

    fn body_tail(&mut self) {}
}
