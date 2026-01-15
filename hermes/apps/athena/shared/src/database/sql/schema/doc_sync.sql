-- Documents local storage.
CREATE TABLE IF NOT EXISTS document (
    cid             TEXT NOT NULL,          -- CID calculated over document bytes.
    document        BLOB NOT NULL,          -- Document cbor-encoded bytes.
    inserted_at     TIMESTAMP NOT NULL,     -- Timestamp when document was inserted at.
    topic           TEXT NOT NULL,          -- IPFS pubsub topic.
    metadata        BLOB,                   -- Cbor-encoded metadata.
    PRIMARY KEY (cid)
);
