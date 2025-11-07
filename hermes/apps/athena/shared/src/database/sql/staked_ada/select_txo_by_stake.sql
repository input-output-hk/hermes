SELECT
    stake_address,
    slot_no,
    txn_index,
    txo,
    value,
    txn_id,
    spent_slot
FROM txo_by_stake
WHERE stake_address = ?;
