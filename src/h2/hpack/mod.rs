/*!
Utilities for field section compression and decompression.

# Compression
Field section compression is the process of compressing a set of field lines to form a field block.

The build_* functions can be used to encode different field representations to a sequence of octets.

# Decompression
Field section decompression is the process of decoding a field block into a set of field lines.

The get_* functions can be used to parse a field block and the dynamic table size update instruction.
The [`HfriResult`] represent different parsing result.

# Index
This module provides static_table_* functions for working with static table.
*/

mod index;

use super::prty::*;
use super::*;
pub use index::*;

///Builds field representation.
///An indexed header field representation identifies an entry in either the static table or the dynamic table.
#[inline]
pub fn build_indexed(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 7, 0x80, o);
}

///Builds field representation.
///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
#[inline]
pub fn build_incremental_indexing_indexed_name(n: usize, value: OctetsRef, o: &mut dyn PutU8) {
    encode_integer(n, 6, 0x40, o);
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field representation.
///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
#[inline]
pub fn build_incremental_indexing_new_name(name: OctetsRef, value: OctetsRef, o: &mut dyn PutU8) {
    o.put_u8(0x40);
    if name.huffman() {
        encode_literal_huffman(&name, o);
    } else {
        encode_literal(&name, o);
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field representation.
///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
#[inline]
pub fn build_without_indexing_indexed_name(n: usize, value: OctetsRef, o: &mut dyn PutU8) {
    encode_integer(n, 4, 0x00, o);
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field representation.
///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
#[inline]
pub fn build_without_indexing_new_name(name: OctetsRef, value: OctetsRef, o: &mut dyn PutU8) {
    o.put_u8(0x00);
    if name.huffman() {
        encode_literal_huffman(&name, o);
    } else {
        encode_literal(&name, o);
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field representation.
///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
///Intermediaries MUST use the same representation for encoding this header field.
#[inline]
pub fn build_never_indexed_indexed_name(n: usize, value: OctetsRef, o: &mut dyn PutU8) {
    encode_integer(n, 4, 0x10, o);
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field representation.
///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
///Intermediaries MUST use the same representation for encoding this header field.
#[inline]
pub fn build_never_indexed_new_name(name: OctetsRef, value: OctetsRef, o: &mut dyn PutU8) {
    o.put_u8(0x10);
    if name.huffman() {
        encode_literal_huffman(&name, o);
    } else {
        encode_literal(&name, o);
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds instruction.
///A dynamic table size update signals a change to the size of the dynamic table.
#[inline]
pub fn build_dynamic_table_size_update(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 5, 0x20, o);
}

///Represents a result of parsing header field representation or instruction.
#[repr(u8)]
pub enum HfriResult {
    ///An indexed header field representation identifies an entry in either the static table or the dynamic table.
    Indexed(usize),

    ///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
    IncrementalIndexingIndexedName(usize, FieldValue),

    ///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
    IncrementalIndexingNewName(FieldName, FieldValue),

    ///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
    WithoutIndexingIndexedName(usize, FieldValue),

    ///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
    WithoutIndexingNewName(FieldName, FieldValue),

    ///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
    ///Intermediaries MUST use the same representation for encoding this header field.
    NeverIndexedIndexedName(usize, FieldValue),

    ///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
    ///Intermediaries MUST use the same representation for encoding this header field.
    NeverIndexedNewName(FieldName, FieldValue),

    ///A dynamic table size update signals a change to the size of the dynamic table.
    DynamicTableSizeUpdate(usize),
}

///Parses some header field representations and instructions to vec.
///Returns an error if parsing failure.
#[inline]
pub fn get_hfris_to_vec(o: &mut dyn GetU8) -> Result<Vec<HfriResult>, &'static str> {
    let mut v = Vec::new();
    get_hfris(|r| v.push(r), o)?;
    Ok(v)
}

///Parses some header field representations and instructions.
///Returns an error if parsing failure.
#[inline]
pub fn get_hfris(mut f: impl FnMut(HfriResult), o: &mut dyn GetU8) -> Result<(), &'static str> {
    while o.is_surplus() {
        f(get_one_hfri(o)?);
    }
    Ok(())
}

///Parses a header field representation or instruction.
///Returns an error if parsing failure.
#[inline(always)]
pub fn get_one_hfri(o: &mut dyn GetU8) -> Result<HfriResult, &'static str> {
    let i = o.get_u8().ok_or(READ_BYTE_ERROR)?;
    let r = match i {
        129..255 => HfriResult::Indexed((i & 0x7f) as usize),
        255 => HfriResult::Indexed(decode_integer(127, o)?),
        128 => return Err("error: The index value of 0 is not used."),
        65..127 => {
            let value = decode_literal(o)?.into();
            HfriResult::IncrementalIndexingIndexedName((i & 0x3f) as usize, value)
        }
        127 => {
            let n = decode_integer(63, o)?;
            let value = decode_literal(o)?.into();
            HfriResult::IncrementalIndexingIndexedName(n, value)
        }
        64 => {
            let name = decode_literal(o)?.into();
            let value = decode_literal(o)?.into();
            HfriResult::IncrementalIndexingNewName(name, value)
        }
        1..15 => {
            let value = decode_literal(o)?.into();
            HfriResult::WithoutIndexingIndexedName(i as usize, value)
        }
        15 => {
            let n = decode_integer(15, o)?;
            let value = decode_literal(o)?.into();
            HfriResult::WithoutIndexingIndexedName(n, value)
        }
        0 => {
            let name = decode_literal(o)?.into();
            let value = decode_literal(o)?.into();
            HfriResult::WithoutIndexingNewName(name, value)
        }
        17..31 => {
            let value = decode_literal(o)?.into();
            HfriResult::NeverIndexedIndexedName((i & 0x0f) as usize, value)
        }
        31 => {
            let n = decode_integer(15, o)?;
            let value = decode_literal(o)?.into();
            HfriResult::NeverIndexedIndexedName(n, value)
        }
        16 => {
            let name = decode_literal(o)?.into();
            let value = decode_literal(o)?.into();
            HfriResult::NeverIndexedNewName(name, value)
        }
        32..63 => HfriResult::DynamicTableSizeUpdate((i & 0x1f) as usize),
        63 => HfriResult::DynamicTableSizeUpdate(decode_integer(31, o)?),
    };
    Ok(r)
}
