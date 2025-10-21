-- This could be ADA or a native asset being spent.
-- This can represent a spend on either immutable data or volatile data.
CREATE TABLE IF NOT EXISTS txi_by_txn_id (
    txn_id          BLOB NOT NULL,          -- 32 Bytes Transaction Hash that was spent.
    txo             INTEGER NOT NULL,       -- Index of the TXO which was spent

    -- Non key data, we can only spend a transaction hash/txo once, so this should be unique in any event.
    slot_no         INTEGER NOT NULL,       -- slot number when the spend occurred.

    PRIMARY KEY (txn_id, txo)
);
