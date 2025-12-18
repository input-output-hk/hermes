//! The test Hermes App.
#![allow(
    clippy::missing_safety_doc,
    clippy::missing_docs_in_private_items,
    clippy::expect_used
)]

use catalyst_signed_doc::{
    Builder, CatalystSignedDocument, ContentType, DocumentRef,
    catalyst_id::CatalystId,
    doc_types,
    providers::{CatalystIdProvider, CatalystSignedDocumentProvider},
    uuid::{self, Uuid},
};
use ed25519_dalek::{SigningKey, VerifyingKey, ed25519::signature::SignerMut as _};
use serde_json::json;

mod bindings {

    wit_bindgen::generate!({
        world: "hermes:app/hermes",
        path: "../../../../../../wasm/wasi/wit",
        inline: "
            package hermes:app;

            world hermes {
                import hermes:logging/api;
                import wasi:random/random@0.2.6;
                import wasi:cli/environment@0.2.6;
                import wasi:clocks/wall-clock@0.2.6;

                export hermes:init/event;
            }
        ",
        generate_all,
    });
}

struct CustomProvider;

#[async_trait::async_trait]
impl CatalystSignedDocumentProvider for CustomProvider {
    async fn try_get_doc(
        &self,
        _doc_ref: &DocumentRef,
    ) -> anyhow::Result<Option<CatalystSignedDocument>> {
        Ok(None)
    }

    async fn try_get_last_doc(
        &self,
        _id: uuid::UuidV7,
    ) -> anyhow::Result<Option<CatalystSignedDocument>> {
        Ok(None)
    }

    async fn try_get_first_doc(
        &self,
        _id: uuid::UuidV7,
    ) -> anyhow::Result<Option<CatalystSignedDocument>> {
        Ok(None)
    }

    fn future_threshold(&self) -> Option<std::time::Duration> {
        Some(Duration::from_secs(5))
    }

    fn past_threshold(&self) -> Option<Duration> {
        Some(Duration::from_secs(5))
    }
}

#[async_trait::async_trait]
impl CatalystIdProvider for CustomProvider {
    async fn try_get_registered_key(
        &self,
        _kid: &CatalystId,
    ) -> anyhow::Result<Option<VerifyingKey>> {
        Ok(None)
    }
}

struct FailedInitApp;
use std::{panic, time::Duration};
impl bindings::exports::hermes::init::event::Guest for FailedInitApp {
    fn init() -> bool {
        panic::set_hook(Box::new(|info| {
            let msg = format!("PANIC OCCURRED: {info}");
            test_log(&msg);
        }));
        test_log("init started");

        // let mut provider = TestCatalystProvider::default();
        let provider = CustomProvider;
        test_log("after provider");

        let mut sk = SigningKey::from_bytes(&[0; 32]);
        test_log("after from bytes");
        let kid = CatalystId::new("cardano", None, sk.verifying_key()).as_admin();
        test_log("after cat id new");

        // provider.add_sk(kid.clone(), sk.clone());
        test_log("after add sk");
        let deterministic_uuid = Uuid::now_v7();
        test_log("after deterministic uuid");
        let doc = Builder::new()
            .with_json_metadata(json!({
                "id": deterministic_uuid,
                "ver": deterministic_uuid,
                "type": doc_types::PROPOSAL,
                "content-type": ContentType::Json,
            }))
            .expect("smth")
            .with_json_content(&json!({}))
            .expect("smth")
            .add_signature(|m| sk.sign(&m).to_vec(), kid)
            .expect("smth")
            .build()
            .expect("smth");
        test_log("before spawn");

        let result = futures::executor::block_on(async {
            catalyst_signed_doc::validator::validate(&doc, &provider).await
        });

        test_log(&format!("result: {result:?}"));
        // wasm_bindgen_futures::spawn_local(async move {
        //     test_log("before validate");
        //     let result =
        //         catalyst_signed_doc::validator::validate(&doc,
        // &TestCatalystProvider::default())             .await;
        //     test_log(&format!("result: {result:?}"));
        // });

        false
    }
}

bindings::export!(FailedInitApp with_types_in bindings);

fn test_log(s: &str) {
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
