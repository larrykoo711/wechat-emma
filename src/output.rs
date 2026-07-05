//! Rendering of command results as colored human text or structured JSON.

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
    Message(String),
}

pub fn render(report: &Report, json: bool) -> String {
    if json {
        let body = serde_json::json!({ "ok": true, "data": report });
        return serde_json::to_string_pretty(&body).unwrap();
    }
    match report {
        Report::Message(m) => m.clone(),
        Report::Added(row) => format!("Created instance {} ({})", row.index, row.name),
        Report::Removed { index, purged } => {
            if purged.is_empty() {
                format!("Removed instance {index} (account data kept)")
            } else {
                format!(
                    "Removed instance {index} (purged {} data paths)",
                    purged.len()
                )
            }
        }
        Report::List(rows) => {
            let mut out = String::new();
            for r in rows {
                let note = r.note.as_deref().unwrap_or("");
                let state = if r.running { "running" } else { "stopped" };
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
            let mut out = format!("Original WeChat: {original_version}\n");
            for r in rows {
                out.push_str(&format!("[{}] {}  {}\n", r.index, r.name, r.version));
            }
            if *stale {
                out.push_str("Some copies are behind; run `wxemma rebuild`.");
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
