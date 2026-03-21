use async_trait::async_trait;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use tokio::sync::RwLock;

#[derive(Debug)]
pub enum AcquireLockResult {
    Acquired,
    AlreadyLocked(Value),
}

#[derive(Debug)]
pub enum ReleaseLockResult {
    Released,
    NotLocked,
    LockMismatch(Value),
}

#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn get_state(&self, key: &str) -> Result<Option<Value>, StoreError>;
    async fn upsert_state(&self, key: &str, state: &Value) -> Result<(), StoreError>;
    async fn delete_state(&self, key: &str) -> Result<(), StoreError>;
    async fn get_lock(&self, key: &str) -> Result<Option<Value>, StoreError>;
    async fn acquire_lock(
        &self,
        key: &str,
        lock_info: &Value,
    ) -> Result<AcquireLockResult, StoreError>;
    async fn release_lock(
        &self,
        key: &str,
        expected_lock_id: Option<&str>,
    ) -> Result<ReleaseLockResult, StoreError>;
}

pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tf_http_states (
                resource_key TEXT PRIMARY KEY,
                state JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tf_http_locks (
                resource_key TEXT PRIMARY KEY,
                lock_id TEXT NOT NULL,
                lock_info JSONB NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl StateStore for PgStore {
    async fn get_state(&self, key: &str) -> Result<Option<Value>, StoreError> {
        let row = sqlx::query_scalar::<_, Value>(
            "SELECT state FROM tf_http_states WHERE resource_key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn upsert_state(&self, key: &str, state: &Value) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO tf_http_states (resource_key, state, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (resource_key)
             DO UPDATE SET state = EXCLUDED.state, updated_at = NOW()",
        )
        .bind(key)
        .bind(state)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_state(&self, key: &str) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM tf_http_states WHERE resource_key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_lock(&self, key: &str) -> Result<Option<Value>, StoreError> {
        let row = sqlx::query_scalar::<_, Value>(
            "SELECT lock_info FROM tf_http_locks WHERE resource_key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn acquire_lock(
        &self,
        key: &str,
        lock_info: &Value,
    ) -> Result<AcquireLockResult, StoreError> {
        let lock_id = lock_info
            .get("ID")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let inserted = sqlx::query_scalar::<_, Value>(
            "INSERT INTO tf_http_locks (resource_key, lock_id, lock_info, updated_at)
             VALUES ($1, $2, $3, NOW())
             ON CONFLICT (resource_key) DO NOTHING
             RETURNING lock_info",
        )
        .bind(key)
        .bind(lock_id)
        .bind(lock_info)
        .fetch_optional(&self.pool)
        .await?;

        if inserted.is_some() {
            return Ok(AcquireLockResult::Acquired);
        }

        let current = sqlx::query_scalar::<_, Value>(
            "SELECT lock_info FROM tf_http_locks WHERE resource_key = $1",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await?;

        Ok(AcquireLockResult::AlreadyLocked(current))
    }

    async fn release_lock(
        &self,
        key: &str,
        expected_lock_id: Option<&str>,
    ) -> Result<ReleaseLockResult, StoreError> {
        let current = sqlx::query_scalar::<_, Value>(
            "SELECT lock_info FROM tf_http_locks WHERE resource_key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        let Some(current_lock) = current else {
            return Ok(ReleaseLockResult::NotLocked);
        };

        if let Some(expected_id) = expected_lock_id {
            let current_id = current_lock
                .get("ID")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if current_id != expected_id {
                return Ok(ReleaseLockResult::LockMismatch(current_lock));
            }
        }

        sqlx::query("DELETE FROM tf_http_locks WHERE resource_key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(ReleaseLockResult::Released)
    }
}

#[cfg(test)]
pub struct InMemoryStore {
    states: RwLock<HashMap<String, Value>>,
    locks: RwLock<HashMap<String, Value>>,
}

#[cfg(test)]
impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            locks: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl StateStore for InMemoryStore {
    async fn get_state(&self, key: &str) -> Result<Option<Value>, StoreError> {
        Ok(self.states.read().await.get(key).cloned())
    }

    async fn upsert_state(&self, key: &str, state: &Value) -> Result<(), StoreError> {
        self.states
            .write()
            .await
            .insert(key.to_string(), state.clone());
        Ok(())
    }

    async fn delete_state(&self, key: &str) -> Result<(), StoreError> {
        self.states.write().await.remove(key);
        Ok(())
    }

    async fn get_lock(&self, key: &str) -> Result<Option<Value>, StoreError> {
        Ok(self.locks.read().await.get(key).cloned())
    }

    async fn acquire_lock(
        &self,
        key: &str,
        lock_info: &Value,
    ) -> Result<AcquireLockResult, StoreError> {
        let mut locks = self.locks.write().await;
        if let Some(existing) = locks.get(key) {
            return Ok(AcquireLockResult::AlreadyLocked(existing.clone()));
        }

        locks.insert(key.to_string(), lock_info.clone());
        Ok(AcquireLockResult::Acquired)
    }

    async fn release_lock(
        &self,
        key: &str,
        expected_lock_id: Option<&str>,
    ) -> Result<ReleaseLockResult, StoreError> {
        let mut locks = self.locks.write().await;
        let Some(current) = locks.get(key).cloned() else {
            return Ok(ReleaseLockResult::NotLocked);
        };

        if let Some(expected_id) = expected_lock_id {
            let current_id = current
                .get("ID")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if current_id != expected_id {
                return Ok(ReleaseLockResult::LockMismatch(current));
            }
        }

        locks.remove(key);
        Ok(ReleaseLockResult::Released)
    }
}
