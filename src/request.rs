use crate::slice_index_into_str;
use std::collections::HashMap;
use std::ptr;

///Represents an HTTP request.
#[derive(Debug)]
pub struct Request {
    origin: *const [u8],
    method: String,
    _method_index: (usize, usize),
    target: String,
    _target_index: (usize, usize),
    version: String,
    _version_index: (usize, usize),
    fields: HashMap<String, Vec<u8>>,
    _fields_index: Vec<(usize, usize, usize, usize)>,
    body: usize,
}

impl Request {
    fn new(origin: &[u8]) -> Self {
        Request {
            origin,
            method: String::new(),
            _method_index: (0, 0),
            target: String::new(),
            _target_index: (0, 0),
            version: String::new(),
            _version_index: (0, 0),
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

    pub fn method(&mut self) -> &String {
        if self.method.is_empty() {
            let index = self._method_index;
            let s = self.origin();
            self.method = slice_index_into_str(s, index);
        }
        &self.method
    }

    pub fn target(&mut self) -> &String {
        if self.target.is_empty() {
            let index = self._target_index;
            let s = self.origin();
            self.target = slice_index_into_str(s, index);
        }
        &self.target
    }

    pub fn version(&mut self) -> &String {
        if self.version.is_empty() {
            let index = self._version_index;
            let s = self.origin();
            self.version = slice_index_into_str(s, index);
        }
        &self.target
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

///Builder can parse request bytes to `Request`.
pub struct RequestBuilder {
    parser: Parser,
}

impl RequestBuilder {
    pub fn new() -> Self {
        RequestBuilder {
            parser: Parser::new(),
        }
    }

    pub fn from_bytes(&mut self, bytes: &[u8]) -> Request {
        let mut request = Request::new(bytes);
        self.parser.accept_request(&mut request);
        request
    }
}

struct Parser {
    current_function: fn(&mut Parser),
    b: u8,
    n: usize,
    field_index: (usize, usize, usize),
    request: *mut Request,
}

impl Parser {
    fn new() -> Self {
        Parser {
            current_function: Self::method_first,
            b: 0,
            n: 0,
            field_index: (0, 0, 0),
            request: ptr::null_mut(),
        }
    }

    fn accept_request(&mut self, request: &mut Request) {
        self.request = request;
        let o = request.origin();
        for i in o {
            self.accept(*i);
        }
    }

    fn accept(&mut self, b: u8) {
        self.b = b;
        (self.current_function)(self);
        self.n += 1;
    }

    fn request(&mut self) -> &mut Request {
        unsafe {
            match self.request.as_mut() {
                Some(r) => r,
                None => {
                    self.request = &mut Request::new(b"");
                    &mut *self.request
                }
            }
        }
    }

    fn method_first(&mut self) {
        let b = self.b;
        if b == b'\r' || b == b'\n' {
        } else {
            if b.is_ascii_alphabetic() {
                self.request()._method_index.0 = self.n;
                self.current_function = Self::method_tail;
            } else {
            }
        }
    }

    fn method_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_alphabetic() {
        } else {
            self.request()._method_index.1 = self.n;
            if b == b' ' {
                self.current_function = Self::target_first;
                return;
            }
        }
    }

    fn target_first(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
            self.request()._target_index.0 = self.n;
            self.current_function = Self::target_tail;
        } else {
        }
    }

    fn target_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_graphic() {
        } else {
            self.request()._target_index.1 = self.n;
            if b == b' ' {
                self.current_function = Self::version_first;
                return;
            }
        }
    }

    fn version_first(&mut self) {
        let b = self.b;
        if b.is_ascii_alphanumeric() {
            self.request()._version_index.0 = self.n;
            self.current_function = Self::version_tail;
        } else {
        }
    }

    fn version_tail(&mut self) {
        let b = self.b;
        if b.is_ascii_alphanumeric() || b == b'.' {
        } else {
            self.request()._version_index.1 = self.n;
            self.current_function = Self::cr_behind_version;
            self.cr_behind_version();
        }
    }

    fn cr_behind_version(&mut self) {
        let b = self.b;
        if b == b'\r' {
            self.current_function = Self::lf_behind_version;
        } else if b == b'\n' {
        }
    }

    fn lf_behind_version(&mut self) {
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
            self.request().field_index(f);
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
        self.request().body = self.n;
        self.current_function = Self::body_tail;
    }

    fn body_tail(&mut self) {}
}
