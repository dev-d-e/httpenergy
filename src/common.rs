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

pub(crate) fn into_str(buf: &[u8]) -> String {
    let mut p = utf8parse::Parser::new();
    let mut s = StrWrapper::new();
    for b in buf {
        p.advance(&mut s, *b);
    }
    s.0
}

pub(crate) fn slice_index_into_str(buf: &[u8], index: (usize, usize)) -> String {
    if index.1 > index.0 {
        into_str(&buf[index.0..index.1])
    } else {
        String::new()
    }
}
