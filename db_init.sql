CREATE TABLE IF NOT EXISTS tf_http_states (
    resource_key TEXT PRIMARY KEY,
    state JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tf_http_locks (
    resource_key TEXT PRIMARY KEY,
    lock_id TEXT NOT NULL,
    lock_info JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
