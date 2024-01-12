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
mod request;
mod response;

use common::*;
pub use request::*;
pub use response::*;

///Parse request data
pub fn to_request(bytes: &[u8]) -> Request {
    RequestBuilder::new().from_bytes(bytes)
}

///Parse response data
pub fn to_response(bytes: &[u8]) -> Response {
    ResponseBuilder::new().from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request() {
        let s = "GET / HTTP1.1\r\n";
        let mut r = to_request(s.as_bytes());
        let m = r.method().to_string();
        println!("{:?}\n{:?}", m, r);
        assert_eq!(m, "GET");
    }

    #[test]
    fn test_response() {
        let s = "HTTP1.1 200 \r\n";
        let mut r = to_response(s.as_bytes());
        let c = r.status_code().to_string();
        println!("{:?}\n{:?}", c, r);
        assert_eq!(c, "200");
    }
}
