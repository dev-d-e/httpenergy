const FRAME_HEADER_LENGTH: usize = 9;

const DATA_FRAME_TYPE: u8 = 0x00;
const HEADERS_FRAME_TYPE: u8 = 0x01;
const PRIORITY_FRAME_TYPE: u8 = 0x02;
const RST_STREAM_FRAME_TYPE: u8 = 0x03;
const SETTINGS_FRAME_TYPE: u8 = 0x04;
const PUSH_PROMISE_FRAME_TYPE: u8 = 0x05;
const PING_FRAME_TYPE: u8 = 0x06;
const GOAWAY_FRAME_TYPE: u8 = 0x07;
const WINDOW_UPDATE_FRAME_TYPE: u8 = 0x08;
const CONTINUATION_FRAME_FRAME_TYPE: u8 = 0x09;

const UNUSED_FLAGS: u8 = 0b0000_0000;
const PADDED_FLAG: u8 = 0b0000_1000;
const END_STREAM_FLAG: u8 = 0b0000_0001;
const PRIORITY_FLAG: u8 = 0b0010_0000;
const END_HEADERS_FLAG: u8 = 0b0000_0100;
const ACK_FLAG: u8 = 0b0000_0001;

const RESERVED: u8 = 0b0111_1111;
const STREAM_IDENTIFIER_ZERO: u32 = 0;

const EXCLUSIVE: u8 = 0b1000_0000;

fn new_header(length: u32, frame_type: u8, flags: u8, stream_identifier: u32) -> Vec<u8> {
    let a = length.to_be_bytes();
    let b = stream_identifier.to_be_bytes();
    let mut o = Vec::with_capacity(FRAME_HEADER_LENGTH + length as usize);
    o.push(a[1]);
    o.push(a[2]);
    o.push(a[3]);
    o.push(frame_type);
    o.push(flags);
    o.push(b[0] & RESERVED);
    o.push(b[1]);
    o.push(b[2]);
    o.push(b[3]);
    o
}

fn fill_header(length: u32, frame_type: u8, flags: u8, stream_identifier: u32, o: &mut [u8]) {
    let a = length.to_be_bytes();
    let b = stream_identifier.to_be_bytes();
    o[0] = a[1];
    o[1] = a[2];
    o[2] = a[3];
    o[3] = frame_type;
    o[4] = flags;
    o[5] = b[0] & RESERVED;
    o[6] = b[1];
    o[7] = b[2];
    o[8] = b[3];
}

macro_rules! frame_header {
    () => {
        ///Returns length value.
        pub fn length(&self) -> u32 {
            u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]])
        }

        ///Returns frame type.
        pub fn frame_type(&self) -> u8 {
            self.0[3]
        }

        ///Returns flags.
        pub fn get_flags(&self) -> u8 {
            self.0[4]
        }

        #[inline(always)]
        fn is_flag(&self, f: u8) -> bool {
            self.0[4] & f == f
        }

        ///Returns stream identifier.
        pub fn stream_identifier(&self) -> u32 {
            u32::from_be_bytes([self.0[5] & RESERVED, self.0[6], self.0[7], self.0[8]])
        }

        ///Returns data length.
        pub fn data_length(&self, pad_length: u8) -> u32 {
            let a = self.length();
            let b = pad_length as u32;
            if a > b {
                a - b
            } else {
                0
            }
        }
    };
}

#[derive(Debug)]
pub struct Data(Vec<u8>);

const DATA_PAD_LENGTH_INDEX: usize = 9;

impl Data {
    pub fn new(length: u32, padded: bool, end_stream: bool, stream_identifier: u32) -> Self {
        let mut frame = new_header(
            length,
            DATA_FRAME_TYPE,
            Self::flags(padded, end_stream),
            stream_identifier,
        );
        if padded {
            frame.insert(DATA_PAD_LENGTH_INDEX, 0);
        }
        Self(frame)
    }

    #[inline(always)]
    fn flags(padded: bool, end_stream: bool) -> u8 {
        let mut o = UNUSED_FLAGS;
        if padded {
            o |= PADDED_FLAG;
        }
        if end_stream {
            o |= END_STREAM_FLAG;
        }
        o
    }

    frame_header!();

    pub fn padded(&self) -> bool {
        self.is_flag(PADDED_FLAG)
    }

    pub fn end_stream(&self) -> bool {
        self.is_flag(END_STREAM_FLAG)
    }

    pub fn set_pad_length(&mut self, n: u8) {
        if self.padded() {
            self.0[DATA_PAD_LENGTH_INDEX] = n;
        }
    }

    pub fn pad_length(&self) -> u8 {
        if self.padded() {
            self.0[DATA_PAD_LENGTH_INDEX]
        } else {
            0
        }
    }

    pub fn push_data(&mut self, value: u8) {
        self.0.push(value);
    }

