/*!
Utilities for field section compression and decompression.

# Compression
Field section compression is the process of compressing a set of field lines to form a field block.

The [`Instructions`] type can be used to encode different field representations to a sequence of octets.

The [`FieldRep`] enum help to represent different field representations, you can use [`FieldRep::encode`] directly.

# Decompression
Field section decompression is the process of decoding a field block into a set of field lines.

To parse a field block, you need an implementation of [`DistributeInstructions`], then you can use [`Instructions::decode`] to decode bytes.

# Index
This module provides the [`Indices`] trait for working with indexing tables.
*/

mod index;

use super::prty::*;
use crate::{OctetsRef, ReadByte, WriteByte};
pub use index::*;

///Represents field. A header field can be represented in encoded form either as a literal or as an index.
pub enum FieldRep<'a> {
    Indexed(usize),
    IncrementalIndexingIndexedName(usize, OctetsRef<'a>),
    IncrementalIndexingNewName(OctetsRef<'a>, OctetsRef<'a>),
    WithoutIndexingIndexedName(usize, OctetsRef<'a>),
    WithoutIndexingNewName(OctetsRef<'a>, OctetsRef<'a>),
    NeverIndexedIndexedName(usize, OctetsRef<'a>),
    NeverIndexedNewName(OctetsRef<'a>, OctetsRef<'a>),
}

impl<'a> FieldRep<'a> {
    #[inline]
    pub fn encode(self, writer: &mut impl WriteByte) {
        match self {
            Self::Indexed(n) => {
                Instructions::indexed(n, writer);
            }
            Self::IncrementalIndexingIndexedName(n, value) => {
                Instructions::incremental_indexing_indexed_name(n, value, writer);
            }
            Self::IncrementalIndexingNewName(name, value) => {
                Instructions::incremental_indexing_new_name(name, value, writer);
            }
            Self::WithoutIndexingIndexedName(n, value) => {
                Instructions::without_indexing_indexed_name(n, value, writer);
            }
            Self::WithoutIndexingNewName(name, value) => {
                Instructions::without_indexing_new_name(name, value, writer);
            }
            Self::NeverIndexedIndexedName(n, value) => {
                Instructions::never_indexed_indexed_name(n, value, writer);
            }
            Self::NeverIndexedNewName(name, value) => {
                Instructions::never_indexed_new_name(name, value, writer);
            }
        }
    }
}

///Utilities for instructions.
///
///Builds index or literal header field to instructions.
///Include different header field representations and the dynamic table size update instruction.
///
///Decodes instruction bytes.
pub struct Instructions;

impl Instructions {
    ///An indexed header field representation identifies an entry in either the static table or the dynamic table.
    #[inline]
    pub fn indexed(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 7, 0x80, writer);
    }

    ///Builds with indexed name.
    ///
    ///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
    #[inline]
    pub fn incremental_indexing_indexed_name(
        n: usize,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        encode_integer(n, 6, 0x40, writer);
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Builds with new name.
    ///
    ///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
    #[inline]
    pub fn incremental_indexing_new_name(
        name: OctetsRef,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        writer.put(0x40);
        if name.huffman() {
            encode_literal_huffman_encoded(&name, writer);
        } else {
            encode_literal(&name, writer);
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Builds with indexed name.
    ///
    ///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
    #[inline]
    pub fn without_indexing_indexed_name(n: usize, value: OctetsRef, writer: &mut impl WriteByte) {
        encode_integer(n, 4, 0x00, writer);
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Builds with new name.
    ///
    ///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
    #[inline]
    pub fn without_indexing_new_name(
        name: OctetsRef,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        writer.put(0x00);
        if name.huffman() {
            encode_literal_huffman_encoded(&name, writer);
        } else {
            encode_literal(&name, writer);
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Builds with indexed name.
    ///
    ///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
    ///Intermediaries MUST use the same representation for encoding this header field.
    #[inline]
    pub fn never_indexed_indexed_name(n: usize, value: OctetsRef, writer: &mut impl WriteByte) {
        encode_integer(n, 4, 0x10, writer);
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Builds with new name.
    ///
    ///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
    ///Intermediaries MUST use the same representation for encoding this header field.
    #[inline]
    pub fn never_indexed_new_name(name: OctetsRef, value: OctetsRef, writer: &mut impl WriteByte) {
        writer.put(0x10);
        if name.huffman() {
            encode_literal_huffman_encoded(&name, writer);
        } else {
            encode_literal(&name, writer);
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///A dynamic table size update signals a change to the size of the dynamic table.
    #[inline]
    pub fn dynamic_table_size_update(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 5, 0x20, writer);
    }

    ///Decodes instruction bytes with an implementation of `DistributeInstructions`.
    #[inline]
    pub fn decode(reader: &mut impl ReadByte, ins: &mut impl DistributeInstructions) {
        while let Some(i) = reader.fetch() {
            decode_u8(i, reader, ins);
        }
    }
}

///A trait to parse instructions. distributes result.
pub trait DistributeInstructions {
    ///An indexed header field representation identifies an entry in either the static table or the dynamic table.
    fn indexed(&mut self, n: usize);

    ///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
    fn incremental_indexing_indexed_name(&mut self, n: usize, value: Vec<u8>);

    ///A literal header field with incremental indexing representation results in appending a header field to the decoded header list and inserting it as a new entry into the dynamic table.
    fn incremental_indexing_new_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
    fn without_indexing_indexed_name(&mut self, n: usize, value: Vec<u8>);

    ///A literal header field without indexing representation results in appending a header field to the decoded header list without altering the dynamic table.
    fn without_indexing_new_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
    ///Intermediaries MUST use the same representation for encoding this header field.
    fn never_indexed_indexed_name(&mut self, n: usize, value: Vec<u8>);

    ///A literal header field never-indexed representation results in appending a header field to the decoded header list without altering the dynamic table.
    ///Intermediaries MUST use the same representation for encoding this header field.
    fn never_indexed_new_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///A dynamic table size update signals a change to the size of the dynamic table.
    fn dynamic_table_size_update(&mut self, n: usize);
}

#[inline]
fn decode_u8(i: u8, reader: &mut impl ReadByte, ins: &mut impl DistributeInstructions) {
    match i {
        129..255 => {
            ins.indexed((i & 0x7f) as usize);
        }
        255 => {
            let r = decode_integer(127, reader);
            ins.indexed(r);
        }
        128 => {}
        65..127 => {
            let value = decode_literal_to_vec(reader);
            ins.incremental_indexing_indexed_name((i & 0x3f) as usize, value);
        }
        127 => {
            let r = decode_integer(63, reader);
            let value = decode_literal_to_vec(reader);
            ins.incremental_indexing_indexed_name(r, value);
        }
        64 => {
            let name = decode_literal_to_vec(reader);
            let value = decode_literal_to_vec(reader);
            ins.incremental_indexing_new_name(name, value);
        }
        1..15 => {
            let value = decode_literal_to_vec(reader);
            ins.without_indexing_indexed_name(i as usize, value);
        }
        15 => {
            let r = decode_integer(15, reader);
            let value = decode_literal_to_vec(reader);
            ins.without_indexing_indexed_name(r, value);
        }
        0 => {
            let name = decode_literal_to_vec(reader);
            let value = decode_literal_to_vec(reader);
            ins.without_indexing_new_name(name, value);
        }
        17..31 => {
            let value = decode_literal_to_vec(reader);
            ins.never_indexed_indexed_name((i & 0x0f) as usize, value);
        }
        31 => {
            let r = decode_integer(15, reader);
            let value = decode_literal_to_vec(reader);
            ins.never_indexed_indexed_name(r, value);
        }
        16 => {
            let name = decode_literal_to_vec(reader);
            let value = decode_literal_to_vec(reader);
            ins.never_indexed_new_name(name, value);
        }
        32..63 => {
            ins.dynamic_table_size_update((i & 0x1f) as usize);
        }
        63 => {
            let r = decode_integer(31, reader);
            ins.dynamic_table_size_update(r);
        }
    }
}
