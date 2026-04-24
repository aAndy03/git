use std::path::PathBuf;
use std::time::Duration;

use directories::UserDirs;
use gpui::{Context, Entity, IntoElement, Render, Window, black, div, prelude::*, px, rgb, white};
use gpui_component::input::InputState;

use crate::core::{AppCore, RefreshSource, TreeNode, WorkspaceState};
use crate::fs_adapter::{EntryInfo, FileSystemAdapter};
use crate::persistence::{PersistedState, Persistence};
use crate::services::watcher::WorkspaceWatcherService;

use super::detail_panel;
use super::file_action_panel::{self, ImportPathPickerEntry, ImportPathPickerState};
use super::file_explorer;
use super::workspace_controls::{self, WorkspacePickerState};

pub struct AppView {
    core: AppCore,
    fs: FileSystemAdapter,
    persistence: Option<Persistence>,
    watcher_service: Option<WorkspaceWatcherService>,
    status_message: String,
    workspace_picker: Option<WorkspacePickerState>,
    import_picker: Option<ImportPathPickerState>,
    picked_import_source: Option<PathBuf>,
    workspace_input: Entity<InputState>,
    create_name_input: Entity<InputState>,
    rename_name_input: Entity<InputState>,
    copy_target_input: Entity<InputState>,
    move_target_input: Entity<InputState>,
    import_source_input: Entity<InputState>,
    import_target_input: Entity<InputState>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fs = FileSystemAdapter::new();
        let mut core = AppCore::default();
        let mut status_message = String::new();

        let persistence = match Persistence::new() {
            Ok(persistence) => Some(persistence),
            Err(err) => {
                status_message = format!("Persistence disabled: {err}");
                None
            }
        };

        if let Some(persistence) = persistence.as_ref() {
            match persistence.load_state() {
                Ok(saved) => {
                    let workspace_state = WorkspaceState {
                        workspace_root: saved.workspace_root,
                        expanded_paths: saved.expanded_paths,
                        selected_path: saved.selected_path,
                    };

                    if let Err(err) = core.command_restore_session(&fs, workspace_state) {
                        status_message = format!("Could not restore workspace root: {err}");
                    }
                }
                Err(err) => {
                    status_message = format!("Could not load persisted state: {err}");
                }
            }
        }

        let workspace_default = core
            .workspace_root()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default();

        let workspace_input = cx.new(|cx| {
            let mut state =
                InputState::new(window, cx).placeholder("C:\\Users\\name\\Documents\\workspace");
            if !workspace_default.is_empty() {
                state = state.default_value(workspace_default.clone());
            }
            state
        });

