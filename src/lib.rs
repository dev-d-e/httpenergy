//!A library for parsing HTTP request and response.
//!
//!```
//!use httpenergy::{to_request, Request};
//!use httpenergy::{to_response, Response};
//!
//!let request = to_request();
//!request.method();
//!
//!let response = to_response();
//!response.status_code();
//!```
//!

#![allow(dead_code)]

mod common;
#[macro_use]
mod request;
mod response;

pub use request::*;
pub use response::*;

///Parse request bytes to `RequestUnits`.
pub fn to_request(bytes: &[u8]) -> RequestUnits {
    RequestBuilder::new().from_bytes(bytes)
}

///Parse request bytes to `Request`.
pub fn into_request(bytes: Vec<u8>) -> Request {
    RequestBuilder::new().from_vec(bytes)
}

///Parse response bytes to `ResponseUnits`.
pub fn to_response(bytes: &[u8]) -> ResponseUnits {
    ResponseBuilder::new().from_bytes(bytes)
}

///Parse response bytes to `Response`.
pub fn into_response(bytes: Vec<u8>) -> Response {
    ResponseBuilder::new().from_vec(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request() {
        let s =
            "GET / HTTP1.1\r\nAccept: text/*, text/plain, text/plain;format=flowed, */*\r\n\r\n";
        let mut r = to_request(s.as_bytes());
        let m = r.method().to_string();
        let v = r.header_value_string("Accept");
        println!("{:?}  {:?}", m, v);
        assert_eq!(m, "GET");
    }

    #[test]
    fn test_response() {
        let s = "HTTP1.1 200 \r\nAccept: text/*, text/plain, text/plain;format=flowed, */*\r\n\r\n";
        let mut r = to_response(s.as_bytes());
        let c = r.status_code().to_string();
        let v = r.header_value_string("Accept");
        println!("{:?}  {:?}", c, v);
        assert_eq!(c, "200");
    }
}
