//! System operation boundary: real macOS tools behind a mockable trait.

use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub trait SystemOps {
    fn app_exists(&self, path: &Path) -> bool;
    fn ditto(&self, src: &Path, dst: &Path) -> Result<()>;
    fn remove_dir(&self, path: &Path) -> Result<()>;
    fn clear_xattr(&self, path: &Path) -> Result<()>;
    /// Bare ad-hoc re-sign `path` (no entitlements). Data isolation comes from
    /// the copy's bundle id, not a sandbox — adding `app-sandbox` crashes
    /// WeChat's WeChatAppEx font engine on launch, so we deliberately do not.
    fn codesign(&self, path: &Path) -> Result<()>;
    fn verify_signature(&self, path: &Path) -> Result<()>;
    /// The container-owner bundle id recorded in a sandbox container's metadata,
    /// or `None` if the path has no readable container metadata. Used by
    /// `remove` to refuse deleting data that belongs to the original app.
    fn container_owner(&self, container_dir: &Path) -> Option<String>;
    fn open_app(&self, path: &Path) -> Result<()>;
    fn kill_matching(&self, needle: &str) -> Result<()>;
    /// Whether any process is running from `app_dir`'s MacOS folder.
    fn is_running(&self, app_dir: &Path) -> bool;
    /// Whether Xcode Command Line Tools are installed (codesign needs them).
    fn clt_installed(&self) -> bool;
    fn euid_is_root(&self) -> bool;
}

fn run(cmd: &mut Command) -> Result<()> {
    let out = cmd.output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(Error::System(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ))
    }
}

pub struct RealSystemOps;

