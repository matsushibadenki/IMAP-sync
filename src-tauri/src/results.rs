//! Whitelisted migration reports and explicit failed-row retry selection.
use serde_json::{json, Value};
use std::collections::HashSet;

pub fn report(record: &Value, running: bool) -> Value {
    let running = running || record["running"] == true;
    let settings = &record["settings"];
    let mut rows = Vec::new();
    for (index, source) in settings["accounts1"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let destination = &settings["accounts2"][index];
        let src = source["user"].as_str().unwrap_or("");
        let dst = destination["user"].as_str().unwrap_or("");
        if src.is_empty() && dst.is_empty() {
            continue;
        }
        let row = index + 1;
        let result = record["results"]
            .as_array()
            .into_iter()
            .flatten()
            .rev()
            .find(|r| r["row"] == row);
        let status = if let Some(r) = result {
            if r["interrupted"] == true || r["cancelled"] == true {
                "stopped"
            } else if r["exitCode"] == 0 {
                "success"
            } else {
                "failed"
            }
        } else if running && record["activeRow"] == row {
            "running"
        } else {
            "pending"
        };
        rows.push(json!({"row":row,"source":src,"destination":dst,"status":status,"exitCode":result.and_then(|r|r["exitCode"].as_i64()),"transferred":result.and_then(|r|r["transferred"].as_u64()),"errors":result.and_then(|r|r["errors"].as_u64())}));
    }
    json!({"version":1,"accountMode":if record["kind"]=="single" {"single"}else{"multiple"},"id":record["id"].as_str().unwrap_or("current"),"retryOf":record["retryOf"].as_str(),"startedAt":record["startedAt"].as_str(),"dryRun":settings["dryRun"]==true,"running":running,"resolved":record["resolved"]==true,"sourceHost":settings["host1"].as_str().unwrap_or(""),"destinationHost":settings["host2"].as_str().unwrap_or(""),"transferred":record["transferred"].as_u64(),"errors":record["errors"].as_u64(),"rows":rows})
}
pub fn retry_settings(record: &Value, selected: &Value) -> Result<Value, String> {
    let selected = selected
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 1000)
        .ok_or("RESULTS_SELECTION")?;
    let mut rows = HashSet::new();
    let view = report(record, false);
    for value in selected {
        let row = value
            .as_u64()
            .filter(|n| *n > 0 && *n <= 1000)
            .ok_or("RESULTS_SELECTION")? as usize;
        if !rows.insert(row)
            || !view["rows"].as_array().unwrap().iter().any(|r| {
                r["row"] == row && matches!(r["status"].as_str(), Some("failed" | "stopped"))
            })
        {
            return Err("RESULTS_SELECTION".into());
        }
    }
    let mut settings = crate::batch_recovery::clean(&record["settings"]);
    for side in ["1", "2"] {
        for (index, account) in settings[format!("accounts{side}")]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            if !rows.contains(&(index + 1)) {
                account["user"] = json!("");
            }
        }
    }
    settings["dryRun"] = json!(false);
    Ok(settings)
}
fn csv_cell(text: &str) -> String {
    let leading = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    let prefix = if leading
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'))
    {
        "'"
    } else {
        ""
    };
    format!("\"{}{}\"", prefix, text.replace('"', "\"\""))
}
pub fn export(record: &Value, format: &str) -> Result<Vec<u8>, String> {
    let view = report(record, false);
    match format {
        "json" => serde_json::to_vec_pretty(&view).map_err(|_| "RESULTS_EXPORT_FAILED".into()),
        "csv" => Ok(csv_reports(&[view])),
        _ => Err("RESULTS_FORMAT".into()),
    }
}
fn csv_reports(reports: &[Value]) -> Vec<u8> {
    let mut text=String::from("\u{feff}run_id,started_at,dry_run,source_host,destination_host,row,source_user,destination_user,status,exit_code,transferred,errors,account_mode\r\n");
    for view in reports {
        for row in view["rows"].as_array().unwrap() {
            let values = [
                &view["id"],
                &view["startedAt"],
                &view["dryRun"],
                &view["sourceHost"],
                &view["destinationHost"],
                &row["row"],
                &row["source"],
                &row["destination"],
                &row["status"],
                &row["exitCode"],
                &row["transferred"],
                &row["errors"],
                &view["accountMode"],
            ];
            text.push_str(
                &values
                    .iter()
                    .map(|v| {
                        csv_cell(&if v.is_null() {
                            String::new()
                        } else if let Some(s) = v.as_str() {
                            s.into()
                        } else {
                            v.to_string()
                        })
                    })
                    .collect::<Vec<_>>()
                    .join(","),
            );
            text.push_str("\r\n");
        }
    }
    text.into_bytes()
}
/// Export the exact filter semantics used by search, without its page limit.
/// Inputs are whitelisted reports produced by `report`, never raw checkpoints.
pub fn export_matching(
    reports: Vec<Value>,
    filters: &Value,
    format: &str,
) -> Result<(Vec<u8>, usize), String> {
    if !matches!(format, "csv" | "json") {
        return Err("RESULTS_FORMAT".into());
    }
    let reports = matching(reports, filters)?;
    if reports.is_empty() {
        return Err("RESULTS_EXPORT_EMPTY".into());
    }
    let count = reports.len();
    let bytes = if format == "csv" {
        csv_reports(&reports)
    } else {
        let row_count: usize = reports
            .iter()
            .map(|r| r["rows"].as_array().map_or(0, Vec::len))
            .sum();
        serde_json::to_vec_pretty(&json!({"version":1,"exportedAt":chrono::Utc::now().to_rfc3339(),"filters":{"query":filters["query"].as_str().unwrap_or("").trim(),"status":filters["status"].as_str().unwrap_or("all"),"mode":filters["mode"].as_str().unwrap_or("all")},"runCount":count,"rowCount":row_count,"reports":reports})).map_err(|_| "RESULTS_EXPORT_FAILED")?
    };
    Ok((bytes, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"id":"example","settings":{"host1":"source","host2":"dest","pass1":"secret","accounts1":[{"user":"one","password":"secret"},{"user":" =HYPERLINK(\"bad\")","password":"hidden"},{"user":"three"}],"accounts2":[{"user":"a"},{"user":"b"},{"user":"c"}]},"results":[{"row":1,"exitCode":0},{"row":2,"exitCode":161}],"access_token":"token-secret","logs":["private-log"]})
    }
    #[test]
    fn only_failed_rows_can_be_selected_and_original_is_unchanged() {
        let value = fixture();
        let draft = retry_settings(&value, &json!([2])).unwrap();
        assert_eq!(draft["accounts1"][0]["user"], "");
        assert_eq!(draft["accounts1"][1]["password"], "");
        for rows in [
            json!([]),
            json!([1]),
            json!([3]),
            json!([2, 2]),
            json!([999]),
            json!([-1]),
        ] {
            assert!(retry_settings(&value, &rows).is_err());
        }
        assert_eq!(value["results"][0]["exitCode"], 0);
    }
    #[test]
    fn exported_fields_exclude_credentials_logs_and_formula_execution() {
        let value = fixture();
        for format in ["csv", "json"] {
            let text = String::from_utf8(export(&value, format).unwrap()).unwrap();
            for secret in [
                "secret",
                "hidden",
                "private-log",
                "access_token",
                "password",
            ] {
                assert!(!text.contains(secret));
            }
        }
        let csv = String::from_utf8(export(&value, "csv").unwrap()).unwrap();
        assert!(csv.contains("\"' =HYPERLINK(\"\"bad\"\")\""));
        assert!(csv.starts_with('\u{feff}'));
        assert!(export(&value, "xml").is_err());
    }
    #[test]
    fn matching_exports_all_pages_with_search_semantics_and_complete_account_rows() {
        let reports = (0..65)
            .map(|i| {
                let mut record = fixture();
                record["id"] = json!(format!("{i:032x}"));
                record["startedAt"] = json!("2026-10-31T00:00:00Z");
                record["settings"]["accounts1"][0]["user"] = json!("日本語@example.test");
                record["settings"]["dryRun"] = json!(i % 3 == 0);
                record["resolved"] = json!(i < 63);
                record["results"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"row":3,"exitCode":0}));
                if i % 5 == 0 {
                    record["results"][1]["exitCode"] = json!(0);
                }
                if i % 2 == 1 {
                    record["kind"] = json!("single");
                    record["settings"]["accounts1"] =
                        json!([{"user":"日本語@example.test","password":"hidden"}]);
                    record["settings"]["accounts2"] = json!([{"user":"destination@test"}]);
                    record["results"] = json!([{"row":1,"exitCode":if i%5==0 {0} else {161}}]);
                }
                if i >= 63 {
                    record["results"] = json!([]);
                    record["activeRow"] = json!(1);
                }
                report(&record, i == 64)
            })
            .collect::<Vec<_>>();
        for status in ["all", "success", "failed", "unfinished", "running"] {
            for mode in ["all", "sync", "dry"] {
                let filters = json!({"query":"SOURCE 日本語","status":status,"mode":mode,"offset":50,"limit":1,"password":"hidden","lang":"ja"});
                let first = search(
                    reports.clone(),
                    &json!({"query":"SOURCE 日本語","status":status,"mode":mode,"limit":7}),
                )
                .unwrap();
                let total = first["total"].as_u64().unwrap() as usize;
                if total == 0 {
                    assert_eq!(
                        export_matching(reports.clone(), &filters, "json").unwrap_err(),
                        "RESULTS_EXPORT_EMPTY"
                    );
                    continue;
                }
                let (bytes, count) = export_matching(reports.clone(), &filters, "json").unwrap();
                let json: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(count, total);
                assert_eq!(json["runCount"], total);
                assert_eq!(json["reports"].as_array().unwrap().len(), total);
                assert_eq!(
                    json["filters"],
                    json!({"query":"SOURCE 日本語","status":status,"mode":mode})
                );
                let mut expected = Vec::new();
                for offset in (0..total).step_by(7) {
                    let page=search(reports.clone(),&json!({"query":"SOURCE 日本語","status":status,"mode":mode,"limit":7,"offset":offset})).unwrap();
                    expected.extend(
                        page["reports"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|r| r["id"].clone()),
                    );
                }
                assert_eq!(
                    json["reports"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r["id"].clone())
                        .collect::<Vec<_>>(),
                    expected
                );
                let row_count: usize = json["reports"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r["rows"].as_array().unwrap().len())
                    .sum();
                assert_eq!(json["rowCount"], row_count);
                let (csv, csv_count) = export_matching(reports.clone(), &filters, "csv").unwrap();
                assert_eq!(csv_count, count);
                let csv = String::from_utf8(csv).unwrap();
                assert!(csv.starts_with('\u{feff}'));
                assert_eq!(csv.matches("run_id,started_at").count(), 1);
                assert_eq!(csv.split("\r\n").count(), row_count + 2);
                for text in [String::from_utf8(bytes).unwrap(), csv] {
                    for secret in [
                        "hidden",
                        "secret",
                        "password",
                        "access_token",
                        "private-log",
                        "canDelete",
                    ] {
                        assert!(!text.contains(secret));
                    }
                }
            }
        }
        let (csv, _) =
            export_matching(reports.clone(), &json!({"status":"failed"}), "csv").unwrap();
        assert!(String::from_utf8(csv)
            .unwrap()
            .contains("\"' =HYPERLINK(\"\"bad\"\")\""));
        assert_eq!(
            export_matching(reports.clone(), &json!({"query":"no-such-run"}), "csv").unwrap_err(),
            "RESULTS_EXPORT_EMPTY"
        );
        assert_eq!(
            export_matching(reports.clone(), &json!({"status":"invalid"}), "json").unwrap_err(),
            "RESULTS_FILTER"
        );
        assert_eq!(
            export_matching(reports, &json!({}), "xml").unwrap_err(),
            "RESULTS_FORMAT"
        );
    }
}

