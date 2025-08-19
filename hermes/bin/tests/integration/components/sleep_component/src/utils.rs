use url::Url;

use crate::bindings::{self, hermes::http_request::api::Payload};

// TODO[RC]: Handle errors
// gracefully.
pub(crate) fn make_payload(
    http_server: &str,
    request_id: Option<u64>,
) -> Payload {
    let parsed = Url::parse(http_server).expect("invalid URL");
    let scheme = parsed.scheme();
    let host_uri = parsed.host_str().expect("invalid host URI").to_string();
    let port = parsed.port_or_known_default().expect("invalid port");
    let body = make_body(&host_uri);

    Payload {
        host_uri: format!("{scheme}://{host_uri}"),
        port,
        body,
        request_id,
    }
}

fn make_body(host_uri: &str) -> Vec<u8> {
    let request_body = format!(
        "GET /test.txt HTTP/1.1\r\n\
        Host: {host_uri}\r\n\
        Content-Type: application/json\r\n\
        Content-Length: 15\r\n\
        Connection: close\r\n\
        \r\n\
        {{\"key\":\"value\"}}"
    );
    request_body.into_bytes()
}

pub(crate) fn test_log(s: &str) {
    bindings::hermes::logging::api::log(
        bindings::hermes::logging::api::Level::Trace,
        None,
        None,
        None,
        None,
        None,
        format!("[TEST] {s}").as_str(),
        None,
    );
}

pub(crate) fn busy_wait_s(secs: u64) {
    let start = bindings::wasi::clocks::monotonic_clock::now();
    let target = start
        .checked_add(
            secs.checked_mul(1_000_000_000)
                .expect("multiplication overflowed"),
        )
        .expect("addition overflowed");
    loop {
        if bindings::wasi::clocks::monotonic_clock::now() >= target {
            break;
        }
    }
}
