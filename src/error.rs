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
