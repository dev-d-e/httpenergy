use httpenergy::h2::frame::*;
use httpenergy::h2::hpack::*;
use httpenergy::h2::*;
use httpenergy::*;

#[test]
pub fn h2_request() {
    let mut r = H2Request::new("POST");
    r.set_scheme(Some("https"));
    r.set_authority(Some("example.org"));
    r.set_path(Some("/resource"));
    r.add_field("content-type", "image/jpeg");
    r.add_field("host", "example.org");
    r.add_field("content-length", "123");
    r.add_field(
        "Accept",
        "text/*, text/plain, text/plain;format=flowed, */*",
    );
    r.add_field("Accept", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    r.add_field("Accept", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    let mut helper = HeadersHelper::new(1, 100, 100);
    handle_request_pseudo_header_fields(&r, &mut helper);
    handle_fields(&r, &mut helper);
    let mut s = Vec::new();
    helper.export(&mut s);
    assert!(s.len() > 0);

    let mut t = DynamicTable::default();
    let mut req = H2Request::default();
    let mut g = s.into_get();
    if let Ok(rst) = get_frame(&mut g) {
        match rst {
            FrameResult::Headers(o) => {
                let mut r = Vec::new();
                if let Some(mut f) = o.field_block_fragment(&mut g) {
                    r = f.get_surplus().to_vec();
                }
                while let Ok(rst) = get_frame(&mut g) {
                    match rst {
                        FrameResult::Continuation(b) => {
                            if let Some(mut f) = b.field_block_fragment(&mut g) {
                                r.extend_from_slice(f.get_surplus());
                            }
                        }
                        _ => {}
                    }
                }
                if let Ok(v) = get_hfris_to_vec(&mut r.into_get()) {
                    let v = update_dynamic_table_to_vec(v, &mut t);
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
