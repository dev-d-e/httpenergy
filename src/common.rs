pub(crate) const CR: u8 = b'\r';

pub(crate) const LF: u8 = b'\n';

pub(crate) const SPACE: u8 = b' ';

pub(crate) const HTAB: u8 = 9;

pub(crate) const COLON: u8 = b':';

pub(crate) const DOT: u8 = b'.';

pub(crate) const HYPHEN: u8 = b'-';

#[inline]
pub(crate) fn is_crlf(b: u8) -> bool {
    b == CR || b == LF
}

#[inline]
pub(crate) fn is_whitespace(b: u8) -> bool {
    b == SPACE || b == HTAB
}

pub(crate) const VERSION: &str = "HTTP/1.1";

#[inline]
pub(crate) fn trim_whitespace(buf: &[u8], mut index0: usize, mut index1: usize) -> (usize, usize) {
    let mut i = index0;
    while i < index1 {
        if !is_whitespace(buf[i]) {
            index0 = i;
            break;
        }
        i += 1;
    }
    i = index1;
    while i > index0 {
        i -= 1;
        if !is_whitespace(buf[i]) {
            index1 = i + 1;
            break;
        }
    }
    (index0, index1)
}

struct StrWrapper(String);

impl utf8parse::Receiver for StrWrapper {
    fn codepoint(&mut self, c: char) {
        self.0.push(c);
    }

    fn invalid_sequence(&mut self) {}
}

impl StrWrapper {
    fn new() -> Self {
        StrWrapper(String::new())
    }
}

pub(crate) fn to_str(mut iter: impl Iterator<Item = u8>) -> String {
    let mut p = utf8parse::Parser::new();
    let mut s = StrWrapper::new();
    while let Some(b) = iter.next() {
        p.advance(&mut s, b);
    }
    s.0
}

pub(crate) fn into_str(buf: &[u8]) -> String {
    let mut p = utf8parse::Parser::new();
    let mut s = StrWrapper::new();
    for b in buf {
        p.advance(&mut s, *b);
    }
    s.0
}
