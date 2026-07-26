# AGENTS.md

This file provides guidance to AI coding agents (Claude Code and others) when working with code in this repository.

## Project

Featherboard is a self-hosted customer feedback board (feedback board + voting + roadmap + changelog) written in Rust. The project is being built in the open, so commit history is treated as a public artifact — changes should be deliberate and well-scoped rather than squashed or reworked after the fact.

**Stack:** `axum` + `sqlx`/SQLite (single database for both self-hosted and hosted deployments) + `askama` server-rendered templates (compiled into the binary) + `htmx` for interactivity (no client-side framework). Billing via Stripe Checkout + webhooks. Email via `lettre`/SMTP. Ships as a single binary — no Docker or Postgres required to self-host.

**License:** AGPL-3.0.

**Note:** this is a clean-room implementation — do not copy source code from any similar existing product when researching prior art or feature behavior.

Repo currently has scaffolding in place; features are being built incrementally.

## Commands

- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Run a single test: `cargo test <test_name>`

## Workflow conventions

- **Git commits:** never run `git add`, `git commit`, `git push`, or any other git write/history-modifying command. Staging, committing, and pushing are exclusively the user's own actions — write files to disk and propose the commit, then stop and let the user handle git. This applies even if asked to "save progress" or similar.
- **Branch naming:** `type/short-description` (e.g. `chore/project-setup`, `feat/...`, `fix/...`).
