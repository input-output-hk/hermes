use httpmock::{Method::GET, MockServer};

pub fn start() -> MockServer {
    let server = MockServer::start();

    let _hello_mock = server.mock(|when, then| {
        when.method(GET).path("/test.txt");
        then.status(200)
            .header("content-type", "text/html; charset=UTF-8")
            .body("This is the content of the 'test.txt' file");
    });

    server
}
