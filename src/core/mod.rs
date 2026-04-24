use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::fs_adapter::{EntryInfo, FileSystemAdapter};

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
}

#[derive(Debug, Default, Clone)]
pub struct AppCore {
    workspace_root: Option<PathBuf>,
    expanded_paths: BTreeSet<PathBuf>,
    selected_path: Option<PathBuf>,
}

impl AppCore {
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.workspace_root.as_ref()
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.selected_path.as_ref()
    }

    pub fn expanded_paths(&self) -> &BTreeSet<PathBuf> {
        &self.expanded_paths
    }

    pub fn set_workspace_root(&mut self, path: PathBuf) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("workspace path does not exist: {}", path.display()));
        }
        if !path.is_dir() {
            return Err(format!(
                "workspace path is not a directory: {}",
                path.display()
            ));
        }

        let root = dunce::canonicalize(&path).map_err(|err| {
            format!(
                "failed to canonicalize workspace root {}: {err}",
                path.display()
            )
        })?;

        self.workspace_root = Some(root.clone());
        self.selected_path = Some(root.clone());
        self.expanded_paths.clear();
        self.expanded_paths.insert(root);

        Ok(())
    }

    pub fn replace_expanded_paths(&mut self, expanded_paths: BTreeSet<PathBuf>) {
        self.expanded_paths = expanded_paths;
        if let Some(root) = self.workspace_root.as_ref() {
            self.expanded_paths.insert(root.clone());
        }
    }

    pub fn restore_selected_path(
        &mut self,
        fs: &FileSystemAdapter,
        selected_path: Option<PathBuf>,
    ) {
        let Some(root) = self.workspace_root.as_ref().cloned() else {
            return;
        };

        if let Some(path) = selected_path {
            if let Ok(entry) = fs.stat_entry(&root, &path) {
                self.selected_path = Some(entry.path);
                return;
            }
        }

        self.selected_path = Some(root);
    }

    pub fn select_path(&mut self, path: PathBuf) {
        self.selected_path = Some(path);
    }

    pub fn toggle_expanded(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if self.expanded_paths.contains(&path) {
            self.expanded_paths.remove(&path);
        } else {
            self.expanded_paths.insert(path);
        }
    }

    pub fn visible_tree(&self, fs: &FileSystemAdapter) -> Result<Vec<TreeNode>, String> {
        let Some(root) = self.workspace_root.as_ref() else {
            return Ok(Vec::new());
        };

        let root_name = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string());

        let mut nodes = vec![TreeNode {
            path: root.clone(),
            name: root_name,
            depth: 0,
            is_dir: true,
        }];

        self.collect_dir_children(fs, root, 0, &mut nodes)?;
        Ok(nodes)
    }

    pub fn create_directory(
        &mut self,
        fs: &FileSystemAdapter,
        name: &str,
    ) -> Result<PathBuf, String> {
        let root = self.required_root()?;
        let parent = self.insertion_parent(fs)?;
        validate_entry_name(name)?;

        let target = parent.join(name);
        fs.create_dir(root, &target)?;
        self.expanded_paths.insert(parent.clone());
        self.selected_path = Some(target.clone());

        Ok(target)
    }

    pub fn create_file(&mut self, fs: &FileSystemAdapter, name: &str) -> Result<PathBuf, String> {
        let root = self.required_root()?;
        let parent = self.insertion_parent(fs)?;
        validate_entry_name(name)?;

        let target = parent.join(name);
        fs.create_file(root, &target)?;
        self.expanded_paths.insert(parent);
        self.selected_path = Some(target.clone());

        Ok(target)
    }

    pub fn rename_selected(
        &mut self,
        fs: &FileSystemAdapter,
        new_name: &str,
    ) -> Result<PathBuf, String> {
        validate_entry_name(new_name)?;
        let root = self.required_root()?.to_path_buf();
        let selected = self
            .selected_path
            .clone()
            .ok_or_else(|| "nothing selected for rename".to_string())?;

        if selected == root {
            return Err("cannot rename workspace root".to_string());
        }

        let parent = selected
            .parent()
            .ok_or_else(|| format!("selected path has no parent: {}", selected.display()))?
            .to_path_buf();
        let target = parent.join(new_name);

        fs.rename(&root, &selected, &target)?;

        if self.expanded_paths.contains(&selected) {
            self.expanded_paths.remove(&selected);
            self.expanded_paths.insert(target.clone());
        }
        self.selected_path = Some(target.clone());

        Ok(target)
    }

    pub fn delete_selected(&mut self, fs: &FileSystemAdapter) -> Result<PathBuf, String> {
        let root = self.required_root()?.to_path_buf();
        let selected = self
            .selected_path
            .clone()
            .ok_or_else(|| "nothing selected for delete".to_string())?;

        if selected == root {
            return Err("cannot delete workspace root".to_string());
        }

        fs.delete_to_trash(&root, &selected)?;

        self.expanded_paths
            .retain(|candidate| !candidate.starts_with(&selected));

        let fallback = selected
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or(root);
        self.selected_path = Some(fallback.clone());

        Ok(fallback)
    }

    pub fn selected_directory_entries(
        &self,
        fs: &FileSystemAdapter,
    ) -> Result<Vec<EntryInfo>, String> {
        let root = self.required_root()?;
        let Some(selected) = self.selected_path.as_ref() else {
            return Ok(Vec::new());
        };

        let stat = fs.stat_entry(root, selected)?;
        if stat.is_dir {
            fs.list_dir(root, selected)
        } else {
            let parent = selected
                .parent()
                .ok_or_else(|| format!("selected file has no parent: {}", selected.display()))?;
            fs.list_dir(root, parent)
        }
    }

    pub fn selected_entry(&self, fs: &FileSystemAdapter) -> Result<Option<EntryInfo>, String> {
        let root = self.required_root()?;
        let Some(selected) = self.selected_path.as_ref() else {
            return Ok(None);
        };

        fs.stat_entry(root, selected).map(Some)
    }

    fn required_root(&self) -> Result<&PathBuf, String> {
        self.workspace_root
            .as_ref()
            .ok_or_else(|| "workspace root not set".to_string())
    }

    fn insertion_parent(&self, fs: &FileSystemAdapter) -> Result<PathBuf, String> {
        let root = self.required_root()?;

        if let Some(selected) = self.selected_path.as_ref() {
            let stat = fs.stat_entry(root, selected)?;
            if stat.is_dir {
                return Ok(selected.clone());
            }

            if let Some(parent) = selected.parent() {
                return Ok(parent.to_path_buf());
            }
        }

        Ok(root.clone())
    }

    fn collect_dir_children(
        &self,
        fs: &FileSystemAdapter,
        directory: &Path,
        depth: usize,
        nodes: &mut Vec<TreeNode>,
    ) -> Result<(), String> {
        let Some(root) = self.workspace_root.as_ref() else {
            return Ok(());
        };

        if !self.expanded_paths.contains(directory) {
            return Ok(());
        }

        let children = fs.list_dir(root, directory)?;

        for child in children {
            nodes.push(TreeNode {
                path: child.path.clone(),
                name: child.name.clone(),
                depth: depth + 1,
                is_dir: child.is_dir,
            });

            if child.is_dir {
                self.collect_dir_children(fs, &child.path, depth + 1, nodes)?;
            }
        }

        Ok(())
    }
}

fn validate_entry_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("name cannot be empty".to_string());
    }

    if trimmed.ends_with(' ') || trimmed.ends_with('.') {
        return Err("name cannot end with a space or period on Windows".to_string());
    }

    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || invalid_chars.contains(&ch))
    {
        return Err("name contains invalid characters".to_string());
    }

    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .to_ascii_uppercase();
    if reserved.contains(&stem.as_str()) {
        return Err("name is a reserved device identifier on Windows".to_string());
    }

    Ok(())
}
