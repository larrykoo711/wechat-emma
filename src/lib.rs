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
