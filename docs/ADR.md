# ADR: Phase 0 Foundation Decisions

Date: 2026-04-24
Status: Accepted
Scope: Offline-first Windows 11 Rust file manager baseline

## ADR-001: Metadata Database

Decision:

- Use SQLite through `rusqlite` with bundled SQLite on Windows.

Why:

- Strong transactional durability and predictable behavior on local desktop systems.
- Clear migration workflow for evolving schemas.
- Easy local inspection/debugging for metadata issues.

Consequences:

- Repository layer must encapsulate all SQL access.
- Schema migrations are mandatory for structural changes.

## ADR-002: Snapshot and Version Storage Strategy

Decision:

- Keep source files in place and store managed snapshots in app-local storage.
- Snapshot payloads are checksummed with `blake3` and compressed with `zstd`.

Why:

- Preserves the app goal of surfacing real filesystem items without duplicating active files.
- Enables restore/compare workflows without mutating source content during analysis.
- Supports bounded retention and cleanup without touching user originals.

Consequences:

- Snapshot manager must track retention policy, dedupe opportunities, and pruning.
- Restore flow must be explicit, confirmed, and auditable.

## ADR-003: Diff Engine Abstraction

Decision:

- Introduce a `DiffEngine` trait as the single diff boundary.
- Default implementation uses `similar`; alternative implementations (for example `imara-diff`) remain pluggable.

Why:

- Avoids lock-in while preserving a stable UI contract.
- Allows benchmark-driven engine replacement without architecture churn.

Consequences:

- DiffViewer consumes normalized diff chunks, not backend-specific output.
- Diff artifacts are cached by content hash pair and options.

## ADR-004: Filesystem Watcher Model (Windows)

Decision:

- Use a `WatcherService` built on `notify` with event coalescing and debounce.
- Apply periodic lightweight reconciliation checks to handle dropped/merged native events.

Why:

- Windows watcher streams can be bursty and noisy during bulk operations.
- Coalescing protects UI responsiveness and reduces redundant state churn.

Consequences:

- Core state updates consume normalized watcher events from a queue.
- Watcher processing runs off the UI thread.

## ADR-005: Local Storage Policy and Cleanup Rules

Decision:

- Store DB, snapshots, and cache under user-local app data paths (`directories` crate).
- Enforce bounded cache/snapshot retention and deterministic cleanup.

Policy rules:

- No app data in arbitrary user folders unless explicitly exported by user action.
- Cleanup runs at startup and graceful shutdown.
- Cleanup removes:
  - stale diff artifacts past TTL
  - orphan snapshot blobs without metadata references
  - obsolete cache entries beyond size/age thresholds

Why:

- Prevent user-system clutter and keep offline-first storage predictable.
- Reduce disk growth risk while preserving required history.

Consequences:

- Cleanup outcomes should be logged for diagnostics.
- Retention thresholds must be configurable with safe defaults.

## Implementation Boundaries Confirmed by This ADR

- UI: emits intents, renders state, no direct filesystem/DB writes.
- Core: validates commands and orchestrates services.
- Persistence: owns schema/migrations/repositories.
- FileSystemAdapter: single write gateway for filesystem mutations.
- Diff services: compute and cache compare artifacts behind `DiffEngine`.

## Phase 0 Completion Check

Phase 0 is complete when these ADR decisions are reflected consistently in:

- `docs/ARCHITECTURE.md`
- `docs/TECH_STACK.md`
- `docs/ROADMAP.md`
and no blocking ambiguity remains for scaffolding the initial project modules.
