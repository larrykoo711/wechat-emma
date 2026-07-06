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
    /// Preferred UI language (`"zh"` or `"en"`). When unset, follow the system
    /// `LANG`. The `--lang` flag overrides this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Per-instance notes. Keyed by the stringified index because TOML table
    /// keys must be strings; the public `set_note`/`note` API takes `u8`.
    #[serde(default)]
    pub notes: BTreeMap<String, String>,
}

fn default_max() -> u8 {
    7
}
fn default_prefix() -> String {
    "WeChat-B".into()
}
fn default_display_base() -> String {
    "WeChat".into()
}
fn default_bundle_base() -> String {
    "com.tencent.xinWeChat.multi".into()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_instances: default_max(),
            prefix: default_prefix(),
            display_base: default_display_base(),
            bundle_id_base: default_bundle_base(),
            lang: None,
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
        self.notes.insert(idx.to_string(), note.to_string());
        Ok(())
    }

    /// The note for an instance index, if any.
    pub fn note(&self, idx: u8) -> Option<String> {
        self.notes.get(&idx.to_string()).cloned()
    }

    /// Remove the note for an instance index.
    pub fn remove_note(&mut self, idx: u8) {
        self.notes.remove(&idx.to_string());
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wxemma")
        .join("config.toml")
}

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
        assert_eq!(loaded.note(2).as_deref(), Some("work"));
    }

    #[test]
    fn non_ascii_note_rejected() {
        let mut cfg = Config::default();
        assert!(cfg.set_note(1, "工作").is_err());
    }
}
