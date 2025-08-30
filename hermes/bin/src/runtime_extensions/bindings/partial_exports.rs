use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::exports::hermes::{
        cardano::{event_on_block::Block, event_on_immutable_roll_forward::SubscriptionId},
        cron::event::CronTagged,
        http_gateway::event::HttpGatewayResponse,
        ipfs::event::PubsubMessage,
        kv_store::event::KvValues,
    },
};

use super::exports::hermes::integration_test::event::TestResult;
use wasmtime::component::{self, ComponentNamedList, Resource, TypedFunc};

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

fn get_typed_func<Params, Return>(
    instance: &component::Instance,
    store: &mut wasmtime::Store<HermesRuntimeContext>,
    wit_name: &str,
) -> Result<TypedFunc<Params, Return>, Error>
where
    Params: ComponentNamedList + component::Lower,
    Return: ComponentNamedList + component::Lift,
{
    let Some(untyped) = instance.get_func(&mut *store, wit_name) else {
        return Err(Error::NotExported);
    };
    untyped.typed(store).map_err(|_| Error::InvalidSignature)
}

impl ComponentInstanceExt for &component::Instance {
    fn hermes_cardano_event_on_block_on_cardano_block(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCardanoBlock, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_cardano_event_on_immutable_roll_forward_on_cardano_immutable_roll_forward(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCardanoImmutableRollForward, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_cron_event_on_cron<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnCron<'p>, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_init_event_init(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Init, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_ipfs_event_on_topic<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnTopic<'p>, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_kv_store_event_kv_update<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<KvUpdate<'p>, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_integration_test_event_test(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Test, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_integration_test_event_bench(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Bench, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_http_gateway_event_reply<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<Reply<'p>, Error> {
        get_typed_func(self, store, "TODO")
    }

    fn hermes_http_request_event_on_http_response<'p>(
        self,
        store: &mut wasmtime::Store<HermesRuntimeContext>,
    ) -> Result<OnHttpResponse<'p>, Error> {
        get_typed_func(self, store, "TODO")
    }
}
