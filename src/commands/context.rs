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
