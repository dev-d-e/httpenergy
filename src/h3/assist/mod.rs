use super::qpack::{
    DistributeEncoderInstructions, DynamicIndices, EncoderInstructions, StaticTable,
};
use crate::ReadByte;
use getset::{Getters, MutGetters};

///A helper to parse encoder instructions.
#[derive(Getters, MutGetters)]
pub struct H3EncoderInstructionsHelper<'a, T>
where
    T: DynamicIndices,
{
    #[getset(get = "pub", get_mut = "pub")]
    dynamic_indices: &'a mut T,
}

impl<'a, T> DistributeEncoderInstructions for H3EncoderInstructionsHelper<'a, T>
where
    T: DynamicIndices,
{
    fn set_dynamic_table_capacity(&mut self, n: usize) {
        self.dynamic_indices.set_capacity(n);
    }

    fn insert_with_name_reference(&mut self, t_bit: bool, n: usize, value: Vec<u8>) {
        if t_bit {
            if let Some(name) = StaticTable::get_name(n) {
                self.dynamic_indices.add(name.to_vec(), value);
            }
        } else {
            if let Some(name) = self.dynamic_indices.get_name(n) {
                self.dynamic_indices.add(name.to_vec(), value);
            }
        }
    }

    fn insert_with_literal_name(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.dynamic_indices.add(name, value);
    }

    fn duplicate(&mut self, n: usize) {
        if let Some((name, value)) = self.dynamic_indices.get_entry(n) {
            self.dynamic_indices.add(name.to_vec(), value.to_vec());
        }
    }
}

impl<'a, T> H3EncoderInstructionsHelper<'a, T>
where
    T: DynamicIndices,
{
    ///Creates.
    pub fn new(dynamic_indices: &'a mut T) -> Self {
        Self { dynamic_indices }
    }

    ///Decodes bytes.
    pub fn decode(&mut self, reader: &mut impl ReadByte) {
        EncoderInstructions::decode(reader, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::*;
    use crate::h3::qpack::{DistributeFieldInstructions, DynamicTable, FieldInstructions};

    struct TestFieldInstructions;

    impl DistributeFieldInstructions for TestFieldInstructions {
        fn prefix(&mut self, required_insert_count: usize, s_bit: bool, delta_base: usize) {
            println!(
                "prefix: {:?} {:?} {:?}",
                required_insert_count, s_bit, delta_base
            );
        }

        fn indexed_field_line(&mut self, t_bit: bool, n: usize) {
            println!("indexed_field_line: {:?} {:?}", t_bit, n);
        }

        fn indexed_field_line_with_post_base_index(&mut self, n: usize) {
            println!("indexed_field_line_with_post_base_index: {:?}", n);
        }

        fn literal_field_line_with_name_reference(
            &mut self,
            n_bit: bool,
            t_bit: bool,
            n: usize,
            value: Vec<u8>,
        ) {
            println!(
                "literal_field_line_with_name_reference: {:?} {:?} {:?} {:?}",
                n_bit,
                t_bit,
                n,
                vec_to_str(value)
            );
            if t_bit {
                if let Some(name) = StaticTable::get_name(n) {
                    println!(
                        "literal_field_line_with_name_reference: {:?}",
                        into_str(name),
                    );
                }
            }
        }

        fn literal_field_line_with_post_base_name_reference(
            &mut self,
            n_bit: bool,
            n: usize,
            value: Vec<u8>,
        ) {
            println!(
                "literal_field_line_with_post_base_name_reference: {:?} {:?} {:?}",
                n_bit,
                n,
                vec_to_str(value)
            );
        }

        fn literal_field_line_with_literal_name(
            &mut self,
            n_bit: bool,
            name: Vec<u8>,
            value: Vec<u8>,
        ) {
            println!(
                "literal_field_line_with_literal_name: {:?} {:?} {:?}",
                n_bit,
                vec_to_str(name),
                vec_to_str(value)
            );
        }
    }

    #[test]
    fn instructions() {
        let a: &[u8] = &[
            0x00, 0x00, 0x51, 0x0b, 0x2f, 0x69, 0x6e, 0x64, 0x65, 0x78, 0x2e, 0x68, 0x74, 0x6d,
            0x6c,
        ];
        let mut s = TestFieldInstructions;
        let mut o = a;
        FieldInstructions::decode(&mut o, &mut s);

        let a: &[u8] = &[
            0x3f, 0xbd, 0x01, 0xc0, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70,
            0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0xc1, 0x0c, 0x2f, 0x73, 0x61, 0x6d, 0x70, 0x6c,
            0x65, 0x2f, 0x70, 0x61, 0x74, 0x68,
        ];
        let mut t = DynamicTable::new();
        let mut h = H3EncoderInstructionsHelper::new(&mut t);
        let mut o = a;
        h.decode(&mut o);
        println!("DynamicTable: {:?}", t);
        assert_eq!(t.size(), 106);
    }
}
