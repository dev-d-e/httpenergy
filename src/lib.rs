//!A library for parsing HTTP request and response.
//!
//!```
//!use httpenergy::{H1Request, H1RequestUnits, H1RequestDecoder};
//!
//!let request = H1Request::with_method_target("GET", "/");
//!let mut s = Vec::new();
//!request.export(&mut s);
//!
//!let units=H1RequestUnits::new(&s);
//!let mut r =H1Request::new();
//!units.copy_to_request(&mut r);
//!println!("H1RequestUnits: {:?}", r.method());
//!
//!let decoder = H1RequestDecoder::new(s);
//!let r2 =decoder.to_request();
//!println!("H1RequestDecoder: {:?}", r2.method());
//!
//!```
//!

#![allow(dead_code)]

mod common;
pub mod h2;
mod io;
mod prty;
#[macro_use]
mod request;
mod response;

pub use io::*;
pub use prty::*;
pub use request::*;
pub use response::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request() {
        let mut r = H1Request::with_method_target("GET", "/");
        r.headers_mut().add_field(
            "Accept".to_string(),
            Vec::from("text/*, text/plain, text/plain;format=flowed, */*"),
        );
        r.headers_mut()
            .add_field("Host".to_string(), Vec::from("www "));

        let mut s = Vec::new();
        r.export(&mut s);

        let mut u = H1RequestUnits::new(&s);
        println!("Accept: {:?}", u.header_value_string("Accept"));
        s.extend_from_slice(b"aaaaaaaaaaaaaaaaaaaa");
        u.set_slice(&s);
        println!("Host: {:?}", u.header_value_string("Host"));

        let d = H1RequestDecoder::new(s);
        let rst = d.to_request();
        println!("{:?}", rst);
        assert_eq!("GET", rst.method());
    }

    #[test]
    fn test_response() {
        let mut r = H1Response::with_status_code("200");
        r.headers_mut().add_field(
            "Accept".to_string(),
            Vec::from("text/*, text/plain, text/plain;format=flowed, */*"),
        );

        let mut s = Vec::new();
        r.export(&mut s);

        let mut u = H1ResponseUnits::new(&s);
        println!("Accept: {:?}", u.header_value_string("Accept"));

        let d = H1ResponseDecoder::new(s);
        let rst = d.to_response();
        println!("{:?}", rst);
        assert_eq!("200", rst.status_code());
    }
}
