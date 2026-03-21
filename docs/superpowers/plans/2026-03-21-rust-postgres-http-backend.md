# Rust + Postgres Terraform HTTP Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python/MySQL backend with a Rust/Postgres service fully compatible with Terraform/OpenTofu HTTP backend semantics.

**Architecture:** Build an `axum` service with a focused routing layer and a `sqlx`-backed store layer. Use path-based resource keys and lock-aware write behavior using Terraform lock ID query semantics.

**Tech Stack:** Rust 2021, axum, tokio, sqlx (postgres), serde/serde_json, tracing.

---

### Task 1: Scaffold Rust service and dependencies

**Files:**
- Create or modify: `Cargo.toml`
- Create or modify: `src/main.rs`

- [ ] **Step 1: Add crate metadata and dependencies**
- [ ] **Step 2: Add server bootstrap with env-driven config**
- [ ] **Step 3: Build debug binary**
Run: `cargo build`
Expected: compile succeeds

### Task 2: Implement config and persistence layers

**Files:**
- Create or modify: `src/config.rs`
- Create or modify: `src/store.rs`

- [ ] **Step 1: Add config parser for env vars and auth validation**
- [ ] **Step 2: Add Postgres schema auto-init and CRUD/lock methods**
- [ ] **Step 3: Add in-memory store for fast protocol tests**
- [ ] **Step 4: Run compile check**
Run: `cargo check`
Expected: no compile errors
- [ ] **Step 5: Validate Postgres schema initialization path**
Run: `DATABASE_URL=postgres://user:pass@localhost:5432/tf_backend cargo run`
Expected: service starts and auto-creates tables

### Task 3: Implement HTTP backend-compatible handlers

**Files:**
- Create or modify: `src/app.rs`

- [ ] **Step 1: Add route coverage for GET/POST/PUT/PATCH/DELETE/LOCK/UNLOCK**
- [ ] **Step 2: Enforce lock ID behavior on state writes/deletes**
- [ ] **Step 3: Implement optional basic auth guard**
- [ ] **Step 4: Add protocol-focused unit tests in module**
- [ ] **Step 5: Run tests**
Run: `cargo test app::tests`
Expected: tests pass

### Task 4: Replace legacy assets and docs

**Files:**
- Modify: `README.md`
- Modify: `Dockerfile`
- Modify: `db_init.sql`
- Modify: `.gitignore`
- Delete: `app.py`, `requirements.txt`, `config_example.ini`, `.flaskenv`

- [ ] **Step 1: Document Rust/Postgres runtime and Terraform config**
- [ ] **Step 2: Replace Docker image with Rust release build flow**
- [ ] **Step 3: Update SQL init script to Postgres tables**
- [ ] **Step 4: Remove legacy Python files**
- [ ] **Step 5: Verify no Python runtime assets remain (except intentional docs/examples)**
Run: `git grep -nE "(flask|requirements\.txt|app\.py|\.flaskenv)"`
Expected: no runtime references in active implementation paths

### Task 5: Agent guidance refresh and final verification

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Update AGENTS guide to Rust/Postgres commands and style**
- [ ] **Step 2: Run formatting**
Run: `cargo fmt --all`
Expected: no diff after formatting rerun
- [ ] **Step 3: Run strict lint**
Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: zero warnings/errors
- [ ] **Step 4: Run full tests**
Run: `cargo test`
Expected: all tests pass
