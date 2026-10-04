/*
 * Copyright 2026 Rein contributors
 * SPDX-License-Identifier: Apache-2.0
 */

use rein_core::ArtifactRef as CoreArtifactRef;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Content-addressed bytes stored below one driver-owned state directory.
///
/// The caller owns the state directory. This module does not add an OS sandbox
/// against a hostile local process concurrently replacing validated paths.
#[derive(Clone, Debug)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, artifact: &CoreArtifactRef) -> io::Result<PathBuf> {
        validate_hash(&artifact.hash)?;
        Ok(self.root.join(&artifact.hash))
    }

    pub fn put(&self, bytes: &[u8]) -> io::Result<CoreArtifactRef> {
        let artifact = CoreArtifactRef {
            hash: digest_hex(bytes),
            len: bytes.len() as u64,
        };
        let target = self.path_for(&artifact)?;
        if target.exists() || fs::symlink_metadata(&target).is_ok() {
            self.verify(&artifact)?;
            return Ok(artifact);
        }

        let (temp, mut file) = loop {
            let candidate = self.temp_path();
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => break (candidate, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        let result = (|| {
            file.write_all(bytes)?;
            file.flush()?;
            file.sync_all()?;
            match fs::hard_link(&temp, &target) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    self.verify(&artifact)?;
                    Ok(())
                }
                Err(error) => Err(error),
            }
        })();
        let _ = fs::remove_file(&temp);
        result?;
        self.verify(&artifact)?;
        Ok(artifact)
    }

    pub fn read(&self, artifact: &CoreArtifactRef) -> io::Result<Vec<u8>> {
        let path = self.path_for(artifact)?;
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() {
            return Err(invalid("artifact path is not a regular file"));
        }
        if metadata.len() != artifact.len {
            return Err(invalid("artifact length mismatch"));
        }
        let bytes = fs::read(path)?;
        verify_bytes(artifact, &bytes)?;
        Ok(bytes)
    }

    pub fn verify(&self, artifact: &CoreArtifactRef) -> io::Result<()> {
        let path = self.path_for(artifact)?;
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() {
            return Err(invalid("artifact path is not a regular file"));
        }
        if metadata.len() != artifact.len {
            return Err(invalid("artifact length mismatch"));
        }
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        io::Read::read_to_end(&mut file, &mut bytes)?;
        verify_bytes(artifact, &bytes)
    }

    fn temp_path(&self) -> PathBuf {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.root
            .join(format!(".artifact-tmp-{}-{id}", std::process::id()))
    }
}

fn validate_hash(hash: &str) -> io::Result<()> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid(
            "artifact hash must be 64 lowercase hexadecimal characters",
        ));
    }
    Ok(())
}

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn verify_bytes(artifact: &CoreArtifactRef, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() as u64 != artifact.len {
        return Err(invalid("artifact length mismatch"));
    }
    if digest_hex(bytes) != artifact.hash {
        return Err(invalid("artifact hash mismatch"));
    }
    Ok(())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, write};
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    #[test]
    fn puts_reads_and_deduplicates_content() {
        let dir = tempdir().unwrap();
        let store = ArtifactStore::new(dir.path().join("artifacts")).unwrap();
        let first = store.put(b"hello").unwrap();
        let path = store.path_for(&first).unwrap();
        let before = fs::metadata(&path).unwrap();
        let second = store.put(b"hello").unwrap();
        let after = fs::metadata(&path).unwrap();
        assert_eq!(first, second);
        assert_eq!(store.read(&first).unwrap(), b"hello");
        assert_eq!(before.len(), after.len());
    }

    #[test]
    fn rejects_corrupt_existing_content_without_overwriting_it() {
        let dir = tempdir().unwrap();
        let store = ArtifactStore::new(dir.path().join("artifacts")).unwrap();
        let artifact = store.put(b"original").unwrap();
        let path = store.path_for(&artifact).unwrap();
        write(&path, b"corrupt").unwrap();
        assert!(store.put(b"original").is_err());
        assert_eq!(fs::read(path).unwrap(), b"corrupt");
    }

    #[test]
    fn rejects_missing_length_and_hash_mismatches() {
        let dir = tempdir().unwrap();
        let store = ArtifactStore::new(dir.path().join("artifacts")).unwrap();
        let artifact = store.put(b"bytes").unwrap();
        assert!(store
            .verify(&CoreArtifactRef {
                hash: "1".repeat(64),
                len: 0,
            })
            .is_err());
        assert!(store
            .verify(&CoreArtifactRef {
                hash: artifact.hash.clone(),
                len: 99
            })
            .is_err());
        assert!(store
            .verify(&CoreArtifactRef {
                hash: "0".repeat(64),
                len: 5
            })
            .is_err());
        assert!(store
            .verify(&CoreArtifactRef {
                hash: "f".repeat(64),
                len: 5
            })
            .is_err());
        assert!(store
            .path_for(&CoreArtifactRef {
                hash: "../escape".into(),
                len: 0
            })
            .is_err());
    }

    #[test]
    fn rejects_symlink_artifact_path_without_reading_outside() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        write(outside.path().join("canary"), b"secret").unwrap();
        let store = ArtifactStore::new(dir.path().join("artifacts")).unwrap();
        let artifact = CoreArtifactRef {
            hash: "a".repeat(64),
            len: 6,
        };
        symlink(
            outside.path().join("canary"),
            store.path_for(&artifact).unwrap(),
        )
        .unwrap();
        assert!(store.read(&artifact).is_err());
        assert_eq!(fs::read(outside.path().join("canary")).unwrap(), b"secret");
    }
}
