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
pub mod h2;
mod response;

pub use request::*;
pub use response::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request() {
        let mut s = Vec::from(
            "GET / HTTP1.1\r\nAccept: text/*, text/plain, text/plain;format=flowed, */*\r\n",
        );
        let mut r = new_request_units(&s);
        println!("Accept: {:?}", r.header_value_string("Accept", &s));
        s.extend_from_slice("Host:www\r\n\r\n ".as_bytes());
        if r.is_finish() {
            r = new_request_units(&s);
        }
        println!("Host: {:?}", r.header_value_string("Host", &s));

        let mut r = pack_request("GET", "/");
        r.set_header(
            String::from("Accept"),
            Vec::from("text/*, text/plain, text/plain;format=flowed, */*"),
        );
        let s = r.pack();
        let mut r = to_request(s);
        println!("Accept: {:?}", r.header_value_string("Accept"));
        assert_eq!(r.method(), "GET");
    }

    #[test]
    fn test_response() {
        let s = Vec::from(
            "HTTP1.1 200 \r\nAccept: text/*, text/plain, text/plain;format=flowed, */*\r\n\r\n",
        );
        let mut r = new_response_units(&s);
        println!("Accept: {:?}", r.header_value_string("Accept", &s));

        let mut r = pack_response("200");
        r.set_header(
            String::from("Accept"),
            Vec::from("text/*, text/plain, text/plain;format=flowed, */*"),
        );
        let s = r.pack();
        let mut r = to_response(s);
        println!("Accept: {:?}", r.header_value_string("Accept"));
        assert_eq!(r.status_code(), "200");
    }
}
