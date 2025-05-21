/*!
Utilities for the frame.

This module provides several encoder and decoder types for working with frames.

# encoder
Each frame type can create a new encoder, then encode to a `WriteByte`.

# decoder
To parse a frame, you can use [`FrameDecoder`] to decode a byte slice, returns a specific frame type.
*/

use super::hpack::DecodeInstructions;
use crate::{ReadByte, WriteByte};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::collections::HashSet;
use std::io::Error;

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
fn check_capacity(capacity: usize) -> usize {
    match capacity {
        0 => 4096,
        1..MAX_FRAME_LENGTH => capacity,
        _ => MAX_FRAME_LENGTH,
    }
}

#[inline(always)]
fn fill_header(
    length: u32,
    frame_type: u8,
    flags: u8,
    stream_identifier: u32,
    o: &mut impl WriteByte,
) -> Option<Error> {
    let a = length.to_be_bytes();
    let b = stream_identifier.to_be_bytes();
    o.put_all(&a[1..]);
    o.put(frame_type);
    o.put(flags);
    o.put(b[0] & RESERVED);
    o.put_all(&b[1..])
}

#[inline(always)]
fn fill_priority(
    exclusive: bool,
    stream_dependency: u32,
    weight: u8,
    o: &mut impl WriteByte,
) -> Option<Error> {
    let a = stream_dependency.to_be_bytes();
    if exclusive {
        o.put(a[0] | EXCLUSIVE);
    } else {
        o.put(a[0] & RESERVED);
    }
    o.put_all(&a[1..]);
    o.put(weight)
}

#[inline(always)]
fn fill_stream_id(stream_id: u32, writer: &mut impl WriteByte) -> Option<Error> {
    let a = stream_id.to_be_bytes();
    writer.put(a[0] & RESERVED);
    writer.put_all(&a[1..])
}

#[inline(always)]
fn padded(a: bool, b: usize) -> bool {
    if b >= MAX_FRAME_LENGTH {
        false
    } else {
        a
    }
}

#[inline(always)]
fn length(a: usize) -> u32 {
    if a <= MAX_FRAME_LENGTH {
        a as u32
    } else {
        MAX_FRAME_LENGTH as u32
    }
}

#[inline(always)]
fn pad_length(a: usize, b: u8) -> (u32, u8) {
    if a < MAX_FRAME_LENGTH {
        let c = a + b as usize;
        if c <= MAX_FRAME_LENGTH {
            (c as u32, b)
        } else {
            (MAX_FRAME_LENGTH as u32, (MAX_FRAME_LENGTH - a) as u8)
        }
    } else {
        (MAX_FRAME_LENGTH as u32, 0)
    }
}

///A builder which encodes data into DATA frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct DataEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    padded: bool,
    #[getset(get_copy = "pub", set = "pub")]
    end_stream: bool,
    #[getset(get_copy = "pub", set = "pub")]
    pad_length: u8,
    #[getset(get = "pub", get_mut = "pub")]
    data: Vec<u8>,
}

impl std::fmt::Debug for DataEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut o = f.debug_struct("DataEncoder");
        o.field("stream_identifier", &self.stream_identifier)
            .field("padded", &self.padded)
            .field("end_stream", &self.end_stream);
        if self.padded {
            o.field("pad_length", &self.pad_length);
        }
        o.field("data len", &self.data.len()).finish()
    }
}

impl DataEncoder {
    ///Creates with a stream identifier and data capacity.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            padded: false,
            end_stream: false,
            pad_length: 0,
            data: Vec::with_capacity(check_capacity(capacity)),
        }
    }

    ///Creates with capacity 16,777,215.
    pub fn max(stream_identifier: u32) -> Self {
        Self::new(stream_identifier, MAX_FRAME_LENGTH)
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

    ///Returns None if the data length <= 16,777,215, otherwise returns a newly vector containing bytes in the range [16777215..].
    pub fn check_length(&mut self) -> Option<Vec<u8>> {
        if self.data.len() > MAX_FRAME_LENGTH {
            Some(self.data.split_off(MAX_FRAME_LENGTH))
        } else {
            None
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let flags = self.flags();
        let stream = self.stream_identifier;
        if padded(self.padded, self.data.len()) {
            let (length, pad_length) = pad_length(1 + self.data.len(), self.pad_length);
            fill_header(length, DATA_FRAME_TYPE, flags, stream, writer);
            writer.put(pad_length);
            writer.put_all(&self.data);
            writer.put_repeat(pad_length as usize, 0)
        } else {
            let length = length(self.data.len());
            fill_header(length, DATA_FRAME_TYPE, flags, stream, writer);
            writer.put_all(&self.data)
        }
    }
}

///A builder which encodes field block into HEADERS frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct HeadersEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    priority: bool,
    #[getset(get_copy = "pub", set = "pub")]
    padded: bool,
    #[getset(get_copy = "pub", set = "pub")]
    end_headers: bool,
    #[getset(get_copy = "pub", set = "pub")]
    end_stream: bool,
    #[getset(get_copy = "pub", set = "pub")]
    pad_length: u8,
    #[getset(get_copy = "pub", set = "pub")]
    exclusive: bool,
    #[getset(get_copy = "pub", set = "pub")]
    stream_dependency: u32,
    #[getset(get_copy = "pub", set = "pub")]
    weight: u8,
    #[getset(get = "pub", get_mut = "pub")]
    field_block_fragment: Vec<u8>,
}

