mod index;

use super::prty::*;
use crate::{ReadByte, WriteByte};
pub use index::*;

///Represents header field.
pub enum FieldRepresentation<'a> {
    Indexed(usize),
    IndexedNameValue(FieldIndexType, usize, &'a [u8]),
    NewNameValue(FieldIndexType, &'a [u8], &'a [u8]),
}

impl<'a> FieldRepresentation<'a> {
    pub fn encode(self, writer: &mut impl WriteByte) {
        match self {
            Self::Indexed(n) => {
                EncodeInstructions::indexed(n, writer);
            }
            Self::IndexedNameValue(t, n, value) => {
                t.indexed_name(n, value, writer);
            }
            Self::NewNameValue(t, name, value) => {
                t.new_name(name, value, writer);
            }
        }
    }
}

///Represents index type of header field.
pub enum FieldIndexType {
    IncrementalIndexing,
    WithoutIndexing,
    NeverIndexed,
}

impl FieldIndexType {
    ///Indexed Name
    pub fn indexed_name(self, n: usize, value: &[u8], writer: &mut impl WriteByte) {
        match self {
            Self::IncrementalIndexing => {
                EncodeInstructions::incremental_indexing_indexed_name(n, value, writer);
            }
            Self::WithoutIndexing => {
                EncodeInstructions::without_indexing_indexed_name(n, value, writer);
            }
            Self::NeverIndexed => {
                EncodeInstructions::never_indexed_indexed_name(n, value, writer);
            }
        }
    }

    ///New Name
    pub fn new_name(self, name: &[u8], value: &[u8], writer: &mut impl WriteByte) {
        match self {
            Self::IncrementalIndexing => {
                EncodeInstructions::incremental_indexing_new_name(name, value, writer);
            }
            Self::WithoutIndexing => {
                EncodeInstructions::without_indexing_new_name(name, value, writer);
            }
            Self::NeverIndexed => {
                EncodeInstructions::never_indexed_new_name(name, value, writer);
            }
        }
    }
}

///Process index or literal header field to instructions.
///Include different header field representations and the dynamic table size update instruction.
pub struct EncodeInstructions;

impl EncodeInstructions {
    ///Indexed Header Field Representation.
    ///An indexed header field representation identifies an entry in either the static table or the dynamic table.
    pub fn indexed(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 7, 0x80, writer);
    }

    ///Literal Header Field with Incremental Indexing -- Indexed Name
    pub fn incremental_indexing_indexed_name(n: usize, value: &[u8], writer: &mut impl WriteByte) {
        encode_integer(n, 6, 0x40, writer);
        encode_literal_huffman_encoded(value, writer);
    }

    ///Literal Header Field with Incremental Indexing -- New Name
    pub fn incremental_indexing_new_name(name: &[u8], value: &[u8], writer: &mut impl WriteByte) {
        writer.put(0x40);
        encode_literal_huffman_encoded(name, writer);
        encode_literal_huffman_encoded(value, writer);
    }

    ///Literal Header Field without Indexing -- Indexed Name
    pub fn without_indexing_indexed_name(n: usize, value: &[u8], writer: &mut impl WriteByte) {
        encode_integer(n, 4, 0x00, writer);
        encode_literal_huffman_encoded(value, writer);
    }

    ///Literal Header Field without Indexing -- New Name
    pub fn without_indexing_new_name(name: &[u8], value: &[u8], writer: &mut impl WriteByte) {
        writer.put(0x00);
        encode_literal_huffman_encoded(name, writer);
        encode_literal_huffman_encoded(value, writer);
    }

    ///Literal Header Field Never Indexed -- Indexed Name
    pub fn never_indexed_indexed_name(n: usize, value: &[u8], writer: &mut impl WriteByte) {
        encode_integer(n, 4, 0x10, writer);
        encode_literal_huffman_encoded(value, writer);
    }

    ///Literal Header Field Never Indexed -- New Name
    pub fn never_indexed_new_name(name: &[u8], value: &[u8], writer: &mut impl WriteByte) {
        writer.put(0x10);
        encode_literal_huffman_encoded(name, writer);
        encode_literal_huffman_encoded(value, writer);
    }

    ///Maximum Dynamic Table Size Change
    pub fn dynamic_table_size_update(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 5, 0x20, writer);
    }
}

///Process decode instructions.
///Include different header field representations and the dynamic table size update instruction.
pub trait DecodeInstructions {
    ///Indexed Header Field Representation.
    ///An indexed header field representation identifies an entry in either the static table or the dynamic table.
    fn indexed(&mut self, n: usize);

    ///Literal Header Field with Incremental Indexing -- Indexed Name
    fn incremental_indexing_indexed_name(&mut self, n: usize, value: Vec<u8>);

    ///Literal Header Field with Incremental Indexing -- New Name
    fn incremental_indexing_new_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///Literal Header Field without Indexing -- Indexed Name
    fn without_indexing_indexed_name(&mut self, n: usize, value: Vec<u8>);

    ///Literal Header Field without Indexing -- New Name
    fn without_indexing_new_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///Literal Header Field Never Indexed -- Indexed Name
    fn never_indexed_indexed_name(&mut self, n: usize, value: Vec<u8>);

    ///Literal Header Field Never Indexed -- New Name
    fn never_indexed_new_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///Maximum Dynamic Table Size Change
    fn dynamic_table_size_update(&mut self, n: usize);
}

///Decode a byte slice.
pub fn decode(mut buffer: &[u8], ins: &mut impl DecodeInstructions) {
    let reader = &mut buffer;
    while let Some(i) = reader.fetch() {
        decode_u8(i, reader, ins);
    }
}

fn decode_u8(i: u8, reader: &mut impl ReadByte, ins: &mut impl DecodeInstructions) {
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
