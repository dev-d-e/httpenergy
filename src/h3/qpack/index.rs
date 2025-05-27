use crate::common::*;
use getset::CopyGetters;
use std::collections::VecDeque;

///A trait for dynamic table index address space.
pub trait DynamicIndices {
    ///Dynamic table size
    fn size(&self) -> usize;

    ///Set Dynamic Table Capacity
    fn set_capacity(&mut self, n: usize);

    ///Count of entries inserted
    fn max_absolute(&self) -> usize;

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
}

///Dynamic Table.
#[derive(CopyGetters)]
pub struct DynamicTable {
    #[getset(get_copy = "pub")]
    capacity: usize,
    absolute: usize,
    buffer: VecDeque<(Vec<u8>, Vec<u8>)>,
}

impl std::fmt::Debug for DynamicTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.buffer.iter().map(|(a, b)| (into_str(a), into_str(b))))
            .finish()
    }
}

impl DynamicTable {
    ///Creates an empty dynamic table with capacity 4096.
    pub fn new() -> Self {
        Self {
            capacity: 4096,
            absolute: 0,
            buffer: VecDeque::new(),
        }
    }

    ///Clears the dynamic table.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl DynamicIndices for DynamicTable {
    fn size(&self) -> usize {
        let mut i = 0;
        for (a, b) in &self.buffer {
            i += a.len() + b.len() + 32;
        }
        i
    }

    fn set_capacity(&mut self, n: usize) {
        self.capacity = n;
        self.eviction();
    }

    fn max_absolute(&self) -> usize {
        self.absolute
    }

    fn eviction(&mut self) {
        while self.size() > self.capacity {
            self.buffer.pop_back();
        }
    }

    fn add(&mut self, name: Vec<u8>, value: Vec<u8>) {
        self.buffer.push_front((name, value));
        self.absolute += 1;
        self.eviction();
    }

    fn get_entry(&self, n: usize) -> Option<(&[u8], &[u8])> {
        self.buffer.get(n).map(|s| (s.0.as_slice(), s.1.as_slice()))
    }

    fn find_name_value(&self, name: &[u8], value: &[u8]) -> Vec<usize> {
        let mut v = Vec::new();
        for (i, s) in self.buffer.iter().enumerate() {
            if s.0 == name && s.1 == value {
                v.push(i);
            }
        }
        v
    }

    fn find_name(&self, name: &[u8]) -> Vec<usize> {
        let mut v = Vec::new();
        for (i, a) in self.buffer.iter().enumerate() {
            if a.0 == name {
                v.push(i);
            }
        }
        v
    }
}

///Static Table.
pub struct StaticTable;

impl StaticTable {
    ///Returns an entry corresponding to the index.
    pub fn get_entry(n: usize) -> Option<(&'static [u8], &'static [u8])> {
        if n < STATIC_TABLE_LEN {
            let o = STATIC_TABLE[n];
            Some((o.0.as_bytes(), o.1.as_bytes()))
        } else {
            None
        }
    }

    ///Returns a name corresponding to the index.
    pub fn get_name(n: usize) -> Option<&'static [u8]> {
        Self::get_entry(n).map(|s| s.0)
    }
}

const STATIC_TABLE_LEN: usize = 99;
const STATIC_TABLE: [(&str, &str); STATIC_TABLE_LEN] = [
    (":authority", ""),
    (":path", "/"),
    ("age", "0"),
    ("content-disposition", ""),
    ("content-length", "0"),
    ("cookie", ""),
    ("date", ""),
    ("etag", ""),
    ("if-modified-since", ""),
    ("if-none-match", ""),
    ("last-modified", ""),
    ("link", ""),
    ("location", ""),
    ("referer", ""),
    ("set-cookie", ""),
    (":method", "CONNECT"),
    (":method", "DELETE"),
    (":method", "GET"),
    (":method", "HEAD"),
    (":method", "OPTIONS"),
    (":method", "POST"),
    (":method", "PUT"),
    (":scheme", "http"),
    (":scheme", "https"),
    (":status", "103"),
    (":status", "200"),
    (":status", "304"),
    (":status", "404"),
    (":status", "503"),
    ("accept", "*/*"),
    ("accept", "application/dns-message"),
    ("accept-encoding", "gzip, deflate, br"),
    ("accept-ranges", "bytes"),
    ("access-control-allow-headers", "cache-control"),
    ("access-control-allow-headers", "content-type"),
    ("access-control-allow-origin", "*"),
    ("cache-control", "max-age=0"),
    ("cache-control", "max-age=2592000"),
    ("cache-control", "max-age=604800"),
    ("cache-control", "no-cache"),
    ("cache-control", "no-store"),
    ("cache-control", "public, max-age=31536000"),
    ("content-encoding", "br"),
    ("content-encoding", "gzip"),
    ("content-type", "application/dns-message"),
    ("content-type", "application/javascript"),
    ("content-type", "application/json"),
    ("content-type", "application/x-www-form-urlencoded"),
    ("content-type", "image/gif"),
    ("content-type", "image/jpeg"),
    ("content-type", "image/png"),
    ("content-type", "text/css"),
    ("content-type", "text/html; charset=utf-8"),
    ("content-type", "text/plain"),
    ("content-type", "text/plain;charset=utf-8"),
    ("range", "bytes=0-"),
    ("strict-transport-security", "max-age=31536000"),
    (
        "strict-transport-security",
        "max-age=31536000; includesubdomains",
    ),
    (
        "strict-transport-security",
        "max-age=31536000; includesubdomains; preload",
    ),
    ("vary", "accept-encoding"),
    ("vary", "origin"),
    ("x-content-type-options", "nosniff"),
    ("x-xss-protection", "1; mode=block"),
    (":status", "100"),
    (":status", "204"),
    (":status", "206"),
    (":status", "302"),
    (":status", "400"),
    (":status", "403"),
    (":status", "421"),
    (":status", "425"),
    (":status", "500"),
    ("accept-language", ""),
    ("access-control-allow-credentials", "FALSE"),
    ("access-control-allow-credentials", "TRUE"),
    ("access-control-allow-headers", "*"),
    ("access-control-allow-methods", "get"),
    ("access-control-allow-methods", "get, post, options"),
    ("access-control-allow-methods", "options"),
    ("access-control-expose-headers", "content-length"),
    ("access-control-request-headers", "content-type"),
    ("access-control-request-method", "get"),
    ("access-control-request-method", "post"),
    ("alt-svc", "clear"),
    ("authorization", ""),
    (
        "content-security-policy",
        "script-src 'none'; object-src 'none'; base-uri 'none'",
    ),
    ("early-data", "1"),
    ("expect-ct", ""),
    ("forwarded", ""),
    ("if-range", ""),
    ("origin", ""),
    ("purpose", "prefetch"),
    ("server", ""),
    ("timing-allow-origin", "*"),
    ("upgrade-insecure-requests", "1"),
    ("user-agent", ""),
    ("x-forwarded-for", ""),
    ("x-frame-options", "deny"),
    ("x-frame-options", "sameorigin"),
];
