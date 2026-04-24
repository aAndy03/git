use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub byte_len: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileSystemAdapter;

impl FileSystemAdapter {
    pub fn new() -> Self {
        Self
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

            entries.push(EntryInfo {
                path,
                name,
                is_dir,
                byte_len,
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
