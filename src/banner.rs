//! ASCII-art logo shown at startup and in `--version`.

/// The full startup banner: logo plus a one-line tagline.
pub fn startup_banner() -> String {
    format!(
        "{}\n  一台 Mac，几个微信，各过各的 · v{}\n",
        LOGO,
        env!("CARGO_PKG_VERSION")
    )
}

/// The string printed for `--version`: logo plus version.
pub fn version_string() -> String {
    format!("{}\nwxemma v{}", LOGO, env!("CARGO_PKG_VERSION"))
}

/// `ansi_shadow`-style "Wx Emma" logo — bold solid-block glyphs with a shadow.
const LOGO: &str = r#"
██╗    ██╗██╗  ██╗    ███████╗███╗   ███╗███╗   ███╗ █████╗
██║    ██║╚██╗██╔╝    ██╔════╝████╗ ████║████╗ ████║██╔══██╗
██║ █╗ ██║ ╚███╔╝     █████╗  ██╔████╔██║██╔████╔██║███████║
██║███╗██║ ██╔██╗     ██╔══╝  ██║╚██╔╝██║██║╚██╔╝██║██╔══██║
╚███╔███╔╝██╔╝ ██╗    ███████╗██║ ╚═╝ ██║██║ ╚═╝ ██║██║  ██║
 ╚══╝╚══╝ ╚═╝  ╚═╝    ╚══════╝╚═╝     ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string_contains_version() {
        assert!(version_string().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn startup_banner_contains_logo_and_tagline() {
        let b = startup_banner();
        assert!(b.contains("██"));
        assert!(b.contains("微信"));
    }
}
