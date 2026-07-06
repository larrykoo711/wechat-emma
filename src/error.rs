//! Domain errors and their exit-code mapping.

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
