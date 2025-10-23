-- Delete Transaction Output since slot number.
DELETE FROM txo_by_stake
WHERE slot_no >= ?;
