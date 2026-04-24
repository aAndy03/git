use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fs_adapter::{EntryInfo, FileSystemAdapter};

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
}

#[derive(Debug, Default, Clone)]
pub struct WorkspaceState {
    pub workspace_root: Option<PathBuf>,
    pub expanded_paths: BTreeSet<PathBuf>,
    pub selected_path: Option<PathBuf>,
}

#[derive(Debug, Default, Clone)]
pub struct AppCore {
    workspace_state: WorkspaceState,
    refresh_state: RefreshState,
}

#[derive(Debug, Default, Clone)]
pub struct RefreshState {
    pub watcher_active: bool,
    pub last_refresh_at: Option<SystemTime>,
    pub last_refresh_source: Option<RefreshSource>,
    pub last_watcher_event_count: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum RefreshSource {
    Manual,
    Watcher,
}

impl RefreshSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Watcher => "watcher",
        }
    }
}

impl AppCore {
    pub fn workspace_state(&self) -> &WorkspaceState {
        &self.workspace_state
    }

    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.workspace_state.workspace_root.as_ref()
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.workspace_state.selected_path.as_ref()
    }

    pub fn expanded_paths(&self) -> &BTreeSet<PathBuf> {
        &self.workspace_state.expanded_paths
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

        self.workspace_state.workspace_root = Some(root.clone());
        self.workspace_state.selected_path = Some(root.clone());
        self.workspace_state.expanded_paths.clear();
        self.workspace_state.expanded_paths.insert(root);

        Ok(())
    }

    pub fn command_set_watcher_active(&mut self, watcher_active: bool) {
        self.refresh_state.watcher_active = watcher_active;
    }

    pub fn command_apply_refresh(
        &mut self,
        fs: &FileSystemAdapter,
        source: RefreshSource,
        watcher_event_count: u32,
    ) -> Result<(), String> {
        let Some(root) = self.workspace_root().cloned() else {
            return Err("workspace root not set".to_string());
        };

        if fs.stat_entry(&root, &root).is_err() {
            self.workspace_state = WorkspaceState::default();
            self.refresh_state.last_refresh_at = Some(SystemTime::now());
            self.refresh_state.last_refresh_source = Some(source);
            self.refresh_state.last_watcher_event_count = watcher_event_count;
            return Err("workspace root is no longer accessible".to_string());
        }

        let selected_path = self.workspace_state.selected_path.clone();
        self.restore_selected_path(fs, selected_path);

        self.refresh_state.last_refresh_at = Some(SystemTime::now());
        self.refresh_state.last_refresh_source = Some(source);
        self.refresh_state.last_watcher_event_count = watcher_event_count;

        Ok(())
    }

    pub fn watcher_status_line(&self) -> String {
        let watcher_state = if self.refresh_state.watcher_active {
            "watching"
        } else {
            "not watching"
        };

        let refresh_at = self
            .refresh_state
            .last_refresh_at
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| format!("{}s", duration.as_secs()))
            .unwrap_or_else(|| "never".to_string());

        let source = self
            .refresh_state
            .last_refresh_source
            .map(RefreshSource::label)
            .unwrap_or("none");

        format!(
            "Watcher: {watcher_state}, Last Refresh: {refresh_at}, Source: {source}, Watcher Events: {}",
            self.refresh_state.last_watcher_event_count
        )
    }

    pub fn command_restore_session(
        &mut self,
        fs: &FileSystemAdapter,
        workspace_state: WorkspaceState,
    ) -> Result<(), String> {
        let Some(workspace_root) = workspace_state.workspace_root else {
            self.workspace_state = WorkspaceState::default();
            return Ok(());
        };

        self.set_workspace_root(workspace_root)?;
        self.replace_expanded_paths(workspace_state.expanded_paths);
        self.restore_selected_path(fs, workspace_state.selected_path);

        Ok(())
    }

    pub fn replace_expanded_paths(&mut self, expanded_paths: BTreeSet<PathBuf>) {
        self.workspace_state.expanded_paths = expanded_paths;
        if let Some(root) = self.workspace_state.workspace_root.as_ref() {
            self.workspace_state.expanded_paths.insert(root.clone());
        }
    }

    pub fn restore_selected_path(
        &mut self,
        fs: &FileSystemAdapter,
        selected_path: Option<PathBuf>,
    ) {
        let Some(root) = self.workspace_state.workspace_root.as_ref().cloned() else {
            return;
        };

        if let Some(path) = selected_path {
            if let Ok(entry) = fs.stat_entry(&root, &path) {
                self.workspace_state.selected_path = Some(entry.path);
                return;
            }
        }

        self.workspace_state.selected_path = Some(root);
    }

    pub fn command_select_path(&mut self, path: PathBuf) {
        self.workspace_state.selected_path = Some(path);
    }

    pub fn command_toggle_expanded(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if self.workspace_state.expanded_paths.contains(&path) {
            self.workspace_state.expanded_paths.remove(&path);
        } else {
            self.workspace_state.expanded_paths.insert(path);
        }
    }

    pub fn visible_tree(&self, fs: &FileSystemAdapter) -> Result<Vec<TreeNode>, String> {
        let Some(root) = self.workspace_state.workspace_root.as_ref() else {
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
        self.workspace_state.expanded_paths.insert(parent.clone());
        self.workspace_state.selected_path = Some(target.clone());

        Ok(target)
    }

    pub fn create_file(&mut self, fs: &FileSystemAdapter, name: &str) -> Result<PathBuf, String> {
        let root = self.required_root()?;
        let parent = self.insertion_parent(fs)?;
        validate_entry_name(name)?;

        let target = parent.join(name);
        fs.create_file(root, &target)?;
        self.workspace_state.expanded_paths.insert(parent);
        self.workspace_state.selected_path = Some(target.clone());

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
            .workspace_state
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

        if self.workspace_state.expanded_paths.contains(&selected) {
            self.workspace_state.expanded_paths.remove(&selected);
            self.workspace_state.expanded_paths.insert(target.clone());
        }
        self.workspace_state.selected_path = Some(target.clone());

        Ok(target)
    }

    pub fn delete_selected(&mut self, fs: &FileSystemAdapter) -> Result<PathBuf, String> {
        let root = self.required_root()?.to_path_buf();
        let selected = self
            .workspace_state
            .selected_path
            .clone()
            .ok_or_else(|| "nothing selected for delete".to_string())?;

        if selected == root {
            return Err("cannot delete workspace root".to_string());
        }

        fs.delete_to_trash(&root, &selected)?;

        self.workspace_state
            .expanded_paths
            .retain(|candidate| !candidate.starts_with(&selected));

        let fallback = selected
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or(root);
        self.workspace_state.selected_path = Some(fallback.clone());

        Ok(fallback)
    }

    pub fn copy_selected_to(
        &mut self,
        fs: &FileSystemAdapter,
        target: PathBuf,
    ) -> Result<PathBuf, String> {
        let root = self.required_root()?.to_path_buf();
        let selected = self
            .workspace_state
            .selected_path
            .clone()
            .ok_or_else(|| "nothing selected for copy".to_string())?;

        if selected == root {
            return Err("cannot copy workspace root".to_string());
        }

        let target_stat = fs.stat_entry(&root, &target)?;
        if !target_stat.is_dir {
            return Err(format!(
                "copy target is not a directory: {}",
                target_stat.path.display()
            ));
        }

        let copied = fs.copy_entry(&root, &selected, &target_stat.path)?;
        self.workspace_state
            .expanded_paths
            .insert(target_stat.path.clone());
        self.workspace_state.selected_path = Some(copied.clone());

        Ok(copied)
    }

    pub fn move_selected_to(
        &mut self,
        fs: &FileSystemAdapter,
        target: PathBuf,
    ) -> Result<PathBuf, String> {
        let root = self.required_root()?.to_path_buf();
        let selected = self
            .workspace_state
            .selected_path
            .clone()
            .ok_or_else(|| "nothing selected for move".to_string())?;

        if selected == root {
            return Err("cannot move workspace root".to_string());
        }

        let target_stat = fs.stat_entry(&root, &target)?;
        if !target_stat.is_dir {
            return Err(format!(
                "move target is not a directory: {}",
                target_stat.path.display()
            ));
        }

        let moved = fs.move_entry(&root, &selected, &target_stat.path)?;
        self.remap_expanded_prefix(&selected, &moved);
        self.workspace_state
            .expanded_paths
            .insert(target_stat.path.clone());
        self.workspace_state.selected_path = Some(moved.clone());

        Ok(moved)
    }

    pub fn import_entry_into_workspace(
        &mut self,
        fs: &FileSystemAdapter,
        source: PathBuf,
        target: PathBuf,
    ) -> Result<PathBuf, String> {
        let root = self.required_root()?.to_path_buf();

        let target_stat = fs.stat_entry(&root, &target)?;
        if !target_stat.is_dir {
            return Err(format!(
                "import target is not a directory: {}",
                target_stat.path.display()
            ));
        }

        let imported = fs.import_entry(&root, &source, &target_stat.path)?;
        self.workspace_state
            .expanded_paths
            .insert(target_stat.path.clone());
        self.workspace_state.selected_path = Some(imported.clone());

        Ok(imported)
    }

    pub fn selected_entry(&self, fs: &FileSystemAdapter) -> Result<Option<EntryInfo>, String> {
        let root = self.required_root()?;
        let Some(selected) = self.workspace_state.selected_path.as_ref() else {
            return Ok(None);
        };

        fs.stat_entry(root, selected).map(Some)
    }

    fn required_root(&self) -> Result<&PathBuf, String> {
        self.workspace_state
            .workspace_root
            .as_ref()
            .ok_or_else(|| "workspace root not set".to_string())
    }

    fn insertion_parent(&self, fs: &FileSystemAdapter) -> Result<PathBuf, String> {
        let root = self.required_root()?;

        if let Some(selected) = self.workspace_state.selected_path.as_ref() {
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
        let Some(root) = self.workspace_state.workspace_root.as_ref() else {
            return Ok(());
        };

        if !self.workspace_state.expanded_paths.contains(directory) {
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

    fn remap_expanded_prefix(&mut self, old_prefix: &Path, new_prefix: &Path) {
        let mut remapped = BTreeSet::new();

        for path in &self.workspace_state.expanded_paths {
            if let Ok(suffix) = path.strip_prefix(old_prefix) {
                remapped.insert(new_prefix.join(suffix));
            } else {
                remapped.insert(path.clone());
            }
        }

        if let Some(root) = self.workspace_state.workspace_root.as_ref() {
            remapped.insert(root.clone());
        }

        self.workspace_state.expanded_paths = remapped;
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
