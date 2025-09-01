use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::exports::hermes::{
        cardano::{event_on_block::Block, event_on_immutable_roll_forward::SubscriptionId},
        cron::event::CronTagged,
        http_gateway::event::HttpGatewayResponse,
        integration_test::event::TestResult,
        ipfs::event::PubsubMessage,
        kv_store::event::KvValues,
    },
};

use wasmtime::{
    component::{self, ComponentNamedList, Resource, TypedFunc},
    AsContextMut,
};

fn get_typed_func<Params, Return>(
    instance: &component::Instance,
    store: &mut wasmtime::Store<HermesRuntimeContext>,
    wit_interface_name: &str,
    wit_func_name: &str,
) -> Result<TypedFunc<Params, Return>, Error>
where
    Params: ComponentNamedList + component::Lower,
    Return: ComponentNamedList + component::Lift,
{
    let Some(untyped) = instance
        .get_export_index(store.as_context_mut(), None, wit_interface_name)
        .and_then(|interface_idx| {
            instance.get_export_index(store.as_context_mut(), Some(&interface_idx), wit_func_name)
        })
        .and_then(|func_idx| instance.get_func(store.as_context_mut(), func_idx))
    else {
        return Err(Error::NotExported);
    };
    untyped.typed(store).map_err(|_| Error::InvalidSignature)
}

macro_rules! define_exports {
    ($(
        #[wit($wit_interface:literal, $wit_func:literal)]
        fn $rust_func:ident$(<$l:lifetime>)?($($param_name:ident: $param:ty),* $(,)?) $(-> $return:ty)?;
    )*) => {
        #[allow(dead_code)]
        pub trait ComponentInstanceExt1 {$(
            #[doc = concat!($wit_func , " from \"", $wit_interface, "\n\n# Params\n\n" $(, "- ", stringify!($param_name))*)]
            fn $rust_func$(<$l>)?(
                self,
                store: &mut ::wasmtime::Store<$crate::runtime_context::HermesRuntimeContext>
            ) -> Result<::wasmtime::component::TypedFunc<($($param,)*), ($($return,)?)>, Error>;
        )*}
    };
}

define_exports! {
    #[wit("hermes:init/event", "init")]
    fn init() -> bool;

    #[wit("hermes:cardano/event-on-block", "on-cardano-block")]
    fn hermes_cardano_event_on_block_on_cardano_block(
        subscription_id: Resource<SubscriptionId>, block_id: Resource<Block>,
    );

    #[wit("hermes:cardano/event-on-immutable-roll-forward", "on-immutable-roll-forward")]
    fn hermes_cardano_event_on_immutable_roll_forward_on_cardano_immutable_roll_forward(
        subscription_id: Resource<SubscriptionId>, block_id: Resource<Block>,
    );

    #[wit("hermes:cron/event", "on-cron")]
    fn hermes_cron_event_on_cron<'p>(
        event: &'p CronTagged, last: bool,
    ) -> bool;
}

pub type OnCardanoBlock = TypedFunc<(Resource<SubscriptionId>, Resource<Block>), ()>;

pub type OnCardanoImmutableRollForward = OnCardanoBlock;

pub type OnCron<'a> = TypedFunc<(&'a CronTagged, bool), (bool,)>;

pub type Init = TypedFunc<(), (bool,)>;

pub type OnTopic<'a> = TypedFunc<(&'a PubsubMessage,), (bool,)>;

pub type KvUpdate<'a> = TypedFunc<(&'a str, &'a KvValues), ()>;

pub type Test = TypedFunc<(u32, bool), (Option<TestResult>,)>;

pub type Bench = Test;

pub type Reply<'a> = TypedFunc<
    (
        &'a Vec<u8>,
        &'a Vec<(String, Vec<String>)>,
        &'a str,
        &'a str,
    ),
    (Option<HttpGatewayResponse>,),
>;

type OnHttpResponse<'a> = TypedFunc<(Option<u64>, &'a [u8]), ()>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("event handler is not exported")]
    NotExported,
    #[error("invalid event handler signature")]
    InvalidSignature,
}

pub trait ComponentInstanceExt {
    fn hermes_cardano_event_on_block_on_cardano_block(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCardanoBlock, Error>;

    fn hermes_cardano_event_on_immutable_roll_forward_on_cardano_immutable_roll_forward(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCardanoImmutableRollForward, Error>;

    fn hermes_cron_event_on_cron<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCron<'p>, Error>;

    fn hermes_init_event_init(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Init, Error>;

    fn hermes_ipfs_event_on_topic<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnTopic<'p>, Error>;

    fn hermes_kv_store_event_kv_update<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<KvUpdate<'p>, Error>;

    fn hermes_integration_test_event_test(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Test, Error>;

    fn hermes_integration_test_event_bench(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Bench, Error>;

    fn hermes_http_gateway_event_reply<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Reply<'p>, Error>;

    fn hermes_http_request_event_on_http_response<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnHttpResponse<'p>, Error>;
}

impl ComponentInstanceExt for &component::Instance {
    fn hermes_cardano_event_on_block_on_cardano_block(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCardanoBlock, Error> {
        get_typed_func(
            self,
            store,
            "hermes:cardano/event-on-block",
            "on-cardano-block",
        )
    }

    fn hermes_cardano_event_on_immutable_roll_forward_on_cardano_immutable_roll_forward(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCardanoImmutableRollForward, Error> {
        get_typed_func(
            self,
            store,
            "hermes:cardano/event-on-immutable-roll-forward",
            "on-cardano-immutable-roll-forward",
        )
    }

    fn hermes_cron_event_on_cron<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCron<'p>, Error> {
        get_typed_func(self, store, "hermes:cron/event", "on-cron")
    }

    fn hermes_init_event_init(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Init, Error> {
        get_typed_func(self, store, "hermes:init/event", "init")
    }

    fn hermes_ipfs_event_on_topic<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnTopic<'p>, Error> {
        get_typed_func(self, store, "hermes:ipfs/event", "on-topic")
    }

    fn hermes_kv_store_event_kv_update<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<KvUpdate<'p>, Error> {
        get_typed_func(self, store, "hermes:kv-store/event", "kv-update")
    }

    fn hermes_integration_test_event_test(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Test, Error> {
        get_typed_func(self, store, "hermes:integration-test/event", "test")
    }

    fn hermes_integration_test_event_bench(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Bench, Error> {
        get_typed_func(self, store, "hermes:integration-test/event", "bench")
    }

    fn hermes_http_gateway_event_reply<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Reply<'p>, Error> {
        get_typed_func(self, store, "hermes:http-gateway/event", "reply")
    }

    fn hermes_http_request_event_on_http_response<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnHttpResponse<'p>, Error> {
        get_typed_func(self, store, "hermes:http-request/event", "on-http-response")
    }
}
