# Research Summary

## Scope Covered

This summary captures targeted research for:

- `gpui-component 0.5.1` and GPUI desktop architecture
- Windows 11 desktop app constraints for Rust tools
- Offline-first local persistence and metadata storage options
- Versioned file manager UX patterns
- Split-view diff UX and Rust diff engine options

## Key Findings

1. GPUI stack is viable for high-performance desktop UI

- `gpui-component 0.5.1` provides practical component primitives for complex layout/stateful desktop interactions.

1. Offline-first model should be metadata-first, file-in-place

- Keep user files where they are.
- Persist app metadata, version indexes, and UI state locally.
- Store snapshot payloads in managed app data with retention rules.

1. SQLite is the strongest phase-1 metadata choice

- `rusqlite` offers robust local durability and low operational complexity.
- `redb` is a credible pure Rust alternative, but SQLite is easier for query-rich metadata and migrations.

1. Diff should be abstracted from day one

- A `DiffEngine` trait prevents early lock-in and allows benchmark-driven selection.
- `similar` and `imara-diff` are the best initial candidates for text compare.

1. Windows file operations need strict adapter boundaries

- Centralize create/rename/move/copy/delete in one adapter.
- Use Recycle Bin delete integration and path validation by default.

1. Compare UX should mimic proven patterns

- Side-by-side diff, chunk navigation, collapse unchanged blocks, and synchronized scrolling are essential.
- Both Monaco and CodeMirror docs reinforce these patterns as user expectations.

## Recommended Initial Decisions

- Use `gpui` + `gpui-component 0.5.1` for UI foundation.
- Use `rusqlite` for metadata DB and migrations.
- Use `blake3` + `zstd` for snapshot integrity/compression.
- Use `notify` + `walkdir` + `ignore` for filesystem indexing/watch.
- Implement diff abstraction with `similar` first and benchmark `imara-diff`.

## Open Questions for Early Spikes

1. Snapshot strategy depth

- Full snapshot only initially, or early delta/chunk optimization?

1. Large document behavior

- At what size threshold do we switch to progressive diff rendering?

1. Viewer format support

- Which non-text formats are phase-1 vs phase-2 (image, markdown, PDF)?

1. Watcher coalescing policy

- What debounce window best balances responsiveness vs event storms?

## Practical Implications

- The app can ship value early with a safe file explorer + local metadata baseline.
- Advanced versioning and timeline features should build atop a stable storage and adapter core.
- Most future risk is operational (large trees, watcher churn, diff latency), not conceptual.

## Suggested Immediate Next Build Step

Build the minimal shell + `FileExplorer` + SQLite-backed workspace state first, then layer versioning and compare features incrementally.