/// Filters apply to whole runs; a selected run always retains all of its rows.
pub fn deletable(report: &Value) -> bool {
    report["resolved"] == true
        && report["running"] != true
        && report["rows"].as_array().is_some_and(|rows| {
            !rows.is_empty()
                && rows.iter().all(|r| {
                    matches!(r["status"].as_str(), Some("success" | "failed"))
                        || (report["accountMode"] == "single" && r["status"] == "stopped")
                })
        })
}
fn matching(mut reports: Vec<Value>, query: &Value) -> Result<Vec<Value>, String> {
    let text = query["query"].as_str().unwrap_or("").trim().to_lowercase();
    if text.chars().count() > 200 {
        return Err("RESULTS_FILTER".into());
    }
    let status = query["status"].as_str().unwrap_or("all");
    let mode = query["mode"].as_str().unwrap_or("all");
    if !["all", "success", "failed", "unfinished", "running"].contains(&status)
        || !["all", "sync", "dry"].contains(&mode)
    {
        return Err("RESULTS_FILTER".into());
    }
    reports.retain(|report| {
        let rows = report["rows"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        let status_matches = match status {
            "success" => {
                report["resolved"] == true
                    && !rows.is_empty()
                    && rows.iter().all(|r| r["status"] == "success")
            }
            "failed" => rows
                .iter()
                .any(|r| matches!(r["status"].as_str(), Some("failed" | "stopped"))),
            "unfinished" => report["resolved"] != true && report["running"] != true,
            "running" => report["running"] == true,
            _ => true,
        };
        let mode_matches = match mode {
            "sync" => report["dryRun"] != true,
            "dry" => report["dryRun"] == true,
            _ => true,
        };
        let mut searchable = [
            "id",
            "retryOf",
            "startedAt",
            "sourceHost",
            "destinationHost",
        ]
        .iter()
        .filter_map(|key| report[*key].as_str())
        .collect::<Vec<_>>();
        for row in rows {
            searchable.extend([
                row["source"].as_str().unwrap_or(""),
                row["destination"].as_str().unwrap_or(""),
            ]);
        }
        let haystack = searchable.join(" ").to_lowercase();
        status_matches
            && mode_matches
            && text.split_whitespace().all(|word| haystack.contains(word))
    });
    reports.sort_by(|a, b| {
        b["startedAt"]
            .as_str()
            .cmp(&a["startedAt"].as_str())
            .then_with(|| b["id"].as_str().cmp(&a["id"].as_str()))
    });
    Ok(reports)
}
pub fn search(reports: Vec<Value>, query: &Value) -> Result<Value, String> {
    let mut reports = matching(reports, query)?;
    let limit = query["limit"].as_u64().unwrap_or(20).clamp(1, 50) as usize;
    let requested = query["offset"].as_u64().unwrap_or(0).min(usize::MAX as u64) as usize;
    let total = reports.len();
    let offset = if total == 0 {
        0
    } else {
        requested.min((total - 1) / limit * limit)
    };
    for report in &mut reports {
        report["canDelete"] = json!(deletable(report));
    }
    let page = reports
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(json!({"reports":page,"total":total,"offset":offset,"limit":limit}))
}

#[cfg(test)]
mod history_tests {
    use super::*;
    fn run(id: &str, statuses: &[&str], dry: bool, resolved: bool, running: bool) -> Value {
        json!({"id":id,"startedAt":"2026-10-31T00:00:00Z","sourceHost":"IMAP.Example.test","destinationHost":"dest.test","dryRun":dry,"resolved":resolved,"running":running,"rows":statuses.iter().enumerate().map(|(i,s)|json!({"row":i+1,"source":"日本語@example.test","destination":"target@test","status":s})).collect::<Vec<_>>()})
    }
    #[test]
    fn filters_combine_and_search_only_visible_identifiers() {
        let reports = vec![
            run("a", &["success"], false, true, false),
            run("b", &["success", "failed"], false, true, false),
            run("c", &["stopped", "pending"], false, false, false),
            run("d", &["success"], true, true, false),
            run("e", &["running"], false, false, true),
        ];
        let successful = search(
            reports.clone(),
            &json!({"status":"success","mode":"sync","query":"EXAMPLE 日本語"}),
        )
        .unwrap();
        assert_eq!(successful["total"], 1);
        assert_eq!(successful["reports"][0]["id"], "a");
        assert_eq!(
            search(reports.clone(), &json!({"status":"failed"})).unwrap()["total"],
            2
        );
        assert_eq!(
            search(reports.clone(), &json!({"status":"unfinished"})).unwrap()["total"],
            1
        );
        assert_eq!(
            search(reports.clone(), &json!({"status":"running"})).unwrap()["total"],
            1
        );
        assert_eq!(
            search(reports.clone(), &json!({"mode":"dry"})).unwrap()["total"],
            1
        );
        assert_eq!(
            search(reports.clone(), &json!({"query":"no-match"})).unwrap()["total"],
            0
        );
        assert!(search(reports.clone(), &json!({"status":"invalid"})).is_err());
        assert!(search(reports, &json!({"query":"x".repeat(201)})).is_err());
    }
    #[test]
    fn pagination_is_bounded_stable_and_recovers_after_last_page_deletion() {
        let reports = (0..45)
            .map(|i| run(&format!("{i:03}"), &["success"], false, true, false))
            .collect::<Vec<_>>();
        let page = search(reports.clone(), &json!({"limit":20,"offset":20})).unwrap();
        assert_eq!(page["reports"].as_array().unwrap().len(), 20);
        assert_eq!(page["total"], 45);
        assert_eq!(page["reports"][0]["id"], "024");
        let page = search(reports[..40].to_vec(), &json!({"limit":20,"offset":40})).unwrap();
        assert_eq!(page["offset"], 20);
        assert_eq!(page["reports"].as_array().unwrap().len(), 20);
        let page = search(reports, &json!({"limit":999})).unwrap();
        assert_eq!(page["limit"], 50);
        assert_eq!(search(vec![], &json!({"offset":999})).unwrap()["offset"], 0);
    }
    #[test]
    fn only_resolved_complete_runs_are_deletable() {
        assert!(deletable(&run(
            "a",
            &["success", "failed"],
            false,
            true,
            false
        )));
        for record in [
            run("a", &["success"], false, false, false),
            run("a", &["success"], false, true, true),
            run("a", &["stopped"], false, true, false),
            run("a", &["pending"], false, true, false),
            run("a", &[], false, true, false),
        ] {
            assert!(!deletable(&record));
        }
    }
}
