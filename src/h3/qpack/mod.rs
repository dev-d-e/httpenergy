/*!
Utilities for field compression and decompression.

# Compression
The build_* functions can be used to build [encoder instructions, decoder instructions, field line representations].

# Decompression
The get_* functions can be used to parse [encoder instructions, decoder instructions, field line representations].

# Index
This module provides static_table_* functions for working with static table.
*/

mod index;

use super::prty::*;
use super::*;
use crate::h2::prty::*;
use derive_more::Debug;
pub use index::*;

///Builds encoder instruction.
///Set Dynamic Table Capacity.
#[inline]
pub fn build_set_dynamic_table_capacity(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 5, 0x20, o);
}

///Builds encoder instruction.
///Adds an entry to the dynamic table where the field name matches the field name of an entry stored in the static or the dynamic table.
///
///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
#[inline]
pub fn build_insert_with_name_reference(
    t_bit: bool,
    n: usize,
    value: OctetsRef,
    o: &mut dyn PutU8,
) {
    if t_bit {
        encode_integer(n, 6, 0xc0, o);
    } else {
        encode_integer(n, 6, 0x80, o);
    }

    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds encoder instruction.
///Adds an entry to the dynamic table where both the field name and the field value are represented as string literals.
#[inline]
pub fn build_insert_with_literal_name(name: OctetsRef, value: OctetsRef, o: &mut dyn PutU8) {
    if name.huffman() {
        encode_prefix_literal_huffman(&name, 5, 0x60, o);
    } else {
        encode_prefix_literal(&name, 5, 0x40, o);
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds encoder instruction.
///Duplicates an existing entry in the dynamic table.
#[inline]
pub fn build_duplicate(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 5, 0x00, o);
}

///Parses encoder instructions to vec.
///Returns an error if parsing failure.
pub fn get_einsts_to_vec(o: &mut dyn GetU8) -> Result<Vec<EinstResult>, &'static str> {
    let mut v = Vec::new();
    get_einsts(|r| v.push(r), o)?;
    Ok(v)
}

///Parses encoder instructions.
///Returns an error if parsing failure.
///
///An encoder sends encoder instructions on the encoder stream to set the capacity of the dynamic table and add dynamic table entries.
///Instructions adding table entries can use existing entries to avoid transmitting redundant information.
///The name can be transmitted as a reference to an existing entry in the static or the dynamic table or as a string literal.
///For entries that already exist in the dynamic table, the full entry can also be used by reference, creating a duplicate entry.
pub fn get_einsts(mut f: impl FnMut(EinstResult), o: &mut dyn GetU8) -> Result<(), &'static str> {
    while o.is_surplus() {
        f(get_one_einst(o)?);
    }
    Ok(())
}

///Parses a encoder instruction.
///Returns an error if parsing failure.
#[inline(always)]
pub fn get_one_einst(o: &mut dyn GetU8) -> Result<EinstResult, &'static str> {
    let i = o.get_u8().ok_or(READ_BYTE_ERROR)?;
    let r = match i {
        32..63 => EinstResult::SetDynamicTableCapacity((i & 0x1f) as usize),
        63 => EinstResult::SetDynamicTableCapacity(decode_integer(31, o)?),
        192..255 => {
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithNameReference {
                t_bit: true,
                n: (i & 0x3f) as usize,
                value,
            }
        }
        255 => {
            let n = decode_integer(63, o)?;
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithNameReference {
                t_bit: true,
                n,
                value,
            }
        }
        128..191 => {
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithNameReference {
                t_bit: false,
                n: (i & 0x3f) as usize,
                value,
            }
        }
        191 => {
            let n = decode_integer(63, o)?;
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithNameReference {
                t_bit: false,
                n,
                value,
            }
        }
        96..127 => {
            let name = decode_n_huffman((i & 0x1f) as usize, o)?.into();
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithLiteralName { name, value }
        }
        127 => {
            let n = decode_integer(31, o)?;
            let name = decode_n_huffman(n, o)?.into();
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithLiteralName { name, value }
        }
        64..95 => {
            let name = decode_n_literal((i & 0x1f) as usize, o)?.into();
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithLiteralName { name, value }
        }
        95 => {
            let n = decode_integer(31, o)?;
            let name = decode_n_literal(n, o)?.into();
            let value = decode_literal(o)?.into();
            EinstResult::InsertWithLiteralName { name, value }
        }
        0..31 => EinstResult::Duplicate(i as usize),
        31 => EinstResult::Duplicate(decode_integer(31, o)?),
    };
    Ok(r)
}

