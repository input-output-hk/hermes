-- Insert document ignoring insertion of the same documents
INSERT INTO document VALUES (?, ?, ?, ?)
ON CONFLICT(cid) DO NOTHING;