//! Protected Chronicle export: a manifest-carrying JSON/CSV bundle of the recorded history.
//!
//! Read-only: takes `&Universe`, never `&mut`, and writes only into a freshly created
//! destination directory (existing folders are never replaced).

use std::path::Path;

use crate::history::{Category, ChronicleFilter, Event};
use crate::time;
use crate::Universe;

/// One exported event with the calendar date rendered for its CSV cell.
pub struct Row<'a> {
    pub date: String,
    pub event: &'a Event,
}

/// The matching events in chronological order (see `History::chronicle`) with their dates.
pub fn rows<'a>(u: &'a Universe, f: &ChronicleFilter) -> Vec<Row<'a>> {
    u.history
        .chronicle(f)
        .into_iter()
        .map(|event| Row {
            date: time::format_date(event.time, u.start_time, u.gregorian()),
            event,
        })
        .collect()
}

/// RFC-4180 quoting: a field is quoted when it contains a comma, quote or line break, and
/// embedded quotes are doubled; records are CRLF-terminated. Missing ids become empty cells.
fn quote(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\r') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// The `chronicle.csv` body: `time_s,date,category,importance,title,detail,system,body,civ`.
pub fn csv(rows: &[Row]) -> String {
    let mut out = String::from("time_s,date,category,importance,title,detail,system,body,civ\r\n");
    for r in rows {
        let e = r.event;
        let cells = [
            e.time.to_string(),
            r.date.clone(),
            e.category.label().to_string(),
            e.importance.to_string(),
            e.title.clone(),
            e.detail.clone(),
            e.system.map(|v| v.to_string()).unwrap_or_default(),
            e.body.map(|v| v.to_string()).unwrap_or_default(),
            e.civ.map(|v| v.to_string()).unwrap_or_default(),
        ];
        for (i, cell) in cells.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&quote(cell));
        }
        out.push_str("\r\n");
    }
    out
}

/// The `manifest.json` payload. No wall-clock fields, so exporting the same universe twice
/// produces byte-identical bundles.
pub fn manifest(_u: &Universe, f: &ChronicleFilter, rows: &[Row]) -> serde_json::Value {
    let categories: Vec<&'static str> = Category::ALL
        .iter()
        .filter(|c| f.categories[c.index()])
        .map(|c| c.label())
        .collect();
    serde_json::json!({
        "format": "cosmogon-chronicle",
        "schema_version": 1,
        "app_version": env!("CARGO_PKG_VERSION"),
        "filters": {
            "min_importance": f.min_importance,
            "categories": categories,
            "search": f.text,
            "system": f.system,
            "body": f.body,
            "civ": f.civ,
        },
        "event_count": rows.len(),
        "first_event_time_s": rows.first().map(|r| r.event.time),
        "last_event_time_s": rows.last().map(|r| r.event.time),
        "files": ["chronicle.json", "chronicle.csv"],
        "limitations": [
            "The Chronicle retains at most 4000 minor events; older minor events were pruned during the run and cannot be exported.",
            "Events are simulation-authored narrative records, not measurements.",
        ],
    })
}

