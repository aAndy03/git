use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn copy_entry_with_conflict_resolution(
    source: &Path,
    target_directory: &Path,
) -> Result<PathBuf, String> {
    let target = resolve_non_conflicting_target_path(source, target_directory)?;
    copy_path_recursive(source, &target)?;
    Ok(target)
}

pub fn move_entry_with_conflict_resolution(
    source: &Path,
    target_directory: &Path,
) -> Result<PathBuf, String> {
    let target = resolve_non_conflicting_target_path(source, target_directory)?;

    match fs::rename(source, &target) {
        Ok(()) => Ok(target),
        Err(_) => {
            copy_path_recursive(source, &target)?;
            remove_source_entry(source)?;
            Ok(target)
        }
    }
}

fn resolve_non_conflicting_target_path(
    source: &Path,
    target_directory: &Path,
) -> Result<PathBuf, String> {
    let source_name = source.file_name().ok_or_else(|| {
        format!(
            "cannot resolve source file name component for {}",
            source.display()
        )
    })?;

    let source_meta = fs::metadata(source)
        .map_err(|err| format!("failed to stat source {}: {err}", source.display()))?;

    let (base_name, extension) = split_name_for_conflict(source_name, source_meta.is_dir());

    let mut attempt = 0usize;
    loop {
        let candidate_name = format_conflict_name(&base_name, &extension, attempt);
        let candidate = target_directory.join(candidate_name);
        if !candidate.exists() {
            return Ok(candidate);
        }

        attempt += 1;
        if attempt > 10_000 {
            return Err(format!(
                "too many naming conflicts under {}",
                target_directory.display()
            ));
        }
    }
}

fn split_name_for_conflict(source_name: &OsStr, is_dir: bool) -> (String, String) {
    let source_text = source_name.to_string_lossy().to_string();
    if is_dir {
        return (source_text, String::new());
    }

    let source_path = Path::new(&source_text);
    let base_name = source_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or(source_text.clone());
    let extension = source_path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();

    (base_name, extension)
}

fn format_conflict_name(base_name: &str, extension: &str, attempt: usize) -> String {
    match attempt {
        0 => format!("{base_name}{extension}"),
        1 => format!("{base_name} (copy){extension}"),
        _ => format!("{base_name} (copy {attempt}){extension}"),
    }
}

fn copy_path_recursive(source: &Path, target: &Path) -> Result<(), String> {
    let source_meta = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to stat source {}: {err}", source.display()))?;

    if source_meta.file_type().is_symlink() {
        return Err(format!(
            "symbolic links are not supported for copy: {}",
            source.display()
        ));
    }

    if source_meta.is_file() {
        fs::copy(source, target).map_err(|err| {
            format!(
                "failed to copy file {} to {}: {err}",
                source.display(),
                target.display()
            )
        })?;
        return Ok(());
    }

    if !source_meta.is_dir() {
        return Err(format!(
            "unsupported source type for copy: {}",
            source.display()
        ));
    }

    fs::create_dir(target)
        .map_err(|err| format!("failed to create directory {}: {err}", target.display()))?;

    for entry in WalkDir::new(source).min_depth(1) {
        let entry = entry.map_err(|err| {
            format!(
                "failed while walking source directory {}: {err}",
                source.display()
            )
        })?;

        let source_entry = entry.path();
        let relative = source_entry.strip_prefix(source).map_err(|err| {
            format!(
                "failed to resolve relative path for {} from {}: {err}",
                source_entry.display(),
                source.display()
            )
        })?;
        let target_entry = target.join(relative);

        if entry.file_type().is_symlink() {
            return Err(format!(
                "symbolic links are not supported for copy: {}",
                source_entry.display()
            ));
        }

        if entry.file_type().is_dir() {
            fs::create_dir(&target_entry).map_err(|err| {
                format!(
                    "failed to create directory {}: {err}",
                    target_entry.display()
                )
            })?;
            continue;
        }

        if entry.file_type().is_file() {
            fs::copy(source_entry, &target_entry).map_err(|err| {
                format!(
                    "failed to copy file {} to {}: {err}",
                    source_entry.display(),
                    target_entry.display()
                )
            })?;
            continue;
        }

        return Err(format!(
            "unsupported entry type while copying: {}",
            source_entry.display()
        ));
    }

    Ok(())
}

fn remove_source_entry(source: &Path) -> Result<(), String> {
    let source_meta = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to stat source {}: {err}", source.display()))?;

    if source_meta.is_file() {
        fs::remove_file(source)
            .map_err(|err| format!("failed to remove source file {}: {err}", source.display()))?;
        return Ok(());
    }

    if source_meta.is_dir() {
        fs::remove_dir_all(source).map_err(|err| {
            format!(
                "failed to remove source directory {}: {err}",
                source.display()
            )
        })?;
        return Ok(());
    }

    Err(format!(
        "unsupported source type for remove after move: {}",
        source.display()
    ))
}
