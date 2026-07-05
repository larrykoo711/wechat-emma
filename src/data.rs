//! Standard macOS per-app data locations and purge logic.

use crate::error::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The eight standard locations a sandboxed macOS app writes to, for `bundle_id`.
pub fn data_paths(home: &Path, bundle_id: &str) -> Vec<PathBuf> {
    let lib = home.join("Library");
    vec![
        lib.join("Containers").join(bundle_id),
        lib.join("Application Support").join(bundle_id),
        lib.join("Preferences").join(format!("{bundle_id}.plist")),
        lib.join("Saved Application State")
            .join(format!("{bundle_id}.savedState")),
        lib.join("HTTPStorages").join(bundle_id),
        lib.join("HTTPStorages")
            .join(format!("{bundle_id}.binarycookies")),
        lib.join("Caches").join(bundle_id),
        lib.join("WebKit").join(bundle_id),
    ]
}

/// Remove every existing data path for `bundle_id`; return the ones removed.
pub fn purge(home: &Path, bundle_id: &str) -> Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    for path in data_paths(home, bundle_id) {
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
            removed.push(path);
        } else if path.exists() {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    Ok(removed)
}

/// The real user's home, resolving `SUDO_USER` when running under sudo.
pub fn user_home() -> Option<PathBuf> {
    if let Ok(user) = std::env::var("SUDO_USER") {
        if !user.is_empty() {
            let out = Command::new("dscl")
                .args([".", "-read", &format!("/Users/{user}"), "NFSHomeDirectory"])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(path) = text.split_whitespace().nth(1) {
                return Some(PathBuf::from(path));
            }
        }
    }
    dirs::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn data_paths_covers_eight_locations() {
        let home = std::path::Path::new("/Users/x");
        let paths = data_paths(home, "com.tencent.xinWeChat.multi1");
        assert_eq!(paths.len(), 8);
        assert!(paths
            .iter()
            .any(|p| p.ends_with("Containers/com.tencent.xinWeChat.multi1")));
        assert!(paths
            .iter()
            .any(|p| p.ends_with("Preferences/com.tencent.xinWeChat.multi1.plist")));
    }

    #[test]
    fn purge_removes_only_existing() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let containers = home.join("Library/Containers/com.tencent.xinWeChat.multi1");
        std::fs::create_dir_all(&containers).unwrap();
        let removed = purge(home, "com.tencent.xinWeChat.multi1").unwrap();
        assert!(removed
            .iter()
            .any(|p| p.ends_with("Containers/com.tencent.xinWeChat.multi1")));
        assert!(!containers.exists());
    }
}
