/*
 * Copyright 2026 Rein contributors
 * SPDX-License-Identifier: Apache-2.0
 */
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

pub struct DriverOwner {
    _file: File,
}
impl DriverOwner {
    pub fn acquire(state_dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(state_dir)?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(state_dir.join("driver.lock"))?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(std::fs::TryLockError::WouldBlock) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "driver owner active",
            )),
            Err(std::fs::TryLockError::Error(error)) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_is_exclusive_and_reusable() {
        let dir = tempfile::tempdir().unwrap();
        let first = DriverOwner::acquire(dir.path()).unwrap();
        assert!(
            matches!(DriverOwner::acquire(dir.path()), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
        );
        drop(first);
        assert!(DriverOwner::acquire(dir.path()).is_ok());
    }
}