///Represents a result of parsing encoder instruction.
#[repr(u8)]
pub enum EinstResult {
    ///Set Dynamic Table Capacity.
    SetDynamicTableCapacity(usize),

    ///Adds an entry to the dynamic table where the field name matches the field name of an entry stored in the static or the dynamic table.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    InsertWithNameReference {
        t_bit: bool,
        n: usize,
        value: FieldValue,
    },

    ///Adds an entry to the dynamic table where both the field name and the field value are represented as string literals.
    InsertWithLiteralName { name: FieldName, value: FieldValue },

    ///Duplicates an existing entry in the dynamic table.
    Duplicate(usize),
}

///Builds decoder instruction.
///After processing an encoded field section whose declared Required Insert Count is not zero, the decoder emits a Section Acknowledgment instruction.
#[inline]
pub fn build_section_acknowledgment(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 7, 0x80, o);
}

///Builds decoder instruction.
///When a stream is reset or reading is abandoned, the decoder emits a Stream Cancellation instruction.
#[inline]
pub fn build_stream_cancellation(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 6, 0x40, o);
}

///Builds decoder instruction.
///Insert Count Increment instruction.
#[inline]
pub fn build_insert_count_increment(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 6, 0x00, o);
}

///Parses decoder instructions to vec.
///Returns an error if parsing failure.
pub fn get_dinsts_to_vec(o: &mut dyn GetU8) -> Result<Vec<DinstResult>, &'static str> {
    let mut v = Vec::new();
    get_dinsts(|r| v.push(r), o)?;
    Ok(v)
}

///Parses decoder instructions.
///Returns an error if parsing failure.
///
///A decoder sends decoder instructions on the decoder stream to inform the encoder about the processing of field sections and table updates to ensure consistency of the dynamic table.
pub fn get_dinsts(mut f: impl FnMut(DinstResult), o: &mut dyn GetU8) -> Result<(), &'static str> {
    while o.is_surplus() {
        f(get_one_dinst(o)?);
    }
    Ok(())
}

///Parses a decoder instruction.
///Returns an error if parsing failure.
#[inline(always)]
pub fn get_one_dinst(o: &mut dyn GetU8) -> Result<DinstResult, &'static str> {
    let i = o.get_u8().ok_or(READ_BYTE_ERROR)?;
    let r = match i {
        128..255 => DinstResult::SectionAcknowledgment((i & 0x7f) as usize),
        255 => DinstResult::SectionAcknowledgment(decode_integer(127, o)?),
        64..127 => DinstResult::StreamCancellation((i & 0x3f) as usize),
        127 => DinstResult::StreamCancellation(decode_integer(63, o)?),
        0..63 => DinstResult::InsertCountIncrement(i as usize),
        63 => DinstResult::InsertCountIncrement(decode_integer(63, o)?),
    };
    Ok(r)
}

///Represents a result of parsing decoder instruction.
#[repr(u8)]
pub enum DinstResult {
    ///After processing an encoded field section whose declared Required Insert Count is not zero, the decoder emits a Section Acknowledgment instruction.
    SectionAcknowledgment(usize),

    ///When a stream is reset or reading is abandoned, the decoder emits a Stream Cancellation instruction.
    StreamCancellation(usize),