    pub fn data(&self) -> &[u8] {
        if self.padded() {
            let n = self.data_length(self.pad_length());
            if n > 0 {
                let n = (n + 10) as usize;
                &self.0[10..n]
            } else {
                &self.0[10..]
            }
        } else {
            &self.0[9..]
        }
    }
}

#[derive(Debug)]
pub struct Headers(Vec<u8>);

impl Headers {
    pub fn new(
        length: u32,
        priority: bool,
        padded: bool,
        end_headers: bool,
        end_stream: bool,
        stream_identifier: u32,
    ) -> Self {
        let frame = new_header(
            length,
            HEADERS_FRAME_TYPE,
            Self::flags(priority, padded, end_headers, end_stream),
            stream_identifier,
        );
        Self(frame)
    }

    #[inline(always)]
    fn flags(priority: bool, padded: bool, end_headers: bool, end_stream: bool) -> u8 {
        let mut o = UNUSED_FLAGS;
        if priority {
            o |= PRIORITY_FLAG;
        }
        if padded {
            o |= PADDED_FLAG;
        }
        if end_headers {
            o |= END_HEADERS_FLAG;
        }
        if end_stream {
            o |= END_STREAM_FLAG;
        }
        o
    }

    frame_header!();

    pub fn priority(&self) -> bool {
        self.is_flag(PRIORITY_FLAG)
    }

    pub fn padded(&self) -> bool {
        self.is_flag(PADDED_FLAG)
    }

    pub fn end_headers(&self) -> bool {
        self.is_flag(END_HEADERS_FLAG)
    }

    pub fn end_stream(&self) -> bool {
        self.is_flag(END_STREAM_FLAG)
    }

    pub fn set_pad_length(&mut self, n: u8) {
        if self.padded() {
            self.0[9] = n;
        }
    }

    pub fn pad_length(&self) -> u8 {
        if self.padded() {
            self.0[9]
        } else {
            0
        }
    }

    pub fn exclusive(&self) -> u8 {
        if self.priority() {
            self.0[10] & EXCLUSIVE
        } else {
            0
        }
    }

    pub fn stream_dependency(&self) -> &[u8] {
        if self.priority() {
            &self.0[10..14]
        } else {
            &self.0[10..14]
        }
    }

    pub fn weight(&self) -> u8 {
        if self.priority() {
            self.0[15]
        } else {
            0
        }
    }

    pub fn field_block_fragment(&self) -> &[u8] {
        if self.padded() {
            let n = self.data_length(self.pad_length());
            if n > 0 {
                let n = (n + 15) as usize;
                &self.0[15..n]
            } else {
                &self.0[15..]
            }
        } else {
            &self.0[9..]
        }
    }
}

const PRIORITY_LENGTH: usize = 0x05;

#[derive(Debug)]
pub struct Priority([u8; FRAME_HEADER_LENGTH + PRIORITY_LENGTH]);

impl Priority {
    pub fn new(stream_identifier: u32) -> Self {
        let mut frame = [0; FRAME_HEADER_LENGTH + PRIORITY_LENGTH];
        fill_header(
            PRIORITY_LENGTH as u32,
            HEADERS_FRAME_TYPE,
            UNUSED_FLAGS,
            stream_identifier,
            &mut frame,
        );
        Self(frame)
    }

    frame_header!();

    pub fn priority(&self) -> bool {
        self.is_flag(PRIORITY_FLAG)
    }

    pub fn exclusive(&self) -> u8 {
        if self.priority() {
            self.0[9] & EXCLUSIVE
        } else {
            0
        }
    }

    pub fn stream_dependency(&self) -> &[u8] {
        if self.priority() {
            &self.0[9..13]
        } else {
            &self.0[9..13]
        }
    }

    pub fn weight(&self) -> u8 {
        if self.priority() {
            self.0[13]
        } else {
            0
        }
    }
}

const RST_STREAM_LENGTH: usize = 0x04;

#[derive(Debug)]
pub struct RstStream([u8; FRAME_HEADER_LENGTH + RST_STREAM_LENGTH]);

impl RstStream {
    pub fn new(stream_identifier: u32) -> Self {
        let mut frame = [0; FRAME_HEADER_LENGTH + RST_STREAM_LENGTH];
        fill_header(
            RST_STREAM_LENGTH as u32,
            RST_STREAM_FRAME_TYPE,
            UNUSED_FLAGS,
            stream_identifier,
            &mut frame,
        );
        Self(frame)
    }

    pub fn error_code(&self) -> &[u8] {
        &self.0[9..13]
    }
}

#[derive(Debug)]
pub struct Settings(Vec<u8>);

impl Settings {
    pub fn new(length: u32, ack: bool) -> Self {
        let frame = new_header(
            length,
            SETTINGS_FRAME_TYPE,
            Self::flags(ack),
            STREAM_IDENTIFIER_ZERO,
        );
        Self(frame)
    }

