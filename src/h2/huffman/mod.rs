mod assist;

use crate::WriteByte;
use assist::*;
use std::collections::{HashMap, HashSet};

///Encodes a slice into huffman encoded.
pub(crate) fn encode_huffman(reader: &[u8], writer: &mut impl WriteByte) {
    let mut buffer: u64 = 0;
    let mut count = 40;
    for &i in reader {
        let (h, n) = HUFFMAN_CODE[i as usize];
        let h = h as u64;
        let h = h << (count - n);
        buffer |= h;
        count -= n;
        while count <= 32 {
            writer.put((buffer >> 32) as u8);
            buffer = buffer << 8;
            count += 8;
        }
    }
    if count != 40 {
        buffer |= (1 << count) - 1;
        writer.put((buffer >> 32) as u8);
    }
}

///Decodes a huffman encoded slice.
pub(crate) fn decode_huffman(reader: &[u8], writer: &mut impl WriteByte) {
    let mut x = 0;
    for &i in reader {
        let o = DECODE_STATE_ARRAY[x as usize];
        let y = (i >> 4) as usize;
        x = o[y].0;
        let n = o[y].1;
        if n >= 0 && n < 256 {
            writer.put(n as u8);
        } else if n == 256 {
            return;
        }

        let o = DECODE_STATE_ARRAY[x as usize];
        let y = (i & 0x0f) as usize;
        x = o[y].0;
        let n = o[y].1;
        if n >= 0 && n < 256 {
            writer.put(n as u8);
        } else if n == 256 {
            return;
        }
    }
}

const NONE: &str = "_";
const SUFFIX: [&str; 16] = [
    "0000", "0001", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001", "1010", "1011",
    "1100", "1101", "1110", "1111",
];

///Build decode state array.
fn build_decode_state_array() -> Vec<[(u8, i16); 16]> {
    let (code, code_map) = get_code();
    let state_map = build_state_map(code);
    let state_index = build_state_index(&state_map);
    to_state_array(state_map, state_index, code_map)
}

fn get_code() -> (Vec<String>, HashMap<String, i16>) {
    let mut vec = Vec::new();
    let mut map = HashMap::new();
    for i in 0..HUFFMAN_CODE.len() {
        let (code, n) = HUFFMAN_CODE[i];
        let s = format!("{:0w$b}", code, w = n as usize);
        vec.push(s.clone());
        map.insert(s, i as i16);
    }
    (vec, map)
}

fn build_state_map(code: Vec<String>) -> HashMap<String, [(String, String); 16]> {
    let mut map = HashMap::new();
    map.insert(String::new(), build_array(""));
    build_possible_state(&code).into_iter().for_each(|i| {
        map.insert(i.to_string(), build_array(&i));
    });
    split_valid_state(&code, &mut map);
    clear_unused_state(&mut map);
    map
}

fn build_array(s: &str) -> [(String, String); 16] {
    std::array::from_fn(|i| (s.to_string() + SUFFIX[i], NONE.to_string()))
}

fn build_possible_state(code: &Vec<String>) -> HashSet<String> {
    let mut hash_set = HashSet::new();
    for i in code {
        let mut s = i.to_string();
        while s.len() > 0 {
            if !hash_set.contains(&s) {
                hash_set.insert(s.clone());
            }
            s.pop();
        }
    }
    hash_set
}

fn split_valid_state(code: &Vec<String>, map: &mut HashMap<String, [(String, String); 16]>) {
    for v in map.values_mut() {
        for i in 0..16 {
            let s = &mut v[i].0;
            for n in code {
                if s.starts_with(n) {
                    s.drain(..n.len());
                    v[i].1 = n.to_string();
                    break;
                }
            }
        }
    }
}

fn clear_unused_state(map: &mut HashMap<String, [(String, String); 16]>) {
    let mut to_remove = Vec::new();
    for k in map.keys() {
        if !find_in_values(k, map) {
            to_remove.push(k.to_string());
        }
    }
    for k in to_remove.iter() {
        map.remove(k);
    }
}

fn find_in_values(k: &String, map: &HashMap<String, [(String, String); 16]>) -> bool {
    for v in map.values() {
        for n in v {
            if *(n.0) == *k {
                return true;
            }
        }
    }
    false
}

fn build_state_index(map: &HashMap<String, [(String, String); 16]>) -> HashMap<String, usize> {
    let mut v: Vec<String> = map.keys().map(|k| k.to_string()).collect();
    v.sort_by(|a, b| {
        let a_len = a.len();
        let b_len = b.len();
        if a_len == b_len {
            a.cmp(&b)
        } else {
            a_len.cmp(&b_len)
        }
    });
    v.into_iter().enumerate().map(|(i, k)| (k, i)).collect()
}

fn to_state_array(
    state_map: HashMap<String, [(String, String); 16]>,
    state_index: HashMap<String, usize>,
    code_map: HashMap<String, i16>,
) -> Vec<[(u8, i16); 16]> {
    if state_map.len() > 256 {
        panic!("For type u8, overflow occurs.");
    }
    let mut vec = Vec::new();
    for (k, v) in state_map {
        if let Some(&index) = state_index.get(&k) {
            let mut state = [(0, -1); 16];
            for n in 0..16 {
                let (s, c) = &v[n];
                if let Some(i) = state_index.get(s) {
                    state[n].0 = *i as u8;
                }
                if c != NONE {
                    if let Some(c) = code_map.get(c) {
                        state[n].1 = *c;
                    }
                }
            }
            vec.push((index, state));
        }
    }
    vec.sort_by(|a, b| a.0.cmp(&b.0));
    vec.into_iter().map(|i| i.1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build() {
        let v = build_decode_state_array();
        let r = v == DECODE_STATE_ARRAY;
        println!("build: {}", r);
    }
}
