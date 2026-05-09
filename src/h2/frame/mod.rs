/*!
Utilities for the frame.

This module provides several types and functions for working with frames.

Each frame type can be created, then use export method.

To parse a frame, you can use [`get_frame`], returns a specific frame type.
*/

use super::*;
use derive_more::{Debug, From};
use std::num::NonZeroUsize;

const FRAME_HEADER_LENGTH: usize = 9;
const MAX_FRAME_LENGTH: usize = 16777215;

const DATA_FRAME_TYPE: u8 = 0x00;
const HEADERS_FRAME_TYPE: u8 = 0x01;
const PRIORITY_FRAME_TYPE: u8 = 0x02;
const RST_STREAM_FRAME_TYPE: u8 = 0x03;
const SETTINGS_FRAME_TYPE: u8 = 0x04;
const PUSH_PROMISE_FRAME_TYPE: u8 = 0x05;
const PING_FRAME_TYPE: u8 = 0x06;
const GOAWAY_FRAME_TYPE: u8 = 0x07;
const WINDOW_UPDATE_FRAME_TYPE: u8 = 0x08;
const CONTINUATION_FRAME_TYPE: u8 = 0x09;

const UNUSED_FLAGS: u8 = 0b0000_0000;
const PADDED_FLAG: u8 = 0b0000_1000;
const END_STREAM_FLAG: u8 = 0b0000_0001;
const PRIORITY_FLAG: u8 = 0b0010_0000;
const END_HEADERS_FLAG: u8 = 0b0000_0100;
const ACK_FLAG: u8 = 0b0000_0001;

const RESERVED: u8 = 0b0111_1111;
const STREAM_IDENTIFIER_ZERO: u32 = 0;

const EXCLUSIVE: u8 = 0b1000_0000;

#[inline(always)]
fn check_capacity(capacity: usize) -> NonZeroUsize {
    let n = match capacity {
        0 => 4096,
        1..MAX_FRAME_LENGTH => capacity,
        _ => MAX_FRAME_LENGTH,
    };
    unsafe { NonZeroUsize::new_unchecked(n) }
}

#[inline(always)]
fn bit_eq(i: u8, f: u8) -> bool {
    i & f == f
}

#[inline(always)]
fn fill_header(length: u32, frame_type: u8, flags: u8, stream: u32, o: &mut dyn PutU8) -> bool {
    let a = length.to_be_bytes();
    let b = stream.to_be_bytes();
    o.put_exact(&a[1..]);
    o.put_u8(frame_type);
    o.put_u8(flags);
    o.put_u8(b[0] & RESERVED);
    o.put_exact(&b[1..])
}

#[inline(always)]
fn fill_priority(exclusive: bool, stream_dependency: u32, weight: u8, o: &mut dyn PutU8) -> bool {
    let a = stream_dependency.to_be_bytes();
    if exclusive {
        o.put_u8(a[0] | EXCLUSIVE);
    } else {
        o.put_u8(a[0] & RESERVED);
    }
    o.put_exact(&a[1..]);
    o.put_u8(weight)
}

#[inline(always)]
fn fill_stream_id(stream_id: u32, o: &mut dyn PutU8) -> bool {
    let a = stream_id.to_be_bytes();
    o.put_u8(a[0] & RESERVED);
    o.put_exact(&a[1..])
}

#[inline(always)]
fn pad_length(a: usize, b: u8) -> (u32, u8) {
    let c = a + b as usize;
    if c <= MAX_FRAME_LENGTH {
        (c as u32, b)
    } else {
        (
            MAX_FRAME_LENGTH as u32,
            MAX_FRAME_LENGTH.saturating_sub(a) as u8,
        )
    }
}

///Represents a DATA frame.
#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
#[getset(get_copy = "pub", set = "pub")]
pub struct Data {
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    padded: bool,
    end_stream: bool,
    pad_length: u8,
    #[debug("{}", data.len())]
    #[getset(skip)]
    #[getset(get = "pub", get_mut = "pub")]
    data: FiniteVec,
}

