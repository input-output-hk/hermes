-- Transaction Outputs (ADA) per stake address.
-- ADA that isn't staked is not present in this table.
CREATE TABLE IF NOT EXISTS txo_by_stake (
    stake_address   BLOB NOT NULL,          -- 29 Byte stake hash (CIP19).
    slot_no         INTEGER NOT NULL,       -- slot number when the spend occurred.
    txn_index       INTEGER NOT NULL,       -- Which Transaction in the Slot is the TXO.
    txo             INTEGER NOT NULL,       -- offset in the txo list of the transaction the txo is in.

    -- Transaction Output Data
    value           TEXT NOT NULL,          -- Lovelace value of the TXO. Decimal encoded.

    -- Data needed to correlate a spent TXO.
    txn_id          BLOB NOT NULL,          -- 32 byte hash of this transaction.

    spent_slot      INTEGER,                -- Slot this TXO was spent in.
                                            -- This is ONLY calculated/stored
                                            -- when first detected in a query lookup.
                                            -- It serves as an optimization on subsequent queries.

    PRIMARY KEY (stake_address, slot_no, txn_index, txo)
);

CREATE INDEX IF NOT EXISTS txo_by_stake_stake_address_idx
    ON txo_by_stake (stake_address);
