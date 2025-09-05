//! Hermes http reply module integration test with WASM runtime.
//! Generate `hermes.rs` with `earthly +gen-bindings` before writing the test.

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            exports hermes:http-gateway/event;
            export hermes:integration-test/event;
        }
    ",
    generate_all,
});

use exports::hermes::{
    http_gateway::event::{Bstr, Guest as _, Headers, HttpGatewayResponse},
    integration_test::event::TestResult,
};

struct TestComponent;

impl exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        match test {
            0 => test_http_reply(run),

            _ => None,
        }
    }

    fn bench(_test: u32, _run: bool) -> Option<TestResult> {
        None
    }
}

fn test_http_reply(run: bool) -> Option<TestResult> {
    let body_bytes: Vec<u8> = (0..1024).map(|_| 0 as u8).collect();
    let header = vec![("key".to_string(), vec!["values".to_string()])];
    let reply = TestComponent::reply(body_bytes, header, "path".to_string(), "method".to_string());

    let status = if let Some(reply) = reply {
        match reply {
            HttpGatewayResponse::Http(http_resp) => http_resp.code == 200,
            HttpGatewayResponse::InternalRedirect(_) => true, // or false, depending on your test logic
        }
    } else {
        false
    };

    Some(TestResult {
        name: "HTTP reply".to_string(),
        status,
    })
}

impl exports::hermes::http_gateway::event::Guest for TestComponent {
    fn reply(
        _body: Bstr,
        _headers: Headers,
        _path: String,
        _method: String,
    ) -> Option<HttpGatewayResponse> {
        None
    }
}
