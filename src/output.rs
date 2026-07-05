//! Rendering of command results as colored human text or structured JSON.
//!
//! JSON output is a stable, English, machine-facing contract (agents parse it),
//! so the `kind`/`value` payload is never localized. Human-facing text IS
//! localized via the i18n catalog.

use rust_i18n::t;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InstanceRow {
    pub index: u8,
    pub name: String,
    pub version: String,
    pub note: Option<String>,
    pub running: bool,
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
            let mut out = String::new();
            for r in rows {
                let note = r.note.as_deref().unwrap_or("");
                let state = if r.running {
                    t!("list.state_running")
                } else {
                    t!("list.state_stopped")
                };
                out.push_str(&format!(
                    "[{}] {}  {}  {}  {}\n",
                    r.index, r.name, r.version, state, note
                ));
            }
            out.trim_end().to_string()
        }
        Report::Status {
            original_version,
            rows,
            stale,
        } => {
            let mut out = format!("{}\n", t!("status.original", version = original_version));
            for r in rows {
                out.push_str(&format!("[{}] {}  {}\n", r.index, r.name, r.version));
            }
            if *stale {
                out.push_str(&t!("status.stale"));
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

    #[test]
    fn human_list_contains_indices() {
        let rows = vec![InstanceRow {
            index: 1,
            name: "WeChat1".into(),
            version: "4.1".into(),
            note: None,
            running: false,
        }];
        let s = render(&Report::List(rows), false);
        assert!(s.contains("1"));
        assert!(s.contains("WeChat1"));
    }
}
