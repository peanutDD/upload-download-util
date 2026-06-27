CREATE TABLE IF NOT EXISTS webdav_locks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    depth TEXT NOT NULL DEFAULT '0',
    owner TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_webdav_locks_user_path
ON webdav_locks(user_id, path);

CREATE INDEX IF NOT EXISTS idx_webdav_locks_expires_at
ON webdav_locks(expires_at);
