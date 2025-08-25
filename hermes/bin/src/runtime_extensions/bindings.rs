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
});

pub mod stub {
    super::bindgen!({
        path: "../../wasm/stub-module/wit",
        trappable_imports: true,
        with: {
            "wasi": super::wasi,
            "hermes:binary/api": super::hermes::binary::api,
            "hermes:cardano/api": super::hermes::cardano::api,
            "hermes:cbor/api": super::hermes::cbor::api,
            "hermes:cron/api": super::hermes::cron::api,
            "hermes:crypto/api": super::hermes::crypto::api,
            "hermes:hash/api": super::hermes::hash::api,
            "hermes:init/api": super::hermes::init::api,
            "hermes:ipfs/api": super::hermes::ipfs::api,
            "hermes:json/api": super::hermes::json::api,
            "hermes:kv-store/api": super::hermes::kv_store::api,
            "hermes:localtime/api": super::hermes::localtime::api,
            "hermes:logging/api": super::hermes::logging::api,
            "hermes:sqlite/api": super::hermes::sqlite::api,
            "hermes:http-request/api": super::hermes::http_request::api,
        },
    });
}
