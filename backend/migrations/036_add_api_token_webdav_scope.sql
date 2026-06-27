ALTER TABLE api_tokens
ADD COLUMN IF NOT EXISTS webdav_enabled BOOLEAN NOT NULL DEFAULT TRUE,
ADD COLUMN IF NOT EXISTS webdav_read_only BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS webdav_root_folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_api_tokens_webdav_root_folder_id
ON api_tokens(webdav_root_folder_id);
