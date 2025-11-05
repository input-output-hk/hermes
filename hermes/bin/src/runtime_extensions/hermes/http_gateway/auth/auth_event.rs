//! Auth Event

use std::sync::mpsc::Sender;

use crate::{
    app::ApplicationName,
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::{
        bindings::{
            hermes::http_gateway::api::{AuthRequest, HttpResponse},
            unchecked_exports,
        },
        hermes::http_gateway::auth::auth_config::AuthLevel as RTEAuthLevel,
    },
};
unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with auth functions
    trait ComponentInstanceExt {
        #[wit("hermes:http-gateway/event-auth", "validate-auth")]
        fn hermes_http_gateway_validate_auth(
            request: &AuthRequest,
        ) -> Option<HttpResponse>;
    }
}

/// Auth validation event
pub(crate) struct AuthValidationEvent {
    /// Request header.
    pub(crate) headers: Vec<(String, Vec<String>)>,
    /// Auth level.
    pub(crate) auth_level: RTEAuthLevel,
    /// Channel to send result.
    pub(crate) result_sender: Sender<HttpResponse>,
}

impl HermesEventPayload for AuthValidationEvent {
    /// Event name
    fn event_name(&self) -> &'static str {
        "validate-auth"
    }

    /// Execute the event
    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        let auth_request = AuthRequest {
            headers: self.headers.clone(),
            auth_level: self.auth_level.clone().into(),
        };

        // Get result from auth module
        let result = module
            .instance
            .hermes_http_gateway_validate_auth(&mut module.store, &auth_request)?;

        // Send result back
        match result {
            Some(r) => self.result_sender.send(r).map_err(Into::into),
            None => Ok(()),
        }
    }
}

// -------- Event Builder ----------

/// Build and send auth validation event
pub(crate) fn build_and_send_auth_event(
    app_name: &ApplicationName,
    headers: Vec<(String, Vec<String>)>,
    auth_level: RTEAuthLevel,
    result_sender: Sender<HttpResponse>,
) -> anyhow::Result<()> {
    /// Auth module name
    const AUTH_MODULE_NAME: &str = "auth";

    // Get the module ID from the name
    let app = crate::reactor::get_app(app_name)?;
    let modules = app.get_module_registry();
    let module_id = modules
        .get(AUTH_MODULE_NAME)
        .ok_or_else(|| anyhow::anyhow!("Module {AUTH_MODULE_NAME} not found"))?
        .clone();

    let auth_event = AuthValidationEvent {
        headers,
        auth_level,
        result_sender,
    };
    // Send the event
    crate::event::queue::send(HermesEvent::new(
        auth_event,
        TargetApp::List(vec![app_name.clone()]),
        TargetModule::List(vec![module_id]),
    ))
}
