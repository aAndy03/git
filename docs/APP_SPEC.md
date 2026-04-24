# Offline-First Windows File Manager App

## Vision

A native Windows 11 desktop app built in Rust using `gpui-components 0.5.1`. The app is an offline-first file manager that surfaces real files and folders without duplicating them, while providing advanced versioning, timeline visualization, and diff-aware split-screen comparison.

## Core Goals

- File manager experience that operates directly on user files and folders
- Offline-first behavior with local persistence and cache management
- Complex GUI with modern Windows 11 styling and responsive layout
- Versioning and restoration for files
- Timeline / node editor visualization for file history and changes
- Split-screen document viewing with diff highlighting and synced scrolling
- Context-aware menus, detail popups, inline editing, and bulk actions
- Efficient caching, lazy rendering, and non-destructive diff workflows
- Clean resource management and no unnecessary user-system clutter

## Target Architecture

1. UI layer: Rust + `gpui-components 0.5.1`
2. Application core: local file metadata, workspace model, version history manager
3. Storage layer: local database + disk-backed caches
4. File system adapter: read/write, import/export, rename, copy/move/delete
5. Diff engine integration: fast comparison for documents and versions

## Suggested Storage Strategy

- Use a native local database for metadata and version tracking
  - SQLite or embedded key-value store (e.g. `sled`)
- Keep actual files on disk in place, except for managed version snapshots
- Store:
  - file entries and folder tree
  - version history records
  - timeline nodes and annotations
  - UI preferences and open-document state
- Use a cache manager for preview thumbnails and diff data

## High-Level Component Breakdown

- `FileExplorer`
  - System-backed file/folder browser
  - Create, import, rename, delete, move, copy
  - Surface files without duplicating them
- `DetailPopup`
  - Metadata and version controls for selected item
  - Actions: restore version, delete version, open details
- `VersionManager`
  - Track file versions and snapshots
  - Support restore, delete, compare
- `TimelineView`
  - Visualize history across one or multiple files
  - Pan/zoom with content bounds and padding
  - Node editor style UI with draggable timeline nodes
- `DocumentViewer`
  - Split-screen view for two files/versions
  - Sync scroll across panes and pages
  - Render supported file types (text, markdown, images, PDFs if possible)
- `DiffViewer`
  - Highlight non-destructive differences between versions
  - Real-time comparison on scroll
- `ContextMenu`
  - Action-aware menus depending on active area
  - File context, empty-space context, title-bar context
- `BulkActionPanel`
  - Multi-select support for folder/file operations
  - Batch copy, move, delete, version apply
- `InlineEditor`
  - Rename and confirmation flows inside the UI
  - Smart validation and focus handling

These high-level shall integrate smaller level components. those smaller level could handle spcific ui elements. and all the levels shall import from `gpui-components` and use its primitives for layout, styling, and interactivity.

## Key Feature Map

- File management
  - Create folders/files
  - Import folders/files and surface them
  - Copy/move/delete acting on real filesystem items
- Versioning
  - Capture versions for files
  - Restore or delete versions
- Timeline + node editor
  - Visual timeline of file/version history
  - Pan/zoom with bounds and padding
- Detail popup
  - Open metadata/details on demand
- Context-aware menus
  - Adjust available actions by activation area
- Split-screen viewer
  - Open two documents side-by-side
  - Sync scroll and page-aware diff handling
  - Support different versions of the same file
- Inline editing / bulk actions
  - Rename in-place
  - Bulk file operations

## Diff Engine Guidance

- Investigate open source diff libraries compatible with Rust
  - `similar` / `similarity` crates for text diffs
  - `ropey` or `xi-rope` for efficient text editing and diff support
- Consider adapting ideas from Visual Studio Code’s diff layout
  - side-by-side comparison with aligned scroll
  - line/character diff highlighting
- For documents with pages, treat page pairs as synchronized units
  - allow one version to scroll independently if page counts differ

## Recommended First Phase

1. Create a minimal Rust workspace and `gpui-components` shell app
2. Implement `FileExplorer` that reads native file system structure
3. Add basic folder/file creation, rename, delete, and import
4. Add metadata model and local persistence layer
5. Build a simple `DocumentViewer` split pane
6. Add version metadata and a simple restore UI

## Recommended Docs Structure

- `docs/APP_SPEC.md` — overall design and feature map
- `docs/ARCHITECTURE.md` — detailed component architecture and data flows
- `docs/ROADMAP.md` — phased implementation plan with milestones
- `docs/TECH_STACK.md` — dependencies, Rust crates, gpui usage notes

## Current Workspace Status

- No existing Rust project files found in the current workspace
- A new docs folder now holds this app specification
- Next step: decide whether to scaffold the Rust app or refine the first prompt

## Next-Step Options

- Create initial Rust + gpui project scaffold
- Define the first feature prompt for `FileExplorer`
- Research Windows file system access and local persistence options
- Develop a component API plan for `gpui-components 0.5.1`
