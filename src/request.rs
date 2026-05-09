use crate::common::*;
use crate::io::*;
use crate::prty::*;
use derive_more::{Debug, Deref, DerefMut};

///Represents an HTTP/1.1 request.
#[derive(Debug, Default, Deref, DerefMut, Getters, Setters)]
pub struct H1Request {
    #[getset(get = "pub", set = "pub")]
    method: FieldValue,
    #[getset(get = "pub", set = "pub")]
    target: FieldValue,
    #[getset(get = "pub", set = "pub")]
    version: FieldValue,
    #[debug(ignore)]
    #[deref]
    #[deref_mut]
    headers_body: Entity,
}

impl H1Request {
    ///Creates.
    pub fn new(method: impl Into<FieldValue>, target: impl Into<FieldValue>) -> Self {
        Self {
            method: method.into(),
            target: target.into(),
            version: VERSION.into(),
            headers_body: Default::default(),
        }
    }

    ///Exports an HTTP/1.1 message.
    pub fn export(&self, o: &mut dyn PutU8) {
        o.put_exact(self.method.as_bytes());
        o.put_u8(SPACE);
        o.put_exact(self.target.as_bytes());
        o.put_u8(SPACE);
        o.put_exact(self.version.as_bytes());
        o.put_u8(CR);
        o.put_u8(LF);
        self.headers_body.export(o);
    }
}

macro_rules! parser_new {
    () => {
        ///Creates.
        pub fn new(v: Vec<u8>) -> Self {
            let mut o = Self {
                inner: v.into(),
                units: Default::default(),
                r: Default::default(),
            };
            o.units.with_phase(Phase::Start, &mut o.inner);
            o
        }
    };
}

macro_rules! parser_header_body {
    () => {
        ///Returns a slice to header value.
        pub fn header_value(&mut self, name: &[u8]) -> Option<Box<dyn GetU8 + '_>> {
            self.units.header_value(name, &mut self.inner)
        }

        ///Returns a header value `Vec`.
        pub fn header_value_vec(&mut self, name: &[u8]) -> Vec<u8> {
            self.units.header_value_vec(name, &mut self.inner)
        }

        ///Returns a header value `String`.
        pub fn header_value_string(&mut self, name: &str) -> Option<String> {
            self.units.header_value_string(name, &mut self.inner)
        }

        ///Returns a slice to body.
        pub fn body(&mut self) -> Option<Box<dyn GetU8 + '_>> {
            self.units.body(&mut self.inner)
        }

        ///Returns true if parsing is finished.
        pub fn is_finish(&self) -> bool {
            self.units.finish
        }

        ///Returns true if format is wrong.
        pub fn err(&self) -> bool {
            self.units.err
        }

        fn split_to_entity(&mut self) {
            let n = self.units.body;
            if n > 0 {
                *self.r.headers_body.body_mut() = self.inner.split_off(n);
            }

            let headers = &mut self.units.headers;
            let m = &mut self.r.headers_body;
            while let Some((a, b, c)) = headers.pop() {
                self.inner.truncate(c);
                let v = self.inner.split_off(b);
                m.add_field(a, v);
            }

            self.r.headers_body.set_err(self.units.err);
        }
    };
}
///Represents a request parser. Hold request bytes.
#[derive(Getters, MutGetters)]
pub struct H1RequestParser {
    inner: VecGet,
    units: H1RequestUnits,
    r: H1Request,
}

impl H1RequestParser {
    parser_new!();

    ///Returns a slice to method value.
    pub fn method(&self) -> &[u8] {
        self.units.method()
    }

    ///Returns a slice to target value.
    pub fn target(&self) -> &[u8] {
        self.units.target()
    }

    ///Returns a slice to version value.
    pub fn version(&self) -> &[u8] {
        self.units.version()
    }

    parser_header_body!();

    ///Splits bytes from self to request.
    pub fn to_request(mut self) -> H1Request {
        self.units.with_phase(Phase::Body, &mut self.inner);

        self.split_to_entity();

        self.r.set_method(self.units.method_vec.into());
        self.r.set_target(self.units.target_vec.into());
        self.r.set_version(self.units.version_vec.into());

        self.r
    }

    ///Copies bytes from self to request.
    pub fn copy_to_request(mut self) -> (H1Request, Vec<u8>) {
        let o = &mut self.inner;
        self.units.with_phase(Phase::Body, o);
        self.units.copy_to_request(o, &mut self.r);
        (self.r, self.inner.take())
    }
}

