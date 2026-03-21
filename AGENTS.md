# AGENTS.md
Operational guide for coding agents in this repository.

## 1) Project Overview
- Language/runtime: Rust (edition 2021).
- HTTP framework: `axum`.
- Database: PostgreSQL via `sqlx`.
- Purpose: Terraform/OpenTofu `backend "http"` compatible state backend.
- Entry point: `src/main.rs`.
- Main routing/HTTP behavior: `src/app.rs`.
- Persistence implementation: `src/store.rs`.

## 2) Rule Files Check (Cursor / Copilot)
Checked at generation time:
- `.cursorrules`: not found.
- `.cursor/rules/`: not found.
- `.github/copilot-instructions.md`: not found.
If these files are added later, treat them as higher-priority instructions.

## 3) Setup Commands
Install Rust toolchain (if missing):
```bash
rustup default stable
```
Install and build dependencies:
```bash
cargo fetch
cargo build
```

## 4) Required Environment Variables
- `DATABASE_URL` (required), e.g. `postgres://user:pass@localhost:5432/tf_backend`.

Optional:
- `LISTEN_ADDR` (default `0.0.0.0:8080`).
- `DB_MAX_CONNECTIONS` (default `20`).
- `RUST_LOG`.
- `HTTP_BASIC_USERNAME` and `HTTP_BASIC_PASSWORD` (must be set together).

## 5) Build / Run Commands
Build debug binary:
```bash
cargo build
```
Build release binary:
```bash
cargo build --release
```
Run locally:
```bash
DATABASE_URL=postgres://user:pass@localhost:5432/tf_backend cargo run
```
Run with custom listen address:
```bash
DATABASE_URL=postgres://user:pass@localhost:5432/tf_backend LISTEN_ADDR=127.0.0.1:8080 cargo run
```
Docker:
```bash
docker build -t tf-http-pg-backend .
docker run --rm -p 8080:8080 -e DATABASE_URL=postgres://user:pass@postgres:5432/tf_backend tf-http-pg-backend
```

## 6) Lint / Format Commands
Format:
```bash
cargo fmt --all
```
Lint (strict):
```bash
cargo clippy --all-targets --all-features -- -D warnings
```
Fast compile check:
```bash
cargo check
```

## 7) Test Commands (Including Single-Test)
Run all tests:
```bash
cargo test
```
Run tests in one module/file target:
```bash
cargo test app::tests
```
Run one exact test (preferred single-test command):
```bash
cargo test lock_then_write_without_id_returns_423 -- --exact
```
Run one integration test binary (if `tests/` exists later):
```bash
cargo test --test backend_contract
```

## 8) Terraform/OpenTofu HTTP Backend Compatibility Rules
- Must support state read (`GET`), update (`POST` default), purge (`DELETE`).
- Must support locking via `LOCK` and unlocking via `UNLOCK` when configured.
- Lock conflict must return `423` or `409` with holding lock info JSON.
- Keep lock ID behavior compatible with `?ID=<lock-id>` query parameter on writes.
- Avoid breaking protocol semantics before adding optional convenience endpoints.

## 9) Rust Code Style Guidelines
### Imports and modules
- Prefer explicit imports; avoid wildcard imports.
- Keep module boundaries focused by responsibility.
- Group imports from std, third-party, then crate modules.

### Formatting and structure
- Always run `cargo fmt --all`.
- Keep handlers thin; move DB logic into store layer.
- Prefer small pure helper functions for parse/validation branches.

### Types and errors
- Use concrete domain enums for branchy outcomes (lock acquired/conflict/etc.).
- Use `Result<T, E>` and typed error enums (`thiserror`) instead of string errors.
- Return explicit HTTP status codes for protocol-significant behavior.

### Naming
- `snake_case`: functions, variables, modules.
- `PascalCase`: structs/enums/traits.
- `SCREAMING_SNAKE_CASE`: constants.
- Use domain terms (`lock_id`, `resource_key`, `lock_info`, `state`).

### Async and performance
- Avoid blocking calls in request path.
- Reuse connection pool; do not open per-request DB connections.
- Keep JSON parse/serialize minimal in hot paths.

### Logging and observability
- Use `tracing` for key lifecycle events and errors.
- Never log credentials, auth headers, or full secrets.

## 10) Database and Migration Guidelines
- PostgreSQL schema should remain backward compatible unless explicitly requested.
- Prefer additive schema changes.
- Keep lock and state operations transactional where consistency matters.
- Use `JSONB` for Terraform/OpenTofu state and lock payloads.

## 11) Change Scope and Safety
- Make minimal, reviewable changes tied to the task.
- Do not add unrelated refactors.
- Preserve protocol behavior first; optimize internals second.
- If behavior changes are intentional, document them in `README.md`.

## 12) Verification Before Completion
Before declaring completion, run:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
If any command cannot run in the current environment, explicitly report what failed and provide the exact command for maintainers to run.