    ///Insert Count Increment instruction.
    InsertCountIncrement(usize),
}

///Builds field line representation.
///Each encoded field section is prefixed with two integers.
///
///The Required Insert Count identifies the state of the dynamic table needed to process the encoded field section. Blocking decoders use the Required Insert Count to determine when it is safe to process the rest of the field section.
///
///The Base is encoded relative to the Required Insert Count using a one-bit Sign ('S') and the Delta Base value.
///A Sign bit of 0 indicates that the Base is greater than or equal to the value of the Required Insert Count; the decoder adds the value of Delta Base to the Required Insert Count to determine the value of the Base.
///A Sign bit of 1 indicates that the Base is less than the Required Insert Count; the decoder subtracts the value of Delta Base from the Required Insert Count and also subtracts one to determine the value of the Base.
#[inline]
pub fn build_prefix(
    required_insert_count: usize,
    s_bit: bool,
    delta_base: usize,
    o: &mut dyn PutU8,
) {
    encode_integer(required_insert_count, 8, 0x00, o);
    if s_bit {
        encode_integer(delta_base, 7, 0x80, o);
    } else {
        encode_integer(delta_base, 7, 0x00, o);
    }
}

///Builds field line representation.
///An indexed field line representation identifies an entry in the static table or an entry in the dynamic table with an absolute index less than the value of the Base.
///
///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
#[inline]
pub fn build_indexed_field_line(t_bit: bool, n: usize, o: &mut dyn PutU8) {
    if t_bit {
        encode_integer(n, 6, 0xc0, o);
    } else {
        encode_integer(n, 6, 0x80, o);
    }
}

///Builds field line representation.
///An indexed field line with post-Base index representation identifies an entry in the dynamic table with an absolute index greater than or equal to the value of the Base.
#[inline]
pub fn build_indexed_field_line_with_post_base_index(n: usize, o: &mut dyn PutU8) {
    encode_integer(n, 4, 0x10, o);
}

