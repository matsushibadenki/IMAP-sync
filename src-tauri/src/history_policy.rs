//! Retention applies only to completed report files, never pair recovery records.
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

pub fn days(value: &Value) -> Result<u64, String> {
    value
        .as_u64()
        .filter(|d| matches!(d, 0 | 30 | 90 | 180 | 365))
        .ok_or_else(|| "HISTORY_POLICY_INVALID".into())
}
pub fn load(dir: &Path) -> Result<u64, String> {
    let path = dir.join("settings").join("history-policy.json");
    match std::fs::read(path) {
        Ok(bytes) => {
            let value: Value =
                serde_json::from_slice(&bytes).map_err(|_| "HISTORY_POLICY_INVALID")?;
            if value["version"] != 1 {
                return Err("HISTORY_POLICY_INVALID".into());
            }
            days(&value["days"])
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(_) => Err("HISTORY_POLICY_FAILED".into()),
    }
}
fn policy_lock(dir: &Path) -> Result<crate::file_lock::Guard, String> {
    std::fs::create_dir_all(dir.join("settings")).map_err(|_| "HISTORY_POLICY_FAILED")?;
    crate::file_lock::acquire(&dir.join("settings").join("history-policy.lock"))
        .map_err(|_| "HISTORY_POLICY_FAILED".into())
}
fn eligible(dir: &Path, record: &Value, retention: u64, now: DateTime<Utc>) -> bool {
    if !crate::results::deletable(&crate::results::report(record, false)) {
        return false;
    }
    if retention == 0 {
        return true;
    }
    // Old reports did not record completion time. Their file modification time is
    // conservative: a long-running transfer is never aged from its start time.
    let timestamp = record
        .get("finishedAt")
        .or_else(|| record.get("interruptionDetectedAt"));
    let completed = if let Some(value) = timestamp {
        value
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
    } else {
        record["id"]
            .as_str()
            .and_then(|id| crate::batch_recovery::read(dir, id).ok())
            .and_then(|(path, _)| std::fs::metadata(path).ok()?.modified().ok())
            .map(DateTime::<Utc>::from)
    };
    completed.is_some_and(|date| date < now - Duration::days(retention as i64))
}
pub fn preview(dir: &Path, retention: u64, now: DateTime<Utc>) -> Value {
    let ids: Vec<String> = crate::batch_recovery::history(dir)
        .iter()
        .filter(|r| eligible(dir, r, retention, now))
        .filter_map(|r| r["id"].as_str().map(str::to_owned))
        .collect();
    json!({"ids":ids,"count":ids.len(),"days":retention})
}
fn remove(dir: &Path, ids: &[String], retention: u64, now: DateTime<Utc>) -> Value {
    let (mut deleted, mut skipped, mut failed) = (0, 0, 0);
    for id in ids {
        let Ok((path, _)) = crate::batch_recovery::read(dir, id) else {
            skipped += 1;
            continue;
        };
        let _guard = match crate::batch_recovery::lock(&path) {
            Ok(guard) => guard,
            Err(error) => {
                if error == "RECOVERY_PAIR_BUSY" {
                    skipped += 1;
                } else {
                    failed += 1;
                }
                continue;
            }
        };
        let Ok((_, record)) = crate::batch_recovery::read(dir, id) else {
            skipped += 1;
            continue;
        };
        if !eligible(dir, &record, retention, now) {
            skipped += 1;
            continue;
        }
        if std::fs::remove_file(path).is_ok() {
            deleted += 1;
        } else {
            failed += 1;
        }
    }
    json!({"deleted":deleted,"skipped":skipped,"failed":failed})
}
/// Only explicitly previewed IDs are removed. Records created after confirmation
/// preparation are never swept as a side effect of saving a policy.
pub fn commit(dir: &Path, body: &Value, now: DateTime<Utc>) -> Result<Value, String> {
    let retention = days(&body["days"])?;
    let values = body["ids"].as_array().ok_or("HISTORY_POLICY_INVALID")?;
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for value in values {
        let id = value.as_str().ok_or("HISTORY_POLICY_INVALID")?;
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("HISTORY_POLICY_INVALID".into());
        }
        if seen.insert(id) {
            ids.push(id.to_owned());
        }
    }
    let _guard = policy_lock(dir)?;
    if body["savePolicy"] == true {
        crate::oauth::write_private(
            &dir.join("settings").join("history-policy.json"),
            &serde_json::to_vec(&json!({"version":1,"days":retention})).unwrap(),
        )
        .map_err(|_| "HISTORY_POLICY_FAILED")?;
    }
    if body["savePolicy"] == true && retention == 0 {
        return Ok(json!({"deleted":0,"skipped":0,"failed":0}));
    }
    Ok(remove(dir, &ids, retention, now))
}
pub fn sweep(dir: &Path, now: DateTime<Utc>) -> Result<Value, String> {
    let _guard = policy_lock(dir)?;
    let retention = load(dir)?;
    if retention == 0 {
        return Ok(json!({"deleted":0,"skipped":0,"failed":0}));
    }
    let preview = preview(dir, retention, now);
    let ids = preview["ids"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    Ok(remove(dir, &ids, retention, now))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn completed(dir: &Path, age: i64, kind: &str) -> (std::path::PathBuf, Value) {
        let body =
            json!({"user1":"a","user2":"b","accounts1":[{"user":"a"}],"accounts2":[{"user":"b"}]});
        let (path, mut record) = if kind == "single" {
            crate::batch_recovery::create_single(dir, &body).unwrap()
        } else {
            crate::batch_recovery::create(dir, &body).unwrap()
        };
        record["resolved"] = json!(true);
        record["results"] = json!([{"row":1,"exitCode":0}]);
        record["finishedAt"] = json!((Utc::now() - Duration::days(age)).to_rfc3339());
        crate::batch_recovery::save(&path, &record).unwrap();
        (path, record)
    }
    #[test]
    fn retention_persists_and_preserves_recent_unfinished_locked_and_recovery_records() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        assert_eq!(load(dir).unwrap(), 0);
        let (old, _) = completed(dir, 91, "single");
        let (recent, _) = completed(dir, 2, "batch");
        let (unfinished, mut pending) = completed(dir, 100, "batch");
        pending["resolved"] = json!(false);
        crate::batch_recovery::save(&unfinished, &pending).unwrap();
        let (locked, _) = completed(dir, 100, "batch");
        let guard = crate::batch_recovery::lock(&locked).unwrap();
        let pair = crate::recovery::mark(
            dir,
            &json!({"host1":"source","host2":"dest","user1":"a","user2":"b"}),
        )
        .unwrap();
        let snapshot = preview(dir, 90, Utc::now());
        let (new, _) = completed(dir, 100, "single");
        let result = commit(
            dir,
            &json!({"days":90,"ids":snapshot["ids"],"savePolicy":true}),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(result["deleted"], 1);
        assert_eq!(result["skipped"], 1);
        assert_eq!(load(dir).unwrap(), 90);
        assert_eq!(crate::recovery::pending(dir).len(), 1);
        assert!(!old.exists());
        for path in [&recent, &unfinished, &locked, &pair, &new] {
            assert!(path.exists());
        }
        drop(guard);
        assert_eq!(sweep(dir, Utc::now()).unwrap()["deleted"], 2);
        assert!(recent.exists());
        assert!(pair.exists());
        assert!(unfinished.exists());
        commit(
            dir,
            &json!({"days":0,"ids":[],"savePolicy":true}),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(
            sweep(dir, Utc::now() + Duration::days(1000)).unwrap()["deleted"],
            0
        );
    }
    #[test]
    fn bulk_rechecks_state_and_rejects_invalid_requests_before_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let (path, mut record) = completed(dir, 100, "batch");
        let snapshot = preview(dir, 0, Utc::now());
        assert_eq!(snapshot["count"], 1);
        assert!(commit(
            dir,
            &json!({"days":7,"ids":snapshot["ids"],"savePolicy":true}),
            Utc::now()
        )
        .is_err());
        assert!(commit(
            dir,
            &json!({"days":0,"ids":[record["id"],"../../bad"]}),
            Utc::now()
        )
        .is_err());
        assert!(path.exists());
        assert_eq!(load(dir).unwrap(), 0);
        assert_eq!(
            commit(
                dir,
                &json!({"days":0,"ids":snapshot["ids"],"savePolicy":true}),
                Utc::now()
            )
            .unwrap()["deleted"],
            0
        );
        assert!(path.exists());
        record["resolved"] = json!(false);
        crate::batch_recovery::save(&path, &record).unwrap();
        assert_eq!(
            commit(dir, &json!({"days":0,"ids":snapshot["ids"]}), Utc::now()).unwrap()["skipped"],
            1
        );
        record["resolved"] = json!(true);
        record["finishedAt"] = json!("bad-date");
        crate::batch_recovery::save(&path, &record).unwrap();
        assert_eq!(preview(dir, 30, Utc::now())["count"], 0);
        assert_eq!(preview(dir, 0, Utc::now())["count"], 1);
        assert_eq!(
            commit(dir, &json!({"days":0,"ids":snapshot["ids"]}), Utc::now()).unwrap()["deleted"],
            1
        );
        assert_eq!(
            commit(dir, &json!({"days":0,"ids":snapshot["ids"]}), Utc::now()).unwrap()["skipped"],
            1
        );
    }
    #[test]
    fn age_uses_completion_or_legacy_mtime_and_corrupt_policy_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();
        let (path, mut record) = completed(dir, 1, "batch");
        let now = Utc::now();
        record["startedAt"] = json!((now - Duration::days(500)).to_rfc3339());
        record["finishedAt"] = json!((now - Duration::days(30)).to_rfc3339());
        crate::batch_recovery::save(&path, &record).unwrap();
        assert_eq!(preview(dir, 30, now)["count"], 0);
        assert_eq!(preview(dir, 30, now + Duration::seconds(1))["count"], 1);
        record.as_object_mut().unwrap().remove("finishedAt");
        crate::batch_recovery::save(&path, &record).unwrap();
        assert_eq!(preview(dir, 30, now)["count"], 0);
        std::fs::File::open(&path)
            .unwrap()
            .set_modified(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
        assert_eq!(preview(dir, 30, now)["count"], 1);
        std::fs::create_dir_all(dir.join("settings")).unwrap();
        std::fs::write(dir.join("settings").join("history-policy.json"), b"broken").unwrap();
        assert!(sweep(dir, now).is_err());
        assert!(path.exists());
        for value in [json!(-1), json!("30"), json!(31), Value::Null] {
            assert!(days(&value).is_err());
        }
        for value in [0, 30, 90, 180, 365] {
            assert_eq!(days(&json!(value)).unwrap(), value);
        }
    }
}