impl Data {
    ///Creates.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            padded: false,
            end_stream: false,
            pad_length: 0,
            data: check_capacity(capacity).into(),
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        let mut o = UNUSED_FLAGS;
        if self.padded {
            o |= PADDED_FLAG;
        }
        if self.end_stream {
            o |= END_STREAM_FLAG;
        }
        o
    }

    ///Exports self into [`PutU8`].
    pub fn export(mut self, o: &mut dyn PutU8) {
        if self.padded && self.data.len() >= MAX_FRAME_LENGTH {
            self.padded = false;
        }
        let flags = self.flags();
        let stream = self.stream_identifier;
        if self.padded {
            let (length, pad_length) = pad_length(1 + self.data.len(), self.pad_length);
            fill_header(length, DATA_FRAME_TYPE, flags, stream, o);
            o.put_u8(pad_length);
            o.put_exact(&self.data);
            o.put_repeat(pad_length as usize, 0);
        } else {
            let length = self.data.len() as u32;
            fill_header(length, DATA_FRAME_TYPE, flags, stream, o);
            o.put_exact(&self.data);
        }
    }
}

///Represents a HEADERS frame.
#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
#[getset(get_copy = "pub", set = "pub")]
pub struct Headers {
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    priority: bool,
    padded: bool,
    end_headers: bool,
    end_stream: bool,
    pad_length: u8,
    exclusive: bool,
    stream_dependency: u32,
    weight: u8,
    #[debug("{}", field_block_fragment.len())]
    #[getset(skip)]
    #[getset(get = "pub", get_mut = "pub")]
    field_block_fragment: FiniteVec,
}

impl Headers {
    ///Creates.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            priority: false,
            padded: false,
            end_headers: false,
            end_stream: false,
            pad_length: 0,
            exclusive: false,
            stream_dependency: 0,
            weight: 0,
            field_block_fragment: check_capacity(capacity).into(),
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        let mut o = UNUSED_FLAGS;
        if self.priority {
            o |= PRIORITY_FLAG;
        }
        if self.padded {
            o |= PADDED_FLAG;
        }
        if self.end_headers {
            o |= END_HEADERS_FLAG;
        }
        if self.end_stream {
            o |= END_STREAM_FLAG;
        }
        o
    }

    ///Exports self into [`PutU8`].
    pub fn export(mut self, o: &mut dyn PutU8) {
        let stream = self.stream_identifier;
        if self.priority {
            let n = 5 + self.field_block_fragment.len();
            if self.padded && n >= MAX_FRAME_LENGTH {
                self.padded = false;
            }
            let flags = self.flags();
            if self.padded {
                let (length, pad_length) = pad_length(1 + n, self.pad_length);
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, o);
                o.put_u8(pad_length);
                fill_priority(self.exclusive, self.stream_dependency, self.weight, o);
                o.put_exact(&self.field_block_fragment);
                o.put_repeat(pad_length as usize, 0);
            } else {
                let length = n as u32;
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, o);
                fill_priority(self.exclusive, self.stream_dependency, self.weight, o);
                o.put_exact(&self.field_block_fragment);
            }
        } else {
            let n = self.field_block_fragment.len();
            if self.padded && n >= MAX_FRAME_LENGTH {
                self.padded = false;
            }
            let flags = self.flags();
            if self.padded {
                let (length, pad_length) = pad_length(1 + n, self.pad_length);
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, o);
                o.put_u8(pad_length);
                o.put_exact(&self.field_block_fragment);
                o.put_repeat(pad_length as usize, 0);
            } else {
                let length = n as u32;
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, o);
                o.put_exact(&self.field_block_fragment);
            }
        }
    }
}

const PRIORITY_LENGTH: u32 = 0x05;

///Represents a PRIORITY frame.
#[derive(CopyGetters, Debug, Setters)]
#[getset(get_copy = "pub", set = "pub")]
pub struct Priority {
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    exclusive: bool,
    stream_dependency: u32,
    weight: u8,
}

