# Architecture: Offline-First Windows File Manager

## 1. Goals and Constraints

This architecture is designed for a Rust desktop app on Windows 11 using `gpui` + `gpui-component 0.5.1`.

Primary goals:

- Operate on real filesystem files and folders (no full content duplication into app-private storage)
- Offline-first behavior with local metadata and version snapshots
- Fast browsing and low-latency UI for large directory trees
- Safe file operations (create, rename, copy, move, delete) with clear recovery behavior
- Split-view document comparison with synchronized navigation and non-destructive diffing

Explicit constraints:

- Windows 11 UX conventions and keyboard accessibility are first-class
- Metadata persistence must remain local and resilient to unexpected shutdown
- Snapshot/version history should be bounded and garbage-collectable

## 2. High-Level System Design

The system is layered into five bounded modules:

1. UI Layer (`gpui`, `gpui-component`)

- Renders all visual components and routes user intents to app core commands.

1. Application Core

- Owns domain models, command handlers, orchestration, and policies.

1. Persistence Layer

- Stores metadata, versions index, timeline graph, UI state, and caches.

1. File System Adapter

- Executes real filesystem operations through a safe abstraction.

1. Diff and Preview Services

- Computes text diffs, manages incremental artifacts, and supports split-view sync state.

## 3. Component Architecture

### 3.1 FileExplorer

Responsibilities:

- Display lazily loaded folder tree and current directory listing
- Support select, multi-select, rename, create, import, move/copy/delete
- Publish selection and operation intents

Inputs:

- `WorkspaceState`, directory listing snapshots, watcher events

Outputs:

- Command intents to App Core (`CreateEntry`, `RenameEntry`, `DeleteEntries`, `MoveEntries`, `CopyEntries`)

### 3.2 DetailPopup

Responsibilities:

- Show metadata for active item (path, size, type, timestamps, tags, version count)
- Trigger version actions (open, restore, prune/delete)

Inputs:

- Selected item ID/path, version summaries

Outputs:

- `RestoreVersion`, `DeleteVersion`, `OpenInViewer`

### 3.3 VersionManager

Responsibilities:

- Capture snapshots according to policy (manual, on-save, periodic, operation-driven)
- Restore snapshots safely
- Prune old snapshots according to retention policy

Inputs:

- File change events and explicit version actions

Outputs:

- Snapshot records, restore outcomes, lifecycle events

### 3.4 TimelineView

Responsibilities:

- Render history as a node/edge timeline
- Pan/zoom with bounds, culling, and viewport-based rendering
- Support selecting two revisions for comparison

Inputs:

- Timeline graph data and filters

Outputs:

- Selection changes (`left_revision`, `right_revision`)

### 3.5 DocumentViewer

Responsibilities:

- Display one or two documents/versions
- Keep split panes synchronized by scroll anchor policy
- Host specialized viewers (text, markdown, image, optional PDF)

Inputs:

- Open document handles + revision IDs

Outputs:

- Scroll state updates and compare requests

### 3.6 DiffViewer

Responsibilities:

- Display line/chunk-level differences without mutating source content
- Support jump-next/jump-prev chunk and collapse unchanged regions
- Cache diff artifacts for fast revisit

Inputs:

- Left/right content streams or snapshot handles

Outputs:

- Visual diff model for rendering

### 3.7 ContextMenu

Responsibilities:

- Generate action sets based on active surface and selection type
- Enforce command availability and dangerous-action confirmation

Inputs:

- Surface context (`file_row`, `folder_row`, `empty_space`, `viewer`, `timeline`)

Outputs:

- User-selected command

### 3.8 BulkActionPanel

Responsibilities:

- Execute validated batch operations and present aggregate status
- Support dry-run preview for move/copy/delete

Inputs:

- Multi-selection + destination policy

Outputs:

- Batch operation commands + progress events

### 3.9 InlineEditor

Responsibilities:

- In-place rename/create flows with focus-safe commit/cancel
- Validate names using filesystem rules and conflict checks

Inputs:

- Active row state and operation mode

Outputs:

- Validated rename/create command

## 4. Data and Storage Architecture

## 4.1 Canonical Data Domains

1. Filesystem truth

- Actual files/folders remain at original locations.

1. Metadata store (local DB)

- Workspace roots, indexed entries, app state, version metadata, timeline graph, settings.

1. Snapshot store (managed app data)

- Content snapshots or delta blocks, compressed and checksummed.

1. Derived cache

- Diff artifacts, thumbnail/preview cache, virtualized list cache.

## 4.2 Suggested Logical Schema

- `workspace`
  - `id`, `name`, `root_path`, `created_at`, `last_opened_at`
- `file_entry`
  - `id`, `workspace_id`, `path`, `kind`, `size`, `mtime`, `hash_hint`, `deleted_at`
- `version_snapshot`
  - `id`, `file_entry_id`, `content_hash`, `stored_blob_path`, `codec`, `created_at`, `author`, `reason`
- `timeline_node`
  - `id`, `workspace_id`, `snapshot_id`, `x`, `y`, `label`, `created_at`
- `timeline_edge`
  - `from_node_id`, `to_node_id`, `edge_kind`
- `diff_artifact`
  - `id`, `left_snapshot_id`, `right_snapshot_id`, `algo`, `artifact_path`, `created_at`, `expires_at`
- `ui_state`
  - `workspace_id`, `pane_layout`, `selection`, `expanded_nodes`, `last_compare_pair`

## 4.3 Persistence Choice

Phase-1 recommendation:

- Metadata DB: `rusqlite` (with `bundled` SQLite for Windows reliability)
- Snapshot compression: `zstd`
- Integrity hash: `blake3`

