use super::huffman::*;
use crate::{ReadByte, WriteByte};

///Represent an integer 'i' on 'w' bits, with prefix 'p'.
#[inline]
pub(crate) fn encode_integer(mut i: usize, w: u8, p: u8, writer: &mut impl WriteByte) {
    if w < 1 || w >= 8 {
        return;
    }
    let a = (1 << w) - 1;
    if i < a {
        writer.put((i as u8) | p);
    } else {
        writer.put((a as u8) | p);
        i -= a;
        while i >= 128 {
            writer.put((i % 128 + 128) as u8);
            i /= 128;
        }
        writer.put(i as u8);
    }
}

#[inline]
pub(crate) fn decode_integer(mut n: usize, reader: &mut impl ReadByte) -> usize {
    let mut m = 0;
    while let Some(i) = reader.fetch() {
        n += (i & 0x7f) as usize * (1 << m);
        m += 7;
        if i & 0x80 == 0x00 {
            break;
        }
    }
    n
}

#[inline]
pub(crate) fn encode_literal_huffman_encoded(reader: &[u8], writer: &mut impl WriteByte) {
    let mut v = Vec::new();
    encode_huffman(reader, &mut v);
    let i = v.len();
    encode_integer(i, 7, 0x80, writer);
    writer.put_all(&v);
}

#[inline]
pub(crate) fn encode_literal(reader: &[u8], writer: &mut impl WriteByte) {
    let i = reader.len();
    encode_integer(i, 7, 0x00, writer);
    writer.put_all(reader);
}

#[inline]
pub(crate) fn decode_literal(reader: &mut impl ReadByte, writer: &mut impl WriteByte) {
    if let Some(i) = reader.fetch() {
        match i {
            128..255 => {
                let r = (i & 0x7f) as usize;
                if let Some(o) = reader.fetch_all(r) {
                    decode_huffman(o, writer);
                }
            }
            255 => {
                let r = decode_integer(127, reader);
                if let Some(o) = reader.fetch_all(r) {
                    decode_huffman(o, writer);
                }
            }
            0..127 => {
                let r = i as usize;
                if let Some(o) = reader.fetch_all(r) {
                    writer.put_all(o);
                }
            }
            127 => {
                let r = decode_integer(127, reader);
                if let Some(o) = reader.fetch_all(r) {
                    writer.put_all(o);
                }
            }
        }
    }
}

#[inline]
pub(crate) fn decode_literal_to_vec(reader: &mut impl ReadByte) -> Vec<u8> {
    let mut v = Vec::new();
    decode_literal(reader, &mut v);
    v
}
