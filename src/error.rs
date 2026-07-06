//! Domain errors and their exit-code mapping.
//!
//! The `#[error(...)]` strings are English and used only for `Debug`/logging and
//! the machine-facing JSON `error.message` (a stable English contract that agents
//! parse). Human-facing text is rendered by [`Error::i18n`], which looks the copy
//! up in the runtime locale catalog. The variant *name* is also the JSON error
//! `code` (extracted from `{e:?}` in the binary layer), so names are a contract —
//! do not rename variants.

use rust_i18n::t;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Usage(String),

    #[error("Need admin rights for this. Add sudo and try again.")]
    SudoRequired,

    #[error("No WeChat at {0}. Install the official app first.")]
    WeChatNotFound(String),

    #[error("There's no copy {0}. Run `wxemma list` to see what you've got.")]
    InstanceNotFound(u8),

    #[error("All {0} slots are taken. Remove one before adding another.")]
    SlotsFull(u8),

    #[error("Notes are ASCII only (no Chinese): {0:?}. Keep it short and simple.")]
    InvalidNote(String),

    #[error("codesign failed: {0}")]
    CodesignFailed(String),

    #[error("Stopped before this could hurt you: the copy is still on bundle id {found}, not {expected}. A copy on the original's id would share — and could corrupt — your real WeChat data. Nothing was created.")]
    RebrandFailed { expected: String, found: String },

    #[error("Not deleting that. This data belongs to {found}, not {expected} — looks like your real WeChat. Your chat history stays put.")]
    RefusedForeignContainer { expected: String, found: String },

    #[error("Nothing to launch — add a copy first.")]
    NothingToLaunch,

    #[error("Missing instance index for remove.")]
    RemoveNeedsIndex { with_yes: bool },

    #[error("{0}")]
    System(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Usage errors exit `2`; all other failures exit `1`.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Usage(_)
            | Error::NothingToLaunch
            | Error::RemoveNeedsIndex { .. }
            | Error::InstanceNotFound(_) => 2,
            _ => 1,
        }
    }

    /// The human-facing message in the current runtime locale. `Usage`/`System`
    /// already carry a runtime string (a system tool's stderr or a pre-built
    /// message), so they pass through verbatim; every other variant is looked up
    /// in the i18n catalog.
    pub fn i18n(&self) -> String {
        match self {
            Error::Usage(s) | Error::System(s) => s.clone(),
            Error::Io(e) => e.to_string(),
            Error::SudoRequired => t!("err.sudo_required").to_string(),
            Error::WeChatNotFound(path) => t!("err.wechat_not_found", path = path).to_string(),
            Error::InstanceNotFound(index) => {
                t!("err.instance_not_found", index = index).to_string()
            }
            Error::SlotsFull(max) => t!("err.slots_full", max = max).to_string(),
            Error::InvalidNote(note) => t!("err.invalid_note", note = note).to_string(),
            Error::CodesignFailed(detail) => t!("err.codesign_failed", detail = detail).to_string(),
            Error::RebrandFailed { expected, found } => {
                t!("err.rebrand_failed", expected = expected, found = found).to_string()
            }
            Error::RefusedForeignContainer { expected, found } => t!(
                "err.refused_foreign_container",
                expected = expected,
                found = found
            )
            .to_string(),
            Error::NothingToLaunch => t!("err.nothing_to_launch").to_string(),
            Error::RemoveNeedsIndex { with_yes } => if *with_yes {
                t!("err.remove_needs_index_yes")
            } else {
                t!("err.remove_needs_index")
            }
            .to_string(),
        }
    }
}

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

    #[test]
    fn new_usage_variants_map_to_code_2() {
        assert_eq!(Error::NothingToLaunch.exit_code(), 2);
        assert_eq!(Error::RemoveNeedsIndex { with_yes: true }.exit_code(), 2);
        assert_eq!(Error::InstanceNotFound(3).exit_code(), 2);
    }

    #[test]
    fn i18n_follows_locale() {
        rust_i18n::set_locale("en");
        assert!(Error::SudoRequired.i18n().contains("admin rights"));
        rust_i18n::set_locale("zh-CN");
        assert!(Error::SudoRequired.i18n().contains("管理员"));
    }

    #[test]
    fn i18n_interpolates_args() {
        rust_i18n::set_locale("en");
        assert!(Error::InstanceNotFound(7).i18n().contains('7'));
    }
}