impl Priority {
    ///Creates.
    pub fn new(stream_identifier: u32) -> Self {
        Self {
            stream_identifier,
            exclusive: false,
            stream_dependency: 0,
            weight: 0,
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        fill_header(
            PRIORITY_LENGTH,
            PRIORITY_FRAME_TYPE,
            UNUSED_FLAGS,
            self.stream_identifier,
            o,
        );
        fill_priority(self.exclusive, self.stream_dependency, self.weight, o);
    }
}

const RST_STREAM_LENGTH: u32 = 0x04;

///Represents a RST_STREAM frame.
#[derive(CopyGetters, Debug, Setters)]
#[getset(get_copy = "pub")]
pub struct RstStream {
    stream_identifier: u32,
    #[getset(set = "pub")]
    error_code: u32,
}

impl RstStream {
    ///Creates.
    pub fn new(stream_identifier: u32, error_code: u32) -> Self {
        Self {
            stream_identifier,
            error_code,
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        fill_header(
            RST_STREAM_LENGTH,
            RST_STREAM_FRAME_TYPE,
            UNUSED_FLAGS,
            self.stream_identifier,
            o,
        );
        u32_to(self.error_code, o);
    }
}

///Represents a SETTINGS frame.
#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
pub struct Settings {
    #[getset(get_copy = "pub", set = "pub")]
    ack: bool,
    #[debug("{}", setting.len())]
    #[getset(get = "pub", get_mut = "pub")]
    setting: Vec<(u16, u32)>,
}

impl Settings {
    ///Creates.
    pub fn new(capacity: usize) -> Self {
        Self {
            ack: false,
            setting: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        if self.ack { ACK_FLAG } else { UNUSED_FLAGS }
    }

    ///Add identifier and value.
    pub fn push(&mut self, identifier: u16, value: u32) -> bool {
        let r = self.setting.len() < self.setting.capacity();
        if r {
            self.setting.push((identifier, value));
        }
        r
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let flags = self.flags();
        let length = self.setting.len() as u32;
        fill_header(
            length,
            SETTINGS_FRAME_TYPE,
            flags,
            STREAM_IDENTIFIER_ZERO,
            o,
        );
        for (identifier, value) in self.setting {
            u16_to(identifier, o);
            u32_to(value, o);
        }
    }
}

///Represents a PUSH_PROMISE frame.
#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
#[getset(get_copy = "pub", set = "pub")]
pub struct PushPromise {
    #[getset(skip)]
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    padded: bool,
    end_headers: bool,
    pad_length: u8,
    promised_stream_id: u32,
    #[debug("{}", field_block_fragment.len())]
    #[getset(skip)]
    #[getset(get = "pub", get_mut = "pub")]
    field_block_fragment: FiniteVec,
}

impl PushPromise {
    ///Creates.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            padded: false,
            end_headers: false,
            pad_length: 0,
            promised_stream_id: 0,
            field_block_fragment: check_capacity(capacity).into(),
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        let mut o = UNUSED_FLAGS;
        if self.padded {
            o |= PADDED_FLAG;
        }
        if self.end_headers {
            o |= END_HEADERS_FLAG;
        }
        o
    }

