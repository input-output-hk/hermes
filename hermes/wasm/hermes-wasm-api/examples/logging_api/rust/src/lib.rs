// src/lib.rs

// Use a procedural macro to generate bindings for the world we specified in
// `host.wit`
wit_bindgen::generate!("hermes");

// Define a custom type and implement the generated `Host` trait for it which
// represents implementing all the necessary exported interfaces for this
// component.
struct MyHost;

impl Host for MyHost {
    fn run() {
        print("Hello, world!");
    }
}

export_host!(MyHost);
