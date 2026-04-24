use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::services::file_ops;

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub byte_len: u64,
    pub last_modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct PickerEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileSystemAdapter;

impl FileSystemAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn list_child_directories(&self, directory: &Path) -> Result<Vec<PathBuf>, String> {
        let canonical_directory = canonicalize_existing(directory)?;
        let reader = fs::read_dir(&canonical_directory).map_err(|err| {
            format!(
                "failed to read picker directory {}: {err}",
                canonical_directory.display()
            )
        })?;

        let mut children = Vec::new();

        for item in reader {
            let item = item.map_err(|err| {
                format!(
                    "failed to read picker entry in {}: {err}",
                    canonical_directory.display()
                )
            })?;

            let metadata = item.metadata().map_err(|err| {
                format!(
                    "failed to read picker metadata for {}: {err}",
                    item.path().display()
                )
            })?;

            if !metadata.is_dir() {
                continue;
            }

            let path = canonicalize_existing(&item.path())?;
            children.push(path);
        }

        children.sort_by(|left, right| {
            left.to_string_lossy()
                .to_lowercase()
                .cmp(&right.to_string_lossy().to_lowercase())
        });

        Ok(children)
    }

    pub fn list_picker_entries(&self, directory: &Path) -> Result<Vec<PickerEntry>, String> {
        let canonical_directory = canonicalize_existing(directory)?;
        let reader = fs::read_dir(&canonical_directory).map_err(|err| {
            format!(
                "failed to read picker directory {}: {err}",
                canonical_directory.display()
            )
        })?;

        let mut entries = Vec::new();

        for item in reader {
            let item = item.map_err(|err| {
                format!(
                    "failed to read picker entry in {}: {err}",
                    canonical_directory.display()
                )
            })?;

            let metadata = item.metadata().map_err(|err| {
                format!(
                    "failed to read picker metadata for {}: {err}",
                    item.path().display()
                )
            })?;

            let path = canonicalize_existing(&item.path())?;
            let name = item.file_name().to_string_lossy().to_string();

            entries.push(PickerEntry {
                path,
                name,
                is_dir: metadata.is_dir(),
            });
        }

        entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });

        Ok(entries)
    }

    pub fn list_dir(&self, root: &Path, dir: &Path) -> Result<Vec<EntryInfo>, String> {
        let root_canon = canonicalize_existing(root)?;
        let dir_canon = self.resolve_existing_path(&root_canon, dir)?;
        ensure_within_root(&root_canon, &dir_canon)?;

        let mut entries = Vec::new();
        let reader = fs::read_dir(&dir_canon)
            .map_err(|err| format!("failed to read directory {}: {err}", dir_canon.display()))?;

        for item in reader {
            let item = item.map_err(|err| {
                format!(
                    "failed to read a directory entry in {}: {err}",
                    dir_canon.display()
                )
            })?;
            let path = canonicalize_existing(&item.path())?;
            ensure_within_root(&root_canon, &path)?;

            let metadata = item.metadata().map_err(|err| {
                format!(
                    "failed to read metadata for {}: {err}",
                    item.path().display()
                )
            })?;
            let is_dir = metadata.is_dir();
            let name = item.file_name().to_string_lossy().to_string();
            let byte_len = if is_dir { 0 } else { metadata.len() };
            let last_modified = metadata.modified().ok();

            entries.push(EntryInfo {
                path,
                name,
                is_dir,
                byte_len,
                last_modified,
            });
        }

        entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });

        Ok(entries)
    }

    pub fn stat_entry(&self, root: &Path, path: &Path) -> Result<EntryInfo, String> {
        let root_canon = canonicalize_existing(root)?;
        let resolved = self.resolve_existing_path(&root_canon, path)?;
        ensure_within_root(&root_canon, &resolved)?;

        let metadata = fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat {}: {err}", resolved.display()))?;

        let name = resolved
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| resolved.display().to_string());

        Ok(EntryInfo {
            path: resolved,
            name,
            is_dir: metadata.is_dir(),
            byte_len: if metadata.is_dir() { 0 } else { metadata.len() },
            last_modified: metadata.modified().ok(),
        })
    }

    pub fn create_file(&self, root: &Path, path: &Path) -> Result<(), String> {
        let root_canon = canonicalize_existing(root)?;
        let resolved = self.resolve_new_path(&root_canon, path)?;
        ensure_within_root(&root_canon, &resolved)?;

        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&resolved)
            .map_err(|err| format!("failed to create file {}: {err}", resolved.display()))?;

        Ok(())
    }

    pub fn create_dir(&self, root: &Path, path: &Path) -> Result<(), String> {
        let root_canon = canonicalize_existing(root)?;
        let resolved = self.resolve_new_path(&root_canon, path)?;
        ensure_within_root(&root_canon, &resolved)?;

        fs::create_dir(&resolved)
            .map_err(|err| format!("failed to create directory {}: {err}", resolved.display()))?;

        Ok(())
    }

    pub fn rename(&self, root: &Path, from: &Path, to: &Path) -> Result<(), String> {
        let root_canon = canonicalize_existing(root)?;
        let from_resolved = self.resolve_existing_path(&root_canon, from)?;
        let to_resolved = self.resolve_new_path(&root_canon, to)?;

        ensure_within_root(&root_canon, &from_resolved)?;
        ensure_within_root(&root_canon, &to_resolved)?;

        fs::rename(&from_resolved, &to_resolved).map_err(|err| {
            format!(
                "failed to rename {} to {}: {err}",
                from_resolved.display(),
                to_resolved.display()
            )
        })?;

        Ok(())
    }

    pub fn copy_entry(
        &self,
        root: &Path,
        from: &Path,
        to_directory: &Path,
    ) -> Result<PathBuf, String> {
        let root_canon = canonicalize_existing(root)?;
        let from_resolved = self.resolve_existing_path(&root_canon, from)?;
        let target_directory = self.resolve_existing_path(&root_canon, to_directory)?;

        ensure_within_root(&root_canon, &from_resolved)?;
        ensure_within_root(&root_canon, &target_directory)?;

        if from_resolved == root_canon {
            return Err("refusing to copy workspace root".to_string());
        }

        let target_meta = fs::metadata(&target_directory).map_err(|err| {
            format!(
                "failed to read destination directory metadata {}: {err}",
                target_directory.display()
            )
        })?;
        if !target_meta.is_dir() {
            return Err(format!(
                "destination path is not a directory: {}",
                target_directory.display()
            ));
        }

        let from_meta = fs::metadata(&from_resolved)
            .map_err(|err| format!("failed to stat source {}: {err}", from_resolved.display()))?;
        if from_meta.is_dir() && target_directory.starts_with(&from_resolved) {
            return Err(format!(
                "cannot copy directory {} into its own subtree {}",
                from_resolved.display(),
                target_directory.display()
            ));
        }

        let copied_path =
            file_ops::copy_entry_with_conflict_resolution(&from_resolved, &target_directory)?;
        let copied_canon = canonicalize_existing(&copied_path)?;
        ensure_within_root(&root_canon, &copied_canon)?;

        Ok(copied_canon)
    }

    pub fn move_entry(
        &self,
        root: &Path,
        from: &Path,
        to_directory: &Path,
    ) -> Result<PathBuf, String> {
        let root_canon = canonicalize_existing(root)?;
        let from_resolved = self.resolve_existing_path(&root_canon, from)?;
        let target_directory = self.resolve_existing_path(&root_canon, to_directory)?;

        ensure_within_root(&root_canon, &from_resolved)?;
        ensure_within_root(&root_canon, &target_directory)?;

        if from_resolved == root_canon {
            return Err("refusing to move workspace root".to_string());
        }

        let target_meta = fs::metadata(&target_directory).map_err(|err| {
            format!(
                "failed to read destination directory metadata {}: {err}",
                target_directory.display()
            )
        })?;
        if !target_meta.is_dir() {
            return Err(format!(
                "destination path is not a directory: {}",
                target_directory.display()
            ));
        }

        let from_meta = fs::metadata(&from_resolved)
            .map_err(|err| format!("failed to stat source {}: {err}", from_resolved.display()))?;
        if from_meta.is_dir() && target_directory.starts_with(&from_resolved) {
            return Err(format!(
                "cannot move directory {} into its own subtree {}",
                from_resolved.display(),
                target_directory.display()
            ));
        }

        let moved_path =
            file_ops::move_entry_with_conflict_resolution(&from_resolved, &target_directory)?;
        let moved_canon = canonicalize_existing(&moved_path)?;
        ensure_within_root(&root_canon, &moved_canon)?;

        Ok(moved_canon)
    }

    pub fn import_entry(
        &self,
        root: &Path,
        source: &Path,
        to_directory: &Path,
    ) -> Result<PathBuf, String> {
        let root_canon = canonicalize_existing(root)?;
        let source_canon = canonicalize_existing(source)?;
        let target_directory = self.resolve_existing_path(&root_canon, to_directory)?;

        ensure_within_root(&root_canon, &target_directory)?;

        if source_canon == root_canon {
            return Err("refusing to import workspace root into itself".to_string());
        }

        let source_meta = fs::metadata(&source_canon)
            .map_err(|err| format!("failed to stat source {}: {err}", source_canon.display()))?;
        if source_meta.is_dir() && target_directory.starts_with(&source_canon) {
            return Err(format!(
                "cannot import directory {} into its own subtree {}",
                source_canon.display(),
                target_directory.display()
            ));
        }

        let imported_path =
            file_ops::copy_entry_with_conflict_resolution(&source_canon, &target_directory)?;
        let imported_canon = canonicalize_existing(&imported_path)?;
        ensure_within_root(&root_canon, &imported_canon)?;

        Ok(imported_canon)
    }

    pub fn delete_to_trash(&self, root: &Path, path: &Path) -> Result<(), String> {
        let root_canon = canonicalize_existing(root)?;
        let resolved = self.resolve_existing_path(&root_canon, path)?;

        ensure_within_root(&root_canon, &resolved)?;
        if resolved == root_canon {
            return Err("refusing to delete workspace root".to_string());
        }

        trash::delete(&resolved)
            .map_err(|err| format!("failed to move {} to trash: {err}", resolved.display()))?;

        Ok(())
    }

    fn resolve_existing_path(&self, root: &Path, path: &Path) -> Result<PathBuf, String> {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };
        canonicalize_existing(&candidate)
    }

    fn resolve_new_path(&self, root: &Path, path: &Path) -> Result<PathBuf, String> {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };

        let parent = candidate.parent().ok_or_else(|| {
            format!(
                "cannot resolve parent directory for path {}",
                candidate.display()
            )
        })?;
        let parent_canon = canonicalize_existing(parent)?;
        let file_name = candidate.file_name().ok_or_else(|| {
            format!(
                "cannot resolve file name component for path {}",
                candidate.display()
            )
        })?;

        Ok(parent_canon.join(file_name))
    }
}

fn canonicalize_existing(path: &Path) -> Result<PathBuf, String> {
    dunce::canonicalize(path)
        .map_err(|err| format!("failed to canonicalize path {}: {err}", path.display()))
}

fn ensure_within_root(root: &Path, candidate: &Path) -> Result<(), String> {
    if candidate.starts_with(root) {
        return Ok(());
    }

    Err(format!(
        "path {} is outside workspace root {}",
        candidate.display(),
        root.display()
    ))
}