impl std::fmt::Debug for HeadersEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut o = f.debug_struct("HeadersEncoder");
        o.field("stream_identifier", &self.stream_identifier)
            .field("priority", &self.priority)
            .field("padded", &self.padded)
            .field("end_headers", &self.end_headers)
            .field("end_stream", &self.end_stream);
        if self.padded {
            o.field("pad_length", &self.pad_length);
        }
        if self.priority {
            o.field("exclusive", &self.exclusive)
                .field("stream_dependency", &self.stream_dependency)
                .field("weight", &self.weight);
        }
        o.field("field_block_fragment len", &self.field_block_fragment.len())
            .finish()
    }
}

impl HeadersEncoder {
    ///Creates with a stream identifier and capacity.
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
            field_block_fragment: Vec::with_capacity(check_capacity(capacity)),
        }
    }

    ///Creates with capacity 16,777,215.
    pub fn max(stream_identifier: u32) -> Self {
        Self::new(stream_identifier, MAX_FRAME_LENGTH)
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

    ///Returns None if the data length <= 16,777,215, otherwise returns a newly vector containing bytes in the range [16777215..].
    pub fn check_length(&mut self) -> Option<Vec<u8>> {
        let n = if self.priority {
            MAX_FRAME_LENGTH - 5
        } else {
            MAX_FRAME_LENGTH
        };
        if self.field_block_fragment.len() > n {
            Some(self.field_block_fragment.split_off(n))
        } else {
            None
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let flags = self.flags();
        let stream = self.stream_identifier;
        if self.priority {
            let n = 5 + self.field_block_fragment.len();
            if padded(self.padded, n) {
                let (length, pad_length) = pad_length(1 + n, self.pad_length);
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, writer);
                writer.put(pad_length);
                fill_priority(self.exclusive, self.stream_dependency, self.weight, writer);
                writer.put_all(&self.field_block_fragment);
                writer.put_repeat(pad_length as usize, 0)
            } else {
                let length = length(n);
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, writer);
                fill_priority(self.exclusive, self.stream_dependency, self.weight, writer);
                writer.put_all(&self.field_block_fragment)
            }
        } else {
            let n = self.field_block_fragment.len();
            if padded(self.padded, n) {
                let (length, pad_length) = pad_length(1 + n, self.pad_length);
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, writer);
                writer.put(pad_length);
                writer.put_all(&self.field_block_fragment);
                writer.put_repeat(pad_length as usize, 0)
            } else {
                let length = length(n);
                fill_header(length, HEADERS_FRAME_TYPE, flags, stream, writer);
                writer.put_all(&self.field_block_fragment)
            }
        }
    }
}

const PRIORITY_LENGTH: usize = 0x05;

///A builder which encodes info into PRIORITY frame.
#[derive(CopyGetters, Setters)]
pub struct PriorityEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    exclusive: bool,
    #[getset(get_copy = "pub", set = "pub")]
    stream_dependency: u32,
    #[getset(get_copy = "pub", set = "pub")]
    weight: u8,
}

impl std::fmt::Debug for PriorityEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PriorityEncoder")
            .field("stream_identifier", &self.stream_identifier)
            .field("exclusive", &self.exclusive)
            .field("stream_dependency", &self.stream_dependency)
            .field("weight", &self.weight)
            .finish()
    }
}

impl PriorityEncoder {
    ///Creates with a stream identifier.
    pub fn new(stream_identifier: u32) -> Self {
        Self {
            stream_identifier,
            exclusive: false,
            stream_dependency: 0,
            weight: 0,
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        fill_header(
            PRIORITY_LENGTH as u32,
            PRIORITY_FRAME_TYPE,
            UNUSED_FLAGS,
            self.stream_identifier,
            writer,
        );
        fill_priority(self.exclusive, self.stream_dependency, self.weight, writer)
    }
}

const RST_STREAM_LENGTH: usize = 0x04;

///A builder which encodes info into RST_STREAM frame.
#[derive(CopyGetters, Setters)]
pub struct RstStreamEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    error_code: u32,
}