///Builds field line representation.
///A literal field line with name reference representation encodes a field line where the field name matches the field name of an entry in the static table
///or the field name of an entry in the dynamic table with an absolute index less than the value of the Base.
///
///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
///
///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
#[inline]
pub fn build_literal_field_line_with_name_reference(
    n_bit: bool,
    t_bit: bool,
    n: usize,
    value: OctetsRef,
    o: &mut dyn PutU8,
) {
    if n_bit {
        if t_bit {
            encode_integer(n, 4, 0x70, o);
        } else {
            encode_integer(n, 4, 0x60, o);
        }
    } else {
        if t_bit {
            encode_integer(n, 4, 0x50, o);
        } else {
            encode_integer(n, 4, 0x40, o);
        }
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field line representation.
///A literal field line with post-Base name reference representation encodes a field line where the field name matches the field name of a dynamic table entry with an absolute index greater than or equal to the value of the Base.
///
///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
#[inline]
pub fn build_literal_field_line_with_post_base_name_reference(
    n_bit: bool,
    n: usize,
    value: OctetsRef,
    o: &mut dyn PutU8,
) {
    if n_bit {
        encode_integer(n, 3, 0x08, o);
    } else {
        encode_integer(n, 3, 0x00, o);
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Builds field line representation.
///The literal field line with literal name representation encodes a field name and a field value as string literals.
///
///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
#[inline]
pub fn build_literal_field_line_with_literal_name(
    n_bit: bool,
    name: OctetsRef,
    value: OctetsRef,
    o: &mut dyn PutU8,
) {
    if n_bit {
        if name.huffman() {
            encode_prefix_literal_huffman(&name, 3, 0x38, o);
        } else {
            encode_prefix_literal(&name, 3, 0x30, o);
        }
    } else {
        if name.huffman() {
            encode_prefix_literal_huffman(&name, 3, 0x28, o);
        } else {
            encode_prefix_literal(&name, 3, 0x20, o);
        }
    }
    if value.huffman() {
        encode_literal_huffman(&value, o);
    } else {
        encode_literal(&value, o);
    }
}

///Parses field section to vec.
///Returns an error if parsing failure.
pub fn get_fsection_to_vec(o: &mut dyn GetU8) -> Result<Vec<FsectionResult>, &'static str> {
    let mut v = Vec::new();
    get_fsection(|r| v.push(r), o)?;
    Ok(v)
}

///Parses field section.
///Returns an error if parsing failure.
///
///An encoded field section consists of a prefix and a possibly empty sequence of representations defined in this section. Each representation corresponds to a single field line.
///These representations reference the static table or the dynamic table in a particular state, but they do not modify that state.
pub fn get_fsection(
    mut f: impl FnMut(FsectionResult),
    o: &mut dyn GetU8,
) -> Result<(), &'static str> {
    let i = o.get_u8().ok_or(READ_BYTE_ERROR)?;
    let required_insert_count = match i {
        0..255 => i as usize,
        255 => decode_integer(255, o)?,
    };
    let i = o.get_u8().ok_or(READ_BYTE_ERROR)?;
    let (s_bit, delta_base) = match i {
        0..127 => (false, i as usize),
        127 => (false, decode_integer(127, o)?),
        128..255 => (true, (i & 0x7f) as usize),
        255 => (true, decode_integer(127, o)?),
    };
    f(FsectionResult::Prefix {
        required_insert_count,
        s_bit,
        delta_base,
    });
    while let Some(i) = o.get_u8() {
        f(get_frep(i, o)?);
    }
    Ok(())
}

#[inline(always)]
fn get_frep(i: u8, o: &mut dyn GetU8) -> Result<FsectionResult, &'static str> {
    let r = match i {
        192..255 => {
            let n = (i & 0x3f) as usize;
            FsectionResult::IndexedFieldLine { t_bit: true, n }
        }
        255 => {
            let n = decode_integer(63, o)?;
            FsectionResult::IndexedFieldLine { t_bit: true, n }
        }
        128..191 => {
            let n = (i & 0x3f) as usize;
            FsectionResult::IndexedFieldLine { t_bit: false, n }
        }
        191 => {
            let n = decode_integer(63, o)?;
            FsectionResult::IndexedFieldLine { t_bit: false, n }
        }
        16..31 => {
            let n = (i & 0x0f) as usize;
            FsectionResult::IndexedFieldLineWithPostBaseIndex(n)
        }
        31 => {
            let n = decode_integer(15, o)?;
            FsectionResult::IndexedFieldLineWithPostBaseIndex(n)
        }
        112..127 => {
            let n = (i & 0x0f) as usize;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: true,
                t_bit: true,
                n,
                value,
            }
        }
        127 => {
            let n = decode_integer(15, o)?;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: true,
                t_bit: true,
                n,
                value,
            }
        }
        96..111 => {
            let n = (i & 0x0f) as usize;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: true,
                t_bit: false,
                n,
                value,
            }
        }
        111 => {
            let n = decode_integer(15, o)?;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: true,
                t_bit: false,
                n,
                value,
            }
        }
        80..95 => {
            let n = (i & 0x0f) as usize;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: false,
                t_bit: true,
                n,
                value,
            }
        }
        95 => {
            let n = decode_integer(15, o)?;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: false,
                t_bit: true,
                n,
                value,
            }
        }
        64..79 => {
            let n = (i & 0x0f) as usize;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: false,
                t_bit: false,
                n,
                value,
            }
        }
        79 => {
            let n = decode_integer(15, o)?;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithNameReference {
                n_bit: false,
                t_bit: false,
                n,
                value,
            }
        }
        8..15 => {
            let n = (i & 0x07) as usize;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithPostBaseNameReference {
                n_bit: true,
                n,
                value,
            }
        }
        15 => {
            let n = decode_integer(7, o)?;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithPostBaseNameReference {
                n_bit: true,
                n,
                value,
            }
        }
        0..7 => {
            let n = i as usize;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithPostBaseNameReference {
                n_bit: false,
                n,
                value,
            }
        }
        7 => {
            let n = decode_integer(7, o)?;
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithPostBaseNameReference {
                n_bit: false,
                n,
                value,
            }
        }
        56..63 => {
            let name = decode_n_huffman((i & 0x07) as usize, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: true,
                name,
                value,
            }
        }
        63 => {
            let n = decode_integer(7, o)?;
            let name = decode_n_huffman(n, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: true,
                name,
                value,
            }
        }
        48..55 => {
            let name = decode_n_literal((i & 0x07) as usize, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: true,
                name,
                value,
            }
        }
        55 => {
            let n = decode_integer(7, o)?;
            let name = decode_n_literal(n, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: true,
                name,
                value,
            }
        }
        40..47 => {
            let name = decode_n_huffman((i & 0x07) as usize, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: false,
                name,
                value,
            }
        }
        47 => {
            let n = decode_integer(7, o)?;
            let name = decode_n_huffman(n, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: false,
                name,
                value,
            }
        }
        32..39 => {
            let name = decode_n_literal((i & 0x07) as usize, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: false,
                name,
                value,
            }
        }
        39 => {
            let n = decode_integer(7, o)?;
            let name = decode_n_literal(n, o)?.into();
            let value = decode_literal(o)?.into();
            FsectionResult::LiteralFieldLineWithLiteralName {
                n_bit: false,
                name,
                value,
            }
        }
    };
    Ok(r)
}

