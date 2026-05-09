/*!
Httpenergy is a crate for working with request and response format in HTTP/1.1, HTTP/2 and HTTP/3 protocols.

This crate does not provide anything about connection.

# Examples
* HTTP/1.1 example

There are two ways for parsing request bytes.
```
use httpenergy::*;
//Creates HTTP/1.1 request.
let request = H1Request::new("GET", "/");
let mut v = Vec::new();
request.export(&mut v);

//Parses bytes into request.
let mut g = v.into_get();
let units = H1RequestUnits::new(&mut g);
let mut r = H1Request::default();
units.copy_to_request(&mut g, &mut r);
assert_eq!("GET", r.method());
```
```
use httpenergy::*;
//Creates HTTP/1.1 request.
let request = H1Request::new("GET", "/");
let mut v = Vec::new();
request.export(&mut v);

//Parses bytes into request.
let r = H1RequestParser::new(v).to_request();
assert_eq!("GET", r.method());
```

* [HTTP/2 example](h2/index.html)

* [HTTP/3 example](h3/index.html)
*/

#![allow(dead_code)]

mod common;
pub mod h2;
pub mod h3;
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
    fn h1_request() {
        let mut r = H1Request::new("GET", "/");
        r.add_field(
            "Accept",
            "text/*, text/plain, text/plain;format=flowed, */*",
        );
        r.add_field("Host", "www ");

        let mut v = Vec::new();
        r.export(&mut v);

        let mut g = v.into_get();
        let mut o = H1RequestUnits::new(&mut g);
        println!("Accept: {:?}", o.header_value_string("Accept", &mut g));
        g.extend_from_slice(b"aaaaaaaaaaaaaaaaaaaa");
        println!("Host: {:?}", o.header_value_string("Host", &mut g));

        let s = g.take();
        println!("s: {:?}", str::from_utf8(&s).unwrap_or(""));
        let o = H1RequestParser::new(s);
        let rst = o.to_request();
        println!("{:?}", rst);
        assert_eq!("GET", rst.method());
    }

    #[test]
    fn h1_response() {
        let mut r = H1Response::new("200");
        r.add_field(
            "Accept",
            "text/*, text/plain, text/plain;format=flowed, */*",
        );

        let mut s = Vec::new();
        r.export(&mut s);

        let mut g = s.into_get();
        let mut o = H1ResponseUnits::new(&mut g);
        println!("Accept: {:?}", o.header_value_string("Accept", &mut g));

        let s = g.take();
        let o = H1ResponseParser::new(s);
        let rst = o.to_response();
        println!("{:?}", rst);
        assert_eq!("200", rst.status_code());
    }
}