    #[inline(always)]
    fn flags(ack: bool) -> u8 {
        if ack {
            ACK_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    frame_header!();

    pub fn ack(&self) -> bool {
        self.is_flag(ACK_FLAG)
    }

    pub fn setting(&self) -> &[u8] {
        &self.0[9..]
    }
}

#[derive(Debug)]
pub struct PushPromise(Vec<u8>);

impl PushPromise {
    pub fn new(length: u32, padded: bool, end_headers: bool, stream_identifier: u32) -> Self {
        let frame = new_header(
            length,
            PUSH_PROMISE_FRAME_TYPE,
            Self::flags(padded, end_headers),
            stream_identifier,
        );
        Self(frame)
    }

    #[inline(always)]
    fn flags(padded: bool, end_headers: bool) -> u8 {
        let mut o = UNUSED_FLAGS;
        if padded {
            o |= PADDED_FLAG;
        }
        if end_headers {
            o |= END_HEADERS_FLAG;
        }
        o
    }

    frame_header!();

    pub fn padded(&self) -> bool {
        self.is_flag(PADDED_FLAG)
    }

    pub fn end_headers(&self) -> bool {
        self.is_flag(END_HEADERS_FLAG)
    }

    pub fn set_pad_length(&mut self, n: u8) {
        if self.padded() {
            self.0[9] = n;
        }
    }

    pub fn pad_length(&self) -> u8 {
        if self.padded() {
            self.0[9]
        } else {
            0
        }
    }

    pub fn promised_stream_id(&self) -> &[u8] {
        if self.padded() {
            &self.0[10..14]
        } else {
            &self.0[9..13]
        }
    }

    pub fn field_block_fragment(&self) -> &[u8] {
        if self.padded() {
            let n = self.data_length(self.pad_length());
            if n > 0 {
                let n = (n + 15) as usize;
                &self.0[15..n]
            } else {
                &self.0[15..]
            }
        } else {
            &self.0[9..]
        }
    }
}

const PING_LENGTH: usize = 0x08;

#[derive(Debug)]
pub struct Ping([u8; FRAME_HEADER_LENGTH + PING_LENGTH]);

impl Ping {
    pub fn new(ack: bool) -> Self {
        let mut frame = [0; FRAME_HEADER_LENGTH + PING_LENGTH];
        fill_header(
            PING_LENGTH as u32,
            PING_FRAME_TYPE,
            Self::flags(ack),
            STREAM_IDENTIFIER_ZERO,
            &mut frame,
        );
        Self(frame)
    }

    #[inline(always)]
    fn flags(ack: bool) -> u8 {
        if ack {
            ACK_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    frame_header!();

    pub fn ack(&self) -> bool {
        self.is_flag(ACK_FLAG)
    }

    pub fn opaque_data(&self) -> &[u8] {
        &self.0[9..]
    }
}

#[derive(Debug)]
pub struct Goaway(Vec<u8>);

impl Goaway {
    pub fn new(length: u32) -> Self {
        let frame = new_header(
            length,
            GOAWAY_FRAME_TYPE,
            UNUSED_FLAGS,
            STREAM_IDENTIFIER_ZERO,
        );
        Self(frame)
    }

    pub fn last_stream_id(&self) -> &[u8] {
        &self.0[9..13]
    }

    pub fn error_code(&self) -> &[u8] {
        &self.0[13..17]
    }

    pub fn additional_debug_data(&self) -> &[u8] {
        &self.0[17..]
    }
}

const WINDOW_UPDATE_LENGTH: usize = 0x04;

#[derive(Debug)]
pub struct WindowUpdate([u8; FRAME_HEADER_LENGTH + WINDOW_UPDATE_LENGTH]);

impl WindowUpdate {
    pub fn new(stream_identifier: u32) -> Self {
        let mut frame = [0; FRAME_HEADER_LENGTH + WINDOW_UPDATE_LENGTH];
        fill_header(
            WINDOW_UPDATE_LENGTH as u32,
            WINDOW_UPDATE_FRAME_TYPE,
            UNUSED_FLAGS,
            stream_identifier,
            &mut frame,
        );
        Self(frame)
    }

    pub fn window_size_increment(&self) -> &[u8] {
        &self.0[9..]
    }
}

#[derive(Debug)]
pub struct Continuation(Vec<u8>);

impl Continuation {
    pub fn new(length: u32, end_headers: bool, stream_identifier: u32) -> Self {
        let frame = new_header(
            length,
            CONTINUATION_FRAME_FRAME_TYPE,
            Self::flags(end_headers),
            stream_identifier,
        );
        Self(frame)
    }

    #[inline(always)]
    fn flags(end_headers: bool) -> u8 {
        if end_headers {
            END_HEADERS_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    frame_header!();

    pub fn end_headers(&self) -> bool {
        self.is_flag(END_HEADERS_FLAG)
    }

    pub fn field_block_fragment(&self) -> &[u8] {
        &self.0[9..]
    }
}