impl std::fmt::Debug for RstStreamEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RstStreamEncoder")
            .field("stream_identifier", &self.stream_identifier)
            .field("error_code", &self.error_code)
            .finish()
    }
}

impl RstStreamEncoder {
    ///Creates with a stream identifier.
    pub fn new(stream_identifier: u32) -> Self {
        Self {
            stream_identifier,
            error_code: 0,
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        fill_header(
            RST_STREAM_LENGTH as u32,
            RST_STREAM_FRAME_TYPE,
            UNUSED_FLAGS,
            self.stream_identifier,
            writer,
        );
        writer.put_u32(self.error_code)
    }
}

///A builder which encodes info into SETTINGS frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct SettingsEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    ack: bool,
    #[getset(get = "pub", get_mut = "pub")]
    setting: Vec<u8>,
}

impl std::fmt::Debug for SettingsEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsEncoder")
            .field("ack", &self.ack)
            .field("setting len", &self.setting.len())
            .finish()
    }
}

impl SettingsEncoder {
    ///Creates with capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            ack: false,
            setting: Vec::with_capacity(check_capacity(capacity)),
        }
    }

    ///Creates with capacity 16,777,215.
    pub fn max() -> Self {
        Self::new(MAX_FRAME_LENGTH)
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        if self.ack {
            ACK_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    ///Appends identifier and value to the back of buffer.
    pub fn push(&mut self, identifier: u16, value: u32) -> bool {
        if self.setting.surplus_mut() < 6 {
            false
        } else {
            self.setting.put_u16(identifier);
            self.setting.put_u32(value);
            true
        }
    }

    ///Returns None if the data length <= 16,777,215, otherwise returns a newly vector containing bytes in the range [16777215..].
    pub fn check_length(&mut self) -> Option<Vec<u8>> {
        if self.setting.len() > MAX_FRAME_LENGTH {
            Some(self.setting.split_off(MAX_FRAME_LENGTH))
        } else {
            None
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let flags = self.flags();
        let length = length(self.setting.len());
        fill_header(
            length,
            SETTINGS_FRAME_TYPE,
            flags,
            STREAM_IDENTIFIER_ZERO,
            writer,
        );
        writer.put_all(&self.setting)
    }
}

///A builder which encodes field block into PUSH_PROMISE frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct PushPromiseEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    padded: bool,
    #[getset(get_copy = "pub", set = "pub")]
    end_headers: bool,
    #[getset(get_copy = "pub", set = "pub")]
    pad_length: u8,
    #[getset(get_copy = "pub", set = "pub")]
    promised_stream_id: u32,
    #[getset(get = "pub", get_mut = "pub")]
    field_block_fragment: Vec<u8>,
}

impl std::fmt::Debug for PushPromiseEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut o = f.debug_struct("PushPromiseEncoder");
        o.field("stream_identifier", &self.stream_identifier)
            .field("padded", &self.padded)
            .field("end_headers", &self.end_headers);
        if self.padded {
            o.field("pad_length", &self.pad_length);
        }
        o.field("promised_stream_id", &self.promised_stream_id)
            .field("field_block_fragment len", &self.field_block_fragment.len())
            .finish()
    }
}

impl PushPromiseEncoder {
    ///Creates with a stream identifier and capacity.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            padded: false,
            end_headers: false,
            pad_length: 0,
            promised_stream_id: 0,
            field_block_fragment: Vec::with_capacity(check_capacity(capacity)),
        }
    }

    ///Creates with capacity 16,777,215.
    pub fn max(stream_identifier: u32) -> Self {
        Self::new(stream_identifier, MAX_FRAME_LENGTH)
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

    ///Returns None if the data length <= 16,777,215, otherwise returns a newly vector containing bytes in the range [16777215..].
    pub fn check_length(&mut self) -> Option<Vec<u8>> {
        let n = MAX_FRAME_LENGTH - 4;
        if self.field_block_fragment.len() > n {
            Some(self.field_block_fragment.split_off(n))
        } else {
            None
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let flags = self.flags();
        let stream = self.stream_identifier;
        let n = 4 + self.field_block_fragment.len();
        if padded(self.padded, n) {
            let (length, pad_length) = pad_length(1 + n, self.pad_length);
            fill_header(length, PUSH_PROMISE_FRAME_TYPE, flags, stream, writer);
            writer.put(pad_length);
            fill_stream_id(self.promised_stream_id, writer);
            writer.put_all(&self.field_block_fragment);
            writer.put_repeat(pad_length as usize, 0)
        } else {
            let length = length(n);
            fill_header(length, PUSH_PROMISE_FRAME_TYPE, flags, stream, writer);
            fill_stream_id(self.promised_stream_id, writer);
            writer.put_all(&self.field_block_fragment)
        }
    }
}

const PING_LENGTH: usize = 0x08;

///A builder which encodes info into PING frame.
#[derive(CopyGetters, Setters)]
pub struct PingEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    ack: bool,
    #[getset(get_copy = "pub", set = "pub")]
    opaque_data: u64,
}

