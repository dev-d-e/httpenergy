# httpenergy

A library for parsing HTTP request and response.

 ```
use httpenergy::{H1Request, H1RequestUnits, H1RequestDecoder};

let request = H1Request::with_method_target("GET", "/");
let mut s = Vec::new();
request.export(&mut s);

let units=H1RequestUnits::new(&s);
let mut r =H1Request::new();
units.copy_to_request(&mut r);
println!("H1RequestUnits: {:?}", r.method());

let decoder = H1RequestDecoder::new(s);
let r2 =decoder.to_request();
println!("H1RequestDecoder: {:?}", r2.method());
```