    ///Exports self into [`PutU8`].
    pub fn export(mut self, o: &mut dyn PutU8) {
        let n = 4 + self.field_block_fragment.len();
        if self.padded && n >= MAX_FRAME_LENGTH {
            self.padded = false;
        }
        let flags = self.flags();
        let stream = self.stream_identifier;
        if self.padded {
            let (length, pad_length) = pad_length(1 + n, self.pad_length);
            fill_header(length, PUSH_PROMISE_FRAME_TYPE, flags, stream, o);
            o.put_u8(pad_length);
            fill_stream_id(self.promised_stream_id, o);
            o.put_exact(&self.field_block_fragment);
            o.put_repeat(pad_length as usize, 0);
        } else {
            let length = n as u32;
            fill_header(length, PUSH_PROMISE_FRAME_TYPE, flags, stream, o);
            fill_stream_id(self.promised_stream_id, o);
            o.put_exact(&self.field_block_fragment);
        }
    }
}

const PING_LENGTH: u32 = 0x08;

///Represents a PING frame.
#[derive(CopyGetters, Debug, Setters)]
#[getset(get_copy = "pub")]
pub struct Ping {
    stream_identifier: u32,
    #[getset(set = "pub")]
    ack: bool,
    #[getset(set = "pub")]
    opaque_data: u64,
}

impl Ping {
    ///Creates.
    pub fn new(ack: bool, opaque_data: u64) -> Self {
        Self {
            stream_identifier: STREAM_IDENTIFIER_ZERO,
            ack,
            opaque_data,
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        if self.ack { ACK_FLAG } else { UNUSED_FLAGS }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let flags = self.flags();
        let stream = self.stream_identifier;
        fill_header(PING_LENGTH, PING_FRAME_TYPE, flags, stream, o);
        u64_to(self.opaque_data, o);
    }
}

///Represents a GOAWAY frame.
#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
#[getset(get_copy = "pub", set = "pub")]
pub struct Goaway {
    last_stream_id: u32,
    error_code: u32,
    #[debug("{}", additional_debug_data.len())]
    #[getset(skip)]
    #[getset(get = "pub", get_mut = "pub")]
    additional_debug_data: FiniteVec,
}

impl Goaway {
    ///Creates.
    pub fn new(capacity: usize) -> Self {
        Self {
            last_stream_id: 0,
            error_code: 0,
            additional_debug_data: check_capacity(capacity).into(),
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let length = 8 + self.additional_debug_data.len() as u32;
        fill_header(
            length,
            GOAWAY_FRAME_TYPE,
            UNUSED_FLAGS,
            STREAM_IDENTIFIER_ZERO,
            o,
        );
        fill_stream_id(self.last_stream_id, o);
        u32_to(self.error_code, o);
        o.put_exact(&self.additional_debug_data);
    }
}

const WINDOW_UPDATE_LENGTH: u32 = 0x04;

///Represents a WINDOW_UPDATE frame.
#[derive(CopyGetters, Debug, Setters)]
#[getset(get_copy = "pub")]
pub struct WindowUpdate {
    stream_identifier: u32,
    #[getset(set = "pub")]
    window_size_increment: u32,
}

impl WindowUpdate {
    ///Creates.
    pub fn new(stream_identifier: u32, window_size_increment: u32) -> Self {
        Self {
            stream_identifier,
            window_size_increment,
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        fill_header(
            WINDOW_UPDATE_LENGTH,
            WINDOW_UPDATE_FRAME_TYPE,
            UNUSED_FLAGS,
            self.stream_identifier,
            o,
        );
        u32_to(self.window_size_increment, o);
    }
}

///Represents a CONTINUATION frame.
#[derive(CopyGetters, Debug, Getters, MutGetters, Setters)]
pub struct Continuation {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    end_headers: bool,
    #[debug("{}", field_block_fragment.len())]
    #[getset(get = "pub", get_mut = "pub")]
    field_block_fragment: FiniteVec,
}

impl Continuation {
    ///Creates.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            end_headers: false,
            field_block_fragment: check_capacity(capacity).into(),
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        if self.end_headers {
            END_HEADERS_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let flags = self.flags();
        let length = self.field_block_fragment.len() as u32;
        let stream = self.stream_identifier;
        fill_header(length, CONTINUATION_FRAME_TYPE, flags, stream, o);
        o.put_exact(&self.field_block_fragment);
    }
}

#[inline(always)]
fn get_31_uint(o: &[u8]) -> u32 {
    u32::from_be_bytes([o[0] & RESERVED, o[1], o[2], o[3]])
}

#[inline(always)]
fn get_priority(o: &[u8]) -> (bool, u32, u8) {
    let exclusive = bit_eq(o[0], EXCLUSIVE);
    let stream_dependency = get_31_uint(&o[0..4]);
    (exclusive, stream_dependency, o[4])
}

#[inline(always)]
fn check_return<'a>(
    o: bool,
    s: &'a str,
    h: &FrameHeader,
) -> Result<(), (&'a str, Option<FrameHeader>)> {
    if o { Err((s, Some(*h))) } else { Ok(()) }
}

///Represents a parsed frame header.
#[derive(Clone, Copy, Debug, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct FrameHeader {
    length: u32,
    ty: u8,
    flags: u8,
    stream_identifier: u32,
}

impl FrameHeader {
    #[inline(always)]
    fn padded_flag(&self) -> bool {
        bit_eq(self.flags, PADDED_FLAG)
    }

