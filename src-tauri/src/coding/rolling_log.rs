//! Small rolling JSONL helper for self-improve runtime logs.

use serde::de::DeserializeOwned;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;

pub fn archive_path(path: &Path) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path.with_extension("001");
    };
    path.with_file_name(format!("{file_name}.001"))
}

pub fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    append_line_with_limit(path, line, DEFAULT_MAX_BYTES)
}

pub fn append_line_with_limit(path: &Path, line: &str, max_bytes: u64) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    prune_extra_archives(path)?;
    rotate_if_needed(path, line.len() as u64 + 1, max_bytes)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn clear_pair(path: &Path) -> std::io::Result<()> {
    remove_if_exists(path)?;
    remove_if_exists(&archive_path(path))
}

pub fn read_jsonl_pair<T>(path: &Path) -> Vec<T>
where
    T: DeserializeOwned,
{
    let mut records = Vec::new();
    for candidate in [archive_path(path), path.to_path_buf()] {
        let Ok(file) = std::fs::File::open(candidate) else {
            continue;
        };
        records.extend(
            BufReader::new(file)
                .lines()
                .map_while(|line| line.ok())
                .filter_map(|line| serde_json::from_str::<T>(&line).ok()),
        );
    }
    records
}

fn rotate_if_needed(path: &Path, incoming_bytes: u64, max_bytes: u64) -> std::io::Result<()> {
    if max_bytes == 0 || !path.exists() {
        return Ok(());
    }

    let current_bytes = std::fs::metadata(path)?.len();
    if current_bytes.saturating_add(incoming_bytes) <= max_bytes {
        return Ok(());
    }

    let archive = archive_path(path);
    remove_if_exists(&archive)?;
    std::fs::rename(path, archive)
}

fn prune_extra_archives(path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    let archive = archive_path(path);
    let archive_name = archive.file_name().and_then(|name| name.to_str());
    let archive_prefix = format!("{file_name}.");

    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        let entry_path = entry.path();
        let Some(entry_name) = entry_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if Some(entry_name) != archive_name && entry_name.starts_with(&archive_prefix) {
            remove_if_exists(&entry_path)?;
        }
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::tempdir;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Row {
        id: u32,
    }

    #[test]
    fn rotates_to_single_archive() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("self_improve_runs.jsonl");

        append_line_with_limit(&path, r#"{"id":1}"#, 24).unwrap();
        append_line_with_limit(&path, r#"{"id":2}"#, 24).unwrap();
        append_line_with_limit(&path, r#"{"id":3}"#, 24).unwrap();

        assert!(path.exists());
        assert!(archive_path(&path).exists());
        assert!(!dir.path().join("self_improve_runs.jsonl.002").exists());

        let rows: Vec<Row> = read_jsonl_pair(&path);
        assert_eq!(rows, vec![Row { id: 1 }, Row { id: 2 }, Row { id: 3 }]);
    }

    #[test]
    fn prunes_stale_numbered_archives() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("self_improve_gates.jsonl");
        std::fs::write(dir.path().join("self_improve_gates.jsonl.002"), "old").unwrap();

        append_line_with_limit(&path, r#"{"id":1}"#, 1024).unwrap();

        assert!(!dir.path().join("self_improve_gates.jsonl.002").exists());
    }

    #[test]
    fn clear_removes_current_and_archive() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("self_improve_runs.jsonl");
        std::fs::write(&path, "current").unwrap();
        std::fs::write(archive_path(&path), "archive").unwrap();

        clear_pair(&path).unwrap();

        assert!(!path.exists());
        assert!(!archive_path(&path).exists());
    }
}
