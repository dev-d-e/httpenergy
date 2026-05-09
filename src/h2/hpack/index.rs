use crate::prty::*;
use std::collections::HashMap;
use std::sync::LazyLock;

///Returns an entry in the static table corresponding to the index, or an index in the dynamic table.
///
///The static table and the dynamic table are combined into a single index address space.
#[inline(always)]
pub fn static_table_get_entry(n: usize) -> Result<&'static (FieldName, FieldValue), Option<usize>> {
    if n > 0 && n <= STATIC_TABLE_LEN {
        Ok(&ENTRY_ARRAY[n - 1])
    } else if n > STATIC_TABLE_LEN {
        Err(Some(n - STATIC_TABLE_LEN))
    } else {
        Err(None)
    }
}

///Returns field name in the static table corresponding to the index, or an index in the dynamic table.
#[inline(always)]
pub fn static_table_get_name(n: usize) -> Result<&'static FieldName, Option<usize>> {
    static_table_get_entry(n).map(|(a, _)| a)
}

static ENTRY_ARRAY: LazyLock<[(FieldName, FieldValue); STATIC_TABLE_LEN]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        let (a, b) = STATIC_TABLE[i];
        (a.into(), b.into())
    })
});

///Returns an index in the static table corresponding to the entry/name-value pair.
#[inline(always)]
pub fn static_table_get_entry_index(name: &[u8], value: &[u8]) -> Option<usize> {
    ENTRY_MAP.get(&(name, value)).copied()
}

static ENTRY_MAP: LazyLock<HashMap<(&[u8], &[u8]), usize>> = LazyLock::new(|| {
    let mut v = HashMap::new();
    for (i, (a, b)) in STATIC_TABLE.iter().enumerate() {
        v.insert((*a, *b), i + 1);
    }
    v
});

///Returns an index in the static table corresponding to the name.
#[inline(always)]
pub fn static_table_get_name_index(name: &[u8]) -> Option<usize> {
    NAME_MAP.get(name).map(|(n, _)| *n)
}

///Returns field name in the static table corresponding to the name.
#[inline(always)]
pub fn static_table_get_field_name(name: &[u8]) -> Option<&'static FieldName> {
    NAME_MAP.get(name).map(|(_, o)| *o)
}

static NAME_MAP: LazyLock<HashMap<&[u8], (usize, &FieldName)>> = LazyLock::new(|| {
    let mut v = HashMap::new();
    for (i, (o, _)) in STATIC_TABLE.iter().enumerate() {
        if !v.contains_key(o) {
            v.insert(*o, (i + 1, &ENTRY_ARRAY[i].0));
        }
    }
    v
});

///Returns field value in the static table corresponding to the value.
#[inline(always)]
pub fn static_table_get_value(value: &[u8]) -> Option<&'static FieldValue> {
    VALUE_MAP.get(value).map(|o| *o)
}

static VALUE_MAP: LazyLock<HashMap<&[u8], &FieldValue>> = LazyLock::new(|| {
    let mut v = HashMap::new();
    for (i, (_, o)) in STATIC_TABLE.iter().enumerate() {
        if !v.contains_key(o) {
            v.insert(*o, &ENTRY_ARRAY[i].1);
        }
    }
    v
});

const STATIC_TABLE_LEN: usize = 61;
const STATIC_TABLE: [(&[u8], &[u8]); STATIC_TABLE_LEN] = [
    (b":authority", b""),
    (b":method", b"GET"),
    (b":method", b"POST"),
    (b":path", b"/"),
    (b":path", b"/index.html"),
    (b":scheme", b"http"),
    (b":scheme", b"https"),
    (b":status", b"200"),
    (b":status", b"204"),
    (b":status", b"206"),
    (b":status", b"304"),
    (b":status", b"400"),
    (b":status", b"404"),
    (b":status", b"500"),
    (b"accept-charset", b""),
    (b"accept-encoding", b"gzip, deflate"),
    (b"accept-language", b""),
    (b"accept-ranges", b""),
    (b"accept", b""),
    (b"access-control-allow-origin", b""),
    (b"age", b""),
    (b"allow", b""),
    (b"authorization", b""),
    (b"cache-control", b""),
    (b"content-disposition", b""),
    (b"content-encoding", b""),
    (b"content-language", b""),
    (b"content-length", b""),
    (b"content-location", b""),
    (b"content-range", b""),
    (b"content-type", b""),
    (b"cookie", b""),
    (b"date", b""),
    (b"etag", b""),
    (b"expect", b""),
    (b"expires", b""),
    (b"from", b""),
    (b"host", b""),
    (b"if-match", b""),
    (b"if-modified-since", b""),
    (b"if-none-match", b""),
    (b"if-range", b""),
    (b"if-unmodified-since", b""),
    (b"last-modified", b""),
    (b"link", b""),
    (b"location", b""),
    (b"max-forwards", b""),
    (b"proxy-authenticate", b""),
    (b"proxy-authorization", b""),
    (b"range", b""),
    (b"referer", b""),
    (b"refresh", b""),
    (b"retry-after", b""),
    (b"server", b""),
    (b"set-cookie", b""),
    (b"strict-transport-security", b""),
    (b"transfer-encoding", b""),
    (b"user-agent", b""),
    (b"vary", b""),
    (b"via", b""),
    (b"www-authenticate", b""),
];