impl std::fmt::Debug for PingEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PingEncoder")
            .field("ack", &self.ack)
            .field("opaque_data", &self.opaque_data)
            .finish()
    }
}

impl PingEncoder {
    ///Creates.
    pub fn new() -> Self {
        Self {
            ack: false,
            opaque_data: 0,
        }
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        if self.ack {
            ACK_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let flags = self.flags();
        fill_header(
            PING_LENGTH as u32,
            PING_FRAME_TYPE,
            flags,
            STREAM_IDENTIFIER_ZERO,
            writer,
        );
        writer.put_u64(self.opaque_data)
    }
}

///A builder which encodes info into GOAWAY frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct GoawayEncoder {
    #[getset(get_copy = "pub", set = "pub")]
    last_stream_id: u32,
    #[getset(get_copy = "pub", set = "pub")]
    error_code: u32,
    #[getset(get = "pub", get_mut = "pub")]
    additional_debug_data: Vec<u8>,
}

impl std::fmt::Debug for GoawayEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoawayEncoder")
            .field("last_stream_id", &self.last_stream_id)
            .field("error_code", &self.error_code)
            .field(
                "additional_debug_data len",
                &self.additional_debug_data.len(),
            )
            .finish()
    }
}

impl GoawayEncoder {
    ///Creates with capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            last_stream_id: 0,
            error_code: 0,
            additional_debug_data: Vec::with_capacity(check_capacity(capacity)),
        }
    }

    ///Creates with capacity 16,777,215.
    pub fn max() -> Self {
        Self::new(MAX_FRAME_LENGTH)
    }

    ///Returns None if the data length <= 16,777,215, otherwise returns a newly vector containing bytes in the range [16777215..].
    pub fn check_length(&mut self) -> Option<Vec<u8>> {
        let n = MAX_FRAME_LENGTH - 8;
        if self.additional_debug_data.len() > n {
            Some(self.additional_debug_data.split_off(n))
        } else {
            None
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let length = length(8 + self.additional_debug_data.len());
        fill_header(
            length,
            GOAWAY_FRAME_TYPE,
            UNUSED_FLAGS,
            STREAM_IDENTIFIER_ZERO,
            writer,
        );
        fill_stream_id(self.last_stream_id, writer);
        writer.put_u32(self.error_code);
        writer.put_all(&self.additional_debug_data)
    }
}

const WINDOW_UPDATE_LENGTH: usize = 0x04;

///A builder which encodes info into WINDOW_UPDATE frame.
#[derive(CopyGetters, Setters)]
pub struct WindowUpdateEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    window_size_increment: u32,
}

impl std::fmt::Debug for WindowUpdateEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowUpdateEncoder")
            .field("stream_identifier", &self.stream_identifier)
            .field("window_size_increment", &self.window_size_increment)
            .finish()
    }
}

impl WindowUpdateEncoder {
    ///Creates with a stream identifier.
    pub fn new(stream_identifier: u32) -> Self {
        Self {
            stream_identifier,
            window_size_increment: 0,
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        fill_header(
            WINDOW_UPDATE_LENGTH as u32,
            WINDOW_UPDATE_FRAME_TYPE,
            UNUSED_FLAGS,
            self.stream_identifier,
            writer,
        );
        writer.put_u32(self.window_size_increment)
    }
}

///A builder which encodes field block into CONTINUATION frame.
#[derive(CopyGetters, Getters, MutGetters, Setters)]
pub struct ContinuationEncoder {
    #[getset(get_copy = "pub")]
    stream_identifier: u32,
    #[getset(get_copy = "pub", set = "pub")]
    end_headers: bool,
    #[getset(get = "pub", get_mut = "pub")]
    field_block_fragment: Vec<u8>,
}

impl std::fmt::Debug for ContinuationEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContinuationEncoder")
            .field("stream_identifier", &self.stream_identifier)
            .field("end_headers", &self.end_headers)
            .field("field_block_fragment len", &self.field_block_fragment.len())
            .finish()
    }
}

