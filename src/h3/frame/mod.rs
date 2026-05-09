/*!
Utilities for the frame.

This module provides several types and functions for working with frames.

Each frame type can be created, then use export method.

To parse a frame, you can use [`get_frame`], returns a specific frame type.
*/

use super::prty::*;
use super::qpack::*;
use super::*;
use derive_more::{Debug, Deref, DerefMut};
use std::num::NonZeroUsize;

const DATA_FRAME_TYPE: u8 = 0x00;
const HEADERS_FRAME_TYPE: u8 = 0x01;
const CANCEL_PUSH_FRAME_TYPE: u8 = 0x03;
const SETTINGS_FRAME_TYPE: u8 = 0x04;
const PUSH_PROMISE_FRAME_TYPE: u8 = 0x05;
const GOAWAY_FRAME_TYPE: u8 = 0x07;
const MAX_PUSH_ID_FRAME_TYPE: u8 = 0x0d;

#[inline(always)]
fn check_capacity(capacity: u64) -> NonZeroUsize {
    let n = match capacity {
        0 => 4096,
        _ => capacity,
    } as usize;
    unsafe { NonZeroUsize::new_unchecked(n) }
}

#[inline(always)]
fn fill_header(frame_type: u8, length: usize, o: &mut dyn PutU8) {
    o.put_u8(frame_type);
    encode_u64(length as u64, o)
}

///Represents a DATA frame.
#[derive(Debug, Deref, DerefMut)]
pub struct Data {
    #[deref]
    #[deref_mut]
    data: FiniteVec,
}

impl Data {
    ///Creates.
    pub fn new(capacity: u64) -> Self {
        Self {
            data: check_capacity(capacity).into(),
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        fill_header(DATA_FRAME_TYPE, self.data.len(), o);
        o.put_exact(&self.data);
    }
}

///Represents a HEADERS frame.
#[derive(Debug, Deref, DerefMut)]
pub struct Headers {
    #[deref]
    #[deref_mut]
    encoded_field_section: FiniteVec,
}

impl Headers {
    ///Creates.
    pub fn new(capacity: u64) -> Self {
        Self {
            encoded_field_section: check_capacity(capacity).into(),
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        fill_header(HEADERS_FRAME_TYPE, self.encoded_field_section.len(), o);
        o.put_exact(&self.encoded_field_section);
    }
}

///Represents a CANCEL_PUSH frame.
#[derive(CopyGetters, Debug, Setters)]
pub struct CancelPush {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
}

impl CancelPush {
    ///Creates.
    pub fn new(push_id: u64) -> Self {
        Self { push_id }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let push_id = u64_to_var(self.push_id);
        fill_header(CANCEL_PUSH_FRAME_TYPE, push_id.len(), o);
        o.put_exact(&push_id);
    }
}

///Represents a SETTINGS frame.
#[derive(Debug, Deref, DerefMut)]
pub struct Settings {
    #[deref]
    #[deref_mut]
    setting: Vec<(u64, u64)>,
}

impl Settings {
    ///Creates.
    pub fn new(capacity: usize) -> Self {
        Self {
            setting: Vec::with_capacity(capacity),
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let mut v = Vec::with_capacity(self.setting.len() * 16);
        for (a, b) in self.setting {
            encode_u64(a, &mut v);
            encode_u64(b, &mut v);
        }
        fill_header(SETTINGS_FRAME_TYPE, v.len(), o);
        o.put_exact(&v);
    }
}

///Represents a PUSH_PROMISE frame.
#[derive(CopyGetters, Debug, Deref, DerefMut, Setters)]
pub struct PushPromise {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
    #[deref]
    #[deref_mut]
    encoded_field_section: FiniteVec,
}

impl PushPromise {
    ///Creates.
    pub fn new(capacity: u64) -> Self {
        Self {
            push_id: 0,
            encoded_field_section: check_capacity(capacity).into(),
        }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let push_id = u64_to_var(self.push_id);
        fill_header(
            PUSH_PROMISE_FRAME_TYPE,
            push_id.len() + self.encoded_field_section.len(),
            o,
        );
        o.put_exact(&push_id);
        o.put_exact(&self.encoded_field_section);
    }
}

///Represents a GOAWAY frame.
#[derive(CopyGetters, Debug, Setters)]
pub struct Goaway {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
}

impl Goaway {
    ///Creates.
    pub fn new(push_id: u64) -> Self {
        Self { push_id }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let push_id = u64_to_var(self.push_id);
        fill_header(GOAWAY_FRAME_TYPE, push_id.len(), o);
        o.put_exact(&push_id);
    }
}

///Represents a MAX_PUSH_ID frame.
#[derive(CopyGetters, Debug, Setters)]
pub struct MaxPushId {
    #[getset(get_copy = "pub", set = "pub")]
    push_id: u64,
}

impl MaxPushId {
    ///Creates.
    pub fn new(push_id: u64) -> Self {
        Self { push_id }
    }

