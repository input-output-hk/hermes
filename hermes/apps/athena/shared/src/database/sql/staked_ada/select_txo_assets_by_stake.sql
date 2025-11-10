SELECT
    stake_address,
    slot_no,
    txn_index,
    txo,
    policy_id,
    asset_name,
    value
FROM txo_assets_by_stake
WHERE stake_address = ?;
