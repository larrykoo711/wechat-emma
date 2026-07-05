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
    plist_edit::read_string(
        &app.join("Contents/Info.plist"),
        "CFBundleShortVersionString",
    )
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
        note: ctx.cfg.note(idx),
        running: false,
    }
}

fn require_root<S: SystemOps>(ctx: &Ctx<S>) -> Result<()> {
    if ctx.ops.euid_is_root() {
        Ok(())
    } else {
        Err(Error::SudoRequired)
    }
}

pub fn dispatch<S: SystemOps>(ctx: &mut Ctx<S>, cmd: &Commands) -> Result<Report> {
    match cmd {
        Commands::List => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            let rows = set
                .existing_indices()
                .into_iter()
                .map(|i| row_for(ctx, &set, i))
                .collect();
            Ok(Report::List(rows))
        }
        Commands::Status => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            let original = version_of(ctx.ops, &ctx.wechat_app);
            let rows: Vec<InstanceRow> = set
                .existing_indices()
                .into_iter()
                .map(|i| row_for(ctx, &set, i))
                .collect();
            let stale = rows.iter().any(|r| r.version != original);
            Ok(Report::Status {
                original_version: original,
                rows,
                stale,
            })
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
            Ok(Report::Message("msg.launched".into()))
        }
        Commands::Kill => {
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            for i in set.existing_indices() {
                ctx.ops.kill_matching(&format!(
                    "{}/Contents/MacOS/",
                    set.app_path_for(i).display()
                ))?;
            }
            Ok(Report::Message("msg.killed".into()))
        }
        Commands::Doctor => {
            let msg = if ctx.ops.app_exists(&ctx.wechat_app) {
                rust_i18n::t!("doctor.wechat_found").to_string()
            } else {
                rust_i18n::t!("doctor.wechat_missing", path = ctx.wechat_app.display()).to_string()
            };
            // Doctor text is already localized here, so pass it as a literal
            // message rather than a catalog key.
            Ok(Report::Text(msg))
        }
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
                    return Err(Error::Usage("an index is required".into()));
                }
            };
            let app = set.app_path_for(idx);
            if !ctx.ops.app_exists(&app) {
                return Err(Error::InstanceNotFound(idx));
            }
            let inst = set.instance_for(idx);
            ctx.ops
                .kill_matching(&format!("{}/Contents/MacOS/", app.display()))?;
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
            ctx.cfg.remove_note(idx);
            let _ = ctx.cfg.save_to(&crate::config::default_config_path());
            Ok(Report::Removed { index: idx, purged })
        }
        Commands::Rebuild => {
            require_root(ctx)?;
            let set = InstanceSet::new(ctx.ops, &ctx.cfg, ctx.apps_dir.clone());
            for i in set.existing_indices() {
                set.build(i, &ctx.wechat_app)?;
            }
            Ok(Report::Message("msg.rebuilt".into()))
        }
        Commands::Completions { .. } => Ok(Report::Text(String::new())),
    }
}