///Represents a result of parsing field section.
#[derive(Debug)]
#[repr(u8)]
pub enum FsectionResult {
    ///Each encoded field section is prefixed with two integers.
    ///
    ///The Required Insert Count identifies the state of the dynamic table needed to process the encoded field section. Blocking decoders use the Required Insert Count to determine when it is safe to process the rest of the field section.
    ///
    ///The Base is encoded relative to the Required Insert Count using a one-bit Sign ('S') and the Delta Base value.
    ///A Sign bit of 0 indicates that the Base is greater than or equal to the value of the Required Insert Count; the decoder adds the value of Delta Base to the Required Insert Count to determine the value of the Base.
    ///A Sign bit of 1 indicates that the Base is less than the Required Insert Count; the decoder subtracts the value of Delta Base from the Required Insert Count and also subtracts one to determine the value of the Base.
    Prefix {
        required_insert_count: usize,
        s_bit: bool,
        delta_base: usize,
    },

    ///An indexed field line representation identifies an entry in the static table or an entry in the dynamic table with an absolute index less than the value of the Base.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    IndexedFieldLine { t_bit: bool, n: usize },

    ///An indexed field line with post-Base index representation identifies an entry in the dynamic table with an absolute index greater than or equal to the value of the Base.
    IndexedFieldLineWithPostBaseIndex(usize),

    ///A literal field line with name reference representation encodes a field line where the field name matches the field name of an entry in the static table
    ///or the field name of an entry in the dynamic table with an absolute index less than the value of the Base.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    ///
    ///When T=1, the number represents the static table index; when T=0, the number is the relative index of the entry in the dynamic table.
    LiteralFieldLineWithNameReference {
        n_bit: bool,
        t_bit: bool,
        n: usize,
        value: FieldValue,
    },

    ///A literal field line with post-Base name reference representation encodes a field line where the field name matches the field name of a dynamic table entry with an absolute index greater than or equal to the value of the Base.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    LiteralFieldLineWithPostBaseNameReference {
        n_bit: bool,
        n: usize,
        value: FieldValue,
    },

    ///The literal field line with literal name representation encodes a field name and a field value as string literals.
    ///
    ///When the 'N' bit is set, the encoded field line MUST always be encoded with a literal representation.
    LiteralFieldLineWithLiteralName {
        n_bit: bool,
        name: FieldName,
        value: FieldValue,
    },
}
