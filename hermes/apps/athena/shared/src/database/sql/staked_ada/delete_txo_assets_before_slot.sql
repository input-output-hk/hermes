-- Delete Transaction Output Assets before the slot number.
DELETE FROM txo_assets_by_stake
WHERE slot_no <= ?;
