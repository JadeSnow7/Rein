/*
 * Copyright 2026 Rein contributors
 * SPDX-License-Identifier: Apache-2.0
 */

use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Read-only file access confined to one canonical root.
///
/// This does not claim to defend against a hostile local process concurrently
/// replacing path components after validation. The runtime driver owns the
/// isolated workdir for that boundary.
#[derive(Clone, Debug)]
pub struct ReadFileTool {
    root: PathBuf,
    max: usize,
}

impl ReadFileTool {
    pub fn rooted_read_only() -> Self {
        Self {
            root: PathBuf::new(),
            max: 1024 * 1024,
        }
    }

    pub fn new(root: PathBuf, max: usize) -> Self {
        Self { root, max }
    }

    pub fn set_root(&mut self, root: PathBuf) {
        self.root = root;
    }

    pub fn read(&self, relative: &str) -> Result<Vec<u8>, String> {
        let requested = Path::new(relative);
        if relative.is_empty() || requested.is_absolute() {
            return Err("path rejected".into());
        }
        let components: Vec<_> = requested.components().collect();
        if components
            .iter()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err("path rejected".into());
        }

        let root = fs::canonicalize(&self.root).map_err(|e| format!("root unavailable: {e}"))?;
        let joined = root.join(requested);
        let mut cursor = root.clone();
        for component in &components {
            let Component::Normal(name) = component else {
                return Err("path rejected".into());
            };
            cursor.push(name);
            let metadata = fs::symlink_metadata(&cursor).map_err(|e| e.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err("path rejected".into());
            }
        }

        let target = fs::canonicalize(&joined).map_err(|e| e.to_string())?;
        if !target.starts_with(&root) {
            return Err("path rejected".into());
        }
        let metadata = fs::metadata(&target).map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("path rejected".into());
        }

        // Open only after the lexical, symlink, canonical-root, and regular-file checks.
        // A concurrent hostile replacement is outside this tool's local-workdir boundary.
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&target)
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(self.max.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > self.max {
            return Err("too large".into());
        }
        std::str::from_utf8(&bytes).map_err(|_| "not utf8".to_string())?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, set_permissions, symlink_metadata, write, Permissions};
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn tool(root: &Path, max: usize) -> ReadFileTool {
        ReadFileTool::new(root.to_path_buf(), max)
    }

    #[test]
    fn reads_spaces_and_never_writes_root() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("folder with spaces").join("note.txt");
        create_dir(file.parent().unwrap()).unwrap();
        write(&file, "hello").unwrap();
        let before = symlink_metadata(&file).unwrap();
        assert_eq!(
            tool(dir.path(), 32)
                .read("folder with spaces/note.txt")
                .unwrap(),
            b"hello"
        );
        let after = symlink_metadata(&file).unwrap();
        assert_eq!(before.len(), after.len());
        assert!(dir.path().join("canary").metadata().is_err());
    }

    #[test]
    fn rejects_parent_absolute_directory_and_symlink_escape() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write(outside.path().join("canary.txt"), "secret").unwrap();
        create_dir(root.path().join("inside")).unwrap();
        symlink(
            outside.path(),
            root.path().join("inside").join("escape-dir"),
        )
        .unwrap();
        symlink(
            outside.path().join("canary.txt"),
            root.path().join("last-link.txt"),
        )
        .unwrap();
        let t = tool(root.path(), 100);
        assert!(t.read("../canary.txt").is_err());
        assert!(t.read(outside.path().to_str().unwrap()).is_err());
        assert!(t.read("inside/escape-dir/canary.txt").is_err());
        assert!(t.read("last-link.txt").is_err());
        assert_eq!(
            std::fs::read_to_string(outside.path().join("canary.txt")).unwrap(),
            "secret"
        );
    }

    #[test]
    fn rejects_non_utf8_and_real_oversize_without_unbounded_read() {
        let dir = tempdir().unwrap();
        write(dir.path().join("bad.bin"), [0xff, 0xfe]).unwrap();
        write(dir.path().join("large.txt"), vec![b'x'; 17]).unwrap();
        let t = tool(dir.path(), 16);
        assert_eq!(t.read("bad.bin").unwrap_err(), "not utf8");
        assert_eq!(t.read("large.txt").unwrap_err(), "too large");
    }

    #[test]
    fn set_root_changes_only_the_allowed_read_root() {
        let first = tempdir().unwrap();
        let second = tempdir().unwrap();
        write(first.path().join("a.txt"), "a").unwrap();
        write(second.path().join("b.txt"), "b").unwrap();
        let mut t = ReadFileTool::rooted_read_only();
        t.set_root(first.path().to_path_buf());
        assert_eq!(t.read("a.txt").unwrap(), b"a");
        t.set_root(second.path().to_path_buf());
        assert!(t.read("a.txt").is_err());
        assert_eq!(t.read("b.txt").unwrap(), b"b");
    }

    #[test]
    fn read_does_not_require_writable_directory() {
        let dir = tempdir().unwrap();
        write(dir.path().join("read-only.txt"), "ok").unwrap();
        let mut permissions: Permissions = fs::metadata(dir.path()).unwrap().permissions();
        permissions.set_readonly(true);
        set_permissions(dir.path(), permissions).unwrap();
        assert_eq!(tool(dir.path(), 10).read("read-only.txt").unwrap(), b"ok");
        let mut restored = fs::metadata(dir.path()).unwrap().permissions();
        restored.set_readonly(false);
        set_permissions(dir.path(), restored).unwrap();
    }
}