    #[inline(always)]
    fn end_stream_flag(&self) -> bool {
        bit_eq(self.flags, END_STREAM_FLAG)
    }

    #[inline(always)]
    fn priority_flag(&self) -> bool {
        bit_eq(self.flags, PRIORITY_FLAG)
    }

    #[inline(always)]
    fn end_headers_flag(&self) -> bool {
        bit_eq(self.flags, END_HEADERS_FLAG)
    }

    #[inline(always)]
    fn ack_flag(&self) -> bool {
        bit_eq(self.flags, ACK_FLAG)
    }
}

///Represents a result of parsing frame.
#[derive(From)]
#[repr(u8)]
pub enum FrameResult {
    Data(DataResult),
    Headers(HeadersResult),
    Priority(Priority),
    RstStream(RstStream),
    Settings(SettingsResult),
    PushPromise(PushPromiseResult),
    Ping(Ping),
    Goaway(GoawayResult),
    WindowUpdate(WindowUpdate),
    Continuation(ContinuationResult),
}

///Parses bytes. Returns a frame, or error.
pub fn get_frame(o: &mut dyn GetU8) -> Result<FrameResult, (&str, Option<FrameHeader>)> {
    let h = o
        .get_exact(FRAME_HEADER_LENGTH)
        .ok_or_else(|| ("header shortage", None))?;
    let h = FrameHeader {
        length: u32::from_be_bytes([0, h[0], h[1], h[2]]),
        ty: h[3],
        flags: h[4],
        stream_identifier: get_31_uint(&h[5..9]),
    };
    let i = o.index();
    let length = h.length;
    let stream_identifier = h.stream_identifier;
    let mut temp = TempIndex(i, i + length as usize);
    let r = match h.ty {
        DATA_FRAME_TYPE => {
            let padded = h.padded_flag();
            let mut pad_length = 0;
            if padded {
                check_return(length == 0, "invalid length", &h)?;
                pad_length = o.get_u8().ok_or_else(|| ("shortage", Some(h)))?;
                temp.0 += 1;
                check_return(
                    !temp.sub_pad_length(length, pad_length),
                    "protocol error",
                    &h,
                )?;
            }
            o.set_index(temp.1);
            DataResult {
                length,
                stream_identifier,
                padded,
                end_stream: h.end_stream_flag(),
                pad_length,
                temp,
            }
            .into()
        }
        HEADERS_FRAME_TYPE => {
            let padded = h.padded_flag();
            let mut pad_length = 0;
            if padded {
                check_return(length == 0, "invalid length", &h)?;
                pad_length = o.get_u8().ok_or_else(|| ("shortage", Some(h)))?;
                temp.0 += 1;
                check_return(
                    !temp.sub_pad_length(length, pad_length),
                    "protocol error",
                    &h,
                )?;
            }
            let priority = h.priority_flag();
            let mut exclusive = false;
            let mut stream_dependency = 0;
            let mut weight = 0;
            if priority {
                check_return(length < 5, "invalid length", &h)?;
                let i = o.get_exact(5).ok_or_else(|| ("shortage", Some(h)))?;
                (exclusive, stream_dependency, weight) = get_priority(i);
                temp.0 += 5;
            }
            o.set_index(temp.1);
            HeadersResult {
                length,
                stream_identifier,
                priority,
                padded,
                end_headers: h.end_headers_flag(),
                end_stream: h.end_stream_flag(),
                pad_length,
                exclusive,
                stream_dependency,
                weight,
                temp,
            }
            .into()
        }
        PRIORITY_FRAME_TYPE => {
            check_return(length != PRIORITY_LENGTH, "invalid length", &h)?;
            let i = o
                .get_exact(PRIORITY_LENGTH as usize)
                .ok_or_else(|| ("shortage", Some(h)))?;
            let (exclusive, stream_dependency, weight) = get_priority(i);
            Priority {
                stream_identifier,
                exclusive,
                stream_dependency,
                weight,
            }
            .into()
        }
        RST_STREAM_FRAME_TYPE => {
            check_return(length != RST_STREAM_LENGTH, "invalid length", &h)?;
            let error_code = to_u32(o).ok_or_else(|| ("shortage", Some(h)))?;
            RstStream {
                stream_identifier,
                error_code,
            }
            .into()
        }
        SETTINGS_FRAME_TYPE => {
            let mut setting = Vec::new();
            if length > 0 {
                let mut k = o
                    .get_exact_to(length as usize)
                    .ok_or_else(|| ("shortage", Some(h)))?;
                while let Some(v) = k.get_exact(6) {
                    let a = u16::from_be_bytes([v[0], v[1]]);
                    let b = u32::from_be_bytes([v[2], v[3], v[4], v[5]]);
                    setting.push((a, b));
                }
            }
            o.set_index(temp.1);
            SettingsResult {
                length,
                stream_identifier,
                ack: h.ack_flag(),
                setting,
            }
            .into()
        }
        PUSH_PROMISE_FRAME_TYPE => {
            let padded = h.padded_flag();
            let mut pad_length = 0;
            if padded {
                check_return(length == 0, "invalid length", &h)?;
                pad_length = o.get_u8().ok_or_else(|| ("shortage", Some(h)))?;
                temp.0 += 1;
                check_return(
                    !temp.sub_pad_length(length, pad_length),
                    "protocol error",
                    &h,
                )?;
            }
            let i = o.get_exact(4).ok_or_else(|| ("shortage", Some(h)))?;
            let promised_stream_id = get_31_uint(i);
            temp.0 += 4;
            o.set_index(temp.1);
            PushPromiseResult {
                length,
                stream_identifier,
                padded,
                end_headers: h.end_headers_flag(),
                pad_length,
                promised_stream_id,
                temp,
            }
            .into()
        }
        PING_FRAME_TYPE => {
            check_return(length != PING_LENGTH, "invalid length", &h)?;
            let opaque_data = to_u64(o).ok_or_else(|| ("shortage", Some(h)))?;
            Ping {
                stream_identifier,
                ack: h.ack_flag(),
                opaque_data,
            }
            .into()
        }
        GOAWAY_FRAME_TYPE => {
            let i = o.get_exact(4).ok_or_else(|| ("shortage", Some(h)))?;
            let last_stream_id = get_31_uint(i);
            let error_code = to_u32(o).ok_or_else(|| ("shortage", Some(h)))?;
            o.set_index(temp.1);
            GoawayResult {
                length,
                stream_identifier,
                last_stream_id,
                error_code,
                temp,
            }
            .into()
        }
        WINDOW_UPDATE_FRAME_TYPE => {
            check_return(length != WINDOW_UPDATE_LENGTH, "invalid length", &h)?;
            let i = o
                .get_exact(WINDOW_UPDATE_LENGTH as usize)
                .ok_or_else(|| ("shortage", Some(h)))?;
            let window_size_increment = get_31_uint(i);
            WindowUpdate {
                stream_identifier,
                window_size_increment,
            }
            .into()
        }
        CONTINUATION_FRAME_TYPE => {
            o.set_index(temp.1);
            ContinuationResult {
                length,
                stream_identifier,
                end_headers: h.end_headers_flag(),
                temp,
            }
            .into()
        }
        _ => return Err(("invalid type", Some(h))),
    };
    Ok(r)
}

struct TempIndex(usize, usize);

impl TempIndex {
    #[inline(always)]
    fn sub_pad_length(&mut self, length: u32, pad_length: u8) -> bool {
        let p = pad_length as u32;
        if p < length {
            self.1 -= p as usize;
            true
        } else {
            false
        }
    }
}

///Represents a parsed DATA frame.
#[derive(CopyGetters, Debug, Getters, MutGetters)]
#[getset(get_copy = "pub")]
pub struct DataResult {
    length: u32,
    stream_identifier: u32,
    padded: bool,
    end_stream: bool,
    pad_length: u8,
    #[debug(ignore)]
    #[getset(skip)]
    temp: TempIndex,
}

impl DataResult {
    ///Returns data.
    pub fn data<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.temp.0, self.temp.1)
    }
}

///Represents a parsed HEADERS frame.
#[derive(CopyGetters, Debug, Getters, MutGetters)]
#[getset(get_copy = "pub")]
pub struct HeadersResult {
    length: u32,
    stream_identifier: u32,
    priority: bool,
    padded: bool,
    end_headers: bool,
    end_stream: bool,
    pad_length: u8,
    exclusive: bool,
    stream_dependency: u32,
    weight: u8,
    #[debug(ignore)]
    #[getset(skip)]
    temp: TempIndex,
}

impl HeadersResult {
    ///Returns field block fragment.
    pub fn field_block_fragment<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.temp.0, self.temp.1)
    }
}