    ///Exports self into [`PutU8`].
    pub fn export(self, o: &mut dyn PutU8) {
        let push_id = u64_to_var(self.push_id);
        fill_header(MAX_PUSH_ID_FRAME_TYPE, push_id.len(), o);
        o.put_exact(&push_id);
    }
}

///Represents a result of parsing frame.
#[repr(u8)]
pub enum FrameResult {
    Data(DataResult),
    Headers(HeadersResult),
    CancelPush(CancelPush),
    Settings(Settings),
    PushPromise(PushPromiseResult),
    Goaway(Goaway),
    MaxPushId(MaxPushId),
}

///Parses bytes. Returns a frame, or error.
pub fn get_frame(o: &mut dyn GetU8) -> Result<FrameResult, &str> {
    let i = decode_var(o).ok_or("empty")?;
    if i >= 256 {
        return Err("invalid type");
    }
    let length = decode_var(o).ok_or("invalid length")?;
    match i as u8 {
        DATA_FRAME_TYPE => {
            let i_b = o.index();
            let i_c = i_b + length as usize;
            o.set_index(i_c);
            Ok(FrameResult::Data(DataResult(i_b, i_c)))
        }
        HEADERS_FRAME_TYPE => {
            let i_b = o.index();
            let i_c = i_b + length as usize;
            o.set_index(i_c);
            Ok(FrameResult::Headers(HeadersResult(i_b, i_c)))
        }
        CANCEL_PUSH_FRAME_TYPE => {
            if length == 0 {
                return Err("shortage");
            }
            let k = o.get_exact(length as usize).ok_or("shortage")?;
            let push_id = decode_slice_var(k).ok_or("invalid integer")?;
            Ok(FrameResult::CancelPush(CancelPush { push_id }))
        }
        SETTINGS_FRAME_TYPE => {
            let mut setting = Vec::new();
            if length > 0 {
                let mut k = o.get_exact_to(length as usize).ok_or("shortage")?;
                let k = k.as_mut();
                while k.is_surplus() {
                    let a = decode_var(k).ok_or("invalid integer")?;
                    let b = decode_var(k).ok_or("invalid integer")?;
                    setting.push((a, b));
                }
            }
            Ok(FrameResult::Settings(Settings { setting }))
        }
        PUSH_PROMISE_FRAME_TYPE => {
            if length == 0 {
                return Err("shortage");
            }
            let i_b = o.index();
            let i_c = i_b + length as usize;
            let k = o.get_exact(length as usize).ok_or("shortage")?;
            let push_id = decode_slice_var(k).ok_or("invalid integer")?;
            o.set_index(i_c);
            Ok(FrameResult::PushPromise(PushPromiseResult(
                push_id, i_b, i_c,
            )))
        }
        GOAWAY_FRAME_TYPE => {
            if length == 0 {
                return Err("shortage");
            }
            let k = o.get_exact(length as usize).ok_or("shortage")?;
            let push_id = decode_slice_var(k).ok_or("invalid integer")?;
            Ok(FrameResult::Goaway(Goaway { push_id }))
        }
        MAX_PUSH_ID_FRAME_TYPE => {
            if length == 0 {
                return Err("shortage");
            }
            let k = o.get_exact(length as usize).ok_or("shortage")?;
            let push_id = decode_slice_var(k).ok_or("invalid integer")?;
            Ok(FrameResult::MaxPushId(MaxPushId { push_id }))
        }
        _ => Err("invalid type"),
    }
}

///Represents a parsed DATA frame.
pub struct DataResult(usize, usize);

impl DataResult {
    ///Returns data.
    pub fn data<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.0, self.1)
    }

    ///Parses data.
    pub fn get_field(&self, o: &mut dyn GetU8) -> Result<Vec<FsectionResult>, &str> {
        let mut r = self.data(o).ok_or(READ_BYTE_ERROR)?;
        get_fsection_to_vec(r.as_mut())
    }

    pub fn into(self, o: &mut dyn GetU8) -> Option<Data> {
        let mut o = self.data(o)?;
        let r = o.get_surplus().to_vec();
        let data = (check_capacity(r.len() as u64), r).into();
        Some(Data { data })
    }
}

///Represents a parsed HEADERS frame.
pub struct HeadersResult(usize, usize);

impl HeadersResult {
    ///Returns encoded field section.
    pub fn encoded_field_section<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.0, self.1)
    }

    ///Parses encoded field section.
    pub fn get_field(&self, o: &mut dyn GetU8) -> Result<Vec<FsectionResult>, &str> {
        let mut r = self.encoded_field_section(o).ok_or(READ_BYTE_ERROR)?;
        get_fsection_to_vec(r.as_mut())
    }

    pub fn into(self, o: &mut dyn GetU8) -> Option<Headers> {
        let mut o = self.encoded_field_section(o)?;
        let r = o.get_surplus().to_vec();
        let encoded_field_section = (check_capacity(r.len() as u64), r).into();
        Some(Headers {
            encoded_field_section,
        })
    }
}

///Represents a parsed PUSH_PROMISE frame.
pub struct PushPromiseResult(u64, usize, usize);

impl PushPromiseResult {
    ///Returns encoded field section.
    pub fn encoded_field_section<'a>(&self, o: &'a mut dyn GetU8) -> Option<Box<dyn GetU8 + 'a>> {
        o.sub_to(self.1, self.2)
    }

    ///Parses encoded field section.
    pub fn get_field(&self, o: &mut dyn GetU8) -> Result<Vec<FsectionResult>, &str> {
        let mut r = self.encoded_field_section(o).ok_or(READ_BYTE_ERROR)?;
        get_fsection_to_vec(r.as_mut())
    }

    pub fn into(self, o: &mut dyn GetU8) -> Option<PushPromise> {
        let mut o = self.encoded_field_section(o)?;
        let r = o.get_surplus().to_vec();
        let encoded_field_section = (check_capacity(r.len() as u64), r).into();
        Some(PushPromise {
            push_id: self.0,
            encoded_field_section,
        })
    }
}