impl<T: Into<Vec<u8>>> From<T> for H1RequestParser {
    fn from(o: T) -> Self {
        Self::new(o.into())
    }
}

macro_rules! units_new {
    () => {
        ///Creates.
        pub fn new<T: GetU8>(o: &mut T) -> Self {
            let mut s = Self::default();
            s.with_phase(Phase::Start, o);
            s
        }
    };
}

macro_rules! units_header_body {
    () => {
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

        fn header_value_index<T: GetU8>(
            &mut self,
            name: &[u8],
            o: &mut T,
        ) -> Option<(usize, usize)> {
            self.find_header(name).or_else(|| {
                self.set_search(name.into());
                self.with_phase(Phase::Field, o);
                self.find_header(name)
            })
        }

        ///Returns a slice to header value.
        pub fn header_value<'a, T: GetU8>(
            &mut self,
            name: &[u8],
            o: &'a mut T,
        ) -> Option<Box<dyn GetU8 + 'a>> {
            self.header_value_index(name, o)
                .and_then(|r| o.sub_to(r.0, r.1))
        }

        ///Returns a header value `Vec`.
        pub fn header_value_vec<T: GetU8>(&mut self, name: &[u8], o: &mut T) -> Vec<u8> {
            self.header_value(name, o)
                .map(|mut r| r.get_surplus().to_vec())
                .unwrap_or(Vec::new())
        }

        ///Returns a header value `String`.
        pub fn header_value_string<T: GetU8>(&mut self, name: &str, o: &mut T) -> Option<String> {
            let mut r = self.header_value(name.as_bytes(), o)?;
            str::from_utf8(r.get_surplus()).map(|s| s.to_string()).ok()
        }

        ///Returns a index to body.
        pub fn position<T: GetU8>(&mut self, o: &mut T) -> usize {
            self.with_phase(Phase::Body, o);
            self.body
        }

        ///Returns a slice to body.
        pub fn body<'a, T: GetU8>(&mut self, o: &'a mut T) -> Option<Box<dyn GetU8 + 'a>> {
            let n = self.position(o);
            o.sub_to(n, usize::MAX)
        }

        ///Returns true if parsing is finished.
        pub fn is_finish(&self) -> bool {
            self.finish
        }

        ///Returns true if format is wrong.
        pub fn err(&self) -> bool {
            self.err
        }

        fn copy_to_entity<T: GetU8>(&mut self, o: &mut T, r: &mut Entity) {
            for (a, b, c) in self.headers.drain(..) {
                if let Some(mut s) = o.sub_to(b, c) {
                    let s = s.get_surplus();
                    r.add_field(a, into_field_value(s));
                }
            }

            if let Some(mut v) = self.body(o) {
                r.body_mut().extend_from_slice(v.as_mut().get_surplus());
            }

            r.set_err(self.err);
        }
    };
}

macro_rules! parse {
    () => {
        fn with_phase<T: GetU8>(&mut self, phase: Phase, o: &mut T) {
            if !self.is_finish() {
                self.phase = phase;
                self.parse(o);
            }
        }

        fn parse<T: GetU8>(&mut self, o: &mut T) {
            while let Some(b) = o.get_u8() {
                self.b = b;
                (self.current_function)(self);
                self.n += 1;
                if self.suspend {
                    break;
                }
            }
            self.reset();
        }

        fn header_name_first(&mut self) {
            let b = self.b;
            match b {
                CR => {
                    self.current_function = Self::header_name_first_lf;
                }
                LF => {
                    self.header_name_first_lf();
                }
                COLON => {
                    self.current_function = Self::header_value_first;
                }
                _ => {
                    self.header_name.push(b);
                    self.current_function = Self::header_name_tail;
                }
            }
        }

        fn header_name_first_lf(&mut self) {
            self.phase = Phase::Field;
            self.current_function = Self::body_first;
            match self.b {
                LF => {}
                _ => {
                    self.body_first();
                }
            }
        }

        fn header_name_tail(&mut self) {
            let b = self.b;
            match b {
                COLON => {
                    self.current_function = Self::header_value_first;
                }
                CR | LF => {
                    self.current_function = Self::header_value_tail;
                    self.header_value_tail();
                }
                _ => {
                    self.header_name.push(b);
                }
            }
        }

        fn header_value_first(&mut self) {
            match self.b {
                SPACE => {}
                CR | LF => {
                    self.current_function = Self::header_value_tail;
                    self.header_value_tail();
                }
                _ => {
                    self.header_value_index = self.n;
                    self.current_function = Self::header_value_tail;
                }
            }
        }

        fn header_value_tail(&mut self) {
            match self.b {
                SPACE => {
                    self.space_n += 1;
                }
                CR | LF => {
                    let name = std::mem::take(&mut self.header_name);
                    if let Some(s) = &self.search_header_name {
                        if s == &name {
                            if self.phase <= Phase::Field {
                                self.suspend = true;
                            }
                        }
                    }
                    let index = self.header_value_index;
                    let k = self.n - self.space_n;
                    self.headers.push((name, index, k));
                    self.header_value_index = 0;
                    self.space_n = 0;
                    self.current_function = Self::header_value_tail_lf;
                    if self.b == LF {
                        self.header_value_tail_lf();
                    }
                }
                _ => {
                    if self.space_n > 0 {
                        self.space_n = 0;
                    }
                }
            }
        }

        fn header_value_tail_lf(&mut self) {
            self.current_function = Self::header_name_first;
            match self.b {
                LF => {}
                _ => {
                    self.header_name_first();
                }
            }
        }

        fn body_first(&mut self) {
            self.body = self.n;
            self.finish = true;
            self.suspend = true;
            self.current_function = Self::body_tail;
            self.phase = Phase::Body;
        }

        fn body_tail(&mut self) {}
    };
}

