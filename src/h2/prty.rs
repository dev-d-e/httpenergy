use super::huffman::*;
use crate::io::get_bytes;
use crate::{ReadByte, WriteByte};

///Represent an integer 'i' on 'w' bits, with prefix 'p'.
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

///Decode an integer representation.
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

///Represent string literal as octets of huffman encoded.
pub(crate) fn encode_literal_huffman_encoded(reader: &[u8], writer: &mut impl WriteByte) {
    let mut v = Vec::new();
    encode_huffman(reader, &mut v);
    let i = v.len();
    encode_integer(i, 7, 0x80, writer);
    writer.put_all(&v);
}

///Represent string literal as raw octets.
pub(crate) fn encode_literal(reader: &[u8], writer: &mut impl WriteByte) {
    let i = reader.len();
    encode_integer(i, 7, 0x00, writer);
    writer.put_all(reader);
}

///Decode string literal representation
pub(crate) fn decode_literal(reader: &mut impl ReadByte, writer: &mut impl WriteByte) {
    if let Some(i) = reader.fetch() {
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
                writer.put_all(&get_bytes(r, reader));
            }
            127 => {
                let r = decode_integer(127, reader);
                writer.put_all(&get_bytes(r, reader));
            }
        }
    }
}

///Decode string literal representation to vec.
pub(crate) fn decode_literal_to_vec(reader: &mut impl ReadByte) -> Vec<u8> {
    let mut v = Vec::new();
    decode_literal(reader, &mut v);
    v
}