/// Write the filtered Chronicle into `directory` as `manifest.json` + `chronicle.json` +
/// `chronicle.csv`. The destination is exclusively created; existing directories are never
/// replaced, and an empty result refuses before anything is written.
pub fn export(u: &Universe, f: &ChronicleFilter, directory: &Path) -> Result<(), String> {
    let rows = rows(u, f);
    if rows.is_empty() {
        return Err("No events match the Chronicle filter".into());
    }
    if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir(directory).map_err(|e| format!("{}: {e}", directory.display()))?;
    let result = (|| -> Result<(), String> {
        use std::io::Write;
        let manifest = manifest(u, f, &rows);
        let events: Vec<&Event> = rows.iter().map(|r| r.event).collect();
        let files = [
            (
                "manifest.json",
                serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
            ),
            (
                "chronicle.json",
                serde_json::to_vec_pretty(&events).map_err(|e| e.to_string())?,
            ),
            ("chronicle.csv", csv(&rows).into_bytes()),
        ];
        for (name, bytes) in files {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(name))
                .map_err(|e| e.to_string())?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(directory);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::History;
    use crate::universe::{Scenario, UniverseSettings};

    /// A tiny world whose history contains only the events this test pushes.
    fn world(events: &[Event]) -> Universe {
        let mut u = Universe::new(UniverseSettings {
            scenario: Scenario::EmptySystem,
            seed: 1,
            ..Default::default()
        });
        u.history = History::default();
        for e in events {
            u.history.push(e.clone());
        }
        u
    }

    fn event(time: f64, category: Category, importance: u8, title: &str, detail: &str) -> Event {
        Event {
            time,
            category,
            importance,
            title: title.into(),
            detail: detail.into(),
            system: Some(0),
            body: Some(1),
            civ: Some(3),
            cause: None,
        }
    }

    fn root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("cosmogon-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn chronicle_export_creates_manifest_json_and_csv() {
        let u = world(&[
            event(100.0, Category::War, 4, "First War", "Fought over resources."),
            event(50.0, Category::Astronomy, 5, "Comet", "A bright visitor."),
            event(150.0, Category::Technology, 3, "Radio", "Wireless voices."),
        ]);
        let f = ChronicleFilter::default();
        let root = root("chronicle-export");
        let dir = root.join("bundle");
        export(&u, &f, &dir).unwrap();

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["format"], "cosmogon-chronicle");
        assert_eq!(manifest["schema_version"], 1);
        assert_eq!(manifest["event_count"], 3);
        assert_eq!(manifest["first_event_time_s"], 50.0);
        assert_eq!(manifest["last_event_time_s"], 150.0);
        assert_eq!(manifest["filters"]["min_importance"], 1);
        assert_eq!(manifest["limitations"].as_array().unwrap().len(), 2);
        let files: Vec<&str> = manifest["files"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(files, ["chronicle.json", "chronicle.csv"]);
        assert!(manifest.get("exported_at").is_none(), "no wall-clock fields: re-export stays byte-identical");

        let events: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("chronicle.json")).unwrap()).unwrap();
        let rows = events.as_array().unwrap();
        assert_eq!(rows.len(), 3);
        // The JSON rows carry exactly the serde field names of `Event` (order-independent:
        // `serde_json::Value` maps may reorder keys), in chronological order.
        let mut keys: Vec<&str> = rows[0].as_object().unwrap().keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["body", "category", "civ", "detail", "importance", "system", "time", "title"]
        );
        assert_eq!(rows[0]["title"], "Comet");
        assert_eq!(rows[1]["title"], "First War");
        assert_eq!(rows[2]["title"], "Radio");

        let csv = std::fs::read_to_string(dir.join("chronicle.csv")).unwrap();
        assert!(csv.starts_with("time_s,date,category,importance,title,detail,system,body,civ"));
        assert_eq!(csv.lines().count(), 4, "header plus three events");
        assert!(csv.contains("First War"));

        // The second export into the same folder is refused and the original files survive.
        let before = std::fs::read(dir.join("manifest.json")).unwrap();
        assert!(export(&u, &f, &dir).is_err());
        assert_eq!(std::fs::read(dir.join("manifest.json")).unwrap(), before);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn chronicle_export_refuses_existing_dir_and_empty_result() {
        let u = world(&[event(100.0, Category::War, 4, "First War", "Fought.")]);
        let root = root("chronicle-refuse");

        // An empty result refuses before any directory is created.
        let f = ChronicleFilter { text: Some("no such event anywhere".into()), ..Default::default() };
        let dir = root.join("empty");
        let err = export(&u, &f, &dir).unwrap_err();
        assert!(err.contains("No events match the Chronicle filter"), "{err}");
        assert!(!dir.exists());

        // An existing directory is protected: refused, contents untouched, nothing added.
        let existing = root.join("existing");
        std::fs::create_dir(&existing).unwrap();
        std::fs::write(existing.join("keep.txt"), b"mine").unwrap();
        let err = export(&u, &ChronicleFilter::default(), &existing).unwrap_err();
        assert!(err.contains("existing"), "{err}");
        assert_eq!(std::fs::read(existing.join("keep.txt")).unwrap(), b"mine");
        assert!(!existing.join("manifest.json").exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Minimal RFC-4180 reader used to prove the writer round-trips free text exactly.
    fn parse_csv(text: &str) -> Vec<Vec<String>> {
        let mut records = Vec::new();
        let mut fields: Vec<String> = Vec::new();
        let mut field = String::new();
        let mut chars = text.chars().peekable();
        let mut in_quotes = false;
        while let Some(ch) = chars.next() {
            if in_quotes {
                match ch {
                    '"' => {
                        if chars.peek() == Some(&'"') {
                            chars.next();
                            field.push('"');
                        } else {
                            in_quotes = false;
                        }
                    }
                    c => field.push(c),
                }
            } else {
                match ch {
                    '"' => in_quotes = true,
                    ',' => fields.push(std::mem::take(&mut field)),
                    '\r' => {}
                    '\n' => {
                        fields.push(std::mem::take(&mut field));
                        records.push(std::mem::take(&mut fields));
                    }
                    c => field.push(c),
                }
            }
        }
        records
    }

    #[test]
    fn chronicle_csv_escapes_free_text() {
        let tricky = Event {
            time: 42.0,
            category: Category::Contact,
            importance: 5,
            title: "They said \"hello, world\"".into(),
            detail: "Line one\nline two, with a comma and a \"quoted\" phrase.".into(),
            system: Some(0),
            body: None,
            civ: None,
            cause: None,
        };
        let plain = Event {
            time: 7.0,
            category: Category::Life,
            importance: 2,
            title: "Simple title".into(),
            detail: "Simple detail".into(),
            system: None,
            body: None,
            civ: None,
            cause: None,
        };
        let u = world(&[tricky.clone(), plain.clone()]);
        let text = csv(&rows(&u, &ChronicleFilter::default()));

        // Quoted fields double their quotes; unquoted fields stay bare.
        assert!(text.contains("\"They said \"\"hello, world\"\"\""), "{text}");
        assert!(text.contains("\"Line one\nline two, with a comma and a \"\"quoted\"\" phrase.\""));
        assert!(text.contains(",Simple title,Simple detail,,,"), "plain fields stay unquoted");
        // RFC-4180 records are CRLF-terminated.
        assert!(text.ends_with("\r\n"));
        assert!(text.contains(",Simple title,Simple detail,,,\r\n"), "unquoted row ends with CRLF");

        let parsed = parse_csv(&text);
        assert_eq!(parsed.len(), 3, "header plus two rows");
        assert_eq!(
            parsed[0],
            ["time_s", "date", "category", "importance", "title", "detail", "system", "body", "civ"]
        );
        // Chronological order puts the earlier plain event first; its `None` ids are empty cells.
        assert_eq!(parsed[1][4], plain.title);
        assert_eq!(parsed[1][5], plain.detail);
        assert_eq!(parsed[1][6], "");
        assert_eq!(parsed[1][7], "");
        assert_eq!(parsed[1][8], "");
        // The tricky event round-trips exactly through quoting, with system 0 preserved.
        assert_eq!(parsed[2][4], tricky.title);
        assert_eq!(parsed[2][5], tricky.detail);
        assert_eq!(parsed[2][6], "0");
        assert_eq!(parsed[2][7], "");
        assert_eq!(parsed[2][8], "");
    }
}
