use cardano_blockchain_types::{MultiEraBlock, Network, Point};
use cardano_chain_follower::{ChainFollower, Kind};

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::cardano::api::CardanoNetwork,
        hermes::cardano::{
            event::{build_and_send_block_event, build_and_send_rollback_event},
            host::SubscriptionType,
        },
    },
    wasm::module::ModuleId,
};

impl TryFrom<CardanoNetwork> for cardano_blockchain_types::Network {
    type Error = anyhow::Error;

    fn try_from(network: CardanoNetwork) -> Result<Self, Self::Error> {
        match network {
            CardanoNetwork::Mainnet => Ok(cardano_blockchain_types::Network::Mainnet),
            CardanoNetwork::Preprod => Ok(cardano_blockchain_types::Network::Preprod),
            CardanoNetwork::Preview => Ok(cardano_blockchain_types::Network::Preview),
            CardanoNetwork::TestnetMagic(n) => anyhow::bail!("TestnetMagic {n} is not supported"),
        }
    }
}
pub(crate) fn subscribe(
    app: ApplicationName, module_id: ModuleId, start: Point, network: Network,
    subscription_type: SubscriptionType,
) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create Tokio runtime: {e}");
                return;
            },
        };

        rt.block_on(async move {
            let mut follower = ChainFollower::new(network, start, Point::TIP).await;

            while let Some(chain_update) = follower.next().await {
                let block_data = chain_update.block_data();
                match chain_update.kind {
                    Kind::Block if subscription_type == SubscriptionType::Block => {
                        build_and_send_block_event(
                            app.clone(),
                            module_id,
                            network,
                            block_data.raw(),
                            block_data.slot(),
                            block_data.is_mutable(),
                        );
                    },
                    Kind::Rollback if subscription_type == SubscriptionType::Block => {
                        build_and_send_rollback_event(
                            app.clone(),
                            module_id,
                            network,
                            block_data.slot(),
                        )
                    },
                    Kind::ImmutableRollForward
                        if subscription_type == SubscriptionType::ImmutableRollForward =>
                    {
                        build_and_send_roll_forward_event(
                            app.clone(),
                            module_id,
                            network,
                            block_data.slot(),
                        );
                    },
                }
            }
        });
    });
}

pub async fn get_block_relative(
    chain: Network, start: Option<Slot>, step: i64,
) -> Option<MultiEraBlock> {
    // If `start` is None, default to TIP
    let point = if let Some(start_point) = start {
        let target = start_point + step;
        Point::fuzzy(target.into())
    } else {
        Point::TIP
    };
    ChainFollower::get_block(chain, point).await?.data;
}

pub async fn get_tips(chain: Network) -> (Slot, Slot) {
    ChainFollower::get_tips(chain).await
}
