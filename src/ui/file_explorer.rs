use std::collections::BTreeSet;
use std::path::PathBuf;

use gpui::{Entity, IntoElement, div, prelude::*};
use gpui_component::button::Button;

use crate::core::TreeNode;

use super::app_view::AppView;

pub fn render_file_explorer(
    view: Entity<AppView>,
    tree_nodes: Vec<TreeNode>,
    selected_path: Option<PathBuf>,
    expanded_paths: BTreeSet<PathBuf>,
) -> impl IntoElement {
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
                            "[-]"
                        } else {
                            "[+]"
                        }
                    } else {
                        "[f]"
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
}
