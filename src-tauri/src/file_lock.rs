//! Release ownership explicitly, even if a concurrent fork inherited the descriptor.
use std::{fs::File, path::Path};
pub struct Guard {
    file: File,
}
impl Guard {
    #[cfg(test)]
    pub fn unlock(&self) -> std::io::Result<()> {
        self.file.unlock()
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
pub fn acquire(path: &Path) -> Result<Guard, String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|_| "RECOVERY_PAIR_BUSY")?;
    Ok(Guard { file })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_drop_unlocks_even_while_a_duplicate_descriptor_is_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("queue.lock");
        let first = acquire(&path).unwrap();
        assert!(acquire(&path).is_err());
        let duplicate = first.file.try_clone().unwrap();
        drop(first);
        assert!(acquire(&path).is_ok());
        drop(duplicate);
    }
}
