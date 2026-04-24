use std::path::PathBuf;

use gpui::{Entity, IntoElement, div, prelude::*};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};

use super::app_view::AppView;

#[derive(Debug, Clone)]
pub struct WorkspacePickerState {
    pub current_dir: PathBuf,
    pub child_directories: Vec<PathBuf>,
}

pub fn render_workspace_controls(
    view: Entity<AppView>,
    workspace_input: Entity<InputState>,
    create_name_input: Entity<InputState>,
    rename_name_input: Entity<InputState>,
) -> impl IntoElement {
    let open_view = view.clone();
    let browse_view = view.clone();

    let create_folder_view = view.clone();
    let create_file_view = view.clone();

    let rename_view = view.clone();
    let delete_view = view.clone();

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .flex_row()
                .w_full()
                .child(div().flex_1().mr_2().child(Input::new(&workspace_input)))
                .child(
                    Button::new("open-workspace-root")
                        .label("Open Workspace Root")
                        .on_click(move |_, _, cx| {
                            let _ = open_view.update(cx, |this, cx| {
                                this.open_workspace_from_input(cx);
                                cx.notify();
                            });
                        }),
                )
                .child(
                    Button::new("browse-workspace-root")
                        .label("Browse Folder")
                        .on_click(move |_, _, cx| {
                            let _ = browse_view.update(cx, |this, cx| {
                                this.open_workspace_picker();
                                cx.notify();
                            });
                        }),
                ),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .flex_row()
                .w_full()
                .child(div().flex_1().mr_2().child(Input::new(&create_name_input)))
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
                ),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .flex_row()
                .w_full()
                .child(div().flex_1().mr_2().child(Input::new(&rename_name_input)))
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
                ),
        )
}

pub fn render_workspace_picker(
    view: Entity<AppView>,
    picker_state: WorkspacePickerState,
) -> impl IntoElement {
    let go_up_view = view.clone();
    let use_this_view = view.clone();
    let close_view = view.clone();

    div()
        .mt_2()
        .border_1()
        .p_2()
        .child("Workspace Folder Picker")
        .child(format!("Current: {}", picker_state.current_dir.display()))
        .child(
            div()
                .mt_1()
                .flex()
                .flex_row()
                .child(
                    Button::new("picker-go-up")
                        .label("Up")
                        .on_click(move |_, _, cx| {
                            let _ = go_up_view.update(cx, |this, cx| {
                                this.picker_go_up();
                                cx.notify();
                            });
                        }),
                )
                .child(
                    Button::new("picker-use-current")
                        .label("Use This Folder")
                        .on_click(move |_, _, cx| {
                            let _ = use_this_view.update(cx, |this, cx| {
                                this.picker_select_current();
                                cx.notify();
                            });
                        }),
                )
                .child(
                    Button::new("picker-close")
                        .label("Close")
                        .on_click(move |_, _, cx| {
                            let _ = close_view.update(cx, |this, cx| {
                                this.close_workspace_picker();
                                cx.notify();
                            });
                        }),
                ),
        )
        .child(
            div().mt_2().flex().flex_col().children(
                picker_state.child_directories.into_iter().enumerate().map(
                    move |(index, child)| {
                        let open_view = view.clone();
                        let open_path = child.clone();
                        let label = child
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| child.display().to_string());

                        Button::new(("picker-child", index))
                            .label(format!("Open {label}"))
                            .on_click(move |_, _, cx| {
                                let open_path = open_path.clone();
                                let _ = open_view.update(cx, |this, cx| {
                                    this.picker_open_child(open_path);
                                    cx.notify();
                                });
                            })
                    },
                ),
            ),
        )
}
