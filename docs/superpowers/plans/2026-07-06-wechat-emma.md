# wechat-emma Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `wxemma`, a macOS CLI that creates, lists, removes, rebuilds, and launches multiple isolated WeChat instances, with agent-friendly JSON/non-interactive I/O, zh/en i18n, a Homebrew tap, and a companion Claude skill.

**Architecture:** Single Rust crate with a thin `main.rs` over a testable `lib`. Pure data operations are native (`plist` crate for Info.plist, `serde` for config/state); high-risk system operations (`ditto`, `codesign`, `open`, `pkill`) go through a `SystemOps` trait so tests inject a mock and run without sudo or a real WeChat. clap v4 derive for the CLI; `thiserror`/`anyhow` for errors mapped to exit codes.

**Tech Stack:** Rust 2021, clap v4, dialoguer, plist, serde/serde_json, toml, rust-i18n, thiserror, anyhow, owo-colors. GitHub Actions for CI + universal-binary release. Homebrew tap for distribution.

## Global Constraints

- Platform: macOS only. Original app path: `/Applications/WeChat.app`.
- Instance slots: `1..=max_instances`, default max **7**. `add` always fills the smallest free index; a removed middle slot is refilled next.
- Naming (defaults, config-overridable): copy prefix `WeChat-B` (→ `WeChat-B{N}.app`), display name `WeChat{N}`, bundle-id base `com.tencent.xinWeChat.multi` (→ `...multi{N}`).
- Each copy build MUST: set `CFBundleIdentifier`/`CFBundleDisplayName`/`CFBundleName`; delete `CFBundleURLTypes`, `SUPublicEDKey`, `SUEnableInstallerLauncherService`; `xattr -cr`; remove `_CodeSignature`; `codesign --force --deep --sign -`; verify with `codesign --verify --deep --strict` and fail loudly with captured stderr.
- Instance notes: ASCII label only, non-ASCII (incl. Chinese) rejected by design.
- Agent I/O: every command supports `--json`; every prompting command supports `--yes`/`-y` to run non-interactively.
- Exit codes: `0` success, `1` runtime failure, `2` usage error.
- Artifacts (code, comments, commits, PRs, in-repo docs) in English, Conventional Commits. `README.md` and GitHub repo description in Chinese for ordinary users. Compliance framing: efficiency tool for staying signed in to multiple WeChat accounts; MIT, for study and communication only.
- Branching: gitflow. `feature/*` branches off `develop`; each feature merges to `develop` when done.

---

## Feature Branch 1: `feature/scaffold`

Branch: `git checkout develop && git checkout -b feature/scaffold`

### Task 1.1: Crate manifest and entry points

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `rust-toolchain.toml`

**Interfaces:**
- Produces: crate `wechat_emma` (lib) + binary `wxemma`; `wechat_emma::run() -> std::process::ExitCode`.

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "wechat-emma"
version = "0.1.0"
edition = "2021"
description = "A macOS efficiency CLI for running multiple WeChat instances"
license = "MIT"
repository = "https://github.com/larrykoo711/wechat-emma"
readme = "README.md"
rust-version = "1.74"

[[bin]]
name = "wxemma"
path = "src/main.rs"

[lib]
name = "wechat_emma"
path = "src/lib.rs"

[dependencies]
clap = { version = "4", features = ["derive", "env"] }
clap_complete = "4"
dialoguer = "0.11"
plist = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rust-i18n = "3"
thiserror = "1"
anyhow = "1"
owo-colors = "4"
dirs = "5"

[dev-dependencies]
tempfile = "3"

[profile.release]
strip = true
lto = true
codegen-units = 1
```

- [ ] **Step 2: Write `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

- [ ] **Step 3: Write `src/lib.rs`**

```rust
//! wechat-emma: run multiple isolated WeChat instances on macOS.

rust_i18n::i18n!("locales", fallback = "en");

pub mod cli;
pub mod config;
pub mod data;
pub mod error;
pub mod i18n;
pub mod instance;
pub mod output;
pub mod plist_edit;
pub mod sysops;

use std::process::ExitCode;

/// Parse arguments, dispatch, and map the outcome to a process exit code.
pub fn run() -> ExitCode {
    ExitCode::SUCCESS
}
```

- [ ] **Step 4: Write `src/main.rs`**

```rust
fn main() -> std::process::ExitCode {
    wechat_emma::run()
}
```

- [ ] **Step 5: Create empty module files so the crate compiles**

Create each of these with a single doc comment line (real content lands in later tasks):
`src/cli.rs`, `src/config.rs`, `src/data.rs`, `src/error.rs`, `src/i18n.rs`, `src/instance.rs`, `src/output.rs`, `src/plist_edit.rs`, `src/sysops.rs`.

Example for `src/cli.rs`:
```rust
//! Command-line interface definitions.
```

- [ ] **Step 6: Create locale stubs**

Create `locales/en.yml`:
```yaml
_version: 2
app.name: wechat-emma
```
Create `locales/zh-CN.yml`:
```yaml
_version: 2
app.name: wechat-emma
```

- [ ] **Step 7: Build**

