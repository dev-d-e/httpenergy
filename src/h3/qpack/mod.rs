/*!
Utilities for field compression and decompression.

The [`EncoderInstructions`] type provides functions that build and parse encoder instructions.

The [`DecoderInstructions`] type provides functions that build and parse decoder instructions.

The [`FieldInstructions`] type provides functions that build and parse field line representations.

# Index
This module provides the [`DynamicIndices`] trait for working with dynamic table.
*/

mod index;

use crate::h2::huffman::decode_huffman;
use crate::h2::prty::*;
use crate::h3::prty::*;
use crate::{OctetsRef, ReadByte, WriteByte};
pub use index::*;

#[inline]
fn decode_n_huf_to_vec(n: usize, reader: &mut impl ReadByte) -> Vec<u8> {
    let mut v = Vec::new();
    if let Some(o) = reader.fetch_all(n) {
        decode_huffman(o, &mut v);
    }
    v
}

///Utilities for encoder instructions.
///
///An encoder sends encoder instructions on the encoder stream to set the capacity of the dynamic table and add dynamic table entries.
///Instructions adding table entries can use existing entries to avoid transmitting redundant information.
///The name can be transmitted as a reference to an existing entry in the static or the dynamic table or as a string literal.
///For entries that already exist in the dynamic table, the full entry can also be used by reference, creating a duplicate entry.
pub struct EncoderInstructions;

