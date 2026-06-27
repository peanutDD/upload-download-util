ALTER TABLE webdav_locks
ADD COLUMN IF NOT EXISTS api_token_id UUID REFERENCES api_tokens(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_webdav_locks_api_token_id
ON webdav_locks(api_token_id);
