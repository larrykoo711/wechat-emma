use std::path::PathBuf;
use wechat_emma::cli::Commands;
use wechat_emma::commands::{dispatch, Ctx};
use wechat_emma::config::Config;
use wechat_emma::output::Report;
use wechat_emma::sysops::{MockSystemOps, SystemOps};

/// Build a context whose apps dir is a real tempdir so the build pipeline (which
/// writes an Info.plist through the mock's ditto) can run end-to-end.
fn ctx_with(existing: &[u8], dir: &tempfile::TempDir) -> (MockSystemOps, Config, PathBuf, PathBuf) {
    let ops = MockSystemOps::new();
    let cfg = Config::default();
    let apps = dir.path().to_path_buf();
    let wechat = apps.join("WeChat.app");
    ops.set_app(&wechat, true);
    for i in existing {
        // Materialize a placeholder bundle dir so app_exists is true for it.
        std::fs::create_dir_all(apps.join(format!("WeChat-B{i}.app/Contents"))).unwrap();
        ops.set_app(&apps.join(format!("WeChat-B{i}.app")), true);
    }
    (ops, cfg, apps.clone(), wechat)
}

#[test]
fn list_returns_all_existing_indices() {
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[1, 3], &dir);
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: false,
        yes: true,
    };
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
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[1], &dir);
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: false,
        yes: true,
    };
    dispatch(&mut ctx, &Commands::Kill).unwrap();
    assert!(ops.calls().iter().any(|c| c.contains("pkill")));
}

#[test]
fn add_builds_at_smallest_free_index_and_requires_root() {
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[1, 3], &dir);
    ops.set_root(true);
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: false,
        yes: true,
    };
    let report = dispatch(&mut ctx, &Commands::Add { note: None }).unwrap();
    match report {
        Report::Added(row) => assert_eq!(row.index, 2),
        _ => panic!("expected added"),
    }
    assert!(ops
        .calls()
        .iter()
        .any(|c| c.contains("WeChat-B2.app") && c.contains("ditto")));
}

#[test]
fn add_without_root_fails() {
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[], &dir);
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: false,
        yes: true,
    };
    let err = dispatch(&mut ctx, &Commands::Add { note: None }).unwrap_err();
    assert!(matches!(err, wechat_emma::error::Error::SudoRequired));
}

#[test]
fn remove_with_yes_requires_index() {
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[1], &dir);
    ops.set_root(true);
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: true,
        yes: true,
    };
    let err = dispatch(
        &mut ctx,
        &Commands::Remove {
            index: None,
            purge_data: false,
        },
    )
    .unwrap_err();
    assert!(matches!(err, wechat_emma::error::Error::Usage(_)));
}

#[test]
fn remove_existing_index_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[1, 2], &dir);
    ops.set_root(true);
    let app2 = apps.join("WeChat-B2.app");
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: true,
        yes: true,
    };
    let report = dispatch(
        &mut ctx,
        &Commands::Remove {
            index: Some(2),
            purge_data: false,
        },
    )
    .unwrap();
    match report {
        Report::Removed { index, .. } => assert_eq!(index, 2),
        _ => panic!("expected removed"),
    }
    assert!(!ctx.ops.app_exists(&app2));
}

#[test]
fn remove_purge_refuses_foreign_owned_container() {
    // Data-loss regression guard: if the container that would be purged is owned
    // by the ORIGINAL app (not this copy), purging must be refused rather than
    // deleting the original's data.
    let dir = tempfile::tempdir().unwrap();
    let (ops, cfg, apps, wechat) = ctx_with(&[2], &dir);
    ops.set_root(true);
    // Pretend the multi2 container is actually owned by the original bundle id.
    let home = dirs::home_dir().unwrap();
    let container = home
        .join("Library/Containers")
        .join("com.tencent.xinWeChat.multi2");
    ops.set_container_owner(&container, "com.tencent.xinWeChat");
    let mut ctx = Ctx {
        ops: &ops,
        cfg,
        apps_dir: apps,
        wechat_app: wechat,
        json: true,
        yes: true,
    };
    let err = dispatch(
        &mut ctx,
        &Commands::Remove {
            index: Some(2),
            purge_data: true,
        },
    )
    .unwrap_err();
    assert!(matches!(
        err,
        wechat_emma::error::Error::RefusedForeignContainer { .. }
    ));
}
