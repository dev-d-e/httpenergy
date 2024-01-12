# httpenergy

A library for parsing HTTP request and response.

 ```
use httpenergy::{to_request, Request};
use httpenergy::{to_response, Response};

let request = to_request();
request.method();

let response = to_response();
response.status_code();
```
