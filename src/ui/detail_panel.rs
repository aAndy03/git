use gpui::{IntoElement, div, prelude::*};

use crate::fs_adapter::EntryInfo;

pub fn render_detail_panel(
    selected_entry: Option<EntryInfo>,
    selected_entries: Vec<EntryInfo>,
) -> impl IntoElement {
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

    let entries = div()
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
        .child(selected_summary)
        .child(div().mt_2().child(entries))
}
