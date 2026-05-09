use super::huffman::*;
use super::*;

///Represent an integer 'i' on 'w' bits, with prefix 'p'.
#[inline]
pub(crate) fn encode_integer(mut i: usize, w: u8, p: u8, o: &mut dyn PutU8) {
    if w < 1 || w > 8 {
        return;
    }
    let a = (1 << w) - 1;
    if i < a {
        o.put_u8((i as u8) | p);
    } else {
        o.put_u8((a as u8) | p);
        i -= a;
        while i >= 128 {
            o.put_u8((i % 128 + 128) as u8);
            i /= 128;
        }
        o.put_u8(i as u8);
    }
}

const DECODE_INTEGER_ERROR: &str = "HPACK integer error";

#[inline]
pub(crate) fn decode_integer(mut n: usize, o: &mut dyn GetU8) -> Result<usize, &'static str> {
    let mut m = 0;
    while let Some(i) = o.get_u8() {
        let a = (i & 0x7f) as usize * (1 << m);
        n = n.checked_add(a).ok_or(DECODE_INTEGER_ERROR)?;
        m += 7;
        if i & 0x80 == 0x00 {
            return Ok(n);
        }
    }
    Err(DECODE_INTEGER_ERROR)
}

#[inline]
pub(crate) fn encode_literal_huffman(s: &[u8], o: &mut dyn PutU8) {
    let mut v = Vec::new();
    encode_huffman(s, &mut v);
    let i = v.len();
    encode_integer(i, 7, 0x80, o);
    o.put_exact(&v);
}

#[inline]
pub(crate) fn encode_literal(s: &[u8], o: &mut dyn PutU8) {
    let i = s.len();
    encode_integer(i, 7, 0x00, o);
    o.put_exact(s);
}

#[inline]
pub(crate) fn decode_literal(o: &mut dyn GetU8) -> Result<Vec<u8>, &'static str> {
    let i = o.get_u8().ok_or(READ_BYTE_ERROR)?;
    match i {
        128..255 => {
            let r = (i & 0x7f) as usize;
            let r = o.get_exact(r).ok_or(READ_BYTE_ERROR)?;
            let mut v = Vec::with_capacity(r.len());
            decode_huffman(r, &mut v);
            Ok(v)
        }
        255 => {
            let r = decode_integer(127, o)?;
            let r = o.get_exact(r).ok_or(READ_BYTE_ERROR)?;
            let mut v = Vec::with_capacity(r.len());
            decode_huffman(r, &mut v);
            Ok(v)
        }
        0..127 => {
            let r = i as usize;
            o.split_exact(r).ok_or(READ_BYTE_ERROR)
        }
        127 => {
            let r = decode_integer(127, o)?;
            o.split_exact(r).ok_or(READ_BYTE_ERROR)
        }
    }
}
