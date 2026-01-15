SELECT
    document,
    cid,
    inserted_at,
    topic,
    metadata
FROM document
WHERE topic = ?;
