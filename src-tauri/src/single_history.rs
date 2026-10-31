//! Single-account attempt history is independent of mailbox-pair recovery state.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
pub struct Run {
    pub record: Value,
    pub path: Option<PathBuf>,
    guard: Option<crate::file_lock::Guard>,
}
impl Run {
    pub fn start(directory: Option<&Path>, body: &Value) -> Result<Self, String> {
        if let Some(dir) = directory {
            let (path, record) = crate::batch_recovery::create_single(dir, body)?;
            let guard = crate::batch_recovery::lock(&path)?;
            Ok(Self {
                record,
                path: Some(path),
                guard: Some(guard),
            })
        } else {
            Ok(Self {
                record: json!({"version":1,"id":"current","kind":"single","startedAt":chrono::Utc::now().to_rfc3339(),"settings":crate::batch_recovery::single_settings(body),"results":[],"activeRow":1,"resolved":false,"transferred":0,"errors":null}),
                path: None,
                guard: None,
            })
        }
    }
    fn save(&self) -> Result<(), String> {
        if let Some(path) = &self.path {
            crate::batch_recovery::save(path, &self.record)
        } else {
            Ok(())
        }
    }
    pub fn attach_pid(&mut self, pid: u32) -> Result<(), String> {
        self.record["pid"] = json!(pid);
        self.save()
    }
    pub fn finish(
        &mut self,
        code: i32,
        cancelled: bool,
        interrupted: bool,
        transferred: u64,
        errors: Option<u64>,
    ) -> Result<(), String> {
        self.record["results"] = json!([{"row":1,"exitCode":code,"cancelled":cancelled,"interrupted":interrupted,"transferred":transferred,"errors":errors}]);
        self.record["resolved"] = json!(true);
        self.record["finishedAt"] = json!(chrono::Utc::now().to_rfc3339());
        self.record["transferred"] = json!(transferred);
        self.record["errors"] = json!(errors);
        let result = self.save();
        self.guard = None;
        result
    }
    pub fn view(&self, running: bool, transferred: u64, errors: Option<u64>) -> Value {
        let mut value = self.record.clone();
        value["running"] = json!(running);
        if running {
            value["transferred"] = json!(transferred);
            value["errors"] = json!(errors);
        }
        value
    }
}
/// Detect an interrupted attempt only after its owner lock and recorded PID are gone.
/// Pair recovery markers are never modified here.
pub fn reconcile(path: &Path, mut record: Value) -> Value {
    if record["kind"] != "single" || record["resolved"] == true || record["pid"].as_u64().is_none()
    {
        return record;
    }
    let _guard = match crate::batch_recovery::lock(path) {
        Ok(guard) => guard,
        Err(error) => {
            if error == "RECOVERY_PAIR_BUSY" {
                record["running"] = json!(true);
            }
            return record;
        }
    };
    let Ok(fresh) = std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .ok_or(())
    else {
        return record;
    };
    if fresh["resolved"] == true {
        return fresh;
    }
    if fresh["id"] != record["id"]
        || fresh["kind"] != "single"
        || !fresh["pid"]
            .as_u64()
            .is_some_and(|pid| pid > 0 && pid <= u32::MAX as u64)
    {
        return fresh;
    }
    if let Err(error) = crate::recovery::ensure_previous_exited(path) {
        let mut fresh = fresh;
        if error == "RECOVERY_PAIR_BUSY" {
            fresh["running"] = json!(true);
        }
        return fresh;
    }
    record = fresh;
    record["results"] = json!([{"row":1,"exitCode":null,"cancelled":false,"interrupted":true,"transferred":null,"errors":null}]);
    record["resolved"] = json!(true);
    record["interruptionDetectedAt"] = json!(chrono::Utc::now().to_rfc3339());
    record["transferred"] = Value::Null;
    record["errors"] = Value::Null;
    if crate::batch_recovery::save(path, &record).is_err() {
        record["resolved"] = json!(false);
    }
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body() -> Value {
        json!({"host1":"source.test","host2":"dest.test","user1":"source@example.test","user2":"dest@example.test","pass1":"secret-source","pass2":"secret-dest","access_token":"secret-token","clientSecret":"secret-client","dryRun":true})
    }
    #[test]
    fn single_history_survives_restart_without_credentials_or_batch_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let mut run = Run::start(Some(dir.path()), &body()).unwrap();
        let id = run.record["id"].as_str().unwrap().to_string();
        run.finish(0, false, false, 7, Some(0)).unwrap();
        drop(run);
        let (_, record) = crate::batch_recovery::read(dir.path(), &id).unwrap();
        assert_eq!(record["kind"], "single");
        assert_eq!(record["settings"]["multipleAccounts"], false);
        assert!(crate::batch_recovery::pending(dir.path()).is_empty());
        let text = serde_json::to_string(&record).unwrap();
        for secret in [
            "secret-source",
            "secret-dest",
            "secret-token",
            "secret-client",
        ] {
            assert!(!text.contains(secret));
        }
        let report = crate::results::report(&record, false);
        assert_eq!(report["accountMode"], "single");
        assert_eq!(report["rows"].as_array().unwrap().len(), 1);
        assert_eq!(report["rows"][0]["transferred"], 7);
        assert_eq!(report["dryRun"], true);
        let json: Value =
            serde_json::from_slice(&crate::results::export(&record, "json").unwrap()).unwrap();
        assert_eq!(json["rows"][0]["source"], "source@example.test");
        let csv = String::from_utf8(crate::results::export(&record, "csv").unwrap()).unwrap();
        assert!(csv.contains("account_mode"));
        assert!(csv.contains("\"single\""));
    }
    #[test]
    fn stopped_single_archive_is_deletable_without_removing_pair_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let pair = crate::recovery::mark(dir.path(), &body()).unwrap();
        let mut run = Run::start(Some(dir.path()), &body()).unwrap();
        let path = run.path.clone().unwrap();
        let id = run.record["id"].as_str().unwrap().to_string();
        assert!(crate::batch_recovery::delete_history(dir.path(), &id).is_err());
        run.finish(-1, true, true, 2, None).unwrap();
        assert_eq!(
            crate::results::report(&run.record, false)["rows"][0]["status"],
            "stopped"
        );
        crate::batch_recovery::delete_history(dir.path(), &id).unwrap();
        assert!(!path.exists());
        assert!(pair.exists());
    }
    #[test]
    #[cfg(unix)]
    fn interrupted_attempt_is_reconciled_only_when_its_lock_and_pid_are_gone() {
        let dir = tempfile::tempdir().unwrap();
        let mut run = Run::start(Some(dir.path()), &body()).unwrap();
        let path = run.path.clone().unwrap();
        let mut child = std::process::Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap();
        run.attach_pid(child.id()).unwrap();
        child.wait().unwrap();
        let value = reconcile(&path, run.record.clone());
        assert_eq!(value["resolved"], false);
        assert_eq!(value["running"], true);
        let initial = run.record.clone();
        drop(run);
        let value = reconcile(&path, initial);
        assert_eq!(value["resolved"], true);
        assert_eq!(value["results"][0]["interrupted"], true);
        assert!(value["results"][0]["exitCode"].is_null());
    }
}
