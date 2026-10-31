//! Durable, credential-free queue checkpoints. Only Rust records decide which rows to skip.
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub fn clean(body: &Value) -> Value {
    let mut value = crate::recovery::settings(body);
    value["multipleAccounts"] = json!(true);
    for side in ["1", "2"] {
        value[format!("accounts{side}")] = json!(body[format!("accounts{side}")]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| json!({"user":a["user"].as_str().unwrap_or("").trim(),"password":""}))
            .collect::<Vec<_>>());
    }
    value
}
fn path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("BATCH_RECOVERY_INVALID".into());
    }
    Ok(dir.join("batches").join(format!("{id}.json")))
}
pub fn save(path: &Path, value: &Value) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("BATCH_RECOVERY_INVALID")?)
        .map_err(|e| e.to_string())?;
    crate::oauth::write_private(path, &serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
pub fn create(dir: &Path, body: &Value) -> Result<(PathBuf, Value), String> {
    create_record(dir, clean(body), "batch")
}
pub fn single_settings(body: &Value) -> Value {
    let mut settings = crate::recovery::settings(body);
    for side in ["1", "2"] {
        settings[format!("accounts{side}")] =
            json!([{"user":settings[format!("user{side}")],"password":""}]);
    }
    settings
}
pub fn create_single(dir: &Path, body: &Value) -> Result<(PathBuf, Value), String> {
    create_record(dir, single_settings(body), "single")
}
fn create_record(dir: &Path, settings: Value, kind: &str) -> Result<(PathBuf, Value), String> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|_| "BATCH_RECOVERY_INVALID")?;
    let id = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let value = json!({"version":1,"id":id,"kind":kind,"startedAt":chrono::Utc::now().to_rfc3339(),"settings":settings,"results":[],"transferred":0,"errors":0,"activeRow":null,"resolved":false});
    let path = path(dir, &id)?;
    let mut value = value;
    if kind == "single" {
        value["pid"] = json!(std::process::id());
        value["activeRow"] = json!(1);
    }
    save(&path, &value)?;
    Ok((path, value))
}
pub fn load(dir: &Path, id: &str) -> Result<(PathBuf, Value), String> {
    let result = read(dir, id)?;
    if result.1["resolved"] == true || result.1["kind"] != "batch" {
        return Err("BATCH_RECOVERY_INVALID".into());
    }
    Ok(result)
}
pub fn read(dir: &Path, id: &str) -> Result<(PathBuf, Value), String> {
    let path = path(dir, id)?;
    let bytes = std::fs::read(&path).map_err(|_| "BATCH_RECOVERY_INVALID")?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("BATCH_RECOVERY_INVALID".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "BATCH_RECOVERY_INVALID")?;
    if value["version"] != 1
        || !matches!(value["kind"].as_str(), Some("batch" | "single"))
        || value["id"] != id
        || !value["results"].is_array()
    {
        return Err("BATCH_RECOVERY_INVALID".into());
    }
    Ok((path, value))
}
pub fn completed(value: &Value) -> HashSet<usize> {
    value["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["interrupted"] != true)
        .filter_map(|r| r["row"].as_u64().map(|n| n as usize))
        .collect()
}
pub fn validate(value: &Value, body: &Value) -> Result<(), String> {
    let mut expected = clean(&value["settings"]);
    let mut actual = clean(body);
    expected["dryRun"] = json!(false);
    actual["dryRun"] = json!(false);
    if expected != actual {
        return Err("BATCH_RECOVERY_CHANGED".into());
    }
    Ok(())
}
pub fn pending(dir: &Path) -> Vec<Value> {
    let mut entries = Vec::new();
    if let Ok(files) = std::fs::read_dir(dir.join("batches")) {
        for file in files.flatten() {
            let path = file.path();
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.extension().is_some_and(|s| s == "json") {
                if let Ok((_, mut value)) = load(dir, id) {
                    value["settings"] = clean(&value["settings"]);
                    entries.push(value);
                }
            }
        }
    }
    entries.sort_by(|a, b| b["startedAt"].as_str().cmp(&a["startedAt"].as_str()));
    entries
}
pub fn history(dir: &Path) -> Vec<Value> {
    let mut records = Vec::new();
    if let Ok(files) = std::fs::read_dir(dir.join("batches")) {
        for file in files.flatten() {
            let path = file.path();
            if path.extension().is_some_and(|s| s == "json") {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok((_, value)) = read(dir, id) {
                        records.push(crate::single_history::reconcile(&path, value));
                    }
                }
            }
        }
    }
    records.sort_by(|a, b| b["startedAt"].as_str().cmp(&a["startedAt"].as_str()));
    records
}
/// Remove only a completed report, under the same lock used by queue resume.
/// Pair recovery markers and lock files are deliberately retained.
pub fn delete_history(dir: &Path, id: &str) -> Result<(), String> {
    let (path, _) = read(dir, id)?;
    let _guard = lock(&path)?;
    let (_, record) = read(dir, id)?;
    if !crate::results::deletable(&crate::results::report(&record, false)) {
        return Err("RESULTS_PROTECTED".into());
    }
    std::fs::remove_file(path).map_err(|_| "RESULTS_DELETE_FAILED".into())
}
pub fn lock(path: &Path) -> Result<crate::file_lock::Guard, String> {
    crate::file_lock::acquire(&path.with_extension("lock"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_excludes_credentials_and_rejects_changed_pairs() {
        let dir = tempfile::tempdir().unwrap();
        let body = json!({"accounts1":[{"user":"a","password":"source-secret"}],"accounts2":[{"user":"b","password":"dest-secret"}],"pass1":"single-secret","clientSecret":"oauth-secret"});
        let (path, value) = create(dir.path(), &body).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        for secret in [
            "source-secret",
            "dest-secret",
            "single-secret",
            "oauth-secret",
        ] {
            assert!(!raw.contains(secret));
        }
        let mut changed = body.clone();
        changed["accounts1"][0]["password"] = json!("new-password");
        changed["dryRun"] = json!(true);
        assert!(validate(&value, &changed).is_ok());
        changed["accounts2"][0]["user"] = json!("other");
        assert!(validate(&value, &changed).is_err());
        assert!(load(dir.path(), "../../bad").is_err());
    }
    #[test]
    fn completed_rows_require_durable_noninterrupted_results() {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut value) = create(dir.path(), &json!({})).unwrap();
        value["results"] = json!([{"row":1,"exitCode":0},{"row":2,"exitCode":-1,"interrupted":true},{"row":3,"exitCode":161}]);
        save(&path, &value).unwrap();
        let (_, loaded) = load(dir.path(), value["id"].as_str().unwrap()).unwrap();
        assert_eq!(completed(&loaded), HashSet::from([1, 3]));
        value["resolved"] = json!(true);
        save(&path, &value).unwrap();
        assert!(pending(dir.path()).is_empty());
    }
}