#[derive(Default, PartialEq, PartialOrd)]
#[repr(u8)]
pub(crate) enum Phase {
    #[default]
    None = 0,
    Start = 1,
    Field = 2,
    Body = 3,
}

///Represents units of an HTTP/1.1 request.
pub struct H1RequestUnits {
    current_function: fn(&mut Self),
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
    err: bool,
    phase: Phase,
    space_n: usize,
}

impl Default for H1RequestUnits {
    fn default() -> Self {
        Self {
            current_function: Self::method_first,
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
            phase: Default::default(),
            space_n: 0,
        }
    }
}

impl H1RequestUnits {
    units_new!();

    ///Returns a slice to method value.
    pub fn method(&self) -> &[u8] {
        &self.method_vec
    }

    ///Returns a slice to target value.
    pub fn target(&self) -> &[u8] {
        &self.target_vec
    }

    ///Returns a slice to version value.
    pub fn version(&self) -> &[u8] {
        &self.version_vec
    }

    units_header_body!();

    ///Copies bytes to request.
    pub fn copy_to_request<T: GetU8>(mut self, o: &mut T, request: &mut H1Request) {
        self.with_phase(Phase::Body, o);

        self.copy_to_entity(o, &mut request.headers_body);

        request.set_method(self.method_vec.into());
        request.set_target(self.target_vec.into());
        request.set_version(self.version_vec.into());
    }
}

impl H1RequestUnits {
    parse!();

    fn method_first(&mut self) {
        let b = self.b;
        match b {
            SPACE => {}
            _ => {
                self.method_vec.push(b);
                self.current_function = Self::method_tail;
            }
        }
    }

    fn method_tail(&mut self) {
        let b = self.b;
        match b {
            SPACE => {
                self.current_function = Self::target_first;
            }
            _ => {
                self.method_vec.push(b);
            }
        }
    }

    fn target_first(&mut self) {
        let b = self.b;
        match b {
            SPACE => {}
            _ => {
                self.target_vec.push(b);
                self.current_function = Self::target_tail;
            }
        }
    }

    fn target_tail(&mut self) {
        let b = self.b;
        match b {
            SPACE => {
                self.current_function = Self::version_first;
            }
            _ => {
                self.target_vec.push(b);
            }
        }
    }

    fn version_first(&mut self) {
        let b = self.b;
        match b {
            SPACE => {}
            _ => {
                self.version_vec.push(b);
                self.current_function = Self::version_tail;
            }
        }
    }

    fn version_tail(&mut self) {
        let b = self.b;
        match b {
            CR => {
                self.current_function = Self::version_tail_lf;
            }
            LF => {
                self.version_tail_lf();
            }
            _ => {
                self.version_vec.push(b);
            }
        }
    }

    fn version_tail_lf(&mut self) {
        self.current_function = Self::header_name_first;
        if self.phase <= Phase::Start {
            self.suspend = true;
        }
        match self.b {
            LF => {}
            _ => {
                self.header_name_first();
            }
        }
    }
}
