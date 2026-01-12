//! Utility functions for HTTP gateway

use hyper::{HeaderMap, Response, body::Body};

use crate::runtime_extensions::hermes::http_gateway::event::HeadersKV;

/// Extract headers from request into `HeadersKV` format.
pub(crate) fn extract_headers_kv(headers: &HeaderMap) -> HeadersKV {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                vec![value.to_str().unwrap_or_default().to_string()],
            )
        })
        .collect()
}

/// Build HTTP response from status code, headers, and body
pub(crate) fn build_http_response<B>(
    status_code: u16,
    headers: Vec<(String, Vec<String>)>,
    body: Vec<u8>,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let mut response_builder = Response::builder().status(status_code);
    for (key, values) in headers {
        for value in values {
            response_builder = response_builder.header(&key, value);
        }
    }
    Ok(response_builder.body(body.into())?)
}
