//! System operation boundary: real macOS tools behind a mockable trait.

use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub trait SystemOps {
    fn app_exists(&self, path: &Path) -> bool;
    fn ditto(&self, src: &Path, dst: &Path) -> Result<()>;
    fn remove_dir(&self, path: &Path) -> Result<()>;
    fn clear_xattr(&self, path: &Path) -> Result<()>;
    fn codesign(&self, path: &Path) -> Result<()>;
    fn verify_signature(&self, path: &Path) -> Result<()>;
    fn open_app(&self, path: &Path) -> Result<()>;
    fn kill_matching(&self, needle: &str) -> Result<()>;
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
    fn open_app(&self, path: &Path) -> Result<()> {
        run(Command::new("open").arg(path))
    }
    fn kill_matching(&self, needle: &str) -> Result<()> {
        // pkill returns non-zero when no process matched; treat that as success.
        let _ = Command::new("pkill").arg("-f").arg(needle).output()?;
        Ok(())
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
                is_root: Cell::new(false),
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
        pub fn set_root(&self, v: bool) {
            self.is_root.set(v);
        }
    }

    impl SystemOps for MockSystemOps {
        fn app_exists(&self, path: &Path) -> bool {
            self.apps.borrow().contains(path)
        }
        fn ditto(&self, src: &Path, dst: &Path) -> Result<()> {
            self.calls
                .borrow_mut()
                .push(format!("ditto {src:?} {dst:?}"));
            self.apps.borrow_mut().insert(dst.to_path_buf());
            Ok(())
        }
        fn remove_dir(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("remove_dir {path:?}"));
            self.apps.borrow_mut().remove(path);
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
        fn open_app(&self, path: &Path) -> Result<()> {
            self.calls.borrow_mut().push(format!("open {path:?}"));
            Ok(())
        }
        fn kill_matching(&self, needle: &str) -> Result<()> {
            self.calls.borrow_mut().push(format!("pkill {needle}"));
            Ok(())
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
        let m = MockSystemOps::new();
        let src = Path::new("/Applications/WeChat.app");
        let dst = Path::new("/Applications/WeChat-B1.app");
        m.set_app(src, true);
        m.ditto(src, dst).unwrap();
        assert!(m.app_exists(dst));
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
