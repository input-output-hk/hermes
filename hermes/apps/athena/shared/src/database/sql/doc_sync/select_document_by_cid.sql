SELECT
    document,
    inserted_at,
    metadata
FROM document
WHERE cid = ?;
