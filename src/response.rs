use crate::common::*;
use crate::io::*;
use crate::prty::*;
use crate::request::Phase;
use derive_more::{Debug, Deref, DerefMut};

///Represents an HTTP/1.1 response.
#[derive(Debug, Default, Deref, DerefMut, Getters, Setters)]
pub struct H1Response {
    #[getset(get = "pub", set = "pub")]
    version: FieldValue,
    #[getset(get = "pub", set = "pub")]
    status_code: FieldValue,
    #[getset(get = "pub", set = "pub")]
    reason: FieldValue,
    #[deref]
    #[deref_mut]
    headers_body: Entity,
}

impl H1Response {
    ///Creates.
    pub fn new(status_code: impl Into<FieldValue>) -> H1Response {
        Self {
            version: VERSION.into(),
            status_code: status_code.into(),
            reason: Default::default(),
            headers_body: Default::default(),
        }
    }

    ///Exports an HTTP/1.1 message.
    pub fn export(&self, o: &mut dyn PutU8) {
        o.put_exact(self.version.as_bytes());
        o.put_u8(SPACE);
        o.put_exact(self.status_code.as_bytes());
        o.put_u8(SPACE);
        o.put_exact(self.reason.as_bytes());
        o.put_u8(CR);
        o.put_u8(LF);
        self.headers_body.export(o);
    }
}

///Represents a response parser. Hold response bytes.
#[derive(Getters, MutGetters)]
pub struct H1ResponseParser {
    inner: VecGet,
    units: H1ResponseUnits,
    r: H1Response,
}

impl H1ResponseParser {
    parser_new!();

    ///Returns a slice to version value.
    pub fn version(&self) -> &[u8] {
        self.units.version()
    }

    ///Returns a slice to status-code.
    pub fn status_code(&self) -> &[u8] {
        self.units.status_code()
    }

    ///Returns a slice to reason-phrase.
    pub fn reason(&self) -> &[u8] {
        self.units.reason()
    }

    parser_header_body!();

    ///Splits bytes from self to response.
    pub fn to_response(mut self) -> H1Response {
        self.units.with_phase(Phase::Body, &mut self.inner);

        self.split_to_entity();

        self.r.set_version(self.units.version_vec.into());
        self.r.set_status_code(self.units.status_code_vec.into());
        self.r.set_reason(self.units.reason.into());

        self.r
    }

    ///Copies bytes from self to response.
    pub fn copy_to_response(mut self) -> (H1Response, Vec<u8>) {
        let o = &mut self.inner;
        self.units.with_phase(Phase::Body, o);
        self.units.copy_to_response(o, &mut self.r);
        (self.r, self.inner.take())
    }
}

impl<T: Into<Vec<u8>>> From<T> for H1ResponseParser {
    fn from(o: T) -> Self {
        Self::new(o.into())
    }
}

///Represents units of an HTTP/1.1 response.
pub struct H1ResponseUnits {
    current_function: fn(&mut Self),
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
    err: bool,
    phase: Phase,
    space_n: usize,
}

impl Default for H1ResponseUnits {
    fn default() -> Self {
        Self {
            current_function: Self::version_first,
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
            phase: Default::default(),
            space_n: 0,
        }
    }
}

impl H1ResponseUnits {
    units_new!();

    ///Returns a slice to version value.
    pub fn version(&self) -> &[u8] {
        &self.version_vec
    }

    ///Returns a slice to status-code.
    pub fn status_code(&self) -> &[u8] {
        &self.status_code_vec
    }

    ///Returns a slice to reason-phrase.
    pub fn reason(&self) -> &[u8] {
        &self.reason
    }

    units_header_body!();

    ///Copies bytes to response.
    pub fn copy_to_response<T: GetU8>(mut self, o: &mut T, response: &mut H1Response) {
        self.with_phase(Phase::Body, o);

        self.copy_to_entity(o, &mut response.headers_body);

        response.set_version(self.version_vec.into());
        response.set_status_code(self.status_code_vec.into());
        response.set_reason(self.reason.into());
    }
}

impl H1ResponseUnits {
    parse!();

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
            SPACE => {
                self.current_function = Self::status_code_first;
            }
            _ => {
                self.version_vec.push(b);
            }
        }
    }

    fn status_code_first(&mut self) {
        let b = self.b;
        match b {
            SPACE => {}
            _ => {
                self.status_code_vec.push(b);
                self.current_function = Self::status_code_tail;
            }
        }
    }

    fn status_code_tail(&mut self) {
        let b = self.b;
        match b {
            SPACE => {
                self.current_function = Self::reason_first;
            }
            _ => {
                self.status_code_vec.push(b);
            }
        }
    }

    fn reason_first(&mut self) {
        let b = self.b;
        match b {
            SPACE => {}
            CR => {
                self.reason_tail();
            }
            LF => {
                self.reason_tail();
            }
            _ => {
                self.reason.push(b);
                self.current_function = Self::reason_tail;
            }
        }
    }

    fn reason_tail(&mut self) {
        let b = self.b;
        match b {
            CR => {
                self.current_function = Self::version_tail_lf;
            }
            LF => {
                self.version_tail_lf();
            }
            _ => {
                self.reason.push(b);
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
