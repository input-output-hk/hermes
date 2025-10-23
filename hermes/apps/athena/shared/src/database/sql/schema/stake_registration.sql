-- Index of stake registrations.
CREATE TABLE IF NOT EXISTS stake_registration (
    -- Primary Key Data
    stake_address    BLOB NOT NULL,         -- 29 Byte stake hash (CIP19).
    slot_no          INTEGER NOT NULL,      -- slot number when the key_was_registered/re-registered.
    txn_index        INTEGER NOT NULL,      -- Index of the TX which holds the registration data.

    -- Stake key lifecycle data, shows what happened with the stake key at this slot#.
    script           BOOLEAN  NOT NULL,     -- Is the address a script address.
    register         BOOLEAN  NOT NULL,     -- True if the this transaction contains cardano stake registration cert.
    deregister       BOOLEAN  NOT NULL,     -- True if the this transaction contains cardano stake deregistration cert.
    cip36            BOOLEAN  NOT NULL,     -- True if the this transaction contains CIP36 registration.
    pool_delegation  BLOB NOT NULL,         -- Stake was delegated to this Pool address.
                                            -- Not present if delegation did not change.

    PRIMARY KEY (stake_address, script, slot_no, txn_index)
);
