# Terraform/OpenTofu HTTP State Backend (Rust + Postgres)

Production-oriented HTTP state backend compatible with Terraform/OpenTofu `backend "http"`.

## Quick Start (Docker Compose)

```bash
docker compose up --build -d
docker compose logs -f backend
```

Backend URL: `http://127.0.0.1:8080`

Stop services:

```bash
docker compose down
```

Stop and remove database volume:

```bash
docker compose down -v
```

## Features

- Full HTTP backend state API behavior (`GET`, update, `DELETE`, `LOCK`, `UNLOCK`).
- PostgreSQL persistence (`JSONB` state and lock metadata).
- Lock-safe writes using the `?ID=<lock-id>` query parameter.
- Optional HTTP Basic Auth.
- Async Rust implementation with `axum` + `sqlx`.

## Compatibility

This service is designed for Terraform/OpenTofu HTTP backend semantics:

- Read state via `GET`.
- Update state via `POST` (also accepts `PUT`/`PATCH`).
- Purge state via `DELETE`.
- Lock with `LOCK`, unlock with `UNLOCK`.
- Lock contention returns `423 Locked` with current lock body.
- Lock mismatch on unlock returns `409 Conflict` with current lock body.

## Endpoint Behavior

- `GET /state/<name>`: return current state, or `404` if missing.
- `POST /state/<name>`: write state JSON.
- `DELETE /state/<name>`: purge state.
- `LOCK /state/<name>`: acquire lock using JSON lock payload.
- `UNLOCK /state/<name>`: release lock using JSON lock payload.
- Writes/deletes under lock require `?ID=<lock-id>` query parameter.

## Configuration

Required:

- `DATABASE_URL` (example: `postgres://tf_backend:secret@localhost:5432/tf_backend`)

Optional:

- `LISTEN_ADDR` (default: `0.0.0.0:8080`)
- `DB_MAX_CONNECTIONS` (default: `20`)
- `RUST_LOG` (default: `tf_http_pg_backend=info,info`)
- `HTTP_BASIC_USERNAME`
- `HTTP_BASIC_PASSWORD`

If either basic auth variable is set, both must be set.

See `.env.example` for a complete local template.

## Authentication

This backend supports optional HTTP Basic Auth.

- Disabled by default: if `HTTP_BASIC_USERNAME` and `HTTP_BASIC_PASSWORD` are both unset.
- Enabled when both are set.
- If only one is set, the service fails on startup.

Enable auth:

```bash
export HTTP_BASIC_USERNAME="tofu"
export HTTP_BASIC_PASSWORD="change-me"
```

Use with OpenTofu/Terraform HTTP backend:

```hcl
terraform {
  backend "http" {
    address        = "http://127.0.0.1:8080/state/prod"
    lock_address   = "http://127.0.0.1:8080/state/prod"
    unlock_address = "http://127.0.0.1:8080/state/prod"
    username       = "tofu"
    password       = "change-me"
  }
}
```

You can also provide credentials via environment variables:

- `TF_HTTP_USERNAME`
- `TF_HTTP_PASSWORD`

## Local Run

```bash
cargo run
```

Example:

```bash
export DATABASE_URL="postgres://tf_backend:secret@localhost:5432/tf_backend"
export LISTEN_ADDR="0.0.0.0:8080"
cargo run
```

## Docker

```bash
docker build -t tf-http-pg-backend .
docker run --rm -p 8080:8080 -e DATABASE_URL="postgres://tf_backend:secret@postgres:5432/tf_backend" tf-http-pg-backend
```

## Docker Compose

`docker-compose.yml` runs PostgreSQL and backend together, and pulls backend image from GHCR (no local build).

Set image (recommended):

```bash
export BACKEND_IMAGE="ghcr.io/<owner>/<repo>:latest"
```

For this repository:

```bash
export BACKEND_IMAGE="ghcr.io/ginbun/tf-http-backend:latest"
```

```bash
docker compose up -d
```

## OpenTofu/Terraform Backend Example

```hcl
terraform {
  backend "http" {
    address        = "http://127.0.0.1:8080/state/prod"
    lock_address   = "http://127.0.0.1:8080/state/prod"
    unlock_address = "http://127.0.0.1:8080/state/prod"
  }
}
```

You can also split lock/unlock endpoints if needed by setting different URLs.

Run initialization:

```bash
tofu init
```

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Run a single test:

```bash
cargo test lock_then_write_without_id_returns_423 -- --exact
```

## Notes

- The server uses request path as the state resource key.
- Tables are auto-created on startup.
- Previous Python/MySQL implementation was removed by design for this Rust/Postgres rewrite.