impl EncoderInstructions {
    ///Set Dynamic Table Capacity.
    #[inline]
    pub fn set_dynamic_table_capacity(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 5, 0x20, writer);
    }

    ///Adds an entry to the dynamic table where the field name matches the field name of an entry stored in the static or the dynamic table.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    #[inline]
    pub fn insert_with_name_reference(
        t_bit: bool,
        n: usize,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        if t_bit {
            encode_integer(n, 6, 0xc0, writer);
        } else {
            encode_integer(n, 6, 0x80, writer);
        }

        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Adds an entry to the dynamic table where both the field name and the field value are represented as string literals.
    #[inline]
    pub fn insert_with_literal_name(
        name: OctetsRef,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        if name.huffman() {
            encode_prefix_literal_huffman(&name, 5, 0x60, writer);
        } else {
            encode_prefix_literal(&name, 5, 0x40, writer);
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Duplicates an existing entry in the dynamic table.
    #[inline]
    pub fn duplicate(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 5, 0x00, writer);
    }

    ///Decodes instruction bytes with an implementation of `DistributeEncoderInstructions`.
    pub fn decode(reader: &mut impl ReadByte, ins: &mut impl DistributeEncoderInstructions) {
        while let Some(i) = reader.fetch() {
            match i {
                32..63 => {
                    let a = (i & 0x1f) as usize;
                    ins.set_dynamic_table_capacity(a);
                }
                63 => {
                    let a = decode_integer(31, reader);
                    ins.set_dynamic_table_capacity(a);
                }
                192..255 => {
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_name_reference(true, (i & 0x3f) as usize, value);
                }
                255 => {
                    let a = decode_integer(63, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_name_reference(true, a, value);
                }
                128..191 => {
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_name_reference(false, (i & 0x3f) as usize, value);
                }
                191 => {
                    let a = decode_integer(63, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_name_reference(false, a, value);
                }
                96..127 => {
                    let name = decode_n_huf_to_vec((i & 0x1f) as usize, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_literal_name(name, value);
                }
                127 => {
                    let a = decode_integer(31, reader);
                    let name = decode_n_huf_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_literal_name(name, value);
                }
                64..95 => {
                    let name = decode_n_literal_to_vec((i & 0x1f) as usize, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_literal_name(name, value);
                }
                95 => {
                    let a = decode_integer(31, reader);
                    let name = decode_n_literal_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.insert_with_literal_name(name, value);
                }
                0..31 => {
                    ins.duplicate((i & 0x1f) as usize);
                }
                31 => {
                    let a = decode_integer(31, reader);
                    ins.duplicate(a);
                }
            }
        }
    }
}

///A trait to parse encoder instructions. distributes result.
pub trait DistributeEncoderInstructions {
    ///Set Dynamic Table Capacity.
    fn set_dynamic_table_capacity(&mut self, n: usize);

    ///Adds an entry to the dynamic table where the field name matches the field name of an entry stored in the static or the dynamic table.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    fn insert_with_name_reference(&mut self, t_bit: bool, n: usize, value: Vec<u8>);

    ///Adds an entry to the dynamic table where both the field name and the field value are represented as string literals.
    fn insert_with_literal_name(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///Duplicates an existing entry in the dynamic table.
    fn duplicate(&mut self, n: usize);
}

///Utilities for decoder instructions.
///
///A decoder sends decoder instructions on the decoder stream to inform the encoder about the processing of field sections and table updates to ensure consistency of the dynamic table.
pub struct DecoderInstructions;

impl DecoderInstructions {
    ///After processing an encoded field section whose declared Required Insert Count is not zero, the decoder emits a Section Acknowledgment instruction.
    #[inline]
    pub fn section_acknowledgment(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 7, 0x80, writer);
    }

    ///When a stream is reset or reading is abandoned, the decoder emits a Stream Cancellation instruction.
    #[inline]
    pub fn stream_cancellation(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 6, 0x40, writer);
    }

    ///Insert Count Increment instruction.
    #[inline]
    pub fn insert_count_increment(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 6, 0x00, writer);
    }

    ///Decodes instruction bytes with an implementation of `DistributeDecoderInstructions`.
    pub fn decode(reader: &mut impl ReadByte, ins: &mut impl DistributeDecoderInstructions) {
        while let Some(i) = reader.fetch() {
            match i {
                128..255 => {
                    let a = (i & 0x7f) as usize;
                    ins.section_acknowledgment(a);
                }
                255 => {
                    let a = decode_integer(127, reader);
                    ins.section_acknowledgment(a);
                }
                64..127 => {
                    let a = (i & 0x3f) as usize;
                    ins.stream_cancellation(a);
                }
                127 => {
                    let a = decode_integer(63, reader);
                    ins.stream_cancellation(a);
                }
                0..63 => {
                    let a = (i & 0x3f) as usize;
                    ins.insert_count_increment(a);
                }
                63 => {
                    let a = decode_integer(63, reader);
                    ins.insert_count_increment(a);
                }
            }
        }
    }
}

///A trait to parse decoder instructions. distributes result.
pub trait DistributeDecoderInstructions {
    ///After processing an encoded field section whose declared Required Insert Count is not zero, the decoder emits a Section Acknowledgment instruction.
    fn section_acknowledgment(&mut self, n: usize);

    ///When a stream is reset or reading is abandoned, the decoder emits a Stream Cancellation instruction.
    fn stream_cancellation(&mut self, n: usize);

    ///Insert Count Increment instruction.
    fn insert_count_increment(&mut self, n: usize);
}

///Utilities for field line representations.
///
///An encoded field section consists of a prefix and a possibly empty sequence of representations defined in this section. Each representation corresponds to a single field line.
///These representations reference the static table or the dynamic table in a particular state, but they do not modify that state.
pub struct FieldInstructions;

impl FieldInstructions {
    ///Each encoded field section is prefixed with two integers.
    ///
    ///The Required Insert Count identifies the state of the dynamic table needed to process the encoded field section. Blocking decoders use the Required Insert Count to determine when it is safe to process the rest of the field section.
    ///
    ///The Base is encoded relative to the Required Insert Count using a one-bit Sign ('S') and the Delta Base value.
    ///A Sign bit of 0 indicates that the Base is greater than or equal to the value of the Required Insert Count; the decoder adds the value of Delta Base to the Required Insert Count to determine the value of the Base.
    ///A Sign bit of 1 indicates that the Base is less than the Required Insert Count; the decoder subtracts the value of Delta Base from the Required Insert Count and also subtracts one to determine the value of the Base.
    #[inline]
    pub fn prefix(
        required_insert_count: usize,
        s_bit: bool,
        delta_base: usize,
        writer: &mut impl WriteByte,
    ) {
        encode_integer(required_insert_count, 8, 0x00, writer);
        if s_bit {
            encode_integer(delta_base, 7, 0x80, writer);
        } else {
            encode_integer(delta_base, 7, 0x00, writer);
        }
    }

    ///An indexed field line representation identifies an entry in the static table or an entry in the dynamic table with an absolute index less than the value of the Base.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    #[inline]
    pub fn indexed_field_line(t_bit: bool, n: usize, writer: &mut impl WriteByte) {
        if t_bit {
            encode_integer(n, 6, 0xc0, writer);
        } else {
            encode_integer(n, 6, 0x80, writer);
        }
    }

    ///An indexed field line with post-Base index representation identifies an entry in the dynamic table with an absolute index greater than or equal to the value of the Base.
    #[inline]
    pub fn indexed_field_line_with_post_base_index(n: usize, writer: &mut impl WriteByte) {
        encode_integer(n, 4, 0x10, writer);
    }

    ///A literal field line with name reference representation encodes a field line where the field name matches the field name of an entry in the static table
    ///or the field name of an entry in the dynamic table with an absolute index less than the value of the Base.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    #[inline]
    pub fn literal_field_line_with_name_reference(
        n_bit: bool,
        t_bit: bool,
        n: usize,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        if n_bit {
            if t_bit {
                encode_integer(n, 4, 0x70, writer);
            } else {
                encode_integer(n, 4, 0x60, writer);
            }
        } else {
            if t_bit {
                encode_integer(n, 4, 0x50, writer);
            } else {
                encode_integer(n, 4, 0x40, writer);
            }
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///A literal field line with post-Base name reference representation encodes a field line where the field name matches the field name of a dynamic table entry with an absolute index greater than or equal to the value of the Base.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    #[inline]
    pub fn literal_field_line_with_post_base_name_reference(
        n_bit: bool,
        n: usize,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        if n_bit {
            encode_integer(n, 3, 0x08, writer);
        } else {
            encode_integer(n, 3, 0x00, writer);
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///The literal field line with literal name representation encodes a field name and a field value as string literals.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    #[inline]
    pub fn literal_field_line_with_literal_name(
        n_bit: bool,
        name: OctetsRef,
        value: OctetsRef,
        writer: &mut impl WriteByte,
    ) {
        if n_bit {
            if name.huffman() {
                encode_prefix_literal_huffman(&name, 3, 0x38, writer);
            } else {
                encode_prefix_literal(&name, 3, 0x30, writer);
            }
        } else {
            if name.huffman() {
                encode_prefix_literal_huffman(&name, 3, 0x28, writer);
            } else {
                encode_prefix_literal(&name, 3, 0x20, writer);
            }
        }
        if value.huffman() {
            encode_literal_huffman_encoded(&value, writer);
        } else {
            encode_literal(&value, writer);
        }
    }

    ///Decodes instruction bytes with an implementation of `DistributeFieldInstructions`.
    pub fn decode(reader: &mut impl ReadByte, ins: &mut impl DistributeFieldInstructions) {
        if let Some(i) = reader.fetch() {
            let required_insert_count = match i {
                0..255 => i as usize,
                255 => decode_integer(255, reader),
            };
            if let Some(i) = reader.fetch() {
                match i {
                    0..127 => {
                        let a = (i & 0x7f) as usize;
                        ins.prefix(required_insert_count, false, a);
                    }
                    127 => {
                        let a = decode_integer(127, reader);
                        ins.prefix(required_insert_count, false, a);
                    }
                    128..255 => {
                        let a = (i & 0x7f) as usize;
                        ins.prefix(required_insert_count, true, a);
                    }
                    255 => {
                        let a = decode_integer(127, reader);
                        ins.prefix(required_insert_count, true, a);
                    }
                }
            } else {
                return;
            }
        } else {
            return;
        }

        while let Some(i) = reader.fetch() {
            match i {
                192..255 => {
                    let a = (i & 0x3f) as usize;
                    ins.indexed_field_line(true, a);
                }
                255 => {
                    let a = decode_integer(63, reader);
                    ins.indexed_field_line(true, a);
                }
                128..191 => {
                    let a = (i & 0x3f) as usize;
                    ins.indexed_field_line(false, a);
                }
                191 => {
                    let a = decode_integer(63, reader);
                    ins.indexed_field_line(false, a);
                }
                16..31 => {
                    let a = (i & 0x0f) as usize;
                    ins.indexed_field_line_with_post_base_index(a);
                }
                31 => {
                    let a = decode_integer(15, reader);
                    ins.indexed_field_line_with_post_base_index(a);
                }
                112..127 => {
                    let a = (i & 0x0f) as usize;
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(true, true, a, value);
                }
                127 => {
                    let a = decode_integer(15, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(true, true, a, value);
                }
                96..111 => {
                    let a = (i & 0x0f) as usize;
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(true, false, a, value);
                }
                111 => {
                    let a = decode_integer(15, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(true, false, a, value);
                }
                80..95 => {
                    let a = (i & 0x0f) as usize;
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(false, true, a, value);
                }
                95 => {
                    let a = decode_integer(15, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(false, true, a, value);
                }
                64..79 => {
                    let a = (i & 0x0f) as usize;
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(false, false, a, value);
                }
                79 => {
                    let a = decode_integer(15, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_name_reference(false, false, a, value);
                }
                8..15 => {
                    let a = (i & 0x07) as usize;
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_post_base_name_reference(true, a, value);
                }
                15 => {
                    let a = decode_integer(7, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_post_base_name_reference(true, a, value);
                }
                0..7 => {
                    let a = i as usize;
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_post_base_name_reference(false, a, value);
                }
                7 => {
                    let a = decode_integer(7, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_post_base_name_reference(false, a, value);
                }
                56..63 => {
                    let a = (i & 0x07) as usize;
                    let name = decode_n_huf_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(true, name, value);
                }
                63 => {
                    let a = decode_integer(7, reader);
                    let name = decode_n_huf_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(true, name, value);
                }
                48..55 => {
                    let a = (i & 0x07) as usize;
                    let name = decode_n_literal_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(true, name, value);
                }
                55 => {
                    let a = decode_integer(7, reader);
                    let name = decode_n_literal_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(true, name, value);
                }
                40..47 => {
                    let a = (i & 0x07) as usize;
                    let name = decode_n_huf_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(false, name, value);
                }
                47 => {
                    let a = decode_integer(7, reader);
                    let name = decode_n_huf_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(false, name, value);
                }
                32..39 => {
                    let a = (i & 0x07) as usize;
                    let name = decode_n_literal_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(false, name, value);
                }
                39 => {
                    let a = decode_integer(7, reader);
                    let name = decode_n_literal_to_vec(a, reader);
                    let value = decode_literal_to_vec(reader);
                    ins.literal_field_line_with_literal_name(false, name, value);
                }
            }
        }
    }
}

///A trait to parse field line representations. distributes result.
pub trait DistributeFieldInstructions {
    ///Each encoded field section is prefixed with two integers.
    ///
    ///The Required Insert Count identifies the state of the dynamic table needed to process the encoded field section. Blocking decoders use the Required Insert Count to determine when it is safe to process the rest of the field section.
    ///
    ///The Base is encoded relative to the Required Insert Count using a one-bit Sign ('S') and the Delta Base value.
    ///A Sign bit of 0 indicates that the Base is greater than or equal to the value of the Required Insert Count; the decoder adds the value of Delta Base to the Required Insert Count to determine the value of the Base.
    ///A Sign bit of 1 indicates that the Base is less than the Required Insert Count; the decoder subtracts the value of Delta Base from the Required Insert Count and also subtracts one to determine the value of the Base.
    fn prefix(&mut self, required_insert_count: usize, s_bit: bool, delta_base: usize);

    ///An indexed field line representation identifies an entry in the static table or an entry in the dynamic table with an absolute index less than the value of the Base.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    fn indexed_field_line(&mut self, t_bit: bool, n: usize);

    ///An indexed field line with post-Base index representation identifies an entry in the dynamic table with an absolute index greater than or equal to the value of the Base.
    fn indexed_field_line_with_post_base_index(&mut self, n: usize);

    ///A literal field line with name reference representation encodes a field line where the field name matches the field name of an entry in the static table
    ///or the field name of an entry in the dynamic table with an absolute index less than the value of the Base.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    fn literal_field_line_with_name_reference(
        &mut self,
        n_bit: bool,
        t_bit: bool,
        n: usize,
        value: Vec<u8>,
    );

    ///A literal field line with post-Base name reference representation encodes a field line where the field name matches the field name of a dynamic table entry with an absolute index greater than or equal to the value of the Base.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    fn literal_field_line_with_post_base_name_reference(
        &mut self,
        n_bit: bool,
        n: usize,
        value: Vec<u8>,
    );

    ///The literal field line with literal name representation encodes a field name and a field value as string literals.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    fn literal_field_line_with_literal_name(&mut self, n_bit: bool, name: Vec<u8>, value: Vec<u8>);
}
