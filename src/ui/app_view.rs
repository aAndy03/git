use std::path::PathBuf;

use directories::UserDirs;
use gpui::{Context, Entity, IntoElement, Render, Window, black, div, prelude::*, px, rgb, white};
use gpui_component::input::InputState;

use crate::core::{AppCore, TreeNode, WorkspaceState};
use crate::fs_adapter::{EntryInfo, FileSystemAdapter};
use crate::persistence::{PersistedState, Persistence};

use super::detail_panel;
use super::file_explorer;
use super::workspace_controls::{self, WorkspacePickerState};

pub struct AppView {
    core: AppCore,
    fs: FileSystemAdapter,
    persistence: Option<Persistence>,
    status_message: String,
    workspace_picker: Option<WorkspacePickerState>,
    workspace_input: Entity<InputState>,
    create_name_input: Entity<InputState>,
    rename_name_input: Entity<InputState>,
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

        Self {
            core,
            fs,
            persistence,
            status_message,
            workspace_picker: None,
            workspace_input,
            create_name_input,
            rename_name_input,
        }
    }

    pub(crate) fn open_workspace_from_input(&mut self, cx: &mut Context<Self>) {
        let raw_value = self.workspace_input.read(cx).value().to_string();
        let trimmed = raw_value.trim();

        if trimmed.is_empty() {
            self.status_message = "Workspace root path is empty".to_string();
            return;
        }

        self.set_workspace_root(PathBuf::from(trimmed));
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

    fn set_workspace_root(&mut self, path: PathBuf) {
        match self.core.set_workspace_root(path.clone()) {
            Ok(()) => {
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
        let view = cx.entity();
        let (tree_nodes, selected_entry, render_error) = self.tree_and_panel_data();

        let selected_path = self.core.selected_path().cloned();
        let expanded_paths = self.core.expanded_paths().clone();
        let workspace_picker = self.workspace_picker.clone();

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
                            .child(detail_panel::render_detail_panel(selected_entry)),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(px(12.0))
                    .text_color(rgb(0x444444))
                    .child(self.status_message.clone())
                    .when_some(render_error, |this, err| this.child(format!(" | {err}"))),
            )
    }
}
