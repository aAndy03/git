# Tech Stack Recommendations

## 1. Core Application Stack

UI framework:

- `gpui = 0.2.2`
- `gpui-component = 0.5.1`

Why:

- Native Rust desktop rendering model
- Component primitives and styling tools suitable for complex desktop UI
- Alignment with existing GPUI ecosystem used in high-performance editors

Usage pattern:

- Keep UI stateless where possible
- Push side effects into command handlers/services
- Use background tasks for expensive file scans and diff computation

## 2. Persistence and Local-First Storage

Primary recommendation:

- `rusqlite = 0.39.0` with bundled SQLite

Why:

- Reliable embedded DB with mature transaction semantics
- Simple migration path and easy local inspection
- Good fit for metadata-heavy offline-first apps

Alternative options:

- `sqlx = 0.8.6` if async SQL and stronger compile-time query guarantees become necessary
- `redb = 4.1.0` for pure Rust embedded KV storage

Not recommended as primary metadata store for this app stage:

- `sled = 0.34.7` due to project-level caution around production reliability in README notes

## 3. Filesystem and Path Tooling

Discovery, indexing, and watch:

- `walkdir = 2.5.0` for recursive traversal
- `ignore = 0.4.25` for gitignore-like filtering
- `notify = 8.2.0` for filesystem change events (Windows backend support)

Path and storage locations:

- `directories = 6.0.0` for standard app-data/cache directories
- `dunce = 1.0.5` for Windows path normalization ergonomics
- `path-clean = 1.0.1` for lexical path cleaning

File operations:

- `trash = 5.2.5` for Recycle Bin integration
- `fs_extra = 1.3.0` for higher-level copy/move helpers when needed

## 4. Versioning, Snapshot, and Cache

Snapshot integrity and compression:

- `blake3 = 1.8.4` for fast content hashing
- `zstd = 0.13.3` for compact snapshot and diff artifact storage

Optional fast non-crypto fingerprints:

- `xxhash-rust = 0.8.15`

Suggested snapshot model:

- Store source files in place
- Persist version snapshots in app-managed storage
- Index snapshots in SQLite
- Compress large snapshot payloads with zstd

## 5. Diff and Compare

Recommended abstraction:

- Define `DiffEngine` trait with pluggable implementations

Initial engines:

- `similar = 3.1.0` for developer-friendly text diff output
- `imara-diff = 0.2.0` for performance-focused alternatives on large documents
- `diffy = 0.4.2` if patch-generation workflows are needed later

Supporting utility:

- `dissimilar = 1.0.11` for lightweight semantic cleanup style behavior in specific UI modes

UX guidance for split compare:

- Support side-by-side and unified conceptual modes
- Chunk navigation and collapse unchanged regions
- Scroll synchronization anchored to nearest diff chunk

## 6. Suggested Module Layout

- `src/ui/`
  - UI components (`FileExplorer`, `DetailPopup`, `DocumentViewer`, `DiffViewer`, etc.)
- `src/core/`
  - Domain models, command handlers, state orchestration
- `src/fs_adapter/`
  - Real filesystem operations and watcher integration
- `src/persistence/`
  - DB repositories, schema migrations, cache metadata
- `src/services/`
  - Diff service, snapshot service, timeline service

## 7. Operational Defaults

Dependency policy:

- Pin minor versions initially for reproducible onboarding
- Revisit and update quarterly after benchmark/regression checks

Data policy:

- Keep metadata local by default
- Bound cache and snapshot retention by size and age

Safety policy:

- Route all mutating file operations via adapter boundary
- Enforce path guardrails before disk writes

## 8. External References

Framework and ecosystem:

- `gpui`: <https://crates.io/crates/gpui>
- `gpui-component`: <https://crates.io/crates/gpui-component>

Persistence:

- `rusqlite`: <https://crates.io/crates/rusqlite>
- `sqlx`: <https://crates.io/crates/sqlx>
- `redb`: <https://crates.io/crates/redb>

Filesystem:

- `notify`: <https://crates.io/crates/notify>
- `walkdir`: <https://crates.io/crates/walkdir>
- `ignore`: <https://crates.io/crates/ignore>
- `directories`: <https://crates.io/crates/directories>
- `trash`: <https://crates.io/crates/trash>

Diff and versioning support:

- `similar`: <https://crates.io/crates/similar>
- `imara-diff`: <https://crates.io/crates/imara-diff>
- `diffy`: <https://crates.io/crates/diffy>
- `dissimilar`: <https://crates.io/crates/dissimilar>
- `blake3`: <https://crates.io/crates/blake3>
- `zstd`: <https://crates.io/crates/zstd>

UX research references:

- CodeMirror merge docs: <https://codemirror.net/docs/ref/#merge>
- Monaco editor docs: <https://microsoft.github.io/monaco-editor/>
- Files app (Windows file manager UX reference): <https://github.com/files-community/Files>

## 9. Decision Summary

- Choose `rusqlite` first for metadata durability and migration clarity
- Keep source files in place; store only snapshots/caches in app data
- Start with `similar` and benchmark against `imara-diff` before final lock-in
- Prioritize Windows-safe file operations and keyboard-accessible UI behavior from day one

## 10. Phase 0 Locked Technology Decisions

These decisions are locked for project scaffolding and phase-1 implementation.

1. Metadata database (accepted)

- Choice: `rusqlite = 0.39.0` with bundled SQLite.
- Reason: strongest durability and migration clarity for local metadata.
- Rule: all metadata writes are transactional; schema changes use explicit migrations.

1. Snapshot/version storage (accepted)

- Choice: full snapshot payloads in local app-managed store.
- Supporting crates: `blake3 = 1.8.4`, `zstd = 0.13.3`.
- Rule: source files remain in place; snapshots are copy-on-capture and never overwrite source content directly.

1. Diff abstraction (accepted)

- Choice: `DiffEngine` trait with `similar = 3.1.0` as default backend.
- Rule: `imara-diff = 0.2.0` remains a benchmarked alternative behind the same trait boundary.

1. Filesystem watcher model for Windows (accepted)

- Choice: `notify = 8.2.0` watcher service with coalesced event queue.
- Rule: debounce watcher bursts before core-state updates; reconcile with periodic lightweight stat checks when needed.

1. Local storage policy and cleanup (accepted)

- Choice: store DB, snapshots, and cache under user local app-data paths from `directories = 6.0.0`.
- Rule: bounded cache size and snapshot retention policy are mandatory.
- Rule: cleanup runs on startup and graceful shutdown; stale diff artifacts and orphan snapshot records are pruned.
