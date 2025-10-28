//! Implementation of the GET `../assets` endpoint

use std::collections::{HashMap, HashSet};

use cardano_blockchain_types::StakeAddress;
use shared::{
    database::staked_ada::{
        get_txi_by_txn_ids, get_txo_assets_by_stake_address, get_txo_by_stake_address,
        update_txo_spent, UpdateTxoSpentParams,
    },
    utils::{
        common::{
            objects::cardano::{
                network::Network,
                stake_info::{FullStakeInfo, StakeInfo, StakedTxoAssetInfo},
            },
            responses::ErrorResponses,
            types::{
                cardano::{
                    ada_value::AdaValue, asset_name::AssetName, asset_value::AssetValue,
                    cip19_stake_address::Cip19StakeAddress, hash28::HexEncodedHash28,
                    slot_no::SlotNo,
                },
                pallas_big_int_to_num_bigint,
            },
        },
        log,
        settings::Settings,
        sqlite::Connection,
    },
};

use crate::api::types::{
    AllResponses, GetAssetsByStakeAddressQueryKey, GetAssetsByStakeAddressQueryValue, Responses,
    TxoAssetsMap, TxoAssetsState, TxoInfo, TxoMap,
};
use crate::config::DB_BATCH_SIZE;

/// # GET `/staked_ada`
pub(crate) fn endpoint(
    stake_address: Cip19StakeAddress,
    provided_network: Option<Network>,
    slot_num: Option<SlotNo>,
) -> AllResponses {
    match build_full_stake_info_response(stake_address, provided_network, slot_num) {
        Ok(None) => AllResponses::Error(ErrorResponses::NotFound),
        Ok(Some(full_stake_info)) => AllResponses::With(Responses::Ok(full_stake_info)),
        Err(err) => AllResponses::handle_error(&err),
    }
}

/// Building a full stake info response from the provided arguments.
fn build_full_stake_info_response(
    stake_address: Cip19StakeAddress,
    provided_network: Option<Network>,
    slot_num: Option<SlotNo>,
) -> anyhow::Result<Option<FullStakeInfo>> {
    if let Some(provided_network) = provided_network {
        if cardano_blockchain_types::Network::from(provided_network) != Settings::cardano_network()
        {
            return Ok(None);
        }
    }
    let mut persistent_session = Connection::open(false)?;
    let mut volatile_session = Connection::open(true)?;
    let adjusted_slot_num = slot_num.unwrap_or(SlotNo::MAXIMUM);

    let persistent_txo_state = calculate_assets_state(
        &mut persistent_session,
        stake_address.clone(),
        TxoAssetsState::default(),
    )?;

    let volatile_txo_state = calculate_assets_state(
        &mut volatile_session,
        stake_address.clone(),
        persistent_txo_state.clone(),
    )?;

    if volatile_txo_state.is_empty() && persistent_txo_state.is_empty() {
        return Ok(None);
    }
    let persistent_stake_info = build_stake_info(persistent_txo_state, adjusted_slot_num)?;

    let volatile_stake_info = build_stake_info(volatile_txo_state, adjusted_slot_num)?;

    Ok(Some(FullStakeInfo {
        volatile: volatile_stake_info.into(),
        persistent: persistent_stake_info.into(),
    }))
}

/// Calculate the assets state info for a given stake address.
///
/// This function also updates the spent column if it detects that a TXO was spent
/// between lookups.
fn calculate_assets_state(
    session: &mut Connection,
    stake_address: Cip19StakeAddress,
    mut txo_base_state: TxoAssetsState,
) -> anyhow::Result<TxoAssetsState> {
    let address: StakeAddress = stake_address.try_into().map_err(|err| {
        anyhow::anyhow!("Failed to convert Cip19StakeAddress to StakeAddress: {err}")
    })?;

    let (mut txos, txo_assets) = (
        get_txo(session, address.clone())?,
        get_txo_assets(session, address.clone())?,
    );

    let params = update_spent(session, &address, &mut txo_base_state.txos, &mut txos)?;

    // Extend the base state with current session data (used to calculate volatile data)
    let txos = txo_base_state.txos.into_iter().chain(txos).collect();
    let txo_assets: HashMap<_, _> = txo_base_state
        .txo_assets
        .into_iter()
        .chain(txo_assets)
        .collect();
    let mut tx = Connection::begin(session)?;
    if let Err((_, err)) = update_txo_spent(&mut tx, params) {
        tx.rollback()?;
        log::error!("Failed to update TXO spent info, err: {err}");
    }

    Ok(TxoAssetsState { txos, txo_assets })
}

