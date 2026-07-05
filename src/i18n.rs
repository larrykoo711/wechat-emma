//! Locale detection and application.

/// Resolve to `"zh-CN"` or `"en"`: explicit flag first, then `LANG`, else `en`.
pub fn resolve_locale(explicit: Option<&str>, env_lang: Option<&str>) -> &'static str {
    if let Some(flag) = explicit {
        return if flag.starts_with("zh") {
            "zh-CN"
        } else {
            "en"
        };
    }
    match env_lang {
        Some(l) if l.starts_with("zh") => "zh-CN",
        _ => "en",
    }
}

/// Apply the resolved locale to the i18n runtime.
pub fn apply(locale: &str) {
    rust_i18n::set_locale(locale);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_flag_wins() {
        assert_eq!(resolve_locale(Some("zh"), Some("en_US.UTF-8")), "zh-CN");
        assert_eq!(resolve_locale(Some("en"), Some("zh_CN.UTF-8")), "en");
    }

    #[test]
    fn falls_back_to_env_then_default() {
        assert_eq!(resolve_locale(None, Some("zh_CN.UTF-8")), "zh-CN");
        assert_eq!(resolve_locale(None, Some("fr_FR.UTF-8")), "en");
        assert_eq!(resolve_locale(None, None), "en");
    }
}
