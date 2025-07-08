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
    path: "../../wasm/wasi/wit",
    trappable_imports: true,
    // interfaces: "


    //     /// All possible Hermes Imports/Exports
    //     include hermes:binary/all;
    //     include hermes:cardano/all;
    //     include hermes:cbor/all;
    //     include hermes:cron/all;
    //     include hermes:crypto/all;
    //     include hermes:hash/all;
    //     include hermes:init/all;
    //     include hermes:ipfs/all;
    //     include hermes:json/all;
    //     include hermes:kv-store/all;
    //     include hermes:localtime/all;
    //     include hermes:logging/all;
    //     include hermes:sqlite/all;
    //     include hermes:integration-test/all;
    //     include hermes:http-gateway/all;
    // ",
});
