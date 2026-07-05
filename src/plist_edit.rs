//! Typed Info.plist reading and copy-specific edits.

use crate::error::{Error, Result};
use plist::Value;
use std::path::Path;

fn map_err(e: plist::Error) -> Error {
    Error::System(format!("plist error: {e}"))
}

pub fn read_string(plist_path: &Path, key: &str) -> Result<Option<String>> {
    let val = Value::from_file(plist_path).map_err(map_err)?;
    Ok(val
        .as_dictionary()
        .and_then(|d| d.get(key))
        .and_then(|v| v.as_string())
        .map(str::to_owned))
}

pub fn apply_copy_edits(
    plist_path: &Path,
    bundle_id: &str,
    display_name: &str,
    bundle_name: &str,
) -> Result<()> {
    let mut val = Value::from_file(plist_path).map_err(map_err)?;
    let dict = val
        .as_dictionary_mut()
        .ok_or_else(|| Error::System("Info.plist is not a dictionary".into()))?;

    dict.insert("CFBundleIdentifier".into(), Value::String(bundle_id.into()));
    dict.insert(
        "CFBundleDisplayName".into(),
        Value::String(display_name.into()),
    );
    dict.insert("CFBundleName".into(), Value::String(bundle_name.into()));

    dict.remove("CFBundleURLTypes");
    dict.remove("SUPublicEDKey");
    dict.remove("SUEnableInstallerLauncherService");

    val.to_file_xml(plist_path).map_err(map_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::Value;
    use std::path::Path;
    use tempfile::tempdir;

    fn seed_plist(path: &Path) {
        let mut dict = plist::Dictionary::new();
        dict.insert(
            "CFBundleIdentifier".into(),
            Value::String("com.tencent.xinWeChat".into()),
        );
        dict.insert("CFBundleDisplayName".into(), Value::String("WeChat".into()));
        dict.insert("CFBundleName".into(), Value::String("WeChat".into()));
        dict.insert("SUPublicEDKey".into(), Value::String("KEY".into()));
        dict.insert(
            "SUEnableInstallerLauncherService".into(),
            Value::Boolean(true),
        );
        dict.insert("CFBundleURLTypes".into(), Value::Array(vec![]));
        Value::Dictionary(dict).to_file_xml(path).unwrap();
    }

    #[test]
    fn apply_edits_sets_identity_and_strips_keys() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("Info.plist");
        seed_plist(&p);
        apply_copy_edits(&p, "com.tencent.xinWeChat.multi2", "WeChat2", "WeChat2").unwrap();
        assert_eq!(
            read_string(&p, "CFBundleIdentifier").unwrap().as_deref(),
            Some("com.tencent.xinWeChat.multi2")
        );
        assert_eq!(
            read_string(&p, "CFBundleDisplayName").unwrap().as_deref(),
            Some("WeChat2")
        );
        let val = plist::Value::from_file(&p).unwrap();
        let dict = val.as_dictionary().unwrap();
        assert!(dict.get("SUPublicEDKey").is_none());
        assert!(dict.get("SUEnableInstallerLauncherService").is_none());
        assert!(dict.get("CFBundleURLTypes").is_none());
    }
}