///Represents a parsed SETTINGS frame.
#[derive(CopyGetters, Debug, Getters)]
#[getset(get_copy = "pub")]
pub struct SettingsResult {
    length: u32,
    stream_identifier: u32,
    ack: bool,
    #[getset(skip)]
    #[getset(get = "pub")]
    setting: Vec<(u16, u32)>,
}

///Represents a parsed PUSH_PROMISE frame.
#[derive(CopyGetters, Debug, Getters, MutGetters)]
#[getset(get_copy = "pub")]
pub struct PushPromiseResult {
    length: u32,
    stream_identifier: u32,
    padded: bool,
    end_headers: bool,
    pad_length: u8,
    promised_stream_id: u32,
    #[debug(ignore)]
    #[getset(skip)]
    temp: TempIndex,
}

impl PushPromiseResult {
    ///Returns field block fragment.
    pub fn field_block_fragment<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.temp.0, self.temp.1)
    }
}

///Represents a parsed GOAWAY frame.
#[derive(CopyGetters, Debug, Getters, MutGetters)]
#[getset(get_copy = "pub")]
pub struct GoawayResult {
    length: u32,
    stream_identifier: u32,
    last_stream_id: u32,
    error_code: u32,
    #[debug(ignore)]
    #[getset(skip)]
    temp: TempIndex,
}

