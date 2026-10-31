//! Credential-free interruption journal. Mark before spawn; clear only after success.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn identity(body: &Value) -> String {
    let mut parts = Vec::new();
    for side in ["1", "2"] {
        let text = |key: &str| {
            body[format!("{key}{side}")]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let host = text("host").to_lowercase();
        let user = if ["imap.gmail.com", "outlook.office365.com"].contains(&host.as_str()) {
            text("user").to_lowercase()
        } else {
            text("user")
        };
        let security = text("security");
        let port = text("port")
            .parse::<u16>()
            .unwrap_or(if security == "ssl" { 993 } else { 143 });
        parts.push(json!([host, port, security, user]));
    }
    format!("{:x}", Sha256::digest(serde_json::to_vec(&parts).unwrap()))
}
pub fn settings(body: &Value) -> Value {
    let mut result = json!({"multipleAccounts":false});
    for key in [
        "host1",
        "port1",
        "security1",
        "user1",
        "auth1",
        "provider1",
        "host2",
        "port2",
        "security2",
        "user2",
        "auth2",
        "provider2",
        "dryRun",
        "syncDeletes",
        "subscribeFolders",
        "automapFolders",
        "useUid",
        "fastMode",
        "providerSafeMode",
        "maxMessagesPerSecond",
        "maxSleepSeconds",
        "maxBytesAfterMb",
        "errorsMax",
        "timeoutSeconds",
        "stopAfterMb",
        "maxAgeDays",
        "minAgeDays",
        "bandwidthKb",
        "excludePattern",
        "advancedArgs",
    ] {
        let bool_key = [
            "dryRun",
            "syncDeletes",
            "subscribeFolders",
            "automapFolders",
            "useUid",
            "fastMode",
            "providerSafeMode",
        ]
        .contains(&key);
        result[key] = if bool_key {
            json!(body[key].as_bool().unwrap_or(key == "subscribeFolders"))
        } else {
            json!(body[key].as_str().unwrap_or(match key {
                "auth1" | "auth2" => "password",
                "provider1" | "provider2" => "custom",
                _ => "",
            }))
        };
    }
    result
}
pub fn path(directory: &Path, body: &Value) -> PathBuf {
    directory.join(format!("{}.json", identity(body)))
}
pub fn mark(directory: &Path, body: &Value) -> Result<PathBuf, String> {
    std::fs::create_dir_all(directory)
        .map_err(|e| format!("Cannot create recovery journal: {e}"))?;
    let path = path(directory, body);
    let data = json!({"version":1,"startedAt":chrono::Utc::now().to_rfc3339(),"settings":settings(body),"headerComparison":path.exists()});
    crate::oauth::write_private(
        &path,
        &serde_json::to_vec(&data).map_err(|e| e.to_string())?,
    )?;
    Ok(path)
}
pub fn pending(directory: &Path) -> Vec<Value> {
    let mut entries = Vec::new();
    if let Ok(files) = std::fs::read_dir(directory) {
        for file in files.flatten() {
            if file.path().extension().is_some_and(|s| s == "json") {
                let value = std::fs::read(file.path())
                    .ok()
                    .filter(|b| b.len() <= 262144)
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
                if let Some(value) =
                    value.filter(|v| v["version"] == 1 && v["settings"].is_object())
                {
                    if value["resolved"] != true {
                        entries.push(json!({"startedAt":value["startedAt"],"settings":settings(&value["settings"])}));
                    }
                } else {
                    entries.push(json!({"invalid":true}));
                }
            }
        }
    }
    entries.sort_by(|a, b| b["startedAt"].as_str().cmp(&a["startedAt"].as_str()));
    entries
}
pub fn lock(directory: &Path, body: &Value) -> Result<crate::file_lock::Guard, String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    crate::file_lock::acquire(&directory.join(format!("{}.lock", identity(body))))
}
pub fn attach_pid(path: &Path, pid: u32) -> Result<(), String> {
    let mut value: Value = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    value["pid"] = json!(pid);
    crate::oauth::write_private(
        path,
        &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    )
}
pub fn ensure_previous_exited(path: &Path) -> Result<(), String> {
    let value = std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    if let Some(pid) = value
        .and_then(|v| v["pid"].as_u64())
        .filter(|n| *n > 0 && *n <= u32::MAX as u64)
    {
        #[cfg(unix)]
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "RECOVERY_PROCESS_CHECK")?
            .success();
        #[cfg(windows)]
        let alive = {
            let output = std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
                .output()
                .map_err(|_| "RECOVERY_PROCESS_CHECK")?;
            if !output.status.success() {
                return Err("RECOVERY_PROCESS_CHECK".into());
            }
            String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\""))
        };
        if alive {
            return Err("RECOVERY_PAIR_BUSY".into());
        }
    }
    Ok(())
}
pub fn resolve(path: &Path) -> Result<(), String> {
    let mut value: Value = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    value["resolved"] = json!(true);
    value["headerComparison"] = json!(true);
    crate::oauth::write_private(
        path,
        &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    )
}
pub fn fallback(values: &mut Vec<String>) {
    // Getopt::Long applies the final boolean occurrence. Preserve earlier
    // values (including passwords/folder names that resemble option names)
    // and credential argument indexes instead of deleting matching strings.
    values.extend(["--nouseuid".into(), "--nousecache".into()]);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_survives_restart_without_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let body = json!({"host1":"source","host2":"dest","user1":"a","user2":"b","pass1":"secret-one","pass2":"secret-two","accounts1":[{"password":"nested-secret"}],"clientSecret":"client-secret","access_token":"token-secret"});
        let path = mark(dir.path(), &body).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        for secret in [
            "secret-one",
            "secret-two",
            "nested-secret",
            "client-secret",
            "token-secret",
        ] {
            assert!(!raw.contains(secret));
        }
        assert_eq!(pending(dir.path()).len(), 1);
        assert_eq!(
            pending(dir.path())[0]["settings"]["multipleAccounts"],
            false
        );
        let mut changed = body.clone();
        changed["pass1"] = json!("new-secret");
        changed["dryRun"] = json!(true);
        assert_eq!(identity(&body), identity(&changed));
        std::fs::write(&path, b"broken").unwrap();
        assert_eq!(pending(dir.path())[0]["invalid"], true);
    }
    #[test]
    fn pair_lock_excludes_other_instances_and_releases_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let body = json!({"host1":"a","host2":"b"});
        let first = lock(dir.path(), &body).unwrap();
        assert!(lock(dir.path(), &body).is_err());
        drop(first);
        assert!(lock(dir.path(), &body).is_ok());
    }

    #[test]
    fn resolved_recovery_keeps_safe_matching_without_pending_notice() {
        let dir = tempfile::tempdir().unwrap();
        let body = json!({"host1":"a","host2":"b"});
        let path = mark(dir.path(), &body).unwrap();
        resolve(&path).unwrap();
        assert!(pending(dir.path()).is_empty());
        assert!(path.exists());
        mark(dir.path(), &body).unwrap();
        assert_eq!(pending(dir.path()).len(), 1);
    }
    #[test]
    fn live_recorded_process_blocks_recovery_after_parent_lock_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let path = mark(dir.path(), &json!({})).unwrap();
        attach_pid(&path, std::process::id()).unwrap();
        assert_eq!(
            ensure_previous_exited(&path).unwrap_err(),
            "RECOVERY_PAIR_BUSY"
        );
    }
    #[test]
    fn fallback_overrides_uid_and_cache_options() {
        let mut a = vec![
            "--useuid".into(),
            "--usecache".into(),
            "--host1".into(),
            "source".into(),
        ];
        fallback(&mut a);
        assert_eq!(a[0], "--useuid");
        assert_eq!(a[1], "--usecache");
        assert_eq!(&a[a.len() - 2..], ["--nouseuid", "--nousecache"]);
    }
}