/// Returns a map of TXO infos for the given stake address.
fn get_txo(
    session: &mut Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<TxoMap> {
    let txos_stream = get_txo_by_stake_address(session, stake_address)?;

    let txo_map = txos_stream.iter().fold(HashMap::new(), |mut txo_map, row| {
        txo_map.insert(
            (row.txn_id, row.txo),
            TxoInfo {
                value: pallas_big_int_to_num_bigint(&row.value),
                txn_index: row.txn_index.into(),
                txo: row.txo,
                slot_no: row.slot_no.into(),
                spent_slot_no: row.spent_slot.map(Into::into),
            },
        );
        txo_map
    });
    Ok(txo_map)
}

/// Returns a map of txo asset infos for the given stake address.
fn get_txo_assets(
    session: &mut Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<TxoAssetsMap> {
    let assets_txos_stream = get_txo_assets_by_stake_address(session, stake_address)?;

    let tokens_map =
        assets_txos_stream
            .iter()
            .fold(HashMap::new(), |mut tokens_map: TxoAssetsMap, row| {
                let key = GetAssetsByStakeAddressQueryKey {
                    txn_index: row.txn_index,
                    txo: row.txo,
                    slot_no: row.slot_no,
                };
                let value = GetAssetsByStakeAddressQueryValue {
                    policy_id: row.policy_id.to_vec(),
                    asset_name: row.asset_name.as_slice().to_vec(),
                    value: pallas_big_int_to_num_bigint(&row.value),
                };
                match tokens_map.entry(key) {
                    std::collections::hash_map::Entry::Occupied(mut o) => {
                        o.get_mut().push(value);
                    },
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(vec![value]);
                    },
                }
                tokens_map
            });
    Ok(tokens_map)
}

/// Checks if the given TXOs were spent and mark then as such.
/// Separating `base_txos` and `txos` because we don't want to make an update inside the db
/// for the `base_txos` data (it is covering the case when inside the persistent part we
/// have a txo which is spent inside the volatile, so to not incorrectly mix up records
/// from these two tables, inserting some rows from persistent to volatile section).
fn update_spent(
    session: &mut Connection,
    stake_address: &StakeAddress,
    base_txos: &mut TxoMap,
    txos: &mut TxoMap,
) -> anyhow::Result<Vec<UpdateTxoSpentParams>> {
    let txn_hashes = txos
        .iter()
        .chain(base_txos.iter())
        .filter(|(_, txo)| txo.spent_slot_no.is_none())
        .map(|((tx_id, _), _)| *tx_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut params = Vec::new();

    for chunk in txn_hashes.chunks(DB_BATCH_SIZE) {
        let txi_stream = get_txi_by_txn_ids(session, chunk.to_vec())?;

        for row in txi_stream {
            let key = (row.txn_id, row.txo);
            if let Some(txo_info) = txos.get_mut(&key) {
                params.push(UpdateTxoSpentParams {
                    stake_address: stake_address.clone(),
                    txn_index: txo_info.txn_index.into(),
                    txo: txo_info.txo,
                    slot_no: txo_info.slot_no.into(),
                    spent_slot: row.slot_no,
                });

                txo_info.spent_slot_no = Some(row.slot_no.into());
            }
            if let Some(txo_info) = base_txos.get_mut(&key) {
                txo_info.spent_slot_no = Some(row.slot_no.into());
            }
        }
    }

    Ok(params)
}

/// Builds an instance of [`StakeInfo`] based on the TXOs given.
fn build_stake_info(
    mut txo_state: TxoAssetsState,
    slot_num: SlotNo,
) -> anyhow::Result<StakeInfo> {
    let slot_num = slot_num.into();
    let mut total_ada_amount = AdaValue::default();
    let mut last_slot_num = SlotNo::default();
    let mut assets = HashMap::<(HexEncodedHash28, AssetName), AssetValue>::new();

    for txo_info in txo_state.txos.into_values() {
        // Filter out spent TXOs.
        if txo_info.slot_no >= slot_num {
            continue;
        }
        // Filter out spent TXOs.
        if let Some(spent_slot) = txo_info.spent_slot_no {
            if spent_slot <= slot_num {
                continue;
            }
        }

        let value = AdaValue::try_from(txo_info.value)
            .map_err(|err| anyhow::anyhow!("Failed to convert TXO value to AdaValue: {err}"))?;
        total_ada_amount = total_ada_amount.saturating_add(value);

        let key = GetAssetsByStakeAddressQueryKey {
            slot_no: txo_info.slot_no.into(),
            txn_index: txo_info.txn_index.into(),
            txo: txo_info.txo,
        };
        if let Some(native_assets) = txo_state.txo_assets.remove(&key) {
            for native_asset in native_assets {
                let amount = (&native_asset.value).into();
                let policy_hash: HexEncodedHash28 =
                    (&native_asset.policy_id).try_into().map_err(|err| {
                        anyhow::anyhow!("Failed to convert policy_id to HexEncodedHash28: {err}")
                    })?;
                match assets.entry((policy_hash, (&native_asset.asset_name).into())) {
                    std::collections::hash_map::Entry::Occupied(mut o) => {
                        *o.get_mut() = o.get().saturating_add(&amount);
                    },
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(amount.clone());
                    },
                }
            }
        }

        let slot_no = txo_info.slot_no.into();
        if last_slot_num < slot_no {
            last_slot_num = slot_no;
        }
    }

    Ok(StakeInfo {
        ada_amount: total_ada_amount,
        slot_number: last_slot_num,
        assets: assets
            .into_iter()
            .map(|((policy_hash, asset_name), amount)| StakedTxoAssetInfo {
                policy_hash,
                asset_name,
                amount,
            })
            .collect::<Vec<_>>()
            .into(),
    })
}
