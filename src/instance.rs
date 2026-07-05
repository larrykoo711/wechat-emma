//! Instance model, slot scanning, and smallest-free-index allocation.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::plist_edit;
use crate::sysops::SystemOps;
use std::path::{Path, PathBuf};

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
        self.apps_dir
            .join(format!("{}{}.app", self.cfg.prefix, idx))
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

    /// Build (or rebuild) the copy at `idx` from `wechat_app`, running the full
    /// duplicate → rebrand → strip-update-keys → ad-hoc-sign → verify pipeline.
    pub fn build(&self, idx: u8, wechat_app: &Path) -> Result<Instance> {
        let inst = self.instance_for(idx);
        let dst = &inst.app_path;

        if self.ops.app_exists(dst) {
            self.ops
                .kill_matching(&format!("{}/Contents/MacOS/", dst.display()))?;
            self.ops.remove_dir(dst)?;
        }

        self.ops.ditto(wechat_app, dst)?;

        let plist = dst.join("Contents/Info.plist");
        plist_edit::apply_copy_edits(
            &plist,
            &inst.bundle_id,
            &inst.display_name,
            &inst.display_name,
        )?;

        self.ops.clear_xattr(dst)?;
        self.ops.remove_dir(&dst.join("Contents/_CodeSignature"))?;
        self.ops.codesign(dst)?;
        self.ops.verify_signature(dst)?;

        Ok(inst)
    }
}

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
        assert!(matches!(
            set.next_free_index(),
            Err(crate::error::Error::SlotsFull(7))
        ));
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

    #[test]
    fn build_runs_full_pipeline_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().to_path_buf();
        let ops = MockSystemOps::new();
        let cfg = Config::default();
        let wechat = apps.join("WeChat.app");
        // The mock's ditto materializes a seed bundle, so the source only needs
        // to be marked present for any pre-checks.
        ops.set_app(&wechat, true);
        let set = InstanceSet::new(&ops, &cfg, apps.clone());

        let inst = set.build(1, &wechat).unwrap();
        assert_eq!(inst.index, 1);

        // The rebranded, key-stripped plist was written by the real edit step.
        let plist = apps.join("WeChat-B1.app/Contents/Info.plist");
        assert_eq!(
            crate::plist_edit::read_string(&plist, "CFBundleIdentifier")
                .unwrap()
                .as_deref(),
            Some("com.tencent.xinWeChat.multi1")
        );

        // Full pipeline ran in order: ditto → codesign → verify.
        let calls = ops.calls();
        let pos = |needle: &str| calls.iter().position(|c| c.contains(needle));
        assert!(pos("ditto").unwrap() < pos("codesign").unwrap());
        assert!(pos("codesign").unwrap() < pos("verify").unwrap());
    }
}
