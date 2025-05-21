use std::collections::{HashMap, VecDeque};
use std::sync::LazyLock;

///HPACK uses two tables for associating header fields to indexes.
///The static table and the dynamic table are combined into a single index address space.
pub trait Indices {
    ///Dynamic table size
    fn size(&self) -> usize;

    ///Maximum dynamic table size change
    fn size_update(&mut self, n: usize);

    ///Entry Eviction
    fn eviction(&mut self);

    ///A new entry is added to the dynamic table.
    fn add(&mut self, name: Vec<u8>, value: Vec<u8>);

    ///Returns an entry corresponding to the index.
    fn get_entry(&self, n: usize) -> Option<(&[u8], &[u8])>;

    ///Returns a name corresponding to the index.
    fn get_name(&self, n: usize) -> Option<&[u8]> {
        self.get_entry(n).map(|s| s.0)
    }

    ///Returns some indexes corresponding to the name-value pair.
    fn find_name_value(&self, name: &[u8], value: &[u8]) -> Vec<usize>;

    ///Returns some indexes corresponding to the name.
    fn find_name(&self, name: &[u8]) -> Vec<usize>;

    ///Returns result of finding an index.
    fn find_an_index<'a>(&self, name: &[u8], value: &'a [u8]) -> IndexResult<'a> {
        let r = self.find_name_value(name, value);
        if r.is_empty() {
            let r = self.find_name(name);
            if r.is_empty() {
                IndexResult::None
            } else {
                IndexResult::One(r[0], value)
            }
        } else {
            IndexResult::Both(r[0])
        }
    }
}

///Represents the result of finding an index.
pub enum IndexResult<'a> {
    ///Identifies an entry(name-value pair) in either the static table or the dynamic table.
    Both(usize),
    ///Identifies a name in either the static table or the dynamic table.
    One(usize, &'a [u8]),
    ///No index.
    None,
}

///Indexing Tables
pub struct IndexingTables(usize, VecDeque<(Vec<u8>, Vec<u8>)>);

impl IndexingTables {
    ///Creates an empty dynamic table.
    pub fn new() -> Self {
        Self(4096, VecDeque::new())
    }

    ///Clears the dynamic table.
    pub fn clear(&mut self) {
        self.1.clear();
    }
}

impl Indices for IndexingTables {
    fn size(&self) -> usize {
        let mut i = 0;
        for (a, b) in &self.1 {
            i += a.len() + b.len() + 32;
        }
        i
    }

    fn size_update(&mut self, n: usize) {
        self.0 = n;
        self.eviction();
    }

    fn eviction(&mut self) {
        while self.size() > self.0 {
            self.1.pop_back();
        }
    }

    fn add(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.1.push_front((name, value));
        self.eviction();
    }

    fn get_entry(&self, n: usize) -> Option<(&[u8], &[u8])> {
        if n > 0 && n <= STATIC_TABLE_LEN {
            let o = STATIC_TABLE[n - 1];
            return Some((o.0.as_bytes(), o.1.as_bytes()));
        } else if n > STATIC_TABLE_LEN {
            let n = n - STATIC_TABLE_LEN - 1;
            return self.1.get(n).map(|s| (s.0.as_slice(), s.1.as_slice()));
        }
        None
    }

    fn find_name_value(&self, name: &[u8], value: &[u8]) -> Vec<usize> {
        let mut v = Vec::new();
        let mut s = name.to_vec();
        s.extend_from_slice(value);
        if let Some(i) = STATIC_TABLE_INDEX.get(&s) {
            v.push(*i);
        }
        for (i, s) in self.1.iter().enumerate() {
            if s.0 == name && s.1 == value {
                v.push(i + STATIC_TABLE_LEN + 1);
            }
        }
        v
    }

    fn find_name(&self, name: &[u8]) -> Vec<usize> {
        let mut v = Vec::new();
        if let Some(s) = STATIC_TABLE_INDICES.get(name) {
            v.extend_from_slice(s);
        }
        for (i, a) in self.1.iter().enumerate() {
            if a.0 == name {
                v.push(i + STATIC_TABLE_LEN + 1);
            }
        }
        v
    }
}

static STATIC_TABLE_INDEX: LazyLock<HashMap<Vec<u8>, usize>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for i in 0..STATIC_TABLE_LEN {
        let a = STATIC_TABLE[i];
        let mut v = a.0.as_bytes().to_vec();
        v.extend_from_slice(a.1.as_bytes());
        map.insert(v, i + 1);
    }
    map
});

static STATIC_TABLE_INDICES: LazyLock<HashMap<Vec<u8>, Vec<usize>>> = LazyLock::new(|| {
    let mut map: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();
    for i in 0..STATIC_TABLE_LEN {
        let a = STATIC_TABLE[i];
        if let Some(o) = map.get_mut(a.0.as_bytes()) {
            o.push(i + 1);
        } else {
            map.insert(a.0.as_bytes().to_vec(), vec![i + 1]);
        }
    }
    map
});

const STATIC_TABLE_LEN: usize = 61;
const STATIC_TABLE: [(&str, &str); STATIC_TABLE_LEN] = [
    (":authority", ""),
    (":method", "GET"),
    (":method", "POST"),
    (":path", "/"),
    (":path", "/index.html"),
    (":scheme", "http"),
    (":scheme", "https"),
    (":status", "200"),
    (":status", "204"),
    (":status", "206"),
    (":status", "304"),
    (":status", "400"),
    (":status", "404"),
    (":status", "500"),
    ("accept-charset", ""),
    ("accept-encoding", "gzip, deflate"),
    ("accept-language", ""),
    ("accept-ranges", ""),
    ("accept", ""),
    ("access-control-allow-origin", ""),
    ("age", ""),
    ("allow", ""),
    ("authorization", ""),
    ("cache-control", ""),
    ("content-disposition", ""),
    ("content-encoding", ""),
    ("content-language", ""),
    ("content-length", ""),
    ("content-location", ""),
    ("content-range", ""),
    ("content-type", ""),
    ("cookie", ""),
    ("date", ""),
    ("etag", ""),
    ("expect", ""),
    ("expires", ""),
    ("from", ""),
    ("host", ""),
    ("if-match", ""),
    ("if-modified-since", ""),
    ("if-none-match", ""),
    ("if-range", ""),
    ("if-unmodified-since", ""),
    ("last-modified", ""),
    ("link", ""),
    ("location", ""),
    ("max-forwards", ""),
    ("proxy-authenticate", ""),
    ("proxy-authorization", ""),
    ("range", ""),
    ("referer", ""),
    ("refresh", ""),
    ("retry-after", ""),
    ("server", ""),
    ("set-cookie", ""),
    ("strict-transport-security", ""),
    ("transfer-encoding", ""),
    ("user-agent", ""),
    ("vary", ""),
    ("via", ""),
    ("www-authenticate", ""),
];