Run: `cargo build`
Expected: compiles with warnings about unused modules (acceptable at this stage).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml src locales
git commit -m "chore: scaffold crate with binary, lib, and module skeleton"
```

### Task 1.2: Error type and exit-code mapping

**Files:**
- Modify: `src/error.rs`
- Test: `src/error.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `enum Error` (domain errors), `type Result<T> = std::result::Result<T, Error>`, `fn Error::exit_code(&self) -> u8`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_errors_map_to_code_2() {
        let e = Error::Usage("bad".into());
        assert_eq!(e.exit_code(), 2);
    }

    #[test]
    fn runtime_errors_map_to_code_1() {
        let e = Error::SudoRequired;
        assert_eq!(e.exit_code(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib error`
Expected: FAIL — `Error` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Domain errors and their exit-code mapping.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("usage error: {0}")]
    Usage(String),

    #[error("this operation requires administrator privileges (run with sudo)")]
    SudoRequired,

    #[error("WeChat is not installed at {0}")]
    WeChatNotFound(String),

    #[error("instance {0} does not exist")]
    InstanceNotFound(u8),

    #[error("all {0} instance slots are in use")]
    SlotsFull(u8),

    #[error("instance note must be ASCII only: {0:?}")]
    InvalidNote(String),

    #[error("codesign failed: {0}")]
    CodesignFailed(String),

    #[error("{0}")]
    System(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Usage errors exit `2`; all other failures exit `1`.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Usage(_) => 2,
            _ => 1,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib error`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs
git commit -m "feat: add domain error type with exit-code mapping"
```

### Task 1.3: i18n locale detection

**Files:**
- Modify: `src/i18n.rs`
- Test: `src/i18n.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `rust_i18n` macros from `lib.rs`.
- Produces: `fn resolve_locale(explicit: Option<&str>, env_lang: Option<&str>) -> &'static str` returning `"zh-CN"` or `"en"`; `fn apply(locale: &str)` calling `rust_i18n::set_locale`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_flag_wins() {
        assert_eq!(resolve_locale(Some("zh"), Some("en_US.UTF-8")), "zh-CN");
        assert_eq!(resolve_locale(Some("en"), Some("zh_CN.UTF-8")), "en");
    }

    #[test]
    fn falls_back_to_env_then_default() {
        assert_eq!(resolve_locale(None, Some("zh_CN.UTF-8")), "zh-CN");
        assert_eq!(resolve_locale(None, Some("fr_FR.UTF-8")), "en");
        assert_eq!(resolve_locale(None, None), "en");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib i18n`
Expected: FAIL — `resolve_locale` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Locale detection and application.

/// Resolve to `"zh-CN"` or `"en"`: explicit flag first, then `LANG`, else `en`.
pub fn resolve_locale(explicit: Option<&str>, env_lang: Option<&str>) -> &'static str {
    if let Some(flag) = explicit {
        return if flag.starts_with("zh") { "zh-CN" } else { "en" };
    }
    match env_lang {
        Some(l) if l.starts_with("zh") => "zh-CN",
        _ => "en",
    }
}

/// Apply the resolved locale to the i18n runtime.
pub fn apply(locale: &str) {
    rust_i18n::set_locale(locale);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib i18n`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/i18n.rs
git commit -m "feat: add locale detection with flag/env/default precedence"
```

### Task 1.4: Config and per-instance notes

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Error`, `Result` from `crate::error`.
- Produces: `struct Config { max_instances: u8, prefix: String, display_base: String, bundle_id_base: String, notes: BTreeMap<u8, String> }`; `Config::default()`; `Config::load_from(path: &Path) -> Result<Config>` (missing file → default); `Config::save_to(&self, path: &Path) -> Result<()>`; `Config::set_note(&mut self, idx: u8, note: &str) -> Result<()>` (ASCII-validates); `fn default_config_path() -> PathBuf`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempdir().unwrap();
        let cfg = Config::load_from(&dir.path().join("nope.toml")).unwrap();
        assert_eq!(cfg.max_instances, 7);
        assert_eq!(cfg.prefix, "WeChat-B");
    }

    #[test]
    fn roundtrip_persists_notes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.set_note(2, "work").unwrap();
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.notes.get(&2).map(String::as_str), Some("work"));
    }

    #[test]
    fn non_ascii_note_rejected() {
        let mut cfg = Config::default();
        assert!(cfg.set_note(1, "工作").is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib config`
Expected: FAIL — `Config` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! User configuration and per-instance notes, persisted as TOML.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_max")]
    pub max_instances: u8,
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_display_base")]
    pub display_base: String,
    #[serde(default = "default_bundle_base")]
    pub bundle_id_base: String,
    #[serde(default)]
    pub notes: BTreeMap<u8, String>,
}

fn default_max() -> u8 { 7 }
fn default_prefix() -> String { "WeChat-B".into() }
fn default_display_base() -> String { "WeChat".into() }
fn default_bundle_base() -> String { "com.tencent.xinWeChat.multi".into() }

impl Default for Config {
    fn default() -> Self {
        Config {
            max_instances: default_max(),
            prefix: default_prefix(),
            display_base: default_display_base(),
            bundle_id_base: default_bundle_base(),
            notes: BTreeMap::new(),
        }
    }
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| Error::System(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(Error::Io(e)),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| Error::System(e.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn set_note(&mut self, idx: u8, note: &str) -> Result<()> {
        if !note.is_ascii() {
            return Err(Error::InvalidNote(note.to_string()));
        }
        self.notes.insert(idx, note.to_string());
        Ok(())
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wxemma")
        .join("config.toml")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib config`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add TOML config with ASCII-validated instance notes"
```

### Task 1.5: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the CI workflow**

```yaml
name: CI
on:
  push:
    branches: [main, develop]
  pull_request:
jobs:
  check:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

- [ ] **Step 2: Verify locally before relying on CI**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all`
Expected: all pass.

- [ ] **Step 3: Commit and open the feature merge**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add fmt, clippy, and test workflow"
git checkout develop && git merge --no-ff feature/scaffold -m "Merge feature/scaffold into develop"
```

---

## Feature Branch 2: `feature/core-instances`

Branch: `git checkout develop && git checkout -b feature/core-instances`

### Task 2.1: SystemOps trait and mock

**Files:**
- Modify: `src/sysops.rs`
- Test: `src/sysops.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Result`, `Error` from `crate::error`.
- Produces:
  - `trait SystemOps` with methods:
    - `fn app_exists(&self, path: &Path) -> bool`
    - `fn ditto(&self, src: &Path, dst: &Path) -> Result<()>`
    - `fn remove_dir(&self, path: &Path) -> Result<()>`
    - `fn clear_xattr(&self, path: &Path) -> Result<()>`
    - `fn codesign(&self, path: &Path) -> Result<()>`
    - `fn verify_signature(&self, path: &Path) -> Result<()>`
    - `fn open_app(&self, path: &Path) -> Result<()>`
    - `fn kill_matching(&self, needle: &str) -> Result<()>`
    - `fn euid_is_root(&self) -> bool`
  - `struct RealSystemOps`
  - `struct MockSystemOps` (test-only via `#[cfg(any(test, feature = "testing"))]`) recording calls in a `RefCell<Vec<String>>`, with a settable `apps: RefCell<HashSet<PathBuf>>` and `is_root: Cell<bool>`.

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib sysops`
Expected: FAIL — `SystemOps`/`MockSystemOps` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
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
        Err(Error::System(String::from_utf8_lossy(&out.stderr).into_owned()))
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
        run(Command::new("codesign").args(["--verify", "--deep", "--strict"]).arg(path))
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

#[cfg(test)]
mod mock {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use std::path::PathBuf;

    pub struct MockSystemOps {
        calls: RefCell<Vec<String>>,
        apps: RefCell<HashSet<PathBuf>>,
        is_root: Cell<bool>,
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
            self.calls.borrow_mut().push(format!("ditto {src:?} {dst:?}"));
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

#[cfg(test)]
pub use mock::MockSystemOps;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib sysops`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/sysops.rs
git commit -m "feat: add SystemOps trait with real and mock implementations"
```

### Task 2.2: Instance model and index allocation

**Files:**
- Modify: `src/instance.rs`
- Test: `src/instance.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `SystemOps` from `crate::sysops`, `Config` from `crate::config`, `Error`/`Result` from `crate::error`.
- Produces:
  - `struct Instance { index: u8, app_path: PathBuf, bundle_id: String, display_name: String }`
  - `struct InstanceSet<'a, S: SystemOps> { ops: &'a S, cfg: &'a Config, apps_dir: PathBuf }`
  - `InstanceSet::new(ops, cfg, apps_dir)`
  - `fn existing_indices(&self) -> Vec<u8>` (scans `apps_dir` for `{prefix}{N}.app`)
  - `fn next_free_index(&self) -> Result<u8>` (smallest free in `1..=max`, else `Error::SlotsFull`)
  - `fn instance_for(&self, idx: u8) -> Instance` (constructs paths/ids from config)
  - `fn app_path_for(&self, idx: u8) -> PathBuf`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::sysops::MockSystemOps;
    use std::path::PathBuf;

    fn set_with(existing: &[u8]) -> (MockSystemOps, Config, PathBuf) {
        let ops = MockSystemOps::new();
        let cfg = Config::default();
        let apps = PathBuf::from("/Applications");
        for i in existing {
            ops.set_app(&apps.join(format!("WeChat-B{i}.app")), true);
        }
        (ops, cfg, apps)
    }

    #[test]
    fn next_free_fills_smallest_gap() {
        let (ops, cfg, apps) = set_with(&[1, 3]);
        let set = InstanceSet::new(&ops, &cfg, apps);
        assert_eq!(set.next_free_index().unwrap(), 2);
    }

    #[test]
    fn next_free_from_empty_is_one() {
        let (ops, cfg, apps) = set_with(&[]);
        let set = InstanceSet::new(&ops, &cfg, apps);
        assert_eq!(set.next_free_index().unwrap(), 1);
    }

    #[test]
    fn full_returns_slots_full() {
        let (ops, cfg, apps) = set_with(&[1, 2, 3, 4, 5, 6, 7]);
        let set = InstanceSet::new(&ops, &cfg, apps);
        assert!(matches!(set.next_free_index(), Err(crate::error::Error::SlotsFull(7))));
    }

    #[test]
    fn instance_for_builds_paths_and_ids() {
        let (ops, cfg, apps) = set_with(&[]);
        let set = InstanceSet::new(&ops, &cfg, apps);
        let inst = set.instance_for(3);
        assert_eq!(inst.bundle_id, "com.tencent.xinWeChat.multi3");
        assert_eq!(inst.display_name, "WeChat3");
        assert!(inst.app_path.ends_with("WeChat-B3.app"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib instance`
Expected: FAIL — `InstanceSet` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Instance model, slot scanning, and smallest-free-index allocation.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::sysops::SystemOps;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub index: u8,
    pub app_path: PathBuf,
    pub bundle_id: String,
    pub display_name: String,
}

pub struct InstanceSet<'a, S: SystemOps> {
    ops: &'a S,
    cfg: &'a Config,
    apps_dir: PathBuf,
}

impl<'a, S: SystemOps> InstanceSet<'a, S> {
    pub fn new(ops: &'a S, cfg: &'a Config, apps_dir: PathBuf) -> Self {
        InstanceSet { ops, cfg, apps_dir }
    }

    pub fn app_path_for(&self, idx: u8) -> PathBuf {
        self.apps_dir.join(format!("{}{}.app", self.cfg.prefix, idx))
    }

    pub fn existing_indices(&self) -> Vec<u8> {
        (1..=self.cfg.max_instances)
            .filter(|i| self.ops.app_exists(&self.app_path_for(*i)))
            .collect()
    }

    pub fn next_free_index(&self) -> Result<u8> {
        for i in 1..=self.cfg.max_instances {
            if !self.ops.app_exists(&self.app_path_for(i)) {
                return Ok(i);
            }
        }
        Err(Error::SlotsFull(self.cfg.max_instances))
    }

    pub fn instance_for(&self, idx: u8) -> Instance {
        Instance {
            index: idx,
            app_path: self.app_path_for(idx),
            bundle_id: format!("{}{}", self.cfg.bundle_id_base, idx),
            display_name: format!("{}{}", self.cfg.display_base, idx),
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib instance`
Expected: PASS.

- [ ] **Step 5: Commit and merge feature branch**

```bash
git add src/instance.rs
git commit -m "feat: add instance model with smallest-free-index allocation"
git checkout develop && git merge --no-ff feature/core-instances -m "Merge feature/core-instances into develop"
```

---

## Feature Branch 3: `feature/build-pipeline`

Branch: `git checkout develop && git checkout -b feature/build-pipeline`

### Task 3.1: Typed Info.plist editing

**Files:**
- Modify: `src/plist_edit.rs`
- Test: `src/plist_edit.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Error`/`Result` from `crate::error`, `plist` crate.
- Produces:
  - `fn read_string(plist_path: &Path, key: &str) -> Result<Option<String>>`
  - `fn apply_copy_edits(plist_path: &Path, bundle_id: &str, display_name: &str, bundle_name: &str) -> Result<()>` — sets the three identity keys and deletes `CFBundleURLTypes`, `SUPublicEDKey`, `SUEnableInstallerLauncherService`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use plist::Value;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn seed_plist(path: &Path) {
        let mut dict = plist::Dictionary::new();
        dict.insert("CFBundleIdentifier".into(), Value::String("com.tencent.xinWeChat".into()));
        dict.insert("CFBundleDisplayName".into(), Value::String("WeChat".into()));
        dict.insert("CFBundleName".into(), Value::String("WeChat".into()));
        dict.insert("SUPublicEDKey".into(), Value::String("KEY".into()));
        dict.insert("SUEnableInstallerLauncherService".into(), Value::Boolean(true));
        dict.insert("CFBundleURLTypes".into(), Value::Array(vec![]));
        Value::Dictionary(dict).to_file_xml(path).unwrap();
        let _ = BTreeMap::<u8, u8>::new();
    }

    #[test]
    fn apply_edits_sets_identity_and_strips_keys() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("Info.plist");
        seed_plist(&p);
        apply_copy_edits(&p, "com.tencent.xinWeChat.multi2", "WeChat2", "WeChat2").unwrap();
        assert_eq!(read_string(&p, "CFBundleIdentifier").unwrap().as_deref(), Some("com.tencent.xinWeChat.multi2"));
        assert_eq!(read_string(&p, "CFBundleDisplayName").unwrap().as_deref(), Some("WeChat2"));
        let val = plist::Value::from_file(&p).unwrap();
        let dict = val.as_dictionary().unwrap();
        assert!(dict.get("SUPublicEDKey").is_none());
        assert!(dict.get("SUEnableInstallerLauncherService").is_none());
        assert!(dict.get("CFBundleURLTypes").is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib plist_edit`
Expected: FAIL — `apply_copy_edits` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Typed Info.plist reading and copy-specific edits.

use crate::error::{Error, Result};
use plist::Value;
use std::path::Path;

fn map_err(e: plist::Error) -> Error {
    Error::System(format!("plist error: {e}"))
}

pub fn read_string(plist_path: &Path, key: &str) -> Result<Option<String>> {
    let val = Value::from_file(plist_path).map_err(map_err)?;
    Ok(val
        .as_dictionary()
        .and_then(|d| d.get(key))
        .and_then(|v| v.as_string())
        .map(str::to_owned))
}

pub fn apply_copy_edits(
    plist_path: &Path,
    bundle_id: &str,
    display_name: &str,
    bundle_name: &str,
) -> Result<()> {
    let mut val = Value::from_file(plist_path).map_err(map_err)?;
    let dict = val
        .as_dictionary_mut()
        .ok_or_else(|| Error::System("Info.plist is not a dictionary".into()))?;

    dict.insert("CFBundleIdentifier".into(), Value::String(bundle_id.into()));
    dict.insert("CFBundleDisplayName".into(), Value::String(display_name.into()));
    dict.insert("CFBundleName".into(), Value::String(bundle_name.into()));

    dict.remove("CFBundleURLTypes");
    dict.remove("SUPublicEDKey");
    dict.remove("SUEnableInstallerLauncherService");

    val.to_file_xml(plist_path).map_err(map_err)?;
    Ok(())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib plist_edit`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/plist_edit.rs
git commit -m "feat: add typed Info.plist edits for instance copies"
```

### Task 3.2: Account-data locations and purge

**Files:**
- Modify: `src/data.rs`
- Test: `src/data.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Error`/`Result` from `crate::error`.
- Produces:
  - `fn data_paths(home: &Path, bundle_id: &str) -> Vec<PathBuf>` — the 8 standard locations for a bundle id.
  - `fn purge(home: &Path, bundle_id: &str) -> Result<Vec<PathBuf>>` — removes existing ones, returns those removed.
  - `fn user_home() -> Option<PathBuf>` — real home, resolving `SUDO_USER` when running under sudo.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn data_paths_covers_eight_locations() {
        let home = std::path::Path::new("/Users/x");
        let paths = data_paths(home, "com.tencent.xinWeChat.multi1");
        assert_eq!(paths.len(), 8);
        assert!(paths.iter().any(|p| p.ends_with("Containers/com.tencent.xinWeChat.multi1")));
        assert!(paths.iter().any(|p| p.ends_with("Preferences/com.tencent.xinWeChat.multi1.plist")));
    }

    #[test]
    fn purge_removes_only_existing() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let containers = home.join("Library/Containers/com.tencent.xinWeChat.multi1");
        std::fs::create_dir_all(&containers).unwrap();
        let removed = purge(home, "com.tencent.xinWeChat.multi1").unwrap();
        assert!(removed.iter().any(|p| p.ends_with("Containers/com.tencent.xinWeChat.multi1")));
        assert!(!containers.exists());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib data`
Expected: FAIL — `data_paths` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
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
        lib.join("Saved Application State").join(format!("{bundle_id}.savedState")),
        lib.join("HTTPStorages").join(bundle_id),
        lib.join("HTTPStorages").join(format!("{bundle_id}.binarycookies")),
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib data`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/data.rs
git commit -m "feat: add account-data location mapping and purge"
```

### Task 3.3: Copy build pipeline

**Files:**
- Modify: `src/instance.rs`
- Test: `src/instance.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: everything from Task 2.1/2.2, `plist_edit::apply_copy_edits`.
- Produces: `impl InstanceSet` method `fn build(&self, idx: u8, wechat_app: &Path) -> Result<Instance>` performing the full pipeline: remove stale → kill process → ditto → apply plist edits → clear xattr → remove `_CodeSignature` → codesign → verify.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod build_tests {
    use super::*;
    use crate::config::Config;
    use crate::sysops::MockSystemOps;
    use std::path::PathBuf;

    #[test]
    fn build_invokes_pipeline_in_order() {
        let ops = MockSystemOps::new();
        let cfg = Config::default();
        let apps = PathBuf::from("/Applications");
        let wechat = apps.join("WeChat.app");
        ops.set_app(&wechat, true);
        let set = InstanceSet::new(&ops, &cfg, apps);
        // build() edits a real plist inside the copied bundle; the mock ditto
        // does not create files, so we assert on the recorded command order only.
        let _ = set.build(1, &wechat);
        let calls = ops.calls();
        let pos = |needle: &str| calls.iter().position(|c| c.contains(needle));
        assert!(pos("ditto") < pos("codesign"));
        assert!(pos("codesign") < pos("verify"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib instance`
Expected: FAIL — `build` not defined.

- [ ] **Step 3: Write minimal implementation** (append to `impl InstanceSet`)

```rust
use crate::plist_edit;

impl<'a, S: SystemOps> InstanceSet<'a, S> {
    /// Build (or rebuild) the copy at `idx` from `wechat_app`, running the full
    /// duplicate → rebrand → strip-update-keys → ad-hoc-sign → verify pipeline.
    pub fn build(&self, idx: u8, wechat_app: &Path) -> Result<Instance> {
        let inst = self.instance_for(idx);
        let dst = &inst.app_path;

        if self.ops.app_exists(dst) {
            self.ops.kill_matching(&format!("{}/Contents/MacOS/", dst.display()))?;
            self.ops.remove_dir(dst)?;
        }

        self.ops.ditto(wechat_app, dst)?;

        let plist = dst.join("Contents/Info.plist");
        plist_edit::apply_copy_edits(&plist, &inst.bundle_id, &inst.display_name, &inst.display_name)?;

        self.ops.clear_xattr(dst)?;
        self.ops.remove_dir(&dst.join("Contents/_CodeSignature"))?;
        self.ops.codesign(dst)?;
        self.ops.verify_signature(dst)?;

        Ok(inst)
    }
}
```

Note: add `use std::path::Path;` at the top of the file if not already present.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib instance`
Expected: PASS. (The plist edit errors internally on the mock's non-existent file but the test only asserts command ordering via `let _ =`.)

- [ ] **Step 5: Commit and merge**

```bash
git add src/instance.rs
git commit -m "feat: add copy build pipeline (duplicate, rebrand, sign, verify)"
git checkout develop && git merge --no-ff feature/build-pipeline -m "Merge feature/build-pipeline into develop"
```

---

## Feature Branch 4: `feature/commands`

Branch: `git checkout develop && git checkout -b feature/commands`

### Task 4.1: Output renderer (human + JSON)

**Files:**
- Modify: `src/output.rs`
- Test: `src/output.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: `serde_json`.
- Produces:
  - `struct InstanceRow { index: u8, name: String, version: String, note: Option<String>, running: bool }` (Serialize)
  - `enum Report { Added(InstanceRow), List(Vec<InstanceRow>), Removed { index: u8, purged: Vec<String> }, Status { original_version: String, rows: Vec<InstanceRow>, stale: bool }, Message(String) }` (Serialize)
  - `fn render(report: &Report, json: bool) -> String`
  - `fn render_error(code: &str, message: &str, json: bool) -> String`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_report_is_wrapped_with_ok_true() {
        let r = Report::Message("done".into());
        let s = render(&r, true);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], serde_json::json!(true));
    }

    #[test]
    fn json_error_is_wrapped_with_ok_false() {
        let s = render_error("SlotsFull", "no free slots", true);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], serde_json::json!(false));
        assert_eq!(v["error"]["code"], serde_json::json!("SlotsFull"));
    }

    #[test]
    fn human_list_contains_indices() {
        let rows = vec![InstanceRow { index: 1, name: "WeChat1".into(), version: "4.1".into(), note: None, running: false }];
        let s = render(&Report::List(rows), false);
        assert!(s.contains("1"));
        assert!(s.contains("WeChat1"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib output`
Expected: FAIL — types not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Rendering of command results as colored human text or structured JSON.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InstanceRow {
    pub index: u8,
    pub name: String,
    pub version: String,
    pub note: Option<String>,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Report {
    Added(InstanceRow),
    List(Vec<InstanceRow>),
    Removed { index: u8, purged: Vec<String> },
    Status { original_version: String, rows: Vec<InstanceRow>, stale: bool },
    Message(String),
}

pub fn render(report: &Report, json: bool) -> String {
    if json {
        let body = serde_json::json!({ "ok": true, "data": report });
        return serde_json::to_string_pretty(&body).unwrap();
    }
    match report {
        Report::Message(m) => m.clone(),
        Report::Added(row) => format!("Created instance {} ({})", row.index, row.name),
        Report::Removed { index, purged } => {
            if purged.is_empty() {
                format!("Removed instance {index} (account data kept)")
            } else {
                format!("Removed instance {index} (purged {} data paths)", purged.len())
            }
        }
        Report::List(rows) => {
            let mut out = String::new();
            for r in rows {
                let note = r.note.as_deref().unwrap_or("");
                let state = if r.running { "running" } else { "stopped" };
                out.push_str(&format!("[{}] {}  {}  {}  {}\n", r.index, r.name, r.version, state, note));
            }
            out.trim_end().to_string()
        }
        Report::Status { original_version, rows, stale } => {
            let mut out = format!("Original WeChat: {original_version}\n");
            for r in rows {
                out.push_str(&format!("[{}] {}  {}\n", r.index, r.name, r.version));
            }
            if *stale {
                out.push_str("Some copies are behind; run `wxemma rebuild`.");
            }
            out.trim_end().to_string()
        }
    }
}

pub fn render_error(code: &str, message: &str, json: bool) -> String {
    if json {
        let body = serde_json::json!({ "ok": false, "error": { "code": code, "message": message } });
        serde_json::to_string_pretty(&body).unwrap()
    } else {
        format!("error: {message}")
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib output`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/output.rs
git commit -m "feat: add human and JSON output renderer"
```

### Task 4.2: CLI definitions

**Files:**
- Modify: `src/cli.rs`
- Test: `src/cli.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Consumes: clap, clap_complete.
- Produces:
  - `struct Cli { global flags + Commands }` with `#[derive(Parser)]`
  - `enum Commands { Add { note: Option<String> }, List, Status, Remove { index: Option<u8>, purge_data: bool }, Rebuild, Open { index: Option<u8> }, Kill, Doctor, Completions { shell: clap_complete::Shell } }`
  - global flags: `json: bool`, `yes: bool` (`-y`), `lang: Option<String>`, `verbose: bool`.
  - `Cli::parse_from` used in tests.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_add_with_note() {
        let cli = Cli::parse_from(["wxemma", "add", "--note", "work"]);
        assert!(cli.json == false);
        match cli.command {
            Commands::Add { note } => assert_eq!(note.as_deref(), Some("work")),
            _ => panic!("expected add"),
        }
    }

    #[test]
    fn parses_global_json_and_yes() {
        let cli = Cli::parse_from(["wxemma", "--json", "-y", "remove", "2", "--purge-data"]);
        assert!(cli.json && cli.yes);
        match cli.command {
            Commands::Remove { index, purge_data } => {
                assert_eq!(index, Some(2));
                assert!(purge_data);
            }
            _ => panic!("expected remove"),
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib cli`
Expected: FAIL — `Cli` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
//! Command-line interface definitions.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "wxemma", version, about = "Run multiple WeChat instances on macOS")]
pub struct Cli {
    /// Emit machine-readable JSON instead of human text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Assume yes; never prompt (required for non-interactive use).
    #[arg(long, short = 'y', global = true)]
    pub yes: bool,

    /// Force language: `zh` or `en`.
    #[arg(long, global = true)]
    pub lang: Option<String>,

    /// Verbose diagnostics.
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create one instance at the smallest free index.
    Add {
        /// Optional ASCII note/label for the instance.
        #[arg(long)]
        note: Option<String>,
    },
    /// List all instances.
    List,
    /// Show original-vs-copy version status.
    Status,
    /// Remove an instance by index (prompts if omitted).
    Remove {
        index: Option<u8>,
        /// Also delete the instance's account data.
        #[arg(long)]
        purge_data: bool,
    },
    /// Rebuild all copies from the current WeChat version.
    Rebuild,
    /// Launch all copies, or one by index.
    Open { index: Option<u8> },
    /// Terminate all copy processes.
    Kill,
    /// Check the environment is ready.
    Doctor,
    /// Emit a shell completion script.
    Completions { shell: clap_complete::Shell },
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib cli`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add clap CLI definitions with global agent flags"
```

### Task 4.3: Command dispatch and non-mutating commands

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/context.rs`
- Modify: `src/lib.rs` (real `run()` body, add `pub mod commands;`)
- Test: `tests/commands.rs`

**Interfaces:**
- Consumes: `cli`, `config`, `i18n`, `instance`, `output`, `sysops`, `error`.
- Produces:
  - `struct Ctx<'a, S: SystemOps> { ops: &'a S, cfg: Config, apps_dir: PathBuf, wechat_app: PathBuf, json: bool, yes: bool }`
  - `fn dispatch<S: SystemOps>(ctx: &mut Ctx<S>, cmd: &Commands) -> Result<Report>` handling `List`, `Status`, `Open`, `Kill`, `Doctor` in this task; `Add`/`Remove`/`Rebuild` added in 4.4.
  - `lib::run()` wires real ops + parses args + prints rendered output + returns `ExitCode`.

- [ ] **Step 1: Write the failing integration test**

```rust
use wechat_emma::cli::Commands;
use wechat_emma::commands::{dispatch, Ctx};
use wechat_emma::config::Config;
use wechat_emma::output::Report;
use wechat_emma::sysops::MockSystemOps;
use std::path::PathBuf;

fn ctx_with(existing: &[u8]) -> (MockSystemOps, Config, PathBuf, PathBuf) {
    let ops = MockSystemOps::new();
    let cfg = Config::default();
    let apps = PathBuf::from("/Applications");
    ops.set_app(&apps.join("WeChat.app"), true);
    for i in existing {
        ops.set_app(&apps.join(format!("WeChat-B{i}.app")), true);
    }
    (ops, cfg, apps.clone(), apps.join("WeChat.app"))
}

#[test]
fn list_returns_all_existing_indices() {
    let (ops, cfg, apps, wechat) = ctx_with(&[1, 3]);
    let mut ctx = Ctx { ops: &ops, cfg, apps_dir: apps, wechat_app: wechat, json: false, yes: true };
    let report = dispatch(&mut ctx, &Commands::List).unwrap();
    match report {
        Report::List(rows) => {
            let idx: Vec<u8> = rows.iter().map(|r| r.index).collect();
            assert_eq!(idx, vec![1, 3]);
        }
        _ => panic!("expected list"),
    }
}

#[test]
fn kill_records_pkill() {
    let (ops, cfg, apps, wechat) = ctx_with(&[1]);
    let mut ctx = Ctx { ops: &ops, cfg, apps_dir: apps, wechat_app: wechat, json: false, yes: true };
    dispatch(&mut ctx, &Commands::Kill).unwrap();
    assert!(ops.calls().iter().any(|c| c.contains("pkill")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test commands`
Expected: FAIL — `commands` module/`Ctx` not defined.

- [ ] **Step 3: Write `src/commands/context.rs`**

```rust
//! Execution context shared by all commands.

use crate::config::Config;
use crate::sysops::SystemOps;
use std::path::PathBuf;

pub struct Ctx<'a, S: SystemOps> {
    pub ops: &'a S,
    pub cfg: Config,
    pub apps_dir: PathBuf,
    pub wechat_app: PathBuf,
    pub json: bool,
    pub yes: bool,
}
```

- [ ] **Step 4: Write `src/commands/mod.rs` (non-mutating commands)**

```rust
//! Command dispatch.

pub mod context;
pub use context::Ctx;

use crate::cli::Commands;
use crate::error::{Error, Result};
use crate::instance::InstanceSet;
use crate::output::{InstanceRow, Report};
use crate::plist_edit;
use crate::sysops::SystemOps;

fn version_of<S: SystemOps>(_ops: &S, app: &std::path::Path) -> String {
    plist_edit::read_string(&app.join("Contents/Info.plist"), "CFBundleShortVersionString")
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown".into())
}

fn row_for<S: SystemOps>(ctx: &Ctx<S>, set: &InstanceSet<S>, idx: u8) -> InstanceRow {
    let inst = set.instance_for(idx);
    InstanceRow {
        index: idx,
        name: inst.display_name.clone(),
        version: version_of(ctx.ops, &inst.app_path),
        note: ctx.cfg.notes.get(&idx).cloned(),
        running: false,
    }
}

pub fn dispatch<S: SystemOps>(ctx: &mut Ctx<S>, cmd: &Commands) -> Result<Report> {
    match cmd {
        Commands::List => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            let rows = set.existing_indices().into_iter().map(|i| row_for(ctx, &set, i)).collect();
            Ok(Report::List(rows))
        }
        Commands::Status => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            let original = version_of(ctx.ops, &ctx.wechat_app);
            let rows: Vec<InstanceRow> = set.existing_indices().into_iter().map(|i| row_for(ctx, &set, i)).collect();
            let stale = rows.iter().any(|r| r.version != original);
            Ok(Report::Status { original_version: original, rows, stale })
        }
        Commands::Open { index } => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            let targets: Vec<u8> = match index {
                Some(i) => vec![*i],
                None => set.existing_indices(),
            };
            if targets.is_empty() {
                return Err(Error::Usage("no instances to open".into()));
            }
            for i in targets {
                ctx.ops.open_app(&set.app_path_for(i))?;
            }
            Ok(Report::Message("launched".into()))
        }
        Commands::Kill => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            for i in set.existing_indices() {
                ctx.ops.kill_matching(&format!("{}/Contents/MacOS/", set.app_path_for(i).display()))?;
            }
            Ok(Report::Message("all copy processes terminated".into()))
        }
        Commands::Doctor => {
            let mut msgs = Vec::new();
            msgs.push(if ctx.ops.app_exists(&ctx.wechat_app) {
                "WeChat: found".to_string()
            } else {
                "WeChat: MISSING".to_string()
            });
            Ok(Report::Message(msgs.join("\n")))
        }
        Commands::Add { .. } | Commands::Remove { .. } | Commands::Rebuild => {
            Err(Error::Usage("not implemented in this task".into()))
        }
        Commands::Completions { .. } => Ok(Report::Message(String::new())),
    }
}
```

- [ ] **Step 5: Wire `lib::run()`**

Replace the `run()` body in `src/lib.rs` and add `pub mod commands;`:

```rust
pub mod commands;

use std::path::PathBuf;
use std::process::ExitCode;

pub fn run() -> ExitCode {
    use clap::Parser;
    let cli = cli::Cli::parse();

    let locale = i18n::resolve_locale(cli.lang.as_deref(), std::env::var("LANG").ok().as_deref());
    i18n::apply(locale);

    let cfg = config::Config::load_from(&config::default_config_path()).unwrap_or_default();

    // Completions are handled before building a full context.
    if let cli::Commands::Completions { shell } = &cli.command {
        use clap::CommandFactory;
        let mut cmd = cli::Cli::command();
        clap_complete::generate(*shell, &mut cmd, "wxemma", &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }

    let ops = sysops::RealSystemOps;
    let mut ctx = commands::Ctx {
        ops: &ops,
        cfg,
        apps_dir: PathBuf::from("/Applications"),
        wechat_app: PathBuf::from("/Applications/WeChat.app"),
        json: cli.json,
        yes: cli.yes,
    };

    match commands::dispatch(&mut ctx, &cli.command) {
        Ok(report) => {
            println!("{}", output::render(&report, cli.json));
            ExitCode::SUCCESS
        }
        Err(e) => {
            let code = format!("{e:?}");
            let code = code.split_whitespace().next().unwrap_or("Error");
            eprintln!("{}", output::render_error(code, &e.to_string(), cli.json));
            ExitCode::from(e.exit_code())
        }
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test commands && cargo build`
Expected: PASS and binary builds.

- [ ] **Step 7: Commit**

```bash
git add src/commands src/lib.rs tests/commands.rs
git commit -m "feat: add command dispatch and non-mutating commands"
```

### Task 4.4: Mutating commands (add, remove, rebuild)

**Files:**
- Modify: `src/commands/mod.rs`
- Test: `tests/commands.rs`

**Interfaces:**
- Consumes: `InstanceSet::build`, `InstanceSet::next_free_index`, `data::purge`, `data::user_home`, `Config::set_note`/`save_to`, `dialoguer` (only when not `yes`).
- Produces: `Add`, `Remove`, `Rebuild` arms in `dispatch`; a helper `fn require_root<S: SystemOps>(ctx: &Ctx<S>) -> Result<()>`.

- [ ] **Step 1: Write the failing integration test**

```rust
#[test]
fn add_builds_at_smallest_free_index_and_requires_root() {
    let (ops, cfg, apps, wechat) = ctx_with(&[1, 3]);
    ops.set_root(true);
    let mut ctx = Ctx { ops: &ops, cfg, apps_dir: apps, wechat_app: wechat, json: false, yes: true };
    let report = dispatch(&mut ctx, &Commands::Add { note: None }).unwrap();
    match report {
        Report::Added(row) => assert_eq!(row.index, 2),
        _ => panic!("expected added"),
    }
    // pipeline ran for index 2
    assert!(ops.calls().iter().any(|c| c.contains("WeChat-B2.app") && c.contains("ditto")));
}

#[test]
fn add_without_root_fails() {
    let (ops, cfg, apps, wechat) = ctx_with(&[]);
    // is_root defaults to false
    let mut ctx = Ctx { ops: &ops, cfg, apps_dir: apps, wechat_app: wechat, json: false, yes: true };
    let err = dispatch(&mut ctx, &Commands::Add { note: None }).unwrap_err();
    assert!(matches!(err, wechat_emma::error::Error::SudoRequired));
}

#[test]
fn remove_with_yes_requires_index() {
    let (ops, cfg, apps, wechat) = ctx_with(&[1]);
    ops.set_root(true);
    let mut ctx = Ctx { ops: &ops, cfg, apps_dir: apps, wechat_app: wechat, json: true, yes: true };
    let err = dispatch(&mut ctx, &Commands::Remove { index: None, purge_data: false }).unwrap_err();
    assert!(matches!(err, wechat_emma::error::Error::Usage(_)));
}

#[test]
fn remove_existing_index_succeeds() {
    let (ops, cfg, apps, wechat) = ctx_with(&[1, 2]);
    ops.set_root(true);
    let mut ctx = Ctx { ops: &ops, cfg, apps_dir: apps, wechat_app: wechat, json: true, yes: true };
    let report = dispatch(&mut ctx, &Commands::Remove { index: Some(2), purge_data: false }).unwrap();
    match report {
        Report::Removed { index, .. } => assert_eq!(index, 2),
        _ => panic!("expected removed"),
    }
    assert!(!ops.app_exists(&std::path::PathBuf::from("/Applications/WeChat-B2.app")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test commands`
Expected: FAIL — mutating arms return `Usage("not implemented...")`.

- [ ] **Step 3: Write minimal implementation** (replace the placeholder arm)

```rust
fn require_root<S: SystemOps>(ctx: &Ctx<S>) -> Result<()> {
    if ctx.ops.euid_is_root() {
        Ok(())
    } else {
        Err(Error::SudoRequired)
    }
}
```

Replace the `Commands::Add { .. } | Commands::Remove { .. } | Commands::Rebuild` arm with:

```rust
        Commands::Add { note } => {
            require_root(ctx)?;
            let idx = {
                let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
                set.next_free_index()?
            };
            if let Some(n) = note {
                ctx.cfg.set_note(idx, n)?;
                let _ = ctx.cfg.save_to(&crate::config::default_config_path());
            }
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            set.build(idx, &ctx.wechat_app)?;
            Ok(Report::Added(row_for(ctx, &set, idx)))
        }
        Commands::Remove { index, purge_data } => {
            require_root(ctx)?;
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            let idx = match index {
                Some(i) => *i,
                None if ctx.yes => {
                    return Err(Error::Usage("an index is required with --yes".into()));
                }
                None => {
                    // Interactive selection handled in the binary layer; in library
                    // context without a chosen index this is a usage error.
                    return Err(Error::Usage("an index is required".into()));
                }
            };
            let app = set.app_path_for(idx);
            if !ctx.ops.app_exists(&app) {
                return Err(Error::InstanceNotFound(idx));
            }
            let inst = set.instance_for(idx);
            ctx.ops.kill_matching(&format!("{}/Contents/MacOS/", app.display()))?;
            ctx.ops.remove_dir(&app)?;
            let purged = if *purge_data {
                match crate::data::user_home() {
                    Some(home) => crate::data::purge(&home, &inst.bundle_id)?
                        .into_iter()
                        .map(|p| p.display().to_string())
                        .collect(),
                    None => Vec::new(),
                }
            } else {
                Vec::new()
            };
            ctx.cfg.notes.remove(&idx);
            let _ = ctx.cfg.save_to(&crate::config::default_config_path());
            Ok(Report::Removed { index: idx, purged })
        }
        Commands::Rebuild => {
            require_root(ctx)?;
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            for i in set.existing_indices() {
                set.build(i, &ctx.wechat_app)?;
            }
            Ok(Report::Message("all copies rebuilt".into()))
        }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test commands`
Expected: PASS.

- [ ] **Step 5: Add interactive remove selection in the binary layer**

In `src/lib.rs` `run()`, before dispatch, special-case interactive remove:

```rust
    // Interactive remove: no index, not --yes → list and prompt.
    if let cli::Commands::Remove { index: None, purge_data } = &cli.command {
        if !cli.yes {
            let set = instance::InstanceSet::new(&ops, &ctx.cfg, ctx.apps_dir.clone());
            let existing = set.existing_indices();
            if existing.is_empty() {
                println!("{}", output::render(&output::Report::Message("no instances to remove".into()), cli.json));
                return ExitCode::SUCCESS;
            }
            for i in &existing {
                println!("[{}] {}", i, set.instance_for(*i).display_name);
            }
            let idx: u8 = dialoguer::Input::new()
                .with_prompt("index to remove")
                .interact_text()
                .unwrap_or(0);
            let confirm = dialoguer::Confirm::new()
                .with_prompt(format!("remove instance {idx}?"))
                .default(false)
                .interact()
                .unwrap_or(false);
            if !confirm {
                println!("cancelled");
                return ExitCode::SUCCESS;
            }
            let purge = *purge_data
                || dialoguer::Confirm::new()
                    .with_prompt("also delete this instance's account data?")
                    .default(false)
                    .interact()
                    .unwrap_or(false);
            let cmd = cli::Commands::Remove { index: Some(idx), purge_data: purge };
            return finish(commands::dispatch(&mut ctx, &cmd), cli.json);
        }
    }
```

Add a `finish` helper near the bottom of `run()` (refactor the existing match into it):

```rust
fn finish(result: error::Result<output::Report>, json: bool) -> ExitCode {
    match result {
        Ok(report) => {
            println!("{}", output::render(&report, json));
            ExitCode::SUCCESS
        }
        Err(e) => {
            let code = format!("{e:?}");
            let code = code.split_whitespace().next().unwrap_or("Error");
            eprintln!("{}", output::render_error(code, &e.to_string(), json));
            ExitCode::from(e.exit_code())
        }
    }
}
```

And change the final dispatch in `run()` to `return finish(commands::dispatch(&mut ctx, &cli.command), cli.json);`.

- [ ] **Step 6: Run full test suite and build**

Run: `cargo test --all && cargo build`
Expected: PASS and builds.

- [ ] **Step 7: Populate i18n catalogs for all user-facing strings**

Add every human string used above to both `locales/en.yml` and `locales/zh-CN.yml`. Then verify no key is missing with:

Run: `cargo test --all`
Expected: PASS. Commit.

```bash
git add src/commands src/lib.rs tests/commands.rs locales
git commit -m "feat: add add/remove/rebuild commands with interactive remove"
git checkout develop && git merge --no-ff feature/commands -m "Merge feature/commands into develop"
```

---

## Feature Branch 5: `feature/distribution`

Branch: `git checkout develop && git checkout -b feature/distribution`

### Task 5.1: Release workflow (universal binary)

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the release workflow**

```yaml
name: Release
on:
  push:
    tags: ["v*"]
permissions:
  contents: write
jobs:
  build:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin
      - name: Build both architectures
        run: |
          cargo build --release --target aarch64-apple-darwin
          cargo build --release --target x86_64-apple-darwin
      - name: Create universal binary
        run: |
          mkdir -p dist
          lipo -create -output dist/wxemma \
            target/aarch64-apple-darwin/release/wxemma \
            target/x86_64-apple-darwin/release/wxemma
          cd dist
          tar -czf wxemma-macos-universal.tar.gz wxemma
          shasum -a 256 wxemma-macos-universal.tar.gz > wxemma-macos-universal.tar.gz.sha256
      - name: Publish release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            dist/wxemma-macos-universal.tar.gz
            dist/wxemma-macos-universal.tar.gz.sha256
```

- [ ] **Step 2: Validate the workflow YAML locally**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('ok')"`
Expected: `ok`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add universal-binary release workflow"
```

### Task 5.2: Homebrew formula template and tap-update step

**Files:**
- Create: `packaging/wxemma.rb.tmpl`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Produces: a formula template with `__VERSION__`, `__URL__`, `__SHA__` placeholders and a workflow step that fills them and pushes to `larrykoo711/homebrew-tap`.

- [ ] **Step 1: Write the formula template**

Create `packaging/wxemma.rb.tmpl`:
```ruby
class Wxemma < Formula
  desc "Run multiple WeChat instances on macOS"
  homepage "https://github.com/larrykoo711/wechat-emma"
  version "__VERSION__"
  url "__URL__"
  sha256 "__SHA__"
  license "MIT"

  def install
    bin.install "wxemma"
  end

  test do
    assert_match "wxemma", shell_output("#{bin}/wxemma --version")
  end
end
```

- [ ] **Step 2: Add the tap-update job to `release.yml`**

Append this job (requires a `TAP_TOKEN` repo secret with write access to the tap):
```yaml
  update-tap:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Compute formula
        run: |
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG#v}"
          URL="https://github.com/larrykoo711/wechat-emma/releases/download/${TAG}/wxemma-macos-universal.tar.gz"
          curl -sL "$URL" -o pkg.tar.gz
          SHA=$(shasum -a 256 pkg.tar.gz | awk '{print $1}')
          curl -sL "https://raw.githubusercontent.com/larrykoo711/wechat-emma/${TAG}/packaging/wxemma.rb.tmpl" -o tmpl.rb
          sed -e "s|__VERSION__|${VERSION}|" -e "s|__URL__|${URL}|" -e "s|__SHA__|${SHA}|" tmpl.rb > wxemma.rb
      - name: Push to tap
        env:
          TAP_TOKEN: ${{ secrets.TAP_TOKEN }}
        run: |
          git clone "https://x-access-token:${TAP_TOKEN}@github.com/larrykoo711/homebrew-tap.git" tap
          mkdir -p tap/Formula
          cp wxemma.rb tap/Formula/wxemma.rb
          cd tap
          git config user.name "github-actions"
          git config user.email "actions@github.com"
          git add Formula/wxemma.rb
          git commit -m "wxemma ${GITHUB_REF_NAME}"
          git push
```

- [ ] **Step 3: Validate YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('ok')"`
Expected: `ok`.

- [ ] **Step 4: Commit and merge**

```bash
git add packaging/wxemma.rb.tmpl .github/workflows/release.yml
git commit -m "ci: add Homebrew formula template and tap-update job"
git checkout develop && git merge --no-ff feature/distribution -m "Merge feature/distribution into develop"
```

---

## Feature Branch 6: `feature/skill-and-docs`

Branch: `git checkout develop && git checkout -b feature/skill-and-docs`

### Task 6.1: Companion Claude skill

**Files:**
- Create: `skill/SKILL.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: wxemma-multi-wechat
description: Use when a user on macOS needs to run multiple WeChat instances at once (e.g. staying signed in to a personal and a work account). Drives the `wxemma` CLI to add, list, remove, rebuild, and launch isolated WeChat copies. Prefer this over manual steps whenever the task involves multiple simultaneous WeChat logins on a Mac.
---

# Running multiple WeChat instances with wxemma

`wxemma` creates isolated copies of WeChat.app so several accounts can be signed
in at once on one Mac. Install: `brew install larrykoo711/tap/wxemma`.

## Agent usage rules

- Always pass `--json` to parse results, and `--yes` to run without prompts.
- `add`, `remove`, and `rebuild` need `sudo`; tell the user to run those with sudo.
- Exit codes: `0` success, `1` runtime failure, `2` usage error.
- On `{"ok": false, ...}`, read `error.code` to branch.

## Commands

- Create one instance: `sudo wxemma add --json --yes`
- Create with a label (ASCII only): `sudo wxemma add --note work --json --yes`
- List: `wxemma list --json`
- Status vs. original: `wxemma status --json`
- Remove by index: `sudo wxemma remove 2 --json --yes`
- Remove and wipe its data: `sudo wxemma remove 2 --purge-data --json --yes`
- Rebuild after a WeChat update: `sudo wxemma rebuild --json --yes`
- Launch: `wxemma open --json` (all) or `wxemma open 2 --json`

## What it cannot automate (tell the user)

- Each instance must be logged in by scanning a QR code once.
- WeChat's global keyboard shortcuts (screenshot, activate) are per-account and
  stored in encrypted data; ask the user to disable them per instance to avoid
  multiple instances reacting to the same key.
```

- [ ] **Step 2: Commit**

```bash
git add skill/SKILL.md
git commit -m "docs: add companion Claude skill for wxemma"
```

### Task 6.2: Chinese README and repo docs

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write the Chinese README**

```markdown
# wechat-emma（wxemma）

一个 macOS 上的效率小工具。如果你有多个微信需要同时登录（比如一个生活号、一个工作号），
`wxemma` 可以帮你在同一台 Mac 上同时打开多个互相独立的微信，各自登录、互不影响。

## 它能做什么

- 一条命令新增一个独立的微信
- 查看、启动、删除已创建的微信
- 微信升级后一键重建，保持版本一致

## 安装

需要先装好官方微信。然后：

    brew install larrykoo711/tap/wxemma

## 常用命令

- 新增一个微信：`sudo wxemma add`
- 查看已有的微信：`wxemma list`
- 启动全部微信：`wxemma open`
- 删除某一个（会先列出让你选）：`sudo wxemma remove`

删除时会询问是否一并清除该微信的聊天记录，按需选择即可。

## 配合 Claude 使用

本工具附带一个技能，可让 AI 助手直接帮你操作。安装方式：

    在 skills.sh 搜索 wxemma-multi-wechat 并安装

## 使用提醒

- 每个新微信首次使用需要扫码登录一次。
- 如果多个微信对同一个快捷键都有反应，可在各自的「设置 → 快捷键」里关闭全局快捷键。

## 许可

本项目基于 MIT 协议开源，仅限学习和交流使用。
```

- [ ] **Step 2: Commit and merge**

```bash
git add README.md
git commit -m "docs: add Chinese README for ordinary users"
git checkout develop && git merge --no-ff feature/skill-and-docs -m "Merge feature/skill-and-docs into develop"
```

### Task 6.3: Publish to GitHub and cut first release

**Files:** none (operational)

- [ ] **Step 1: Create the GitHub repo and push**

```bash
gh repo create wechat-emma --public --source=. --remote=origin \
  --description "macOS 效率工具：在同一台 Mac 上同时登录多个微信。MIT 协议，仅供学习交流。"
git push -u origin main
git push origin develop
```

- [ ] **Step 2: Merge develop to main via release branch and tag**

```bash
git checkout main && git merge --no-ff develop -m "Release 0.1.0"
git tag v0.1.0
git push origin main --tags
```

- [ ] **Step 3: Verify the release workflow produced the universal binary and updated the tap**

Run: `gh run watch` (or check the Actions tab)
Expected: Release job green; `larrykoo711/homebrew-tap` has `Formula/wxemma.rb`.

- [ ] **Step 4: Verify Homebrew install end-to-end**

Run: `brew install larrykoo711/tap/wxemma && wxemma --version`
Expected: prints the version.

---

## Self-Review Notes

- **Spec coverage:** CLI surface (§3) → Tasks 4.2–4.4; architecture/modules (§4) → Tasks 1.x–4.x; data flow (§5) → 4.3 `run()`; error handling (§6) → 1.2 + JSON error in 4.1; testing (§7) → mock in 2.1, integration in 4.x, CI in 1.5; distribution (§8) → 5.x; skill (§9) → 6.1 + skills.sh in 6.2/README; repo conventions (§10) → gitflow branches throughout, English commits, Chinese README in 6.2; milestones (§11) → the six feature branches.
- **Placeholders:** formula uses intentional `__VERSION__/__URL__/__SHA__` tokens filled by CI (not plan placeholders).
- **Type consistency:** `SystemOps`, `InstanceSet`, `Ctx`, `Report`, `InstanceRow`, `Config` signatures are consistent across tasks; `build`/`next_free_index`/`existing_indices`/`instance_for`/`app_path_for` names match their definitions.
