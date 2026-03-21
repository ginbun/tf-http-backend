# Rust + Postgres Terraform HTTP Backend Design

## Goal

Replace the legacy Python/MySQL service with a Rust/PostgreSQL implementation that is fully compatible with Terraform/OpenTofu `backend "http"` semantics and safe lock behavior.

## Scope

- Implement state read/write/delete over HTTP.
- Implement lock/unlock semantics compatible with Terraform lock payload behavior.
- Persist state and locks in PostgreSQL.
- Keep endpoint model generic by resource path (no legacy `/tfstates/*` compatibility layer).
- Remove Python implementation assets from repository.

Out of scope:

- Legacy API compatibility endpoints.
- Multi-tenant auth systems beyond optional basic auth.

## Protocol Semantics

- `GET {address}`: return state JSON or `404` if missing.
- `POST|PUT|PATCH {address}`: write state JSON, return `200`.
- `DELETE {address}`: delete state, return `200`.
- `LOCK {lock_address}`: acquire lock, return `200`; on conflict return `423` or `409` + lock info JSON.
- `UNLOCK {unlock_address}`: release lock, return `200`; if lock id mismatch return `409` + lock info JSON.
- Write/delete operations must honor `?ID=<lock-id>` when lock exists; if ID is missing or mismatched, return `423` or `409` with current lock info JSON.

## Data Model

`tf_http_states`

- `resource_key TEXT PRIMARY KEY`
- `state JSONB NOT NULL`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`

`tf_http_locks`

- `resource_key TEXT PRIMARY KEY`
- `lock_id TEXT NOT NULL`
- `lock_info JSONB NOT NULL`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`

## Architecture

- `src/main.rs`: boot, config, pool init, migrations, server startup.
- `src/config.rs`: env parsing and validation.
- `src/app.rs`: routing, protocol enforcement, auth checks, request handling.
- `src/store.rs`: `StateStore` abstraction and Postgres implementation.

## Error Handling

- Return protocol-significant status codes for lock/state operations.
- Return JSON error bodies for malformed payloads and internal errors.
- Avoid leaking secrets in logs and responses.

## Performance Considerations

- Async request path (`tokio`, `axum`).
- Shared PostgreSQL connection pool (`DB_MAX_CONNECTIONS`).
- Minimal JSON transformations; store payloads as `JSONB`.

## Test Strategy

- Handler-level tests using in-memory store for protocol behavior.
- Validate lock/write ID semantics and status-code compatibility.
- Keep single-test command clear for rapid iteration.