impl SystemOps for RealSystemOps {
    fn app_exists(&self, path: &Path) -> bool {
        path.is_dir()
    }
    fn ditto(&self, src: &Path, dst: &Path) -> Result<()> {
        run(Command::new("ditto").arg(src).arg(dst))
    }
    fn remove_dir(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
        Ok(())
    }
    fn clear_xattr(&self, path: &Path) -> Result<()> {
        run(Command::new("xattr").arg("-cr").arg(path))
    }
    fn codesign(&self, path: &Path) -> Result<()> {
        let out = Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-", "--timestamp=none"])
            .arg(path)
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(Error::CodesignFailed(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        }
    }
    fn verify_signature(&self, path: &Path) -> Result<()> {
        run(Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(path))
    }
    fn container_owner(&self, container_dir: &Path) -> Option<String> {
        let meta = container_dir.join(".com.apple.containermanagerd.metadata.plist");
        let val = plist::Value::from_file(&meta).ok()?;
        val.as_dictionary()?
            .get("MCMMetadataIdentifier")?
            .as_string()
            .map(str::to_owned)
    }
    fn open_app(&self, path: &Path) -> Result<()> {
        run(Command::new("open").arg(path))
    }
    fn kill_matching(&self, needle: &str) -> Result<()> {
        // pkill returns non-zero when no process matched; treat that as success.
        let _ = Command::new("pkill").arg("-f").arg(needle).output()?;
        Ok(())
    }
    fn is_running(&self, app_dir: &Path) -> bool {
        // pgrep exits 0 when at least one process matches the MacOS exec path.
        Command::new("pgrep")
            .arg("-f")
            .arg(format!("{}/Contents/MacOS/", app_dir.display()))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    fn clt_installed(&self) -> bool {
        // `xcode-select -p` exits 0 and prints the path when CLT is present.
        Command::new("xcode-select")
            .arg("-p")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    fn euid_is_root(&self) -> bool {
        // Safe: geteuid has no failure mode.
        unsafe { libc_geteuid() == 0 }
    }
}

extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

#[cfg(any(test, feature = "testing"))]
mod mock {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// In-memory `SystemOps` for tests: records every call and tracks which app
    /// bundles "exist" and whether the effective user is root.
    pub struct MockSystemOps {
        calls: RefCell<Vec<String>>,
        apps: RefCell<HashSet<PathBuf>>,
        owners: RefCell<std::collections::HashMap<PathBuf, String>>,
        running: RefCell<HashSet<PathBuf>>,
        is_root: Cell<bool>,
    }

    impl Default for MockSystemOps {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockSystemOps {
        pub fn new() -> Self {
            MockSystemOps {
                calls: RefCell::new(Vec::new()),
                apps: RefCell::new(HashSet::new()),
                owners: RefCell::new(std::collections::HashMap::new()),
                running: RefCell::new(HashSet::new()),
                is_root: Cell::new(false),
            }
        }
        /// Mark `app_dir` as having a running process.
        pub fn set_running(&self, app_dir: &Path, running: bool) {
            if running {
                self.running.borrow_mut().insert(app_dir.to_path_buf());
            } else {
                self.running.borrow_mut().remove(app_dir);
            }
        }
        pub fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
        pub fn set_app(&self, path: &Path, exists: bool) {
            if exists {
                self.apps.borrow_mut().insert(path.to_path_buf());
            } else {
                self.apps.borrow_mut().remove(path);
            }
        }
        /// Record the container-owner bundle id reported for `container_dir`.
        pub fn set_container_owner(&self, container_dir: &Path, owner: &str) {
            self.owners
                .borrow_mut()
                .insert(container_dir.to_path_buf(), owner.to_string());
        }
        pub fn set_root(&self, v: bool) {
            self.is_root.set(v);
        }
    }

    impl SystemOps for MockSystemOps {
        fn app_exists(&self, path: &Path) -> bool {
            self.apps.borrow().contains(path)
        }
        fn ditto(&self, src: &Path, dst: &Path) -> Result<()> {
            // Safety rail: the mock materializes a real bundle on disk, so a test
            // that points its apps dir at the live /Applications would litter the
            // user's real machine with fake copies. Refuse loudly instead of
            // silently polluting the system. Tests MUST use a tempdir.
            assert!(
                !dst.starts_with("/Applications") && !dst.starts_with("/System"),
                "mock ditto refuses to write to a real system dir: {dst:?} — use a tempdir"
            );
            self.calls
                .borrow_mut()
                .push(format!("ditto {src:?} {dst:?}"));
            self.apps.borrow_mut().insert(dst.to_path_buf());
            // When the destination lives under a writable directory (tests use a
            // tempdir), materialize a minimal bundle with a seed Info.plist so the
            // real plist-editing step in the build pipeline can run end-to-end.
            let contents = dst.join("Contents");
            if std::fs::create_dir_all(&contents).is_ok() {
                let mut dict = plist::Dictionary::new();
                dict.insert(
                    "CFBundleIdentifier".into(),
                    plist::Value::String("com.tencent.xinWeChat".into()),
                );
                dict.insert(
                    "CFBundleShortVersionString".into(),
                    plist::Value::String("4.1.11".into()),
                );
                dict.insert("CFBundleURLTypes".into(), plist::Value::Array(vec![]));
                let _ = plist::Value::Dictionary(dict).to_file_xml(contents.join("Info.plist"));
            }
            Ok(())
        }
        fn remove_dir(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("remove_dir {path:?}"));
            self.apps.borrow_mut().remove(path);
            let _ = std::fs::remove_dir_all(path);
            Ok(())
        }
        fn clear_xattr(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("xattr {path:?}"));
            Ok(())
        }
        fn codesign(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("codesign {path:?}"));
            Ok(())
        }
        fn verify_signature(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("verify {path:?}"));
            Ok(())
        }
        fn container_owner(&self, container_dir: &Path) -> Option<String> {
            self.owners.borrow().get(container_dir).cloned()
        }
        fn open_app(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("open {path:?}"));
            Ok(())
        }
        fn kill_matching(&self, needle: &str) -> Result<()> {
            self.calls.borrow_mut().push(format!("pkill {needle}"));
            Ok(())
        }
        fn is_running(&self, app_dir: &Path) -> bool {
            self.running.borrow().contains(app_dir)
        }
        fn clt_installed(&self) -> bool {
            true
        }
        fn euid_is_root(&self) -> bool {
            self.is_root.get()
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub use mock::MockSystemOps;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn mock_records_ditto_and_creates_app() {
        // Use a tempdir: the mock ditto writes a real bundle to disk, and the
        // safety rail forbids writing under /Applications.
        let dir = tempfile::tempdir().unwrap();
        let m = MockSystemOps::new();
        let src = dir.path().join("WeChat.app");
        let dst = dir.path().join("WeChat-B1.app");
        m.set_app(&src, true);
        m.ditto(&src, &dst).unwrap();
        assert!(m.app_exists(&dst));
        assert!(m.calls().iter().any(|c| c.contains("ditto")));
    }

    #[test]
    fn mock_euid_toggles() {
        let m = MockSystemOps::new();
        assert!(!m.euid_is_root());
        m.set_root(true);
        assert!(m.euid_is_root());
    }
}
