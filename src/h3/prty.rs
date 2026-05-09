use super::*;
use crate::h2::huffman::*;
use crate::h2::prty::encode_integer;

const USABLE_BITS: u8 = 0b0011_1111;
const TWO_MOST_SIGNIFICANT_BITS_01: u8 = 0b0100_0000;
const TWO_MOST_SIGNIFICANT_BITS_10: u8 = 0b1000_0000;
const TWO_MOST_SIGNIFICANT_BITS_11: u8 = 0b1100_0000;

#[inline]
pub(super) fn encode_var(i: u64, p: u8, o: &mut dyn PutU8) {
    match p {
        0 => {
            o.put_u8(i as u8 & USABLE_BITS);
        }
        1 => {
            o.put_u8((i >> 8) as u8 & USABLE_BITS | TWO_MOST_SIGNIFICANT_BITS_01);
            o.put_u8(i as u8);
        }
        2 => {
            o.put_u8((i >> 24) as u8 & USABLE_BITS | TWO_MOST_SIGNIFICANT_BITS_10);
            o.put_u8((i >> 16) as u8);
            o.put_u8((i >> 8) as u8);
            o.put_u8(i as u8);
        }
        3 => {
            o.put_u8((i >> 56) as u8 | TWO_MOST_SIGNIFICANT_BITS_11);
            o.put_u8((i >> 48) as u8);
            o.put_u8((i >> 40) as u8);
            o.put_u8((i >> 32) as u8);
            o.put_u8((i >> 24) as u8);
            o.put_u8((i >> 16) as u8);
            o.put_u8((i >> 8) as u8);
            o.put_u8(i as u8);
        }
        _ => {}
    }
}

#[inline]
pub(super) fn decode_var(o: &mut dyn GetU8) -> Option<u64> {
    let i = o.get_u8()?;
    let prefix = i >> 6;
    let mut length = 1 << prefix;
    let mut v = (i & USABLE_BITS) as u64;
    length -= 1;
    for _ in 0..length {
        if let Some(i) = o.get_u8() {
            v = (v << 8) | i as u64;
        }
    }
    Some(v)
}

#[inline]
pub(super) fn decode_slice_var(s: &[u8]) -> Option<u64> {
    decode_var(&mut s.into_get())
}

#[inline]
pub(super) fn encode_u64(n: u64, o: &mut dyn PutU8) {
    match n {
        ..=63 => {
            encode_var(n, 0, o);
        }
        ..=16383 => {
            encode_var(n, 1, o);
        }
        ..=1073741823 => {
            encode_var(n, 2, o);
        }
        ..=4611686018427387903 => {
            encode_var(n, 3, o);
        }
        _ => {}
    }
}

#[inline]
pub(super) fn u64_to_var(a: u64) -> Vec<u8> {
    let mut vec = Vec::with_capacity(8);
    encode_u64(a, &mut vec);
    vec
}

#[inline]
pub(crate) fn encode_prefix_literal_huffman(s: &[u8], w: u8, p: u8, o: &mut dyn PutU8) {
    let mut v = Vec::new();
    encode_huffman(s, &mut v);
    let n = v.len();
    encode_integer(n, w, p, o);
    o.put_exact(&v);
}

#[inline]
pub(crate) fn encode_prefix_literal(s: &[u8], w: u8, p: u8, o: &mut dyn PutU8) {
    let n = s.len();
    encode_integer(n, w, p, o);
    o.put_exact(s);
}

#[inline]
pub(crate) fn decode_n_literal(n: usize, o: &mut dyn GetU8) -> Result<Vec<u8>, &'static str> {
    o.split_exact(n).ok_or(READ_BYTE_ERROR)
}

#[inline]
pub(crate) fn decode_n_huffman(n: usize, o: &mut dyn GetU8) -> Result<Vec<u8>, &'static str> {
    let r = o.get_exact(n).ok_or(READ_BYTE_ERROR)?;
    let mut v = Vec::with_capacity(r.len());
    decode_huffman(r, &mut v);
    Ok(v)
}
