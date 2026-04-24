# Roadmap: Offline-First Windows File Manager

## 1. Delivery Principles

- Build vertical slices that are runnable at every milestone
- Keep filesystem operations safe before adding advanced UX
- Treat performance and accessibility as baseline requirements, not polish
- Prefer reversible, non-destructive versioning workflows

## 2. Phase Plan

## Phase 0: Foundation and Decisions (Week 0-1)

Goals:

- Lock technical decisions for persistence, diff abstraction, and watcher model
- Define initial domain schema and migration strategy
- Set coding and test conventions

Exact outcomes:

- A single decision package exists that defines architecture boundaries, storage model, and core service interfaces.
- The project can be scaffolded immediately into `ui`, `core`, `persistence`, `fs_adapter`, and `services` modules.
- Phase 1 implementation risks are reduced to feature execution, not foundational design ambiguity.

Deliverables:

- Architecture docs approved
- Tech stack document approved
- ADR document approved with accepted decisions for DB, snapshots, diff abstraction, watcher model, and storage cleanup rules
- Initial schema draft for `workspace`, `file_entry`, `version_snapshot`, `diff_artifact`, and `ui_state`
- Interface draft for `FileSystemAdapter`, `SnapshotService`, `DiffEngine`, and `WatcherService`

Exit criteria:

- Team can scaffold with low ambiguity
- No unresolved blocking decision remains for metadata DB, snapshot strategy, diff boundary, or watcher model
- Local app-data policy and cleanup rules are explicit and testable
- Architecture, tech stack, and ADR documents are internally consistent

Acceptance criteria (must pass):

- A developer unfamiliar with the project can create the initial module structure from docs alone.
- Filesystem write path is clearly documented as a single adapter boundary.
- Source files in user folders are never treated as app-owned storage.
- Diff backend is replaceable without UI or persistence rewrites.
- Cleanup behavior for cache/orphan artifacts is documented with trigger points (startup/shutdown).

## Phase 1: Shell App + FileExplorer MVP (Week 1-3)

Goals:

- Stand up minimal `gpui` + `gpui-component` app shell
- Implement `FileExplorer` with real filesystem browsing
- Add safe create/rename/delete for files and folders

Deliverables:

- Window shell with navigation regions
- Directory tree + listing with lazy expansion
- Inline rename/create interactions
- Context menu for base actions

Exit criteria:

- User can browse real folders and perform basic CRUD safely

## Phase 2: Local Persistence and Workspace State (Week 3-5)

Goals:

- Add metadata database and workspace management
- Persist open workspace, expanded tree nodes, selection, and UI layout

Deliverables:

- `rusqlite` schema + migrations
- Repository layer for `workspace`, `file_entry`, `ui_state`
- Startup restore of last session state

Exit criteria:

- App restores session and remains stable across restarts

## Phase 3: VersionManager + DetailPopup (Week 5-7)

Goals:

- Capture and store snapshots for tracked files
- Surface metadata and version actions in `DetailPopup`

Deliverables:

- Snapshot pipeline (`blake3` + `zstd`)
- Version list UI and restore action
- Basic retention policy and pruning command

Exit criteria:

- User can create, view, restore, and delete versions

## Phase 4: Split DocumentViewer + DiffViewer (Week 7-10)

Goals:

- Build split-view compare experience
- Add non-destructive text diff and synced scrolling

Deliverables:

- `DocumentViewer` with two panes
- `DiffViewer` with chunk navigation and highlight rendering
- Cached diff artifacts keyed by content hashes

Exit criteria:

- Two revisions can be compared with smooth synchronized navigation

## Phase 5: TimelineView + BulkActionPanel (Week 10-12)

Goals:

- Add node-based timeline exploration
- Add batch operations with validation and progress reporting

Deliverables:

- Timeline graph render + pan/zoom + selection
- Bulk copy/move/delete action panel
- Operation summaries and partial failure handling

Exit criteria:

- User can inspect history graph and execute batch actions confidently

## Phase 6: Hardening, Packaging, and UX Quality (Week 12+)

Goals:

- Improve reliability, performance, and Windows polish
- Package a testable Windows release candidate

Deliverables:

- Watcher stress tests and large-tree benchmarks
- Accessibility pass (focus, keyboard traversal, contrast)
- Installer/package and upgrade strategy

Exit criteria:

- Stable release candidate for wider internal testing

## 3. Early Milestones (Detailed)

## Milestone A: First Runnable Vertical Slice

Scope:

- App shell + FileExplorer read-only browsing

Definition of done:

- Launches in <2s on a typical developer machine
- Can open workspace root and navigate folders

## Milestone B: Safe Local Mutations

Scope:

- Create/rename/delete through filesystem adapter

Definition of done:

- Dangerous operations require clear confirmation
- Delete path supports Recycle Bin mode

## Milestone C: Persistent Session State

Scope:

- DB-backed workspace and UI restoration

Definition of done:

- Last workspace and UI state restore correctly after restart

## Milestone D: First Version Compare

Scope:

- Create snapshot + compare two revisions in split viewer

Definition of done:

- Diff appears within acceptable latency for medium files

## 4. Research Priorities (Execution Order)

R1. Diff engine benchmark (`similar` vs `imara-diff`)

- Measure memory, latency, and output quality on representative datasets

R2. Filesystem watcher behavior on Windows

- Validate rename/move cascades, burst coalescing, and debounce strategy

R3. Snapshot storage policy

- Compare full snapshots + compression vs optional chunked delta follow-up

R4. Rendering strategy for large trees/timelines

- Validate virtualization approach and frame-time budget

R5. PDF/document support path

- Decide optional plugin strategy and fallback behavior for unsupported formats

## 5. Risk Register and Mitigations

Risk: watcher event storms degrade UI responsiveness

- Mitigation: queue coalescing, debounce windows, background processing

Risk: large-file diff latency harms UX

- Mitigation: progressive diff rendering, cached artifacts, algorithm fallback

Risk: snapshot store grows too quickly

- Mitigation: retention policies, compression, TTL-based cache pruning

Risk: path edge cases on Windows cause operation bugs

- Mitigation: canonicalization, normalization, long-path test coverage

Risk: accessibility retrofits become costly

- Mitigation: keyboard/focus acceptance checks in every phase

## 6. Testing Strategy by Phase

- Phase 1-2: unit tests for path validation, adapter guards, and DB repositories
- Phase 3: snapshot integrity tests and restore correctness tests
- Phase 4: diff correctness tests + synchronized-scroll behavior tests
- Phase 5-6: integration tests for bulk operations and timeline interactions

## 7. First Implementation Task Prompt

Use this prompt to begin coding:

"Create a minimal Rust desktop app shell using `gpui` and `gpui-component 0.5.1` on Windows. Implement a `FileExplorer` component that can open a chosen root folder, lazily list directories/files, and support create/rename/delete operations through a filesystem adapter interface (with path validation and Recycle Bin delete mode). Add local persistence for workspace root and expanded tree nodes using SQLite (`rusqlite`). Keep code modular: `ui`, `core`, `fs_adapter`, `persistence` modules, with clear command/event boundaries."
