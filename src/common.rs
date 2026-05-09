use crate::io::*;
use crate::prty::*;
use std::collections::HashMap;
use std::sync::LazyLock;

pub(crate) const CR: u8 = b'\r';

pub(crate) const LF: u8 = b'\n';

pub(crate) const SPACE: u8 = b' ';

pub(crate) const HTAB: u8 = 9;

pub(crate) const COLON: u8 = b':';

pub(crate) const COMMA: u8 = b',';

pub(crate) const HYPHEN: u8 = b'-';

pub(crate) const DOT: u8 = b'.';

pub(crate) const SLASH: u8 = b'/';

pub(crate) const VERSION: &[u8] = b"HTTP/1.1";

#[inline(always)]
pub(crate) fn to_u32(o: &mut dyn GetU8) -> Option<u32> {
    o.get_exact(4)
        .map(|v| [v[0], v[1], v[2], v[3]])
        .map(|v| u32::from_be_bytes(v))
}

#[inline(always)]
pub(crate) fn to_u64(o: &mut dyn GetU8) -> Option<u64> {
    o.get_exact(8)
        .map(|v| [v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]])
        .map(|v| u64::from_be_bytes(v))
}

#[inline(always)]
pub(crate) fn u16_to(n: u16, o: &mut dyn PutU8) {
    o.put_exact(&n.to_be_bytes());
}

#[inline(always)]
pub(crate) fn u32_to(n: u32, o: &mut dyn PutU8) {
    o.put_exact(&n.to_be_bytes());
}

#[inline(always)]
pub(crate) fn u64_to(n: u64, o: &mut dyn PutU8) {
    o.put_exact(&n.to_be_bytes());
}

///Converts a slice to [`FieldName`]. Use into() if there is a &'static \[u8] or `Vec<u8>` or `String`.
#[inline(always)]
pub fn to_field_name(o: &[u8]) -> Option<FieldName> {
    crate::h2::hpack::static_table_get_field_name(o)
        .or(crate::h3::qpack::static_table_get_field_name(o))
        .or(get_field_name(o))
        .cloned()
}

///Converts a slice to [`FieldValue`]. Use into() if there is a &'static \[u8] or `Vec<u8>` or `String`.
#[inline(always)]
pub fn to_field_value(o: &[u8]) -> Option<FieldValue> {
    crate::h2::hpack::static_table_get_value(o)
        .or(crate::h3::qpack::static_table_get_value(o))
        .cloned()
}

///Converts a slice to [`FieldValue`]. Use into() if there is a &'static \[u8] or `Vec<u8>` or `String`.
#[inline(always)]
pub fn into_field_value(o: &[u8]) -> FieldValue {
    to_field_value(o).unwrap_or_else(|| FieldValue::owned(o.to_vec()))
}

#[inline(always)]
fn get_field_name(o: &[u8]) -> Option<&'static FieldName> {
    FIELD_NAME_MAP.get(o)
}

static FIELD_NAME_MAP: LazyLock<HashMap<&[u8], FieldName>> = LazyLock::new(|| {
    let mut v = HashMap::new();
    for &o in FIELD_NAME {
        v.insert(o, o.into());
    }
    v
});

const FIELD_NAME: &[&[u8]] = &[
    b"accept",
    b"accept-charset",
    b"accept-encoding",
    b"accept-language",
    b"accept-ranges",
    b"access-control-allow-credentials",
    b"access-control-allow-headers",
    b"access-control-allow-methods",
    b"access-control-allow-origin",
    b"access-control-expose-headers",
    b"access-control-max-age",
    b"access-control-request-headers",
    b"access-control-request-method",
    b"age",
    b"allow",
    b"alt-svc",
    b"authorization",
    b"cache-control",
    b"cache-status",
    b"cdn-cache-control",
    b"connection",
    b"content-disposition",
    b"content-encoding",
    b"content-language",
    b"content-length",
    b"content-location",
    b"content-range",
    b"content-security-policy",
    b"content-security-policy-report-only",
    b"content-type",
    b"cookie",
    b"dnt",
    b"date",
    b"etag",
    b"expect",
    b"expires",
    b"forwarded",
    b"from",
    b"host",
    b"if-match",
    b"if-modified-since",
    b"if-none-match",
    b"if-range",
    b"if-unmodified-since",
    b"last-modified",
    b"link",
    b"location",
    b"max-forwards",
    b"origin",
    b"pragma",
    b"proxy-authenticate",
    b"proxy-authorization",
    b"public-key-pins",
    b"public-key-pins-report-only",
    b"range",
    b"referer",
    b"referrer-policy",
    b"refresh",
    b"retry-after",
    b"sec-websocket-accept",
    b"sec-websocket-extensions",
    b"sec-websocket-key",
    b"sec-websocket-protocol",
    b"sec-websocket-version",
    b"server",
    b"set-cookie",
    b"strict-transport-security",
    b"te",
    b"trailer",
    b"transfer-encoding",
    b"upgrade",
    b"upgrade-insecure-requests",
    b"user-agent",
    b"vary",
    b"via",
    b"warning",
    b"www-authenticate",
    b"x-content-type-options",
    b"x-dns-prefetch-control",
    b"x-frame-options",
    b"x-xss-protection",
];
