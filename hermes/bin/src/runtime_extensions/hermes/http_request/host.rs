//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::http_request::api::{Host, Payload},
};

fn http_request(payload: Payload) -> String {
    let body_str = String::from_utf8(payload.body).unwrap();
    let request_line = body_str.lines().next().ok_or("Empty HTTP body").unwrap();

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        tracing::error!("E1");
    }
    let path = parts[1];

    let scheme = if payload.host_uri.starts_with("https") {
        "https"
    } else {
        "http"
    };
    let domain = payload
        .host_uri
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    let url = format!("{}://{}:{}{}", scheme, domain, payload.port, path);
    tracing::error!("Full URL: {}", url);

    let client = reqwest::blocking::Client::new();
    let response = if request_line.starts_with("POST") {
        let body_content = body_str.split("\r\n\r\n").last().unwrap_or("");
        client
            .post(&url)
            .body(body_content.to_string())
            .send()
            .unwrap()
    } else {
        client.get(&url).send().unwrap()
    };

    response
        .text()
        .unwrap_or_else(|_| "Failed to read response".to_string())
}

impl Host for HermesRuntimeContext {
    fn send(&mut self, payload: Payload) -> wasmtime::Result<bool> {
        tracing::error!("Sending payload: {payload:?}");


        let res = http_request(payload);
        tracing::error!("POST Response: {res}");

        /*
        let get_body = b"\
        GET /get?param1=value1 HTTP/1.1\r\n\
        Host: httpbin.org\r\n\
        \r\n";

        let get_payload = Payload {
            host_uri: "http://httpbin.org".to_string(),
            port: 80,
            body: get_body.to_vec(),
            request_id: Some("req-get".to_string()),
        };
        let res = http_request(get_payload);
        tracing::error!("GET Response: {res}");
        */

        Ok(true)
    }
}
