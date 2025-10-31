-- Delete ADA or a native asset being spent before slot number.
-- This can represent a spend on either immutable data or volatile data.
DELETE FROM txi_by_txn_id
WHERE slot_no <= ?;
