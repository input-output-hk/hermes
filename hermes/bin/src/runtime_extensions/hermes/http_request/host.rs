//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::http_request::api::{Host, Payload},
        hermes::http_request::{tokio_runtime_task::{parse_payload, ParsedPayload}, STATE},
    },
};

// fn http_request(payload: Payload) -> String {
//     let ParsedPayload {
//         body_str,
//         request_line,
//         url,
//     } = parse_payload(payload);

//     let client = reqwest::blocking::Client::new();
//     let response = if request_line.starts_with("POST") {
//         let body_content = body_str.split("\r\n\r\n").last().unwrap_or("");
//         client
//             .post(&url)
//             .body(body_content.to_string())
//             .send()
//             .unwrap()
//     } else {
//         client.get(&url).send().unwrap()
//     };

//     response
//         .text()
//         .unwrap_or_else(|_| "Failed to read response".to_string())
// }

impl Host for HermesRuntimeContext {
    fn send(&mut self, payload: Payload) -> wasmtime::Result<bool> {
        STATE.tokio_rt_handle.send(payload).unwrap();

        // tracing::error!("Sending payload: {payload:?}");

        // let res = http_request(payload);
        // tracing::error!("POST Response: {res}");

        // let get_body = b"\
        // GET /get?param1=value1 HTTP/1.1\r\n\
        // Host: httpbin.org\r\n\
        // \r\n";
        //
        // let get_payload = Payload {
        // host_uri: "http://httpbin.org".to_string(),
        // port: 80,
        // body: get_body.to_vec(),
        // request_id: Some("req-get".to_string()),
        // };
        // let res = http_request(get_payload);
        // tracing::error!("GET Response: {res}");

        Ok(true)
    }
}
