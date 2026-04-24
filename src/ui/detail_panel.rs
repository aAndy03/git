use std::time::UNIX_EPOCH;

use gpui::{IntoElement, div, prelude::*};

use crate::fs_adapter::EntryInfo;

pub fn render_detail_panel(selected_entry: Option<EntryInfo>) -> impl IntoElement {
    match selected_entry {
        Some(entry) => {
            let kind = if entry.is_dir { "directory" } else { "file" };
            let is_folder = if entry.is_dir { "yes" } else { "no" };
            let last_modified = format_last_modified(entry.last_modified);

            div()
                .flex()
                .flex_col()
                .child(format!("Path: {}", entry.path.display()))
                .child(format!("Kind: {kind}"))
                .child(format!("Size (bytes): {}", entry.byte_len))
                .child(format!("Last Modified: {last_modified}"))
                .child(format!("Is Folder: {is_folder}"))
        }
        None => div().child("No selected entry"),
    }
}

fn format_last_modified(last_modified: Option<std::time::SystemTime>) -> String {
    let Some(last_modified) = last_modified else {
        return "unknown".to_string();
    };

    match last_modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}s since Unix epoch", duration.as_secs()),
        Err(_) => "before Unix epoch".to_string(),
    }
}
