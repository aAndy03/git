use std::path::PathBuf;

use gpui::{Entity, IntoElement, div, prelude::*};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};

use super::app_view::AppView;

#[derive(Debug, Clone)]
pub struct ImportPathPickerEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct ImportPathPickerState {
    pub current_dir: PathBuf,
    pub entries: Vec<ImportPathPickerEntry>,
}

pub fn render_file_action_panel(
    view: Entity<AppView>,
    copy_target_input: Entity<InputState>,
    move_target_input: Entity<InputState>,
    import_source_input: Entity<InputState>,
    import_target_input: Entity<InputState>,
    import_picker: Option<ImportPathPickerState>,
    picked_import_source: Option<PathBuf>,
) -> impl IntoElement {
    let copy_view = view.clone();
    let move_view = view.clone();
    let browse_import_view = view.clone();
    let run_import_view = view.clone();

    div()
        .mt_2()
        .pt_2()
        .border_t_1()
        .child("File Actions")
        .child(
            div()
                .mt_1()
                .flex()
                .flex_row()
                .w_full()
                .child(div().flex_1().mr_2().child(Input::new(&copy_target_input)))
                .child(
                    Button::new("copy-selected")
                        .label("Copy Selected")
                        .on_click(move |_, _, cx| {
                            let _ = copy_view.update(cx, |this, cx| {
                                this.copy_selected_from_input(cx);
                                cx.notify();
                            });
                        }),
                ),
        )
        .child(
            div()
                .mt_1()
                .flex()
                .flex_row()
                .w_full()
                .child(div().flex_1().mr_2().child(Input::new(&move_target_input)))
                .child(
                    Button::new("move-selected")
                        .label("Move Selected")
                        .on_click(move |_, _, cx| {
                            let _ = move_view.update(cx, |this, cx| {
                                this.move_selected_from_input(cx);
                                cx.notify();
                            });
                        }),
                ),
        )
        .child(
            div()
                .mt_1()
                .flex()
                .flex_row()
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .mr_2()
                        .child(Input::new(&import_source_input)),
                )
                .child(
                    Button::new("browse-import-source")
                        .label("Browse Import Source")
                        .on_click(move |_, _, cx| {
                            let _ = browse_import_view.update(cx, |this, cx| {
                                this.open_import_picker(cx);
                                cx.notify();
                            });
                        }),
                ),
        )
        .child(
            div()
                .mt_1()
                .flex()
                .flex_row()
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .mr_2()
                        .child(Input::new(&import_target_input)),
                )
                .child(
                    Button::new("import-into-workspace")
                        .label("Import Into Workspace")
                        .on_click(move |_, _, cx| {
                            let _ = run_import_view.update(cx, |this, cx| {
                                this.import_from_inputs(cx);
                                cx.notify();
                            });
                        }),
                ),
        )
        .when_some(picked_import_source, |this, picked| {
            this.child(format!("Picked import source: {}", picked.display()))
        })
        .when_some(import_picker, |this, picker| {
            this.child(render_import_source_picker(view.clone(), picker))
        })
}

fn render_import_source_picker(
    view: Entity<AppView>,
    picker: ImportPathPickerState,
) -> impl IntoElement {
    let up_view = view.clone();
    let use_current_view = view.clone();
    let close_view = view.clone();

    div()
        .mt_2()
        .border_1()
        .p_2()
        .child("Import Source Picker")
        .child(format!("Current: {}", picker.current_dir.display()))
        .child(
            div()
                .mt_1()
                .flex()
                .flex_row()
                .child(
                    Button::new("import-picker-up")
                        .label("Up")
                        .on_click(move |_, _, cx| {
                            let _ = up_view.update(cx, |this, cx| {
                                this.import_picker_go_up(cx);
                                cx.notify();
                            });
                        }),
                )
                .child(
                    Button::new("import-picker-use-current")
                        .label("Pick Current Folder")
                        .on_click(move |_, _, cx| {
                            let _ = use_current_view.update(cx, |this, cx| {
                                this.import_picker_use_current_folder(cx);
                                cx.notify();
                            });
                        }),
                )
                .child(Button::new("import-picker-close").label("Close").on_click(
                    move |_, _, cx| {
                        let _ = close_view.update(cx, |this, cx| {
                            this.close_import_picker();
                            cx.notify();
                        });
                    },
                )),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .flex_col()
                .children(
                    picker
                        .entries
                        .into_iter()
                        .enumerate()
                        .map(move |(index, entry)| {
                            let entry_view = view.clone();
                            let entry_path = entry.path.clone();

                            if entry.is_dir {
                                Button::new(("import-picker-open-dir", index))
                                    .label(format!("Open Folder {}", entry.name))
                                    .on_click(move |_, _, cx| {
                                        let entry_path = entry_path.clone();
                                        let _ = entry_view.update(cx, |this, cx| {
                                            this.import_picker_open_child(entry_path, cx);
                                            cx.notify();
                                        });
                                    })
                            } else {
                                Button::new(("import-picker-pick-file", index))
                                    .label(format!("Pick File {}", entry.name))
                                    .on_click(move |_, _, cx| {
                                        let entry_path = entry_path.clone();
                                        let _ = entry_view.update(cx, |this, cx| {
                                            this.import_picker_select_entry(entry_path);
                                            cx.notify();
                                        });
                                    })
                            }
                        }),
                ),
        )
}
