//! Rendering of command results as colored human text or structured JSON.
//!
//! JSON output is a stable, English, machine-facing contract (agents parse it),
//! so the `kind`/`value` payload is never localized. Human-facing text IS
//! localized via the i18n catalog.

use owo_colors::OwoColorize;
use rust_i18n::t;
use serde::Serialize;

/// A dim horizontal rule to separate a header from the rows below it.
fn rule() -> String {
    "──────────────────────────────".dimmed().to_string()
}

/// One human-readable instance line. A dangerous copy is rendered in red with a
/// warning so it can't be mistaken for a healthy instance.
fn render_row(r: &InstanceRow) -> String {
    let note = r.note.as_deref().unwrap_or("");
    if let Some(warn) = &r.danger {
        return format!("[{}] {}  ⚠️  {}", r.index, r.name, warn)
            .red()
            .to_string();
    }
    let state = if r.running {
        t!("list.state_running")
    } else {
        t!("list.state_stopped")
    };
    format!(
        "[{}] {}  {}  {}  {}",
        r.index, r.name, r.version, state, note
    )
    .trim_end()
    .to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct InstanceRow {
    pub index: u8,
    pub name: String,
    pub version: String,
    pub note: Option<String>,
    pub running: bool,
    /// Set when the copy is unsafe (e.g. still on the original bundle id, which
    /// would share the original's data). Rendered in red with a warning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub danger: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Report {
    Added(InstanceRow),
    List(Vec<InstanceRow>),
    Removed {
        index: u8,
        purged: Vec<String>,
    },
    Status {
        original_version: String,
        rows: Vec<InstanceRow>,
        stale: bool,
    },
    /// A message identified by an i18n catalog key (localized for humans).
    Message(String),
    /// An already-rendered literal string (not localized further).
    Text(String),
}

pub fn render(report: &Report, json: bool) -> String {
    if json {
        let body = serde_json::json!({ "ok": true, "data": report });
        return serde_json::to_string_pretty(&body).unwrap();
    }
    match report {
        // The payload is a catalog key; translate it for humans.
        Report::Message(key) => t!(key.as_str()).to_string(),
        Report::Text(s) => s.clone(),
        Report::Added(row) => t!("report.created", index = row.index, name = row.name).to_string(),
        Report::Removed { index, purged } => {
            if purged.is_empty() {
                t!("report.removed_kept", index = index).to_string()
            } else {
                t!("report.removed_purged", index = index, count = purged.len()).to_string()
            }
        }
        Report::List(rows) => {
            if rows.is_empty() {
                return t!("list.empty").to_string();
            }
            let mut out = format!("{}\n{}\n", t!("list.header", count = rows.len()), rule());
            for r in rows {
                out.push_str(&render_row(r));
                out.push('\n');
            }
            out.trim_end().to_string()
        }
        Report::Status {
            original_version,
            rows,
            stale,
        } => {
            let mut out = format!(
                "{}\n{}\n",
                t!("status.original", version = original_version),
                rule()
            );
            if rows.is_empty() {
                out.push_str(&t!("list.empty"));
            } else {
                for r in rows {
                    out.push_str(&render_row(r));
                    out.push('\n');
                }
            }
            if *stale {
                out.push('\n');
                out.push_str(&t!("status.stale").yellow().to_string());
            }
            out.trim_end().to_string()
        }
    }
}

pub fn render_error(code: &str, message: &str, json: bool) -> String {
    if json {
        let body =
            serde_json::json!({ "ok": false, "error": { "code": code, "message": message } });
        serde_json::to_string_pretty(&body).unwrap()
    } else {
        format!("error: {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_report_is_wrapped_with_ok_true() {
        let r = Report::Message("done".into());
        let s = render(&r, true);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], serde_json::json!(true));
    }

    #[test]
    fn json_error_is_wrapped_with_ok_false() {
        let s = render_error("SlotsFull", "no free slots", true);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], serde_json::json!(false));
        assert_eq!(v["error"]["code"], serde_json::json!("SlotsFull"));
    }

    fn row(index: u8, danger: Option<&str>) -> InstanceRow {
        InstanceRow {
            index,
            name: format!("WeChat{index}"),
            version: "4.1".into(),
            note: None,
            running: false,
            danger: danger.map(str::to_owned),
        }
    }

    #[test]
    fn human_list_contains_indices() {
        let s = render(&Report::List(vec![row(1, None)]), false);
        assert!(s.contains("1"));
        assert!(s.contains("WeChat1"));
    }

    #[test]
    fn dangerous_copy_shows_warning() {
        let s = render(&Report::List(vec![row(2, Some("shares identity"))]), false);
        assert!(s.contains("⚠️"));
        assert!(s.contains("shares identity"));
    }
}
