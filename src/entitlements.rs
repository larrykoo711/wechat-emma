//! Minimal entitlements for instance copies.
//!
//! The root cause of the data-loss bug: signing a copy with a bare ad-hoc
//! signature (`codesign --sign -` with no entitlements) strips the original
//! app's `com.apple.security.app-sandbox` entitlement. Without the sandbox, the
//! copy is no longer confined to its own container, and WeChat falls back to
//! reading and writing the ORIGINAL account container — so removing a copy can
//! corrupt the original's data.
//!
//! The fix: re-sign each copy while carrying a minimal entitlements set that
//! keeps `app-sandbox` (so `containermanagerd` allocates a private container
//! keyed by the copy's own bundle id) but drops the team-scoped
//! `application-identifier` and `application-groups` (an ad-hoc signature has no
//! team authority for those, and keeping them makes container allocation fail
//! and fall back to the original container).

/// The minimal entitlements plist that isolates a copy into its own sandbox
/// container. Keeping only `app-sandbox` is deliberate — see the module docs.
pub fn sandbox_only_plist() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>com.apple.security.app-sandbox</key>
	<true/>
</dict>
</plist>
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_app_sandbox() {
        let p = sandbox_only_plist();
        assert!(p.contains("com.apple.security.app-sandbox"));
        assert!(p.contains("<true/>"));
    }

    #[test]
    fn drops_team_scoped_entitlements() {
        // These would break ad-hoc container allocation; they must be absent.
        let p = sandbox_only_plist();
        assert!(!p.contains("application-identifier"));
        assert!(!p.contains("application-groups"));
    }

    #[test]
    fn is_well_formed_plist() {
        let p = sandbox_only_plist();
        assert!(p.starts_with("<?xml"));
        assert!(p.contains("<plist"));
        assert!(p.contains("</plist>"));
    }
}
