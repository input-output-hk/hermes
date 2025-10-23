//! Hermes bindings generated with [`::wit_bindgen`].
//! They can be reused when using `share` keyword of [`crate::bindings_generate`] macro.

/// Re-exported [`::wit-bindgen`] crate, so that [`crate::bindings_generate`] is
/// self-reliant.
#[doc(hidden)]
pub use wit_bindgen;

wit_bindgen::generate!({
    world: "imports",
    path: "../../../../wasm/wasi/wit",
    generate_all,
});

/// See [crate-level](crate) documentation.
#[macro_export]
macro_rules! bindings_generate {
    ({
        world: $world:literal,
        path: $path:literal,
        inline: $inline:literal,
        $(with: {$($with_wit:literal: $with_path:path),* $(,)? },)?
        share: [] $(,)?
    }) => {
        ::shared::bindings::wit_bindgen::generate!({
            runtime_path: "::shared::bindings::wit_bindgen::rt",
            world: $world,
            path: $path,
            inline: $inline,
            $(with: { $($with_wit: $with_path,)* },)?
            generate_all,
        });
    };
    ({
        world: $world:literal,
        path: $path:literal,
        inline: $inline:literal,
        $(with: {$($with_wit:literal: $with_path:path),* $(,)? },)?
        share: ["hermes:cardano" $(, $share:tt)* $(,)?] $(,)?
    }) => {
        $crate::bindings_generate!({
            world: $world,
            path: $path,
            inline: $inline,
            with: {
                $($($with_wit: $with_path,)*)?
                "hermes:binary/api": ::shared::bindings::hermes::binary::api,
                "hermes:cbor/api": ::shared::bindings::hermes::cbor::api,
                "hermes:hash/api": ::shared::bindings::hermes::hash::api,
                "hermes:cardano/api": ::shared::bindings::hermes::cardano::api,
            },
            share: [$($share),*],
        });
    };
    ({
        world: $world:literal,
        path: $path:literal,
        inline: $inline:literal,
        $(with: {$($with_wit:literal: $with_path:path),* $(,)? },)?
        share: ["hermes:logging" $(, $share:tt)* $(,)?] $(,)?
    }) => {
        $crate::bindings_generate!({
            world: $world,
            path: $path,
            inline: $inline,
            with: {
                $($($with_wit: $with_path,)*)?
                "hermes:logging/api": ::shared::bindings::hermes::logging::api,
                "hermes:json/api": ::shared::bindings::hermes::json::api,
            },
            share: [$($share),*],
        });
    };
    ({
        world: $world:literal,
        path: $path:literal,
        inline: $inline:literal,
        $(with: {$($with_wit:literal: $with_path:path),* $(,)? },)?
        share: ["hermes:sqlite" $(, $share:tt)* $(,)?] $(,)?
    }) => {
        $crate::bindings_generate!({
            world: $world,
            path: $path,
            inline: $inline,
            with: {
                $($($with_wit: $with_path,)*)?
                "hermes:sqlite/api": ::shared::bindings::hermes::sqlite::api,
            },
            share: [$($share),*]
        });
    };
}