impl ContinuationEncoder {
    ///Creates with a stream identifier and capacity.
    pub fn new(stream_identifier: u32, capacity: usize) -> Self {
        Self {
            stream_identifier,
            end_headers: false,
            field_block_fragment: Vec::with_capacity(check_capacity(capacity)),
        }
    }

    ///Creates with capacity 16,777,215.
    pub fn max(stream_identifier: u32) -> Self {
        Self::new(stream_identifier, MAX_FRAME_LENGTH)
    }

    #[inline(always)]
    fn flags(&self) -> u8 {
        if self.end_headers {
            END_HEADERS_FLAG
        } else {
            UNUSED_FLAGS
        }
    }

    ///Returns None if the data length <= 16,777,215, otherwise returns a newly vector containing bytes in the range [16777215..].
    pub fn check_length(&mut self) -> Option<Vec<u8>> {
        if self.field_block_fragment.len() > MAX_FRAME_LENGTH {
            Some(self.field_block_fragment.split_off(MAX_FRAME_LENGTH))
        } else {
            None
        }
    }

    ///Encodes self into sequential bytes, returning None if no error.
    pub fn encode(self, writer: &mut impl WriteByte) -> Option<Error> {
        let flags = self.flags();
        let length = length(self.field_block_fragment.len());
        fill_header(
            length,
            CONTINUATION_FRAME_TYPE,
            flags,
            self.stream_identifier,
            writer,
        );
        writer.put_all(&self.field_block_fragment)
    }
}

#[inline(always)]
fn bit_eq(i: u8, f: u8) -> bool {
    i & f == f
}

#[inline(always)]
fn get_31_uint(o: &[u8]) -> u32 {
    u32::from_be_bytes([o[0] & RESERVED, o[1], o[2], o[3]])
}

#[inline(always)]
fn get_header(o: &[u8]) -> (u32, u8, u8, u32) {
    let length = u32::from_be_bytes([0, o[0], o[1], o[2]]);
    let stream_identifier = get_31_uint(&o[5..9]);
    (length, o[3], o[4], stream_identifier)
}

#[inline(always)]
fn get_priority(o: &[u8]) -> (bool, u32, u8) {
    let exclusive = bit_eq(o[0], EXCLUSIVE);
    let stream_dependency = get_31_uint(&o[0..4]);
    (exclusive, stream_dependency, o[4])
}

#[inline(always)]
fn check_length(length: u32, v_len: usize, err: &mut HashSet<FrameError>) -> usize {
    let f_len = length as usize + FRAME_HEADER_LENGTH;
    if v_len == f_len {
    } else if v_len < f_len {
        err.insert(FrameError::LengthShortage);
    } else {
        err.insert(FrameError::LengthExcess);
    };
    f_len
}

///Frame error.
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum FrameError {
    InvalidFrameType,
    LengthShortage,
    LengthExcess,
}

