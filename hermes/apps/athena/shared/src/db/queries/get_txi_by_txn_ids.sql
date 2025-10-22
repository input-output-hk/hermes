SELECT
    txn_id,
    txo,
    slot_no
FROM txn_txi_id
WHERE txn_id = ?
