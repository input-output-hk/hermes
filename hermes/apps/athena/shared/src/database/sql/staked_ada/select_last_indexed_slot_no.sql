SELECT MIN(slot_no)
FROM (
    SELECT MAX(slot_no) AS slot_no FROM txi_by_txn_id
    UNION ALL
    SELECT MAX(slot_no) AS slot_no FROM txo_assets_by_stake
    UNION ALL
    SELECT MAX(slot_no) AS slot_no FROM txo_by_stake
)