Rationale:

- Stable, predictable local durability
- Easy migration strategy via SQL migrations
- Good interoperability for inspection and debugging

Alternative:

- `redb` if a pure Rust embedded KV model is later preferred

## 5. File System Adapter Design

All disk operations flow through a single adapter boundary:

Core API surface:

- `list_dir(path)`
- `stat(path)`
- `read_bytes(path)`
- `write_atomic(path, data)`
- `create_file(path)`, `create_dir(path)`
- `rename(src, dst)`
- `copy(src, dst)`
- `move(src, dst)`
- `delete(path, mode)` where mode is `RecycleBin` or `Permanent`

Safety rules:

- Canonicalize and normalize input paths before mutation
- Block operations outside active workspace roots unless explicitly allowed
- Prefer recycle-bin delete via OS integration
- Use staging temp files and atomic rename for writes

Windows-specific handling:

- Normalize display paths and UNC edge cases
- Use known folders for app data and caches
- Watch for case-insensitive conflicts and long-path behavior

## 6. Diff Strategy

Diff pipeline:

1. Fast equality probe (size + hash shortcut)
2. Content classification (text/binary/unsupported)
3. Text diff compute with pluggable engine
4. Chunk post-processing for UI (grouping, collapse ranges, anchors)
5. Cache diff artifact keyed by `(left_hash, right_hash, algo, options)`

Engine plan:

- Primary text diff abstraction supports `similar` and `imara-diff`
- Start with readability-focused output; benchmark both crates on large files
- Binary files: show metadata-level difference and optional hex fallback in later phases

Synchronized split scrolling:

- Anchor by nearest diff chunk and relative offset inside chunk
- Fallback to percentage sync when chunk mapping is unavailable
- Allow temporary desync toggle for mismatched page/file structures

## 7. Runtime and Event Flow

Event sources:

- UI interactions
- Filesystem watcher updates
- Background diff/snapshot jobs

State flow model:

- UI emits command intent -> App Core validates -> Adapter/service executes -> state store updates -> UI re-renders

Background work:

- Diff compute, snapshot compression, and large scans run off UI thread
- Results are emitted as typed events and merged into state atomically

## 8. Performance and Scaling

Performance principles:

- Virtualized file lists and timeline rendering
- Lazy folder expansion and incremental indexing
- Opportunistic caching with TTL and size caps
- Avoid full-file loads for large documents when possible

Expected heavy scenarios:

- Large directory trees (100k+ entries)
- Frequent watcher churn
- Very large text files in split-diff mode

Mitigations:

- Debounced watcher coalescing
- Chunked diff computation
- Backpressure for queue saturation

## 9. Windows 11 UX and Accessibility

UX expectations:

- Keyboard-first interaction model for explorer and context menus
- Predictable focus rings and tab order
- Native-feeling context actions and confirmation dialogs
- Respect system scaling and high-DPI rendering

Accessibility requirements:

- Distinct focus state on all interactive elements
- Shortcut discoverability in menus/tooltips
- Color-safe diff highlighting with non-color indicators

## 10. Reliability and Recovery

Failure strategy:

- Never mutate metadata until filesystem operation result is known
- Journal important app-level operations for undo/diagnostics
- Recover from partial failures by reconciling DB state with current filesystem snapshots

Crash resilience:

- Transactional DB writes
- Atomic snapshot manifest updates
- Startup integrity scan for stale cache/snapshot records

## 11. Security and Privacy

- Keep all app state local by default
- No mandatory cloud dependency
- Store only needed metadata and bounded caches
- Avoid indexing excluded/system-sensitive paths unless user opts in

## 12. Architecture Decisions (Initial)

- AD-01: Use local SQLite metadata DB for phase 1.
- AD-02: Keep source files in place; version history stored as managed snapshots.
- AD-03: Route all file mutations through a single filesystem adapter boundary.
- AD-04: Implement a pluggable text diff engine and benchmark before final lock-in.
- AD-05: Prioritize keyboard accessibility and focus handling from first UI milestone.

## 13. Phase 0 Finalized Architecture Summary

Phase 0 outcome is a locked architecture baseline for scaffolding. These decisions are final for implementation start and should not be reopened unless a blocker appears.

Locked decisions:

- Metadata DB: SQLite via `rusqlite` with bundled SQLite on Windows.
- Snapshot strategy: file-in-place source data + managed local snapshot store under app data.
- Diff strategy: `DiffEngine` trait boundary with `similar` as default implementation for phase 1.
- Watcher model: `notify`-based watcher service with debounced/coalesced event queue.
- Local storage policy: all metadata/snapshots/cache under user-local app data with bounded retention and cleanup at startup/shutdown.

Phase 0 module boundaries (implementation contract):

- UI layer:
  - No direct filesystem or DB access.
  - Emits typed intents/commands and renders state only.
- Core layer:
  - Owns domain models, validation rules, orchestration, and command handling.
  - Calls persistence/adapter/services through explicit interfaces.
- Persistence layer:
  - Owns schema migrations, transactions, and repository APIs.
  - No UI concerns and no direct rendering models.
- File system adapter layer:
  - Single write gateway for all mutating filesystem operations.
  - Enforces path guardrails and delete mode policy.
- Diff services layer:
  - Converts two revisions into cached visual diff artifacts.
  - Isolated from UI rendering details.

Phase 0 acceptance checklist:

- A new developer can scaffold `ui`, `core`, `persistence`, `fs_adapter`, and `services` modules with low ambiguity.
- Storage paths, retention behavior, and cleanup rules are documented and testable.
- File operation safety rules are explicit and centralized.
- Diff and watcher boundaries are interface-first and replaceable.
