use super::huffman::*;
use crate::common::get_bytes;
use bytes::{Buf, BufMut};

///Represent an integer 'i' on 'w' bits, with prefix 'p'.
pub(crate) fn encode_integer(mut i: usize, w: u8, p: u8, writer: &mut impl BufMut) {
    if w < 1 || w >= 8 {
        return;
    }
    let a = (1 << w) - 1;
    if i < a {
        writer.put_u8((i as u8) | p);
    } else {
        writer.put_u8((a as u8) | p);
        i -= a;
        while i >= 128 {
            writer.put_u8((i % 128 + 128) as u8);
            i /= 128;
        }
        writer.put_u8(i as u8);
    }
}

///Decode an integer representation.
pub(crate) fn decode_integer(mut n: usize, reader: &mut impl Buf) -> usize {
    let mut m = 0;
    while reader.has_remaining() {
        let i = reader.get_u8();
        n += (i & 0x7f) as usize * (1 << m);
        m += 7;
        if i & 0x80 == 0x00 {
            break;
        }
    }
    n
}

///Represent string literal as octets of huffman encoded.
pub(crate) fn encode_literal_huffman_encoded(reader: &[u8], writer: &mut impl BufMut) {
    let mut v = Vec::new();
    encode_huffman(reader, &mut v);
    let i = v.len();
    encode_integer(i, 7, 0x80, writer);
    writer.put_slice(&v);
}

///Represent string literal as raw octets.
pub(crate) fn encode_literal(reader: &[u8], writer: &mut impl BufMut) {
    let i = reader.len();
    encode_integer(i, 7, 0x00, writer);
    writer.put_slice(reader);
}

///Decode string literal representation
pub(crate) fn decode_literal(reader: &mut impl Buf, writer: &mut impl BufMut) {
    if reader.has_remaining() {
        let i = reader.get_u8();
        match i {
            128..255 => {
                let r = (i & 0x7f) as usize;
                decode_huffman(&get_bytes(r, reader), writer);
            }
            255 => {
                let r = decode_integer(127, reader);
                decode_huffman(&get_bytes(r, reader), writer);
            }
            0..127 => {
                let r = i as usize;
                writer.put_slice(&get_bytes(r, reader));
            }
            127 => {
                let r = decode_integer(127, reader);
                writer.put_slice(&get_bytes(r, reader));
            }
        }
    }
}

///Decode string literal representation to vec.
pub(crate) fn decode_literal_to_vec(reader: &mut impl Buf) -> Vec<u8> {
    let mut v = Vec::new();
    decode_literal(reader, &mut v);
    v
}
