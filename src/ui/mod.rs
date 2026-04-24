use std::path::PathBuf;

use gpui::{Context, Entity, IntoElement, Render, Window, black, div, prelude::*, px, rgb, white};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};

use crate::core::{AppCore, TreeNode};
use crate::fs_adapter::{EntryInfo, FileSystemAdapter};
use crate::persistence::{PersistedState, Persistence};

pub struct AppView {
    core: AppCore,
    fs: FileSystemAdapter,
    persistence: Option<Persistence>,
    status_message: String,
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
                    if let Some(root) = saved.workspace_root {
                        if let Err(err) = core.set_workspace_root(root) {
                            status_message = format!("Could not restore workspace root: {err}");
                        }
                    }
                    core.replace_expanded_paths(saved.expanded_paths);
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
            workspace_input,
            create_name_input,
            rename_name_input,
        }
    }

    fn open_workspace_from_input(&mut self, cx: &mut Context<Self>) {
        let raw_value = self.workspace_input.read(cx).value().to_string();
        let trimmed = raw_value.trim();

        if trimmed.is_empty() {
            self.status_message = "Workspace root path is empty".to_string();
            return;
        }

        match self.core.set_workspace_root(PathBuf::from(trimmed)) {
            Ok(()) => {
                self.status_message = "Workspace root opened".to_string();
                self.persist_state();
            }
            Err(err) => {
                self.status_message = format!("Failed to open workspace root: {err}");
            }
        }
    }

    fn on_tree_entry_clicked(&mut self, path: PathBuf, is_dir: bool, cx: &mut Context<Self>) {
        self.core.select_path(path.clone());
        if is_dir {
            self.core.toggle_expanded(&path);
        }
        self.persist_state();
        cx.notify();
    }

    fn create_folder_from_input(&mut self, cx: &mut Context<Self>) {
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

    fn create_file_from_input(&mut self, cx: &mut Context<Self>) {
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

    fn rename_selected_from_input(&mut self, cx: &mut Context<Self>) {
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

    fn delete_selected(&mut self) {
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

    fn persist_state(&mut self) {
        let Some(persistence) = self.persistence.as_ref() else {
            return;
        };

        let state = PersistedState {
            workspace_root: self.core.workspace_root().cloned(),
            expanded_paths: self.core.expanded_paths().clone(),
        };

        if let Err(err) = persistence.save_state(&state) {
            self.status_message = format!("Failed to persist UI state: {err}");
        }
    }

    fn tree_and_panel_data(
        &self,
    ) -> (
        Vec<TreeNode>,
        Option<EntryInfo>,
        Vec<EntryInfo>,
        Option<String>,
    ) {
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

        let selected_entries = match self.core.selected_directory_entries(&self.fs) {
            Ok(entries) => entries,
            Err(err) => {
                render_error = Some(format!("Directory list error: {err}"));
                Vec::new()
            }
        };

        (tree_nodes, selected, selected_entries, render_error)
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let (tree_nodes, selected_entry, selected_entries, render_error) =
            self.tree_and_panel_data();

        let workspace_controls = {
            let view = view.clone();
            div()
                .flex()
                .flex_row()
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .mr_2()
                        .child(Input::new(&self.workspace_input)),
                )
                .child(
                    Button::new("open-workspace-root")
                        .label("Open Workspace Root")
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.open_workspace_from_input(cx);
                                cx.notify();
                            });
                        }),
                )
        };

        let create_controls = {
            let create_folder_view = view.clone();
            let create_file_view = view.clone();

            div()
                .flex()
                .flex_row()
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .mr_2()
                        .child(Input::new(&self.create_name_input)),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .child(Button::new("create-folder").label("New Folder").on_click(
                            move |_, _, cx| {
                                let _ = create_folder_view.update(cx, |this, cx| {
                                    this.create_folder_from_input(cx);
                                    cx.notify();
                                });
                            },
                        ))
                        .child(Button::new("create-file").label("New File").on_click(
                            move |_, _, cx| {
                                let _ = create_file_view.update(cx, |this, cx| {
                                    this.create_file_from_input(cx);
                                    cx.notify();
                                });
                            },
                        )),
                )
        };

        let rename_delete_controls = {
            let rename_view = view.clone();
            let delete_view = view.clone();

            div()
                .flex()
                .flex_row()
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .mr_2()
                        .child(Input::new(&self.rename_name_input)),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .child(Button::new("rename-selection").label("Rename").on_click(
                            move |_, _, cx| {
                                let _ = rename_view.update(cx, |this, cx| {
                                    this.rename_selected_from_input(cx);
                                    cx.notify();
                                });
                            },
                        ))
                        .child(
                            Button::new("delete-selection")
                                .label("Delete To Trash")
                                .on_click(move |_, _, cx| {
                                    let _ = delete_view.update(cx, |this, cx| {
                                        this.delete_selected();
                                        cx.notify();
                                    });
                                }),
                        ),
                )
        };

        let selected_path = self.core.selected_path().cloned();
        let expanded_paths = self.core.expanded_paths().clone();

        let tree_panel = {
            let view = view.clone();

            div()
                .flex()
                .flex_col()
                .children(
                    tree_nodes
                        .into_iter()
                        .enumerate()
                        .map(move |(index, node)| {
                            let entry_id = ("tree-node", index);
                            let path = node.path.clone();
                            let is_dir = node.is_dir;
                            let is_selected = selected_path
                                .as_ref()
                                .map(|selected| *selected == node.path)
                                .unwrap_or(false);

                            let indent = "  ".repeat(node.depth);
                            let marker = if is_selected { "*" } else { " " };
                            let symbol = if node.is_dir {
                                if expanded_paths.contains(&node.path) {
                                    "▼"
                                } else {
                                    "▶"
                                }
                            } else {
                                "•"
                            };

                            let label = format!("{indent}{marker} {symbol} {}", node.name);

                            let view = view.clone();
                            Button::new(entry_id)
                                .label(label)
                                .on_click(move |_, _, cx| {
                                    let path = path.clone();
                                    let _ = view.update(cx, |this, cx| {
                                        this.on_tree_entry_clicked(path, is_dir, cx);
                                    });
                                })
                        }),
                )
        };

        let selected_summary = match selected_entry {
            Some(entry) => {
                if entry.is_dir {
                    format!("Selected: {} (folder)", entry.path.display())
                } else {
                    format!(
                        "Selected: {} (file, {} bytes)",
                        entry.path.display(),
                        entry.byte_len
                    )
                }
            }
            None => "Selected: none".to_string(),
        };

        let details_panel = div()
            .flex()
            .flex_col()
            .children(selected_entries.into_iter().map(|entry| {
                let entry_kind = if entry.is_dir { "dir" } else { "file" };
                let suffix = if entry.is_dir {
                    String::new()
                } else {
                    format!(", {} bytes", entry.byte_len)
                };
                div().child(format!("- {} ({entry_kind}{suffix})", entry.name))
            }));

        div()
            .size_full()
            .bg(white())
            .text_color(black())
            .p_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(workspace_controls)
                    .child(create_controls)
                    .child(rename_delete_controls)
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
                                    .child(tree_panel),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .border_1()
                                    .border_color(rgb(0xd0d0d0))
                                    .p_2()
                                    .child("Details")
                                    .child(selected_summary)
                                    .child(div().mt_2().child(details_panel)),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_size(px(12.0))
                            .text_color(rgb(0x444444))
                            .child(self.status_message.clone())
                            .when_some(render_error, |this, err| this.child(format!(" | {err}"))),
                    ),
            )
    }
}