///Frame decoder.
pub enum FrameDecoder<'a> {
    Data(DataDecoder<'a>),
    Headers(HeadersDecoder<'a>),
    Priority(PriorityDecoder<'a>),
    RstStream(RstStreamDecoder<'a>),
    Settings(SettingsDecoder<'a>),
    PushPromise(PushPromiseDecoder<'a>),
    Ping(PingDecoder<'a>),
    Goaway(GoawayDecoder<'a>),
    WindowUpdate(WindowUpdateDecoder<'a>),
    Continuation(ContinuationDecoder<'a>),
    Invalid(FrameError),
}

impl<'a> FrameDecoder<'a> {
    ///Returns a decoder depend on the frame type, or error.
    pub fn decode(buf: &'a [u8]) -> Self {
        if buf.len() >= FRAME_HEADER_LENGTH {
            match buf[3] {
                DATA_FRAME_TYPE => Self::Data(DataDecoder::decode(buf)),
                HEADERS_FRAME_TYPE => Self::Headers(HeadersDecoder::decode(buf)),
                PRIORITY_FRAME_TYPE => {
                    if buf.len() < FRAME_HEADER_LENGTH + PRIORITY_LENGTH {
                        Self::Invalid(FrameError::LengthShortage)
                    } else {
                        Self::Priority(PriorityDecoder::decode(buf))
                    }
                }
                RST_STREAM_FRAME_TYPE => {
                    if buf.len() < FRAME_HEADER_LENGTH + RST_STREAM_LENGTH {
                        Self::Invalid(FrameError::LengthShortage)
                    } else {
                        Self::RstStream(RstStreamDecoder::decode(buf))
                    }
                }
                SETTINGS_FRAME_TYPE => Self::Settings(SettingsDecoder::decode(buf)),
                PUSH_PROMISE_FRAME_TYPE => {
                    if buf.len() < 13 {
                        Self::Invalid(FrameError::LengthShortage)
                    } else {
                        Self::PushPromise(PushPromiseDecoder::decode(buf))
                    }
                }
                PING_FRAME_TYPE => {
                    if buf.len() < FRAME_HEADER_LENGTH + PING_LENGTH {
                        Self::Invalid(FrameError::LengthShortage)
                    } else {
                        Self::Ping(PingDecoder::decode(buf))
                    }
                }
                GOAWAY_FRAME_TYPE => {
                    if buf.len() < 17 {
                        Self::Invalid(FrameError::LengthShortage)
                    } else {
                        Self::Goaway(GoawayDecoder::decode(buf))
                    }
                }
                WINDOW_UPDATE_FRAME_TYPE => {
                    if buf.len() < FRAME_HEADER_LENGTH + WINDOW_UPDATE_LENGTH {
                        Self::Invalid(FrameError::LengthShortage)
                    } else {
                        Self::WindowUpdate(WindowUpdateDecoder::decode(buf))
                    }
                }
                CONTINUATION_FRAME_TYPE => Self::Continuation(ContinuationDecoder::decode(buf)),
                _ => Self::Invalid(FrameError::InvalidFrameType),
            }
        } else {
            Self::Invalid(FrameError::LengthShortage)
        }
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct DataDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    padded: bool,
    end_stream: bool,
    pad_length: u8,
    #[getset(skip)]
    data: (usize, usize),
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for DataDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut o = f.debug_struct("DataDecoder");
        o.field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("padded", &self.padded)
            .field("end_stream", &self.end_stream);
        if self.padded {
            o.field("pad_length", &self.pad_length);
        }
        o.field("err", &self.err).finish()
    }
}

impl<'a> DataDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, flags, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        let f_len = check_length(length, v_len, &mut err);

        let mut data = (FRAME_HEADER_LENGTH, f_len);
        let padded = bit_eq(flags, PADDED_FLAG);
        let mut pad_length = 0;
        if padded {
            data.0 += 1;
            if v_len > 9 {
                pad_length = v[9];
                data.1 = f_len.saturating_sub(pad_length as usize);
            } else {
                err.insert(FrameError::LengthShortage);
            }
        }

        Self {
            length,
            stream_identifier,
            padded,
            end_stream: bit_eq(flags, END_STREAM_FLAG),
            pad_length,
            data,
            buffer: v,
            err,
        }
    }

    ///Returns data.
    pub fn data(&self) -> Option<&[u8]> {
        self.buffer.get(self.data.0..self.data.1)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct HeadersDecoder<'a> {
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
    #[getset(skip)]
    field_block_fragment: (usize, usize),
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for HeadersDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut o = f.debug_struct("HeadersDecoder");
        o.field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("priority", &self.priority)
            .field("padded", &self.padded)
            .field("end_headers", &self.end_headers)
            .field("end_stream", &self.end_stream);
        if self.padded {
            o.field("pad_length", &self.pad_length);
        }
        if self.priority {
            o.field("exclusive", &self.exclusive)
                .field("stream_dependency", &self.stream_dependency)
                .field("weight", &self.weight);
        }
        o.field("err", &self.err).finish()
    }
}

impl<'a> HeadersDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, flags, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        let f_len = check_length(length, v_len, &mut err);

        let mut field_block_fragment = (FRAME_HEADER_LENGTH, f_len);
        let padded = bit_eq(flags, PADDED_FLAG);
        let mut pad_length = 0;
        if padded {
            field_block_fragment.0 += 1;
            if v_len > 9 {
                pad_length = v[9];
                field_block_fragment.1 = f_len.saturating_sub(pad_length as usize);
            } else {
                err.insert(FrameError::LengthShortage);
            }
        }
        let priority = bit_eq(flags, PRIORITY_FLAG);
        let mut exclusive = false;
        let mut stream_dependency = 0;
        let mut weight = 0;
        if priority {
            field_block_fragment.0 += 5;
            if padded {
                if v_len > 14 {
                    (exclusive, stream_dependency, weight) = get_priority(&v[10..15]);
                } else {
                    err.insert(FrameError::LengthShortage);
                }
            } else {
                if v_len > 13 {
                    (exclusive, stream_dependency, weight) = get_priority(&v[9..14]);
                } else {
                    err.insert(FrameError::LengthShortage);
                }
            }
        }

        Self {
            length,
            stream_identifier,
            priority,
            padded,
            end_headers: bit_eq(flags, END_HEADERS_FLAG),
            end_stream: bit_eq(flags, END_STREAM_FLAG),
            pad_length,
            exclusive,
            stream_dependency,
            weight,
            field_block_fragment,
            buffer: v,
            err,
        }
    }

    ///Returns field block fragment.
    pub fn field_block_fragment(&self) -> Option<&[u8]> {
        self.buffer
            .get(self.field_block_fragment.0..self.field_block_fragment.1)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }

    ///Decode field block fragment.
    ///
    ///You need an implementation of `DecodeInstructions`.
    ///
    ///If the END_HEADERS flag unset, the field block fragment is not a complete field section.
    pub fn decode_fields(&self, ins: &mut impl DecodeInstructions) {
        if let Some(o) = self.field_block_fragment() {
            super::hpack::decode(o, ins)
        }
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Setters)]
#[getset(get_copy = "pub")]
pub struct PriorityDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    exclusive: bool,
    stream_dependency: u32,
    weight: u8,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for PriorityDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PriorityDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("exclusive", &self.exclusive)
            .field("stream_dependency", &self.stream_dependency)
            .field("weight", &self.weight)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> PriorityDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, _, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(PRIORITY_LENGTH as u32, v_len, &mut err);

        let mut exclusive = false;
        let mut stream_dependency = 0;
        let mut weight = 0;
        if v_len >= 14 {
            (exclusive, stream_dependency, weight) = get_priority(&v[9..14]);
        } else {
            err.insert(FrameError::LengthShortage);
        }

        Self {
            length,
            stream_identifier,
            exclusive,
            stream_dependency,
            weight,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Setters)]
#[getset(get_copy = "pub")]
pub struct RstStreamDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    error_code: u32,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for RstStreamDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RstStreamDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("error_code", &self.error_code)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> RstStreamDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, _, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(RST_STREAM_LENGTH as u32, v_len, &mut err);

        let mut error_code = 0;
        if v_len >= 13 {
            error_code = u32::from_be_bytes([v[9], v[10], v[11], v[12]]);
        } else {
            err.insert(FrameError::LengthShortage);
        }

        Self {
            length,
            stream_identifier,
            error_code,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct SettingsDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    ack: bool,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for SettingsDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("ack", &self.ack)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> SettingsDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, flags, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(length, v_len, &mut err);

        Self {
            length,
            stream_identifier,
            ack: bit_eq(flags, ACK_FLAG),
            buffer: v,
            err,
        }
    }

    ///Returns setting.
    pub fn setting(&self) -> Option<&[u8]> {
        self.buffer.get(FRAME_HEADER_LENGTH..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }

    ///Decode setting.
    pub fn decode_setting(&self) -> Vec<(u16, u32)> {
        let mut v = Vec::new();
        if let Some(mut o) = self.setting() {
            while let Some(a) = o.fetch_u16() {
                if let Some(b) = o.fetch_u32() {
                    v.push((a, b))
                } else {
                    break;
                }
            }
        }
        v
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct PushPromiseDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    padded: bool,
    end_headers: bool,
    pad_length: u8,
    promised_stream_id: u32,
    #[getset(skip)]
    field_block_fragment: (usize, usize),
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for PushPromiseDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut o = f.debug_struct("PushPromiseDecoder");
        o.field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("padded", &self.padded)
            .field("end_headers", &self.end_headers);
        if self.padded {
            o.field("pad_length", &self.pad_length);
        }
        o.field("promised_stream_id", &self.promised_stream_id)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> PushPromiseDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, flags, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        let f_len = check_length(length, v_len, &mut err);

        let mut field_block_fragment = (FRAME_HEADER_LENGTH, f_len);
        let padded = bit_eq(flags, PADDED_FLAG);
        let mut pad_length = 0;
        let mut promised_stream_id = 0;
        if padded {
            field_block_fragment.0 += 1;
            if v_len > 9 {
                pad_length = v[9];
                field_block_fragment.1 = f_len.saturating_sub(pad_length as usize);
            } else if v_len >= 14 {
                promised_stream_id = get_31_uint(&v[10..14]);
            } else {
                err.insert(FrameError::LengthShortage);
            }
        } else {
            if v_len >= 13 {
                promised_stream_id = get_31_uint(&v[9..13]);
            } else {
                err.insert(FrameError::LengthShortage);
            }
        }

        Self {
            length,
            stream_identifier,
            padded,
            end_headers: bit_eq(flags, END_HEADERS_FLAG),
            pad_length,
            promised_stream_id,
            field_block_fragment,
            buffer: v,
            err,
        }
    }

    ///Returns field block fragment.
    pub fn field_block_fragment(&self) -> Option<&[u8]> {
        self.buffer
            .get(self.field_block_fragment.0..self.field_block_fragment.1)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }

    ///Decode field block fragment.
    ///
    ///You need an implementation of `DecodeInstructions`.
    ///
    ///If the END_HEADERS flag unset, the field block fragment is not a complete field section.
    pub fn decode_fields(&self, ins: &mut impl DecodeInstructions) {
        if let Some(o) = self.field_block_fragment() {
            super::hpack::decode(o, ins)
        }
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct PingDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    ack: bool,
    opaque_data: u64,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for PingDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PingDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("ack", &self.ack)
            .field("opaque_data", &self.opaque_data)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> PingDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, flags, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(PING_LENGTH as u32, v_len, &mut err);

        let opaque_data = if v_len >= 17 {
            u64::from_be_bytes([v[9], v[10], v[11], v[12], v[13], v[14], v[15], v[16]])
        } else {
            0
        };

        Self {
            length,
            stream_identifier,
            ack: bit_eq(flags, ACK_FLAG),
            opaque_data,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct GoawayDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    last_stream_id: u32,
    error_code: u32,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for GoawayDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoawayDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("last_stream_id", &self.last_stream_id)
            .field("error_code", &self.error_code)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> GoawayDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, _, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(length, v_len, &mut err);

        let last_stream_id = if v_len >= 13 {
            get_31_uint(&v[9..13])
        } else {
            err.insert(FrameError::LengthShortage);
            0
        };
        let error_code = if v_len >= 17 {
            u32::from_be_bytes([v[13], v[14], v[15], v[16]])
        } else {
            err.insert(FrameError::LengthShortage);
            0
        };

        Self {
            length,
            stream_identifier,
            last_stream_id,
            error_code,
            buffer: v,
            err,
        }
    }

    ///Returns additional debug data.
    pub fn additional_debug_data(&self) -> Option<&[u8]> {
        self.buffer.get(17..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct WindowUpdateDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    window_size_increment: u32,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for WindowUpdateDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowUpdateDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("window_size_increment", &self.window_size_increment)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> WindowUpdateDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, _, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(WINDOW_UPDATE_LENGTH as u32, v_len, &mut err);

        let window_size_increment = if v_len >= 13 {
            get_31_uint(&v[9..13])
        } else {
            0
        };

        Self {
            length,
            stream_identifier,
            window_size_increment,
            buffer: v,
            err,
        }
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }
}

///A builder which decodes sequential bytes into it.
#[derive(CopyGetters, Getters)]
#[getset(get_copy = "pub")]
pub struct ContinuationDecoder<'a> {
    length: u32,
    stream_identifier: u32,
    end_headers: bool,
    buffer: &'a [u8],
    #[getset(skip)]
    #[getset(get = "pub")]
    err: HashSet<FrameError>,
}

impl<'a> std::fmt::Debug for ContinuationDecoder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContinuationDecoder")
            .field("length", &self.length)
            .field("stream_identifier", &self.stream_identifier)
            .field("end_headers", &self.end_headers)
            .field("err", &self.err)
            .finish()
    }
}

impl<'a> ContinuationDecoder<'a> {
    fn decode(v: &'a [u8]) -> Self {
        let (length, _, flags, stream_identifier) = get_header(v);
        let v_len = v.len();
        let mut err = HashSet::new();
        check_length(length, v_len, &mut err);

        Self {
            length,
            stream_identifier,
            end_headers: bit_eq(flags, END_HEADERS_FLAG),
            buffer: v,
            err,
        }
    }

    ///Returns field block fragment.
    pub fn field_block_fragment(&self) -> Option<&[u8]> {
        self.buffer.get(FRAME_HEADER_LENGTH..)
    }

    ///Returns true if the err is empty.
    pub fn is_correct(&self) -> bool {
        self.err.is_empty()
    }

    ///Decode field block fragment.
    ///
    ///You need an implementation of `DecodeInstructions`.
    ///
    ///If the END_HEADERS flag unset, the field block fragment is not a complete field section.
    pub fn decode_fields(&self, ins: &mut impl DecodeInstructions) {
        if let Some(o) = self.field_block_fragment() {
            super::hpack::decode(o, ins)
        }
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
        let mut o = DataEncoder::max(i);
        o.set_end_stream(true);
        o.set_padded(true);
        o.set_pad_length(a);
        o.data_mut().resize(b, 1);
        let mut writer = Vec::new();
        o.encode(&mut writer);

        match FrameDecoder::decode(&writer) {
            FrameDecoder::Data(o) => {
                assert_eq!(o.stream_identifier(), i);
                assert_eq!(o.end_stream(), true);
                assert_eq!(o.padded(), true);
                assert_eq!(o.pad_length(), a);
                assert_eq!(o.data().map(|i| i.len()).unwrap_or(0), b);
            }
            _ => {}
        }
    }
}