impl GoawayResult {
    ///Returns additional debug data.
    pub fn additional_debug_data<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.temp.0, self.temp.1)
    }
}

///Represents a parsed CONTINUATION frame.
#[derive(CopyGetters, Debug, Getters, MutGetters)]
#[getset(get_copy = "pub")]
pub struct ContinuationResult {
    length: u32,
    stream_identifier: u32,
    end_headers: bool,
    #[debug(ignore)]
    #[getset(skip)]
    temp: TempIndex,
}

impl ContinuationResult {
    ///Returns field block fragment.
    pub fn field_block_fragment<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.temp.0, self.temp.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data() {
        let i = 1;
        let a = 100;
        let b = 1000;
        let mut o = Data::new(i, 10000);
        o.set_end_stream(true);
        o.set_padded(true);
        o.set_pad_length(a);
        o.data_mut().put_repeat(b, 1);
        let mut v = Vec::new();
        o.export(&mut v);

        let mut v = v.into_get();
        if let Ok(f) = get_frame(&mut v) {
            match f {
                FrameResult::Data(o) => {
                    assert_eq!(o.stream_identifier(), i);
                    assert_eq!(o.end_stream(), true);
                    assert_eq!(o.padded(), true);
                    assert_eq!(o.pad_length(), a);
                    assert_eq!(o.data(&mut v).map(|r| r.surplus()).unwrap_or(0), b);
                }
                _ => {}
            }
        }
    }
}
