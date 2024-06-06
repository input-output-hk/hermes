// Allow everything since this is generated code.
#[allow(clippy::all, unused)]

mod hermes {
    //! Hermes `wasmtime::component::bindgen` generated code
    //!
    //! *Note*
    //! Inspect the generated code with:
    //! `cargo expand -p hermes --lib runtime_extensions::bindings`
    //! or with:
    //! `earthly +bindings-expand`

    #![allow(clippy::indexing_slicing)]

    use wasmtime::component::bindgen;

    bindgen!({
        world: "hermes",
        path: "../../wasi/wit",
    });
}

struct TestComponent;
