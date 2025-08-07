//! # DB Component.

// Allow everything since this is generated code.
// TODO[RC]: Why this? Clearly, not all is generated here.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

/// Simple HTTP proxy component for demonstration purposes.
struct DbComponent;

hermes::export!(DbComponent with_types_in hermes);
