UPDATE txo_by_stake
    SET spent_slot = ?
WHERE stake_address = ?
    AND txn_index = ?
    AND txo = ?
    AND slot_no = ?;
