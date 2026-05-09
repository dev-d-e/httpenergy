use httpenergy::h3::frame::*;
use httpenergy::h3::*;
use httpenergy::*;

#[test]
pub fn h3_request() {
    let mut r = H3Request::new("POST");
    r.set_scheme(Some("https"));
    r.set_authority(Some("example.org"));
    r.set_path(Some("/resource"));
    r.add_field("content-type", "image/jpeg");
    r.add_field("host", "example.org");
    r.add_field("content-length", "123");

    let mut h = Headers::new(1000);
    handle_request_pseudo_header_fields(&r, &mut *h);
    handle_fields(&r, &mut *h);
    let mut s = Vec::new();
    h.export(&mut s);

    let mut t = DynamicTable::default();
    let mut req = H3Request::default();
    let mut g = s.into_get();
    if let Ok(rst) = get_frame(&mut g) {
        match rst {
            FrameResult::Headers(o) => {
                if let Ok(v) = o.get_field(&mut g).map_err(|e| println!("{e}")) {
                    let v = handle_field_line_representations_to_vec(v, &mut t);
                    add_fields_to_request(v, &mut req);
                }
            }
            _ => {}
        }
    }

    assert_eq!(r.method(), req.method());
    assert_eq!(r.scheme(), req.scheme());
    assert_eq!(r.authority(), req.authority());
    assert_eq!(r.path(), req.path());
}
