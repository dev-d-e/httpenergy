use crate::{ReadByte, WriteByte};

const USABLE_BITS: u8 = 0b0011_1111;
const TWO_MOST_SIGNIFICANT_BITS_01: u8 = 0b0100_0000;
const TWO_MOST_SIGNIFICANT_BITS_10: u8 = 0b1000_0000;
const TWO_MOST_SIGNIFICANT_BITS_11: u8 = 0b1100_0000;

#[inline]
pub(super) fn encode_var(i: u64, p: u8, writer: &mut impl WriteByte) {
    match p {
        0 => {
            writer.put(i as u8 & USABLE_BITS);
        }
        1 => {
            writer.put((i >> 8) as u8 & USABLE_BITS | TWO_MOST_SIGNIFICANT_BITS_01);
            writer.put(i as u8);
        }
        2 => {
            writer.put((i >> 24) as u8 & USABLE_BITS | TWO_MOST_SIGNIFICANT_BITS_10);
            writer.put((i >> 16) as u8);
            writer.put((i >> 8) as u8);
            writer.put(i as u8);
        }
        3 => {
            writer.put((i >> 56) as u8 | TWO_MOST_SIGNIFICANT_BITS_11);
            writer.put((i >> 48) as u8);
            writer.put((i >> 40) as u8);
            writer.put((i >> 32) as u8);
            writer.put((i >> 24) as u8);
            writer.put((i >> 16) as u8);
            writer.put((i >> 8) as u8);
            writer.put(i as u8);
        }
        _ => {}
    }
}

#[inline]
pub(super) fn decode_var(reader: &mut impl ReadByte) -> u64 {
    if let Some(i) = reader.fetch() {
        let prefix = i >> 6;
        let mut length = 1 << prefix;
        let mut v = (i & USABLE_BITS) as u64;
        length -= 1;
        for _ in 0..length {
            if let Some(i) = reader.fetch() {
                v = (v << 8) | i as u64;
            }
        }
        v
    } else {
        0
    }
}

#[inline]
pub(super) fn encode_u64(n: u64, writer: &mut impl WriteByte) {
    match n {
        ..=63 => {
            encode_var(n, 0, writer);
        }
        ..=16383 => {
            encode_var(n, 1, writer);
        }
        ..=1073741823 => {
            encode_var(n, 2, writer);
        }
        ..=4611686018427387903 => {
            encode_var(n, 3, writer);
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
pub(super) fn u64_2_to_var(a: u64, b: u64) -> Vec<u8> {
    let mut vec = Vec::with_capacity(16);
    encode_u64(a, &mut vec);
    encode_u64(b, &mut vec);
    vec
}

#[inline]
pub(crate) fn decode_n_literal_to_vec(n: usize, buf: &mut impl ReadByte) -> Vec<u8> {
    let mut vec = Vec::new();
    if let Some(o) = buf.fetch_all(n) {
        vec.put_all(o);
    }
    vec
}
