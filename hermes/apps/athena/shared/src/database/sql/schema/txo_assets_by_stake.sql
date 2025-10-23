-- Transaction Outputs (Native Assets) per stake address.
-- Assets that aren't staked are not present in this table.
CREATE TABLE IF NOT EXISTS txo_assets_by_stake (
    stake_address   BLOB NOT NULL,          -- 29 Byte stake hash (CIP19).
    slot_no         INTEGER NOT NULL,       -- slot number when the spend occurred.
    txn_index       INTEGER NOT NULL,       -- Which Transaction in the Slot is the TXO.
    txo             INTEGER NOT NULL,       -- Index of the TXO which was spent
    policy_id       BLOB NOT NULL,          -- asset policy hash (id) (28 byte binary hash)
    asset_name      BLOB NOT NULL,          -- name of the asset policy (UTF8) (0 - 32 bytes)
    
    -- None Key Data of the asset.
    value           BLOB NOT NULL,          -- Value of the asset.

    PRIMARY KEY (stake_address, slot_no, txn_index, txo)
);

CREATE INDEX IF NOT EXISTS txo_assets_by_stake_stake_address_idx 
    ON txo_assets_by_stake (stake_address);
