use httpenergy::*;

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