        let create_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("new entry name (file or folder)"));
        let rename_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("rename to"));
        let copy_target_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("copy target directory path");
            if !workspace_default.is_empty() {
                state = state.default_value(workspace_default.clone());
            }
            state
        });
        let move_target_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("move target directory path");
            if !workspace_default.is_empty() {
                state = state.default_value(workspace_default.clone());
            }
            state
        });
        let import_source_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("import source path (file or folder)")
        });
        let import_target_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("import target directory path");
            if !workspace_default.is_empty() {
                state = state.default_value(workspace_default.clone());
            }
            state
        });

        let mut this = Self {
            core,
            fs,
            persistence,
            watcher_service: None,
            status_message,
            workspace_picker: None,
            import_picker: None,
            picked_import_source: None,
            workspace_input,
            create_name_input,
            rename_name_input,
            copy_target_input,
            move_target_input,
            import_source_input,
            import_target_input,
        };
        this.restart_watcher_for_current_workspace();
        this
    }

    pub(crate) fn open_workspace_from_input(&mut self, cx: &mut Context<Self>) {
        let raw_value = self.workspace_input.read(cx).value().to_string();
        let trimmed = raw_value.trim();

        if trimmed.is_empty() {
            self.status_message = "Workspace root path is empty".to_string();
            return;
        }

        self.set_workspace_root(PathBuf::from(trimmed));
        cx.notify();
    }

    pub(crate) fn open_workspace_picker(&mut self) {
        let start = self
            .core
            .workspace_root()
            .cloned()
            .or_else(|| UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("C:\\"));

        self.refresh_workspace_picker(start);
    }

    pub(crate) fn close_workspace_picker(&mut self) {
        self.workspace_picker = None;
    }

    pub(crate) fn picker_go_up(&mut self) {
        let Some(current) = self
            .workspace_picker
            .as_ref()
            .map(|picker| picker.current_dir.clone())
        else {
            return;
        };

        let Some(parent) = current.parent() else {
            return;
        };

        self.refresh_workspace_picker(parent.to_path_buf());
    }

    pub(crate) fn picker_open_child(&mut self, path: PathBuf) {
        self.refresh_workspace_picker(path);
    }

    pub(crate) fn picker_select_current(&mut self) {
        let Some(current) = self
            .workspace_picker
            .as_ref()
            .map(|picker| picker.current_dir.clone())
        else {
            return;
        };

        self.set_workspace_root(current);
        self.workspace_picker = None;
    }

    pub(crate) fn manual_refresh_workspace(&mut self, cx: &mut Context<Self>) {
        match self
            .core
            .command_apply_refresh(&self.fs, RefreshSource::Manual, 0)
        {
            Ok(()) => {
                self.status_message = "Workspace manually refreshed".to_string();
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Manual refresh failed: {err}");
            }
        }
        cx.notify();
    }

    pub(crate) fn on_tree_entry_clicked(
        &mut self,
        path: PathBuf,
        is_dir: bool,
        cx: &mut Context<Self>,
    ) {
        self.core.command_select_path(path.clone());
        if is_dir {
            self.core.command_toggle_expanded(&path);
        }
        self.persist_state();
        cx.notify();
    }

    pub(crate) fn create_folder_from_input(&mut self, cx: &mut Context<Self>) {
        let name = self.create_name_input.read(cx).value().to_string();

        match self.core.create_directory(&self.fs, name.as_str()) {
            Ok(path) => {
                self.status_message = format!("Created folder {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to create folder: {err}");
            }
        }
    }

    pub(crate) fn create_file_from_input(&mut self, cx: &mut Context<Self>) {
        let name = self.create_name_input.read(cx).value().to_string();

        match self.core.create_file(&self.fs, name.as_str()) {
            Ok(path) => {
                self.status_message = format!("Created file {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to create file: {err}");
            }
        }
    }

    pub(crate) fn rename_selected_from_input(&mut self, cx: &mut Context<Self>) {
        let name = self.rename_name_input.read(cx).value().to_string();

        match self.core.rename_selected(&self.fs, name.as_str()) {
            Ok(path) => {
                self.status_message = format!("Renamed selection to {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to rename selection: {err}");
            }
        }
    }

    pub(crate) fn delete_selected(&mut self) {
        match self.core.delete_selected(&self.fs) {
            Ok(path) => {
                self.status_message =
                    format!("Deleted selection, now focused on {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to delete selection: {err}");
            }
        }
    }

    pub(crate) fn copy_selected_from_input(&mut self, cx: &mut Context<Self>) {
        let raw_target = self.copy_target_input.read(cx).value().to_string();
        let trimmed_target = raw_target.trim();

        if trimmed_target.is_empty() {
            self.status_message = "Copy target directory path is empty".to_string();
            return;
        }

        match self
            .core
            .copy_selected_to(&self.fs, PathBuf::from(trimmed_target))
        {
            Ok(path) => {
                self.status_message = format!("Copied selection to {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to copy selection: {err}");
            }
        }
    }

    pub(crate) fn move_selected_from_input(&mut self, cx: &mut Context<Self>) {
        let raw_target = self.move_target_input.read(cx).value().to_string();
        let trimmed_target = raw_target.trim();

        if trimmed_target.is_empty() {
            self.status_message = "Move target directory path is empty".to_string();
            return;
        }

        match self
            .core
            .move_selected_to(&self.fs, PathBuf::from(trimmed_target))
        {
            Ok(path) => {
                self.status_message = format!("Moved selection to {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to move selection: {err}");
            }
        }
    }

    pub(crate) fn import_from_inputs(&mut self, cx: &mut Context<Self>) {
        let source = self.picked_import_source.clone().or_else(|| {
            let raw = self.import_source_input.read(cx).value().to_string();
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        });

        let Some(source) = source else {
            self.status_message =
                "Import source path is empty, and no source was picked".to_string();
            return;
        };

        let raw_target = self.import_target_input.read(cx).value().to_string();
        let trimmed_target = raw_target.trim();

        if trimmed_target.is_empty() {
            self.status_message = "Import target directory path is empty".to_string();
            return;
        }

        match self.core.import_entry_into_workspace(
            &self.fs,
            source.clone(),
            PathBuf::from(trimmed_target),
        ) {
            Ok(path) => {
                self.status_message = format!(
                    "Imported {} into workspace at {}",
                    source.display(),
                    path.display()
                );
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to import source: {err}");
            }
        }
    }

    pub(crate) fn open_import_picker(&mut self, cx: &mut Context<Self>) {
        let start = self
            .picked_import_source
            .as_ref()
            .map(|path| {
                if path.is_dir() {
                    path.clone()
                } else {
                    path.parent()
                        .map(|parent| parent.to_path_buf())
                        .unwrap_or_else(|| path.clone())
                }
            })
            .or_else(|| {
                let raw = self.import_source_input.read(cx).value().to_string();
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(trimmed))
                }
            })
            .and_then(|candidate| {
                if candidate.is_dir() {
                    Some(candidate)
                } else {
                    candidate.parent().map(|parent| parent.to_path_buf())
                }
            })
            .or_else(|| UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("C:\\"));

        self.refresh_import_picker(start);
    }

    pub(crate) fn close_import_picker(&mut self) {
        self.import_picker = None;
    }

    pub(crate) fn import_picker_go_up(&mut self, _cx: &mut Context<Self>) {
        let Some(current) = self
            .import_picker
            .as_ref()
            .map(|picker| picker.current_dir.clone())
        else {
            return;
        };

        let Some(parent) = current.parent() else {
            return;
        };

        self.refresh_import_picker(parent.to_path_buf());
    }

    pub(crate) fn import_picker_open_child(&mut self, path: PathBuf, _cx: &mut Context<Self>) {
        self.refresh_import_picker(path);
    }

    pub(crate) fn import_picker_use_current_folder(&mut self, _cx: &mut Context<Self>) {
        let Some(current) = self
            .import_picker
            .as_ref()
            .map(|picker| picker.current_dir.clone())
        else {
            return;
        };

        self.picked_import_source = Some(current);
        self.import_picker = None;
    }

    pub(crate) fn import_picker_select_entry(&mut self, path: PathBuf) {
        self.picked_import_source = Some(path);
        self.import_picker = None;
    }

    fn set_workspace_root(&mut self, path: PathBuf) {
        match self.core.set_workspace_root(path.clone()) {
            Ok(()) => {
                self.restart_watcher_for_current_workspace();
                self.status_message = format!("Workspace root opened: {}", path.display());
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to open workspace root: {err}");
            }
        }
    }

    fn refresh_workspace_picker(&mut self, directory: PathBuf) {
        match self.fs.list_child_directories(&directory) {
            Ok(child_directories) => {
                self.workspace_picker = Some(WorkspacePickerState {
                    current_dir: directory,
                    child_directories,
                });
            }
            Err(err) => {
                self.status_message = format!("Failed to browse workspace folders: {err}");
            }
        }
    }

    fn refresh_import_picker(&mut self, directory: PathBuf) {
        match self.fs.list_picker_entries(&directory) {
            Ok(entries) => {
                let entries = entries
                    .into_iter()
                    .map(|entry| ImportPathPickerEntry {
                        path: entry.path,
                        name: entry.name,
                        is_dir: entry.is_dir,
                    })
                    .collect();

                self.import_picker = Some(ImportPathPickerState {
                    current_dir: directory,
                    entries,
                });
            }
            Err(err) => {
                self.status_message = format!("Failed to browse import source paths: {err}");
            }
        }
    }

    fn persist_state(&mut self) {
        let Some(persistence) = self.persistence.as_ref() else {
            return;
        };

        let workspace_state = self.core.workspace_state();

        let state = PersistedState {
            workspace_root: workspace_state.workspace_root.clone(),
            expanded_paths: workspace_state.expanded_paths.clone(),
            selected_path: workspace_state.selected_path.clone(),
        };

        if let Err(err) = persistence.save_state(&state) {
            self.status_message = format!("Failed to persist UI state: {err}");
        }
    }

    fn restart_watcher_for_current_workspace(&mut self) {
        self.watcher_service = None;

        let Some(root) = self.core.workspace_root().cloned() else {
            self.core.command_set_watcher_active(false);
            return;
        };

        match WorkspaceWatcherService::start(&root, Duration::from_millis(350)) {
            Ok(service) => {
                self.core.command_set_watcher_active(true);
                self.watcher_service = Some(service);
            }
            Err(err) => {
                self.core.command_set_watcher_active(false);
                self.status_message = format!("Watcher disabled: {err}");
            }
        }
    }

    fn process_watcher_updates(&mut self) {
        let Some(watcher_service) = self.watcher_service.as_mut() else {
            self.core.command_set_watcher_active(false);
            return;
        };

        let mut refreshed = false;

        while let Some(event) = watcher_service.poll_refresh_event() {
            match self.core.command_apply_refresh(
                &self.fs,
                RefreshSource::Watcher,
                event.event_count,
            ) {
                Ok(()) => {
                    refreshed = true;
                }
                Err(err) => {
                    self.status_message = format!("Watcher refresh failed: {err}");
                }
            }
        }

        if refreshed {
            self.status_message = "Workspace refreshed from filesystem changes".to_string();
            self.persist_state();
        }
    }

    fn tree_and_panel_data(&self) -> (Vec<TreeNode>, Option<EntryInfo>, Option<String>) {
        let mut render_error = None;

        let tree_nodes = match self.core.visible_tree(&self.fs) {
            Ok(nodes) => nodes,
            Err(err) => {
                render_error = Some(format!("Tree load error: {err}"));
                Vec::new()
            }
        };

        let selected = match self.core.selected_entry(&self.fs) {
            Ok(entry) => entry,
            Err(err) => {
                render_error = Some(format!("Selection stat error: {err}"));
                None
            }
        };

        (tree_nodes, selected, render_error)
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.process_watcher_updates();

        let view = cx.entity();
        let (tree_nodes, selected_entry, render_error) = self.tree_and_panel_data();

        let selected_path = self.core.selected_path().cloned();
        let expanded_paths = self.core.expanded_paths().clone();
        let workspace_picker = self.workspace_picker.clone();
        let import_picker = self.import_picker.clone();
        let picked_import_source = self.picked_import_source.clone();
        let watcher_status = self.core.watcher_status_line();

        div()
            .size_full()
            .bg(white())
            .text_color(black())
            .p_2()
            .child(workspace_controls::render_workspace_controls(
                view.clone(),
                self.workspace_input.clone(),
                self.create_name_input.clone(),
                self.rename_name_input.clone(),
            ))
            .when_some(workspace_picker, |this, picker| {
                this.child(workspace_controls::render_workspace_picker(
                    view.clone(),
                    picker,
                ))
            })
            .child(
                div()
                    .mt_2()
                    .flex()
                    .flex_row()
                    .size_full()
                    .child(
                        div()
                            .w(px(460.0))
                            .h_full()
                            .border_1()
                            .border_color(rgb(0xd0d0d0))
                            .p_2()
                            .child("Explorer")
                            .child(file_explorer::render_file_explorer(
                                view.clone(),
                                tree_nodes,
                                selected_path,
                                expanded_paths,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .border_1()
                            .border_color(rgb(0xd0d0d0))
                            .p_2()
                            .child("Details")
                            .child(detail_panel::render_detail_panel(
                                selected_entry.clone(),
                                watcher_status.clone(),
                            ))
                            .child(file_action_panel::render_file_action_panel(
                                view.clone(),
                                self.copy_target_input.clone(),
                                self.move_target_input.clone(),
                                self.import_source_input.clone(),
                                self.import_target_input.clone(),
                                import_picker,
                                picked_import_source,
                            )),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(px(12.0))
                    .text_color(rgb(0x444444))
                    .child(self.status_message.clone())
                    .child(format!(" | {watcher_status}"))
                    .when_some(render_error, |this, err| this.child(format!(" | {err}"))),
            )
    }
}
