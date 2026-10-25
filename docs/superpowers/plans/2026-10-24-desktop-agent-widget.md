# Desktop Agent Widget Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Build a cross-platform floating quota monitor with offline provider icons and interactive CLI sessions.

**Architecture:** Tauri hosts React and a Rust core. SQLite persists settings and profile references. Provider adapters collect quotas; the session service owns PTYs and structured connections; an authenticated bridge carries Claude hooks.

**Tech Stack:** Tauri 2, React/TypeScript, Rust, SQLite, portable-pty, xterm.js.

**Spec:** `docs/superpowers/specs/2026-10-24-desktop-agent-widget-design.md` (approved).

## Global Constraints

- macOS, Windows and Linux; report compositor-specific window limitations.
- Fourteen provider entries with offline icons; account integration status is separate from catalog coverage.
- Configuration precedence: database, process environment, external files, bundled defaults.
- No credentials in renderer payloads, logs, SQLite or commits.
- Missing quotas remain unknown; resetting timestamps cannot manufacture balances.
- Confirmations require explicit action and a live originating connection.
- Docs precede implementation; README includes 中文 and English.

## Review Focus

- Missing fields, zero values, elapsed resets and account switches cannot produce fictitious quota (Tasks 1/2).
- Duplicate, cancelled and cross-session replies cannot approve another operation (Task 3).
- Windows paths with spaces, Unicode output and failed CLI launches produce recoverable errors (Task 3).
- Missing tray or hidden/offscreen windows remain recoverable (Task 4).
- Unavailable CLI, unsupported accounts and network failures remain distinct (Tasks 2/4).

### Task 1: Domain, configuration and provider catalog

Files: `config/defaults.json`, `src-tauri/src/{model,config,store,quota}.rs`, `src/types.ts`, `public/icons/`, build manifests.

Interfaces: `Provider`, `Profile`, `QuotaSnapshot`, `QuotaWindow`, `Settings`; `Store::open`, `load_settings`, `save_settings`; `normalize_quota`.

- [x] Write and run failing tests for quota zero/missing/invalid values and persistent settings precedence.
- [x] Implement typed normalization, SQLite migrations and the configurable fourteen-provider catalog.
- [x] Add offline icons with attribution; run core tests and commit.

### Task 2: Discovery and quota collectors

Files: `src-tauri/src/providers/`, `src-tauri/src/discovery.rs`, `docs/providers.md`.

Interfaces: `discover(profile, provider) -> Discovery`, `refresh(profile, provider, config) -> QuotaSnapshot`; results exclude credentials.

- [x] Write failing fixture tests for Codex, Claude, Google and custom collector payloads, including expired/missing quotas.
- [x] Implement Codex app-server lookup, Claude account/status-line data, Google quota buckets and local installation discovery.
- [x] Add a user-configured JSON collector interface for products without a verified built-in endpoint, with timeout and bounded output.
- [x] Verify with synthetic subprocesses and safe read-only local probes; document actual capabilities and commit.

### Task 3: Sessions and interactive approvals

Files: `src-tauri/src/{sessions,broker,bridge,protocol}.rs`, bridge entry in `main.rs`, `src/components/{Terminal,ApprovalCard}.tsx`.

Interfaces: `start_session`, `write_session`, `resize_session`, `stop_session`, `respond`; session and request IDs are backend-issued.

- [x] Write and run broker tests for duplicate responses, cancellation, isolation and invalid choices.
- [x] Implement portable PTYs with output replay, resize, input and child cleanup.
- [x] Implement structured Codex approval sessions and authenticated Claude permission/question/status-line bridge; preserve raw terminal interaction for all supported CLI executables.
- [x] Test deny/allow/input flows with local fake agents, exit/timeout/Unicode behavior; commit.

### Task 4: Desktop UI and persistent settings

Files: `src/`, `src-tauri/src/{lib,desktop}.rs`, `src-tauri/tauri.conf.json`, capabilities.

Interfaces: dashboard/profile/settings/session IPC commands; dashboard, terminal and interaction events.

- [x] Build compact graphite/ivory floating UI, quota rows, provider logos, approval inbox and expanded sessions/settings.
- [x] Wire real state, agent visibility/order, manual/automatic refresh, CLI/profile configuration and Chinese/English themes.
- [x] Add tray restore, collapse, pin, close/quit and window recovery.
- [ ] Verify keyboard controls, preview layout and native launch on a usable desktop session. Native startup aborts before setup in this environment; visual automation is unavailable. Offline assets and production builds have been checked.

### Task 5: Packaging, verification and docs

Files: CI workflow, README, provider/configuration/validation docs, lockfiles and app icons.

- [x] Run frontend tests/build, Rust tests/formatting/native build and resolve failures.
- [x] Configure native CI checks for all three operating systems; distinguish configuration from executed checks.
- [x] Perform a fresh whole-branch review and fix significant findings with regression tests.
- [x] Update bilingual README with commands, configuration, actual provider support and platform dependencies; commit.

## Execution outcome

Implementation and local automated checks are complete. The provider catalog includes collector fallback for ten agents whose native quota APIs are not implemented. Windows/Linux CI is configured but not run here. Live Codex probing is blocked by protected CLI SQLite state; browser/native UI automation is unavailable. See `docs/validation.md` for the distinction between implemented, locally tested and outstanding external verification.
