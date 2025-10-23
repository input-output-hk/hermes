-- Delete Transaction Output Assets since the slot number.
DELETE FROM txo_assets_by_stake
WHERE slot_no >= ?;