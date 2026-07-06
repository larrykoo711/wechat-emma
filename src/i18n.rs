//! Locale detection and application.

/// Resolve to `"zh-CN"` or `"en"`. Priority: the `--lang` flag, then the saved
/// config preference, then the system `LANG` (Chinese if it starts with `zh`),
/// defaulting to English.
pub fn resolve_locale(
    explicit: Option<&str>,
    config_lang: Option<&str>,
    env_lang: Option<&str>,
) -> &'static str {
    for pref in [explicit, config_lang] {
        if let Some(flag) = pref {
            return if flag.starts_with("zh") {
                "zh-CN"
            } else {
                "en"
            };
        }
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
    fn flag_beats_config_beats_env() {
        // --lang wins over everything.
        assert_eq!(
            resolve_locale(Some("zh"), Some("en"), Some("en_US.UTF-8")),
            "zh-CN"
        );
        // Config preference wins over system LANG.
        assert_eq!(
            resolve_locale(None, Some("zh"), Some("en_US.UTF-8")),
            "zh-CN"
        );
        assert_eq!(resolve_locale(None, Some("en"), Some("zh_CN.UTF-8")), "en");
    }

    #[test]
    fn follows_system_lang_then_defaults_english() {
        assert_eq!(resolve_locale(None, None, Some("zh_CN.UTF-8")), "zh-CN");
        assert_eq!(resolve_locale(None, None, Some("en_US.UTF-8")), "en");
        assert_eq!(resolve_locale(None, None, None), "en");
    }
}
