//! Shared WIT bindings with associated code that is reusable across Athena modules.
//!
//! # Note
//!
//! Currently, sharing can be unintuitive when using [`wit_bindgen::generate`].
//! As of **0.46.0** this macro doesn't provide a way to share WIT **worlds**
//! (only allowing non-recursive per-interface sharing using `with` keyword).
//!
//! This crate implements a macro similar in syntax, but with additional `share` keyword,
//! which allows omitting `with` for transitive dependencies.
//!
//! # Example
//!
//! ```rust, no_run, ignore-x86_64, ignore-aarch64
//! shared::bindings_generate!({
//!     world: "hermes:app/hermes",
//!     path: "../../../../wasm/wasi/wit",
//!     inline: "
//!         package hermes:app;
//!
//!         world hermes {
//!             import hermes:logging/api;
//!             export hermes:init/event;
//!         }
//!     ",
//!     share: ["hermes:logging"],
//! });
//!
//! export!(Component);
//!
//! struct Component;
//!
//! impl exports::hermes::init::event::Guest for Component {
//!     fn init() -> bool {
//!         shared::utils::log::log_info("", "", "", "Hello World!", None);
//!         true
//!     }
//! }
//! ```
//!
//! Keyword `share` in the macro above expands to:
//!
//! ```ignore
//! with: {
//!     "hermes:logging": shared::bindings::hermes::logging::api,
//!     "hermes:json": shared::bindings::hermes::json::api,
//! },
//! ```

pub mod bindings;
pub mod database;
pub mod utils;

pub use cardano_blockchain_types;
pub use catalyst_types;
