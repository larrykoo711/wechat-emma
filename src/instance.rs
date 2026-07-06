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

    /// A one-word danger tag if the copy at `idx` is unsafe, else `None`. Right
    /// now the check that matters: a copy whose bundle id is NOT its expected
    /// `…multi{N}` shares the original's data container and can corrupt it.
    pub fn danger_tag(&self, idx: u8) -> Option<&'static str> {
        let expected = format!("{}{}", self.cfg.bundle_id_base, idx);
        let plist = self.app_path_for(idx).join("Contents/Info.plist");
        match plist_edit::read_string(&plist, "CFBundleIdentifier") {
            Ok(Some(id)) if id == expected => None,
            // Wrong id, missing id, or unreadable plist → treat as dangerous.
            _ => Some("danger.shared_bundle_id"),
        }
    }

    /// Build (or rebuild) the copy at `idx` from `wechat_app`, running the full
    /// duplicate → rebrand → strip-update-keys → ad-hoc-sign → verify pipeline.
    ///
    /// Data isolation comes from the bundle id, not a sandbox: WeChat derives its
    /// data container path from `CFBundleIdentifier`, so a copy whose id is
    /// `…multi{N}` writes to its own `Containers/…multi{N}` and never touches the
    /// original. The critical invariant is therefore that the rebrand actually
    /// took — a copy left on the ORIGINAL bundle id would share the original's
    /// container and corrupt it. `build` verifies the rebranded id before signing
    /// and refuses to produce a copy that is still on the original id.
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

        // Data-loss guard: confirm the copy is really on its own bundle id. If the
        // rebrand did not stick, the copy would share the original's data
        // container; delete the half-built copy and fail loudly rather than ship a
        // dangerous one.
        self.assert_rebranded(dst, &inst.bundle_id)?;

        self.ops.clear_xattr(dst)?;
        self.ops.remove_dir(&dst.join("Contents/_CodeSignature"))?;
        self.ops.codesign(dst)?;
        self.ops.verify_signature(dst)?;

        Ok(inst)
    }

    /// Fail (after removing the copy) unless `dst`'s bundle id equals
    /// `expected_id`. Guards against a half-applied rebrand leaving a copy on the
    /// original id, which would make it share the original's data container.
    fn assert_rebranded(&self, dst: &Path, expected_id: &str) -> Result<()> {
        let plist = dst.join("Contents/Info.plist");
        let actual = plist_edit::read_string(&plist, "CFBundleIdentifier")?;
        if actual.as_deref() == Some(expected_id) {
            return Ok(());
        }
        let _ = self.ops.remove_dir(dst);
        Err(Error::RebrandFailed {
            expected: expected_id.to_string(),
            found: actual.unwrap_or_else(|| "<missing>".into()),
        })
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

    #[test]
    fn build_succeeds_when_rebrand_takes() {
        // Happy path: the mock's ditto seeds the original bundle id, build
        // rewrites it to multi{N}, and the rebrand check passes.
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().to_path_buf();
        let ops = MockSystemOps::new();
        let cfg = Config::default();
        let wechat = apps.join("WeChat.app");
        ops.set_app(&wechat, true);
        let set = InstanceSet::new(&ops, &cfg, apps.clone());

        set.build(1, &wechat).unwrap();

        let plist = apps.join("WeChat-B1.app/Contents/Info.plist");
        assert_eq!(
            crate::plist_edit::read_string(&plist, "CFBundleIdentifier")
                .unwrap()
                .as_deref(),
            Some("com.tencent.xinWeChat.multi1")
        );
    }

    #[test]
    fn build_refuses_copy_left_on_original_bundle_id() {
        // Data-loss regression guard. This is the exact failure that corrupted
        // the user's data: a copy left on the ORIGINAL bundle id shares the
        // original's data container. If the rebrand does not stick, build must
        // delete the copy and fail with RebrandFailed — never ship it.
        let dir = tempfile::tempdir().unwrap();
        let apps = dir.path().to_path_buf();
        let ops = MockSystemOps::new();
        let cfg = Config::default();
        let wechat = apps.join("WeChat.app");
        ops.set_app(&wechat, true);
        let set = InstanceSet::new(&ops, &cfg, apps.clone());

        // Simulate a half-built copy stuck on the original id by directly
        // asserting the invariant checker rejects it.
        let dst = apps.join("WeChat-B1.app");
        std::fs::create_dir_all(dst.join("Contents")).unwrap();
        let mut d = plist::Dictionary::new();
        d.insert(
            "CFBundleIdentifier".into(),
            plist::Value::String("com.tencent.xinWeChat".into()),
        );
        plist::Value::Dictionary(d)
            .to_file_xml(dst.join("Contents/Info.plist"))
            .unwrap();

        let err = set
            .assert_rebranded(&dst, "com.tencent.xinWeChat.multi1")
            .unwrap_err();
        assert!(matches!(err, crate::error::Error::RebrandFailed { .. }));
        // The dangerous half-built copy was removed.
        assert!(!dst.exists());
    }
}
