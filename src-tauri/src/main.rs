#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Manager;
mod batch_recovery;
mod engine;
mod file_lock;
mod history_policy;
mod oauth;
mod recovery;
mod results;
mod single_history;
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
struct Document {
    child: Option<Child>,
    started: Option<String>,
    exit: Option<i32>,
    preview: String,
    logs: VecDeque<Value>,
    secrets: Vec<String>,
    draft: Value,
    run_dir: Option<tempfile::TempDir>,
    stopping: Option<Instant>,
    stop_grace: Duration,
    finished: Option<String>,
    cancelled: bool,
    dry_run: bool,
    transferred: u64,
    errors: Option<u64>,
    batch: Option<Batch>,
    single_history: Option<single_history::Run>,
    oauth: Arc<oauth::Manager>,
    microsoft_oauth: Option<Arc<oauth::Manager>>,
    auth_failed: bool,
    journal_dir: Option<PathBuf>,
    pair_lock: Option<file_lock::Guard>,
    hidden_result: Option<String>,
}
struct Batch {
    pending: VecDeque<(usize, Value)>,
    total: usize,
    current: usize,
    row: usize,
    completed: usize,
    failed: usize,
    transferred: u64,
    errors: Option<u64>,
    results: Vec<Value>,
    executable: PathBuf,
    cache: PathBuf,
    report: Value,
    checkpoint: Option<Value>,
    checkpoint_path: Option<PathBuf>,
    checkpoint_lock: Option<file_lock::Guard>,
}
type Shared = Arc<Mutex<Document>>;
fn string(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}
fn required(v: &Value, key: &str) -> Result<String, String> {
    let s = string(v, key);
    if s.trim().is_empty() {
        Err(format!("Required: {key}"))
    } else if s.contains(['\0', '\r', '\n']) {
        Err(format!("Invalid control character: {key}"))
    } else {
        Ok(s)
    }
}
fn number(v: &Value, key: &str) -> Option<f64> {
    let n = string(v, key).parse::<f64>().ok()?;
    (n.is_finite() && n > 0.0 && n < 1e12).then_some(n)
}
fn endpoint(v: &Value, suffix: &str) -> Result<(String, u16, String), String> {
    let host = required(v, &format!("host{suffix}"))?.trim().to_string();
    let mode = string(v, &format!("security{suffix}"));
    if !["ssl", "starttls", "plain"].contains(&mode.as_str()) {
        return Err("Invalid security mode".into());
    }
    let raw = string(v, &format!("port{suffix}"));
    let port = if raw.is_empty() {
        if mode == "ssl" {
            993
        } else {
            143
        }
    } else {
        raw.parse::<u16>().map_err(|_| "Invalid port")?
    };
    if port == 0 {
        return Err("Invalid port".into());
    }
    Ok((host, port, mode))
}
struct SyncArgs {
    values: Vec<String>,
    passwords: Vec<usize>,
    oauth_accounts: Vec<(usize, String, oauth::Provider)>,
}
impl std::ops::Deref for SyncArgs {
    type Target = [String];
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
fn args(v: &Value) -> Result<SyncArgs, String> {
    let mut a = Vec::new();
    let mut passwords = Vec::new();
    let mut oauth_accounts = Vec::new();
    let source = endpoint(v, "1")?;
    let destination = endpoint(v, "2")?;
    if source.0.eq_ignore_ascii_case(&destination.0)
        && source.1 == destination.1
        && (string(v, "user1") == string(v, "user2")
            || (["imap.gmail.com", "outlook.office365.com"]
                .iter()
                .any(|host| source.0.eq_ignore_ascii_case(host))
                && string(v, "user1").eq_ignore_ascii_case(&string(v, "user2"))))
    {
        return Err("Source and destination must be different mailboxes".into());
    }
    for i in ["1", "2"] {
        let (host, port, mode) = endpoint(v, i)?;
        a.extend([
            format!("--host{i}"),
            host.clone(),
            format!("--port{i}"),
            port.to_string(),
            format!("--user{i}"),
            required(v, &format!("user{i}"))?,
        ]);
        let auth = string(v, &format!("auth{i}"));
        if !["", "password", "google", "microsoft"].contains(&auth.as_str()) {
            return Err("Invalid authentication method".into());
        }
        if auth == "google" || auth == "microsoft" {
            let provider = if auth == "google" {
                oauth::Provider::Google
            } else {
                oauth::Provider::Microsoft
            };
            let expected = if auth == "google" {
                "imap.gmail.com"
            } else {
                "outlook.office365.com"
            };
            if !host.eq_ignore_ascii_case(expected) || mode != "ssl" || port != 993 {
                return Err(provider.error("GMAIL_OAUTH_ENDPOINT".into()));
            }
            let email = required(v, &format!("user{i}"))?;
            a.extend([format!("--oauthaccesstoken{i}"), format!("<{auth} OAuth>")]);
            oauth_accounts.push((a.len() - 1, email, provider));
        } else {
            a.extend([format!("--password{i}"), required(v, &format!("pass{i}"))?]);
            passwords.push(a.len() - 1);
        }
        // Disable protocol auto-detection so the selected transport is enforced.
        a.push(format!("--{}ssl{i}", if mode == "ssl" { "" } else { "no" }));
        a.push(format!(
            "--{}tls{i}",
            if mode == "starttls" { "" } else { "no" }
        ));
        if mode != "plain" {
            for value in [
                "SSL_verify_mode=1".to_string(),
                "SSL_verifycn_scheme=imap".into(),
                format!("SSL_verifycn_name={host}"),
            ] {
                a.extend([format!("--sslargs{i}"), value]);
            }
        }
    }
    for (key, flag) in [
        ("dryRun", "--dry"),
        ("syncDeletes", "--delete2"),
        ("automapFolders", "--automap"),
        ("useUid", "--useuid"),
    ] {
        if v[key] == true {
            a.push(flag.into())
        }
    }
    a.push(
        if v["subscribeFolders"] == false {
            "--nosubscribe"
        } else {
            "--subscribe"
        }
        .into(),
    );
    if v["fastMode"] == true {
        // Skip expensive size scans without changing message identity or flags.
        a.extend(["--nofoldersizes".into(), "--nofoldersizesatend".into()]);
    }
    let safe = v["providerSafeMode"] == true;
    if let (Some(min), Some(max)) = (number(v, "minAgeDays"), number(v, "maxAgeDays")) {
        if min > max {
            return Err("Minimum age must not exceed maximum age".into());
        }
    }
    for (key, flag, scale, default) in [
        ("maxAgeDays", "--maxage", 1., 0.),
        ("minAgeDays", "--minage", 1., 0.),
        ("bandwidthKb", "--maxbytespersecond", 1024., 0.),
        ("maxMessagesPerSecond", "--maxmessagespersecond", 1., 0.5),
        ("maxSleepSeconds", "--maxsleep", 1., 8.),
        ("maxBytesAfterMb", "--maxbytesafter", 1048576., 50.),
        ("stopAfterMb", "--exitwhenover", 1048576., 0.),
        ("errorsMax", "--errorsmax", 1., 10.),
        ("timeoutSeconds", "--timeout1", 1., 180.),
    ] {
        let raw = string(v, key);
        let n = number(v, key);
        if !raw.is_empty() && n.is_none() {
            return Err(format!("Invalid positive number: {key}"));
        }
        if let Some(n) = n.or(if safe && default > 0. {
            Some(default)
        } else {
            None
        }) {
            if key == "errorsMax" && n.fract() != 0.0 {
                return Err("Error limit must be an integer".into());
            }
            let scaled = n * scale;
            let val = if scale > 1.0 {
                scaled.round().max(1.0).to_string()
            } else {
                scaled.to_string()
            };
            a.extend([flag.into(), val.clone()]);
            if key == "timeoutSeconds" {
                a.extend(["--timeout2".into(), val]);
            }
        }
    }
    for line in string(v, "excludePattern")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        a.extend(["--exclude".into(), line.into()]);
    }
    let extra = shell_words::split(&string(v, "advancedArgs")).map_err(|e| e.to_string())?;
    // Only accept known options with their required values; do not let them consume later flags.
    let switches = [
        "--nofoldersizes",
        "--nofoldersizesatend",
        "--skipcrossduplicates",
        "--skipemptyfolders",
        "--addheader",
        "--usecache",
        "--syncinternaldates",
    ];
    let values = ["--regextrans2", "--folder", "--folderrec", "--useheader"];
    let mut tokens = extra.iter();
    while let Some(token) = tokens.next() {
        if switches.contains(&token.as_str()) {
            a.push(token.clone());
        } else if values.contains(&token.as_str()) {
            let value = tokens
                .next()
                .filter(|s| !s.is_empty() && !s.starts_with('-') && !s.contains('\0'))
                .ok_or_else(|| format!("Missing value for {token}"))?;
            a.extend([token.clone(), value.clone()]);
        } else {
            return Err(format!("Unsupported advanced option: {token}"));
        }
    }
    a.extend(["--nolog".into(), "--noreleasecheck".into()]);
    Ok(SyncArgs {
        values: a,
        passwords,
        oauth_accounts,
    })
}
fn preview(a: &SyncArgs) -> String {
    let mut redacted = a.to_vec();
    for &i in &a.passwords {
        redacted[i] = "••••••••".into();
    }
    for (i, _, provider) in &a.oauth_accounts {
        redacted[*i] = if *provider == oauth::Provider::Google {
            "<Google OAuth token file>"
        } else {
            "<Microsoft OAuth token file>"
        }
        .into();
    }
    format!("imapsync {}", shell_words::join(redacted))
}

fn account_jobs(body: &Value) -> Result<Vec<(usize, Value)>, String> {
    account_jobs_skipping(body, &std::collections::HashSet::new())
}
fn account_jobs_skipping(
    body: &Value,
    completed: &std::collections::HashSet<usize>,
) -> Result<Vec<(usize, Value)>, String> {
    if body["multipleAccounts"] != true {
        args(body)?;
        return Ok(vec![(1, body.clone())]);
    }
    let sources = body["accounts1"]
        .as_array()
        .ok_or("Missing source accounts")?;
    let destinations = body["accounts2"]
        .as_array()
        .ok_or("Missing destination accounts")?;
    if sources.len() != destinations.len() || sources.len() > 1000 {
        return Err("Account sheets must have the same number of rows (maximum 1000)".into());
    }
    let mut jobs = Vec::new();
    let mut used_destinations = std::collections::HashSet::new();
    let mut common = body.clone();
    let settings = common.as_object_mut().ok_or("Invalid configuration")?;
    settings.remove("accounts1");
    settings.remove("accounts2");
    for (index, (source, destination)) in sources.iter().zip(destinations).enumerate() {
        if completed.contains(&(index + 1)) {
            continue;
        }
        if [source, destination]
            .iter()
            .all(|v| string(v, "user").trim().is_empty() && string(v, "password").is_empty())
        {
            continue;
        }
        let mut job = common.clone();
        for (suffix, account) in [("1", source), ("2", destination)] {
            job[format!("user{suffix}")] = json!(string(account, "user").trim());
            job[format!("pass{suffix}")] = json!(string(account, "password"));
        }
        args(&job).map_err(|e| format!("Row {}: {e}", index + 1))?;
        let destination_user = string(&job, "user2");
        let destination_key = if ["imap.gmail.com", "outlook.office365.com"]
            .iter()
            .any(|host| string(&job, "host2").eq_ignore_ascii_case(host))
        {
            destination_user.to_lowercase()
        } else {
            destination_user
        };
        if !used_destinations.insert(destination_key) {
            return Err(format!("Row {}: duplicate destination account", index + 1));
        }
        jobs.push((index + 1, job));
    }
    if jobs.is_empty() {
        return Err("Enter at least one account pair".into());
    }
    Ok(jobs)
}

fn job_preview(d: &Document, jobs: &[(usize, Value)]) -> Result<String, String> {
    jobs.iter()
        .map(|(row, body)| Ok(format!("# {row}\n{}", preview(&execution_args(d, body)?))))
        .collect::<Result<Vec<_>, String>>()
        .map(|s| s.join("\n\n"))
}

fn manager_for(d: &Document, provider: oauth::Provider) -> Result<&oauth::Manager, String> {
    if provider == oauth::Provider::Google {
        Ok(&d.oauth)
    } else {
        d.microsoft_oauth
            .as_deref()
            .ok_or("MICROSOFT_OAUTH_RECONNECT".into())
    }
}
fn validate_oauth_accounts(d: &Document, jobs: &[(usize, Value)]) -> Result<(), String> {
    if d.oauth.view()["attempt"]["status"] == "waiting" {
        return Err("GMAIL_OAUTH_BUSY".into());
    }
    if d.microsoft_oauth
        .as_ref()
        .is_some_and(|m| m.view()["attempt"]["status"] == "waiting")
    {
        return Err("MICROSOFT_OAUTH_BUSY".into());
    }
    for (row, body) in jobs {
        for side in ["1", "2"] {
            let auth = string(body, &format!("auth{side}"));
            if ["google", "microsoft"].contains(&auth.as_str()) {
                let provider = if auth == "google" {
                    oauth::Provider::Google
                } else {
                    oauth::Provider::Microsoft
                };
                if !manager_for(d, provider)?.ready(&string(body, &format!("user{side}"))) {
                    return Err(
                        provider.error(format!("GMAIL_OAUTH_RECONNECT: row {row}, side {side}"))
                    );
                }
            }
        }
    }
    Ok(())
}

fn redact_oauth_token(d: &mut Document, email: &str, token: &str) {
    use base64::Engine;
    for secret in [
        token.to_string(),
        base64::engine::general_purpose::STANDARD
            .encode(format!("user={email}\x01auth=Bearer {token}\x01\x01")),
    ] {
        if !d.secrets.contains(&secret) {
            d.secrets.push(secret);
        }
    }
    d.secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
}

fn keep_oauth_fresh(
    shared: Shared,
    manager: Arc<oauth::Manager>,
    microsoft: Option<Arc<oauth::Manager>>,
    dir: PathBuf,
    accounts: Vec<(usize, String, oauth::Provider)>,
) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        let active =
            |d: &Document| d.child.is_some() && d.run_dir.as_ref().is_some_and(|r| r.path() == dir);
        if !active(&shared.lock().unwrap()) {
            return;
        }
        for (index, email, provider) in &accounts {
            let selected = if *provider == oauth::Provider::Google {
                Some(manager.as_ref())
            } else {
                microsoft.as_deref()
            };
            let token = selected
                .ok_or("MICROSOFT_OAUTH_RECONNECT".to_string())
                .and_then(|m| m.token(email))
                .map_err(|e| provider.error(e));
            let mut d = shared.lock().unwrap();
            if !active(&d) {
                return;
            }
            let result = token.and_then(|token| {
                redact_oauth_token(&mut d, email, &token);
                oauth::write_private(&dir.join(format!("oauth-{index}")), token.as_bytes())
            });
            if result.is_err() {
                log(&mut d,format!("OAuth refresh failed for {email}; reconnect this account before restarting."),"error");
                let _ = request_stop(&mut d);
                d.auth_failed = true;
                d.cancelled = false;
                if let Some(batch) = d.batch.as_mut() {
                    batch.pending.clear();
                }
                return;
            }
        }
    });
}

fn resumable_jobs(d: &Document, body: &Value) -> Result<Vec<(usize, Value)>, String> {
    let id = string(body, "resumeBatch");
    if id.is_empty() {
        return account_jobs(body);
    }
    if body["multipleAccounts"] != true {
        return Err("BATCH_RECOVERY_CHANGED".into());
    }
    let (_, saved) =
        batch_recovery::load(d.journal_dir.as_ref().ok_or("BATCH_RECOVERY_INVALID")?, &id)?;
    batch_recovery::validate(&saved, body)?;
    account_jobs_skipping(body, &batch_recovery::completed(&saved))
}
fn current_result(d: &Document) -> Option<Value> {
    let batch = d
        .batch
        .as_ref()
        .filter(|b| {
            d.hidden_result.as_deref() != b.report["id"].as_str()
                && b.checkpoint_path.as_ref().is_none_or(|p| p.exists())
        })
        .map(|b| {
            let mut value = b.report.clone();
            value["settings"]["dryRun"] = json!(d.dry_run);
            value["running"] = json!(d.child.is_some());
            value["activeRow"] = json!(b.row);
            value["results"] = json!(b.results);
            value["transferred"] = json!(b.transferred + d.transferred);
            value["errors"] = json!(b.errors.zip(d.errors).map(|(a, b)| a + b));
            value["resolved"] = b.checkpoint.as_ref().map_or_else(
                || json!(d.child.is_none() && b.completed == b.total && !d.cancelled),
                |v| v["resolved"].clone(),
            );
            value
        });
    batch.or_else(|| {
        d.single_history
            .as_ref()
            .filter(|run| {
                d.hidden_result.as_deref() != run.record["id"].as_str()
                    && run.path.as_ref().is_none_or(|p| p.exists())
            })
            .map(|run| run.view(d.child.is_some(), d.transferred, d.errors))
    })
}
fn result_record(d: &Document, id: &str) -> Result<Value, String> {
    if let Some(value) = current_result(d).filter(|v| v["id"] == id) {
        return Ok(value);
    }
    batch_recovery::read(d.journal_dir.as_ref().ok_or("RESULTS_UNAVAILABLE")?, id).map(|(_, v)| v)
}
fn result_reports(d: &Document) -> Vec<Value> {
    reports_from_parts(
        current_result(d),
        d.journal_dir.as_deref(),
        d.child.is_some(),
    )
}
fn reports_from_parts(current: Option<Value>, dir: Option<&Path>, running: bool) -> Vec<Value> {
    let mut reports = current
        .iter()
        .map(|v| results::report(v, running))
        .collect::<Vec<_>>();
    if let Some(dir) = dir {
        reports.extend(
            batch_recovery::history(dir)
                .iter()
                .filter(|v| current.as_ref().is_none_or(|c| c["id"] != v["id"]))
                .map(|v| results::report(v, false)),
        );
    }
    reports
}
fn delete_result(d: &mut Document, id: &str) -> Result<Value, String> {
    let report = results::report(
        &result_record(d, id).map_err(|_| "RESULTS_UNAVAILABLE")?,
        d.child.is_some() && d.batch.as_ref().is_some_and(|b| b.report["id"] == id),
    );
    if !results::deletable(&report) {
        return Err("RESULTS_PROTECTED".into());
    }
    if id != "current" {
        batch_recovery::delete_history(d.journal_dir.as_ref().ok_or("RESULTS_UNAVAILABLE")?, id)
            .map_err(|error| match error.as_str() {
                "RECOVERY_PAIR_BUSY" => "RESULTS_PROTECTED".into(),
                "BATCH_RECOVERY_INVALID" => "RESULTS_UNAVAILABLE".into(),
                _ => error,
            })?;
    }
    if d.batch.as_ref().is_some_and(|b| b.report["id"] == id)
        || d.single_history
            .as_ref()
            .is_some_and(|run| run.record["id"] == id)
    {
        d.hidden_result = Some(id.into());
    }
    Ok(json!({"deleted":true}))
}
fn session_cleanup(d: &mut Document, body: &Value, preview: bool, response: &mut Value) {
    let current = current_result(d).filter(|r| r["id"] == "current");
    if preview {
        if let Some(record) =
            current.filter(|r| results::deletable(&results::report(r, d.child.is_some())))
        {
            response["sessionStartedAt"] = record["startedAt"].clone();
            response["count"] = json!(response["count"].as_u64().unwrap_or(0) + 1);
        }
    } else if body["sessionStartedAt"].is_string() {
        let removable = current.is_some_and(|r| {
            r["startedAt"] == body["sessionStartedAt"]
                && results::deletable(&results::report(&r, d.child.is_some()))
        });
        let key = if removable && delete_result(d, "current").is_ok() {
            "deleted"
        } else {
            "skipped"
        };
        response[key] = json!(response[key].as_u64().unwrap_or(0) + 1);
    }
}
fn prepare_retry(d: &Document, body: &Value) -> Result<Value, String> {
    if d.child.is_some() {
        return Err("RECOVERY_PAIR_BUSY".into());
    }
    let id = string(body, "id");
    let mut source = result_record(d, &id)?;
    let dir = d.journal_dir.as_ref().ok_or("RESULTS_UNAVAILABLE")?;
    let _lock = if id == "current" {
        None
    } else {
        Some(batch_recovery::lock(&batch_recovery::read(dir, &id)?.0)?)
    };
    if id != "current" {
        source = batch_recovery::read(dir, &id)?.1;
    }
    let settings = results::retry_settings(&source, &body["rows"])?;
    let (path, mut value) = batch_recovery::create(dir, &settings)?;
    value["retryOf"] = json!(id);
    batch_recovery::save(&path, &value)?;
    Ok(value)
}
fn recovery_records(d: &Document) -> Vec<Value> {
    d.journal_dir.as_ref().map_or_else(Vec::new, |dir| {
        let mut values = batch_recovery::pending(dir);
        values.extend(recovery::pending(dir));
        values
    })
}
fn persist_batch(d: &mut Document, resolved: bool) -> Result<(), String> {
    if let Some(b) = d.batch.as_mut() {
        if let (Some(path), Some(value)) = (&b.checkpoint_path, &mut b.checkpoint) {
            value["activeRow"] = json!(b.row);
            value["results"] = json!(b.results);
            value["transferred"] = json!(b.transferred);
            value["errors"] = json!(b.errors);
            value["resolved"] = json!(resolved);
            if resolved {
                value["finishedAt"] = json!(chrono::Utc::now().to_rfc3339());
            }
            batch_recovery::save(path, value)?;
        }
    }
    Ok(())
}
fn start_batch(
    d: &mut Document,
    shared: Shared,
    body: &Value,
    executable: &Path,
    cache: &Path,
) -> Result<(), String> {
    if d.child.is_some() {
        return Err("Sync already running".into());
    }
    let jobs = resumable_jobs(d, body)?; // Validate pending rows before starting any migration.
    let saved = if string(body, "resumeBatch").is_empty() {
        None
    } else {
        Some(batch_recovery::load(
            d.journal_dir.as_ref().ok_or("BATCH_RECOVERY_INVALID")?,
            &string(body, "resumeBatch"),
        )?)
    };
    validate_oauth_accounts(d, &jobs)?;
    let command_preview = job_preview(d, &jobs)?;
    d.secrets = jobs
        .iter()
        .flat_map(|(_, v)| [string(v, "pass1"), string(v, "pass2")])
        .collect();
    d.secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    let (checkpoint_path, checkpoint, checkpoint_lock) = if body["dryRun"] != true {
        if let Some(dir) = &d.journal_dir {
            let (path, value) = if let Some(saved) = saved.clone() {
                saved
            } else {
                batch_recovery::create(dir, body)?
            };
            let lock = batch_recovery::lock(&path)?;
            if saved.is_some()
                && batch_recovery::load(dir, value["id"].as_str().ok_or("BATCH_RECOVERY_INVALID")?)?
                    .1
                    != value
            {
                return Err("BATCH_RECOVERY_CHANGED".into());
            }
            (Some(path), Some(value), Some(lock))
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };
    let saved_value = saved.as_ref().map(|(_, v)| v);
    let results = saved_value
        .and_then(|v| v["results"].as_array())
        .into_iter()
        .flatten()
        .filter(|r| r["interrupted"] != true)
        .cloned()
        .collect::<Vec<_>>();
    let completed = results.len();
    d.hidden_result = None;
    d.single_history = None;
    d.batch = Some(Batch {
        total: jobs.len() + completed,
        pending: jobs.into(),
        current: completed,
        row: 0,
        completed,
        failed: results.iter().filter(|r| r["exitCode"] != 0).count(),
        transferred: saved_value
            .and_then(|v| v["transferred"].as_u64())
            .unwrap_or(0),
        errors: saved_value.map_or(Some(0), |v| v["errors"].as_u64()),
        results,
        report: checkpoint.clone().unwrap_or_else(||json!({"id":"current","startedAt":chrono::Utc::now().to_rfc3339(),"settings":batch_recovery::clean(body)})),
        checkpoint,
        checkpoint_path,
        checkpoint_lock,
        executable: executable.into(),
        cache: cache.into(),
    });
    d.preview = command_preview;
    d.logs.clear();
    d.started = Some(chrono::Utc::now().to_rfc3339());
    if let Err(e) = next_account(d, shared) {
        d.batch = None;
        d.secrets.clear();
        return Err(e);
    }
    Ok(())
}

fn next_account(d: &mut Document, shared: Shared) -> Result<(), String> {
    let batch = d.batch.as_mut().ok_or("No account queue")?;
    let (row, body) = batch.pending.pop_front().ok_or("Account queue empty")?;
    batch.current += 1;
    batch.row = row;
    let executable = batch.executable.clone();
    let cache = batch.cache.clone();
    persist_batch(d, false)?;
    launch_sync(d, shared, &body, Command::new(executable), &cache)
}
fn log(d: &mut Document, message: String, level: &str) {
    let mut message = message;
    for s in &d.secrets {
        if !s.is_empty() {
            message = message.replace(s, "••••••••")
        }
    }
    d.logs
        .push_back(json!({"time":chrono::Utc::now().to_rfc3339(),"level":level,"message":message}));
    while d.logs.len() > 1200 {
        d.logs.pop_front();
    }
}
fn status(d: &Document) -> Value {
    let batch = d.batch.as_ref();
    let transferred = d.transferred + batch.map_or(0, |b| b.transferred);
    let errors = match batch {
        Some(b) => b
            .errors
            .zip(d.errors)
            .map(|(total, current)| total + current),
        None => d.errors,
    };
    json!({"running":d.child.is_some(),"startedAt":d.started,"finishedAt":d.finished,
        "stopping":d.stopping.is_some(),"cancelled":d.cancelled,"dryRun":d.dry_run,
        "transferred":transferred,"errors":errors,"lastExitCode":d.exit,"commandPreview":d.preview,
        "recoveryPending":recovery_records(d).len(),
        "batch":batch.map(|b| json!({"total":b.total,"current":b.current,"row":b.row,"completed":b.completed,"failed":b.failed,"results":b.results}))})
}
fn binary() -> Result<PathBuf, String> {
    engine::resolve()
}

#[cfg(test)]
fn prepare_run(a: &SyncArgs, cache: &Path) -> Result<(tempfile::TempDir, Vec<String>), String> {
    prepare_run_authenticated(a, cache, None)
}
#[cfg(test)]
fn prepare_run_authenticated(
    a: &SyncArgs,
    cache: &Path,
    manager: Option<&oauth::Manager>,
) -> Result<(tempfile::TempDir, Vec<String>), String> {
    prepare_run_oauth(a, cache, manager, None)
}
fn prepare_run_oauth(
    a: &SyncArgs,
    cache: &Path,
    google: Option<&oauth::Manager>,
    microsoft: Option<&oauth::Manager>,
) -> Result<(tempfile::TempDir, Vec<String>), String> {
    // A fresh private directory for each run also isolates concurrent app instances.
    let dir = tempfile::Builder::new()
        .prefix("imapsync-studio-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let mut actual = a.to_vec();
    for &i in &a.passwords {
        let suffix = a[i - 1].trim_start_matches("--password");
        let file = dir.path().join(format!("pass{suffix}"));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&file)
            .and_then(|mut f| f.write_all(a[i].as_bytes()))
            .map_err(|e| e.to_string())?;
        actual[i - 1] = format!("--passfile{suffix}");
        actual[i] = file.to_string_lossy().into_owned();
    }
    for (i, email, provider) in &a.oauth_accounts {
        let manager = if *provider == oauth::Provider::Google {
            google
        } else {
            microsoft
        };
        let token = manager
            .ok_or_else(|| provider.error("GMAIL_OAUTH_RECONNECT".into()))?
            .token(email)
            .map_err(|e| provider.error(e))?;
        let file = dir.path().join(format!("oauth-{}", i));
        oauth::write_private(&file, token.as_bytes())?;
        actual[*i] = file.to_string_lossy().into_owned();
    }
    // Keep imapsync's existing cache location across restarts, particularly for --useuid.
    std::fs::create_dir_all(cache).map_err(|e| format!("Cannot create imapsync cache: {e}"))?;
    actual.extend([
        "--tmpdir".into(),
        cache.to_string_lossy().into_owned(),
        "--pidfile".into(),
        dir.path()
            .join("imapsync.pid")
            .to_string_lossy()
            .into_owned(),
    ]);
    Ok((dir, actual))
}

fn record_output(d: &mut Document, line: String, level: &str) {
    // Count explicit imapsync output, not occurrences of "message" or "error".
    if let Some(count) = line
        .strip_prefix("Messages transferred")
        .and_then(|s| s.split_once(':'))
        .and_then(|(_, s)| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
    {
        d.transferred = count;
    } else if line.starts_with("msg ") && line.contains(" copied to ") {
        d.transferred += 1;
    }
    if let Some(count) = line
        .strip_prefix("Detected ")
        .and_then(|s| s.strip_suffix(" errors"))
        .and_then(|s| s.parse::<u64>().ok())
    {
        d.errors = Some(count);
    }
    log(d, line, level);
}

fn read_output(pipe: impl std::io::Read, shared: Shared, level: &str) -> bool {
    let mut reader = BufReader::new(pipe);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => return true,
            Ok(_) => {
                // A non-UTF-8 folder name must not terminate pipe draining and stall the child.
                let line = String::from_utf8_lossy(&bytes)
                    .trim_end_matches(['\r', '\n'])
                    .to_string();
                record_output(&mut shared.lock().unwrap(), line, level);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                log(
                    &mut shared.lock().unwrap(),
                    format!("Cannot read imapsync output: {e}"),
                    "error",
                );
                return false;
            }
        }
    }
}

fn request_stop(d: &mut Document) -> Result<(), String> {
    let Some(child) = d.child.as_mut() else {
        return Ok(());
    };
    if d.stopping.is_some() || child.try_wait().map_err(|e| e.to_string())?.is_some() {
        return Ok(());
    }
    // A signal can interrupt APPEND after delivery but before the UID cache is
    // updated. imapsync checks this marker between messages, after cache writes.
    let dir = d.run_dir.as_ref().ok_or("Missing run directory")?;
    let pid = std::fs::read_to_string(dir.path().join("imapsync.pid"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(child.id());
    std::fs::write(dir.path().join(format!("imapsync.pidabort{pid}")), b"")
        .map_err(|e| format!("Cannot request a checkpoint stop: {e}"))?;
    d.stopping = Some(Instant::now());
    d.cancelled = true;
    log(
        d,
        "--- Stop requested; waiting for imapsync to exit ---".into(),
        "info",
    );
    Ok(())
}

fn shutdown(shared: &Shared) {
    let (child, dir, grace, pair_lock) = {
        let mut d = shared.lock().unwrap();
        d.oauth.cancel();
        if let Some(m) = &d.microsoft_oauth {
            m.cancel();
        }
        let _ = request_stop(&mut d);
        if let Some(batch) = d.batch.as_mut() {
            batch.pending.clear();
        }
        (
            d.child.take(),
            d.run_dir.take(),
            d.stop_grace,
            d.pair_lock.take(),
        )
    };
    let had_child = child.is_some();
    // Release the document lock so readers can drain output during shutdown.
    if let Some(mut child) = child {
        let deadline = Instant::now() + grace;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    }
    if had_child {
        let mut d = shared.lock().unwrap();
        let transferred = d.transferred;
        let errors = d.errors;
        if let Some(run) = d.single_history.as_mut() {
            let _ = run.finish(-1, true, true, transferred, errors);
        }
    }
    drop(dir);
    drop(pair_lock);
}

fn execution_args(d: &Document, body: &Value) -> Result<SyncArgs, String> {
    let mut a = args(body)?;
    if d.journal_dir
        .as_ref()
        .is_some_and(|dir| recovery::path(dir, body).exists())
    {
        recovery::fallback(&mut a.values);
    }
    Ok(a)
}
fn launch_sync(
    d: &mut Document,
    shared: Shared,
    body: &Value,
    mut cmd: Command,
    cache: &Path,
) -> Result<(), String> {
    if d.child.is_some() {
        return Err("Sync already running".into());
    }
    let pair_lock = d
        .journal_dir
        .as_ref()
        .map(|dir| recovery::lock(dir, body))
        .transpose()?;
    if let Some(dir) = &d.journal_dir {
        recovery::ensure_previous_exited(&recovery::path(dir, body))?;
    }
    let a = execution_args(d, body)?;
    let recovering = d
        .journal_dir
        .as_ref()
        .is_some_and(|dir| recovery::path(dir, body).exists());
    let (dir, actual) = prepare_run_oauth(&a, cache, Some(&d.oauth), d.microsoft_oauth.as_deref())?;
    let oauth_secrets = a
        .oauth_accounts
        .iter()
        .map(|(i, email, _)| {
            std::fs::read_to_string(&actual[*i])
                .map(|token| (email.clone(), token))
                .map_err(|_| "GMAIL_OAUTH_STORAGE".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    cmd.args(&actual)
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let journal = if body["dryRun"] != true {
        d.journal_dir
            .as_ref()
            .map(|dir| recovery::mark(dir, body))
            .transpose()?
    } else {
        None
    };
    let mut single = if d.batch.is_none() {
        Some(single_history::Run::start(d.journal_dir.as_deref(), body)?)
    } else {
        None
    };
    engine::sanitize(&mut cmd);
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            if let Some(run) = single.as_mut() {
                if let Err(save_error) = run.finish(-1, false, false, 0, None) {
                    log(
                        d,
                        format!("Single-account result could not be saved: {save_error}"),
                        "error",
                    );
                }
                d.started = run.record["startedAt"].as_str().map(str::to_string);
                d.finished = run.record["finishedAt"].as_str().map(str::to_string);
                d.dry_run = body["dryRun"] == true;
                d.exit = Some(-1);
                d.cancelled = false;
                d.transferred = 0;
                d.errors = None;
            }
            d.hidden_result = None;
            d.single_history = single;
            return Err(format!("imapsync: {error}"));
        }
    };
    if let Some(run) = single.as_mut() {
        if let Err(error) = run.attach_pid(child.id()) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = run.finish(-1, false, true, 0, None);
            d.hidden_result = None;
            d.single_history = single;
            return Err(error);
        }
    }
    if d.batch.is_none() {
        d.hidden_result = None;
        d.single_history = single;
    }

    if let Some(path) = &journal {
        if let Err(error) = recovery::attach_pid(path, child.id()) {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(run) = d.single_history.as_mut() {
                let _ = run.finish(-1, false, true, 0, None);
            }
            return Err(error);
        }
    }
    let first = d.batch.as_ref().is_none_or(|b| b.current == 1);
    if first {
        d.logs.clear();
        d.started = Some(chrono::Utc::now().to_rfc3339());
    }
    if d.batch.is_none() {
        d.preview = preview(&a);
        d.secrets = vec![string(body, "pass1"), string(body, "pass2")];
        // Longest first prevents a shorter password from partially exposing a longer one.
        d.secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    }
    for (email, token) in oauth_secrets {
        redact_oauth_token(d, &email, &token);
    }
    d.finished = None;
    d.exit = None;
    d.stopping = None;
    let max_sleep =
        number(body, "maxSleepSeconds").unwrap_or(if body["providerSafeMode"] == true {
            8.0
        } else {
            2.0
        });
    d.stop_grace = Duration::from_secs_f64((max_sleep + 2.0).clamp(5.0, 30.0));
    d.cancelled = false;
    d.auth_failed = false;
    d.dry_run = body["dryRun"] == true;
    d.transferred = 0;
    d.errors = None;
    d.run_dir = Some(dir);
    d.pair_lock = pair_lock;
    let message = d.batch.as_ref().map_or_else(
        || "--- Sync started ---".into(),
        |b| format!("--- Account {}/{} (row {}) ---", b.current, b.total, b.row),
    );
    log(d, message, "info");
    if recovering {
        log(d,"--- Recovery: comparing existing message headers without UID cache; this mailbox pair will retain header comparison for future runs. ---".into(),"warning");
    }
    log(d, preview(&a), "command");
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out_state = shared.clone();
    let err_state = shared.clone();
    let out_reader = std::thread::spawn(move || read_output(stdout, out_state, "info"));
    let err_reader = std::thread::spawn(move || read_output(stderr, err_state, "error"));
    d.child = Some(child);
    if !a.oauth_accounts.is_empty() {
        keep_oauth_fresh(
            shared.clone(),
            d.oauth.clone(),
            d.microsoft_oauth.clone(),
            d.run_dir.as_ref().unwrap().path().to_path_buf(),
            a.oauth_accounts,
        );
    }
    std::thread::spawn(move || {
        let code = loop {
            std::thread::sleep(Duration::from_millis(100));
            let mut d = shared.lock().unwrap();
            let force_stop = d.stopping.is_some_and(|t| t.elapsed() >= d.stop_grace);
            if force_stop {
                log(&mut d, "--- Checkpoint stop timed out; forcing termination. The recovery journal is retained for the next run. ---".into(), "warning");
            }
            let Some(child) = d.child.as_mut() else {
                return;
            };
            if force_stop {
                let _ = child.kill();
            }
            match child.try_wait() {
                Ok(Some(exit)) => break exit.code().unwrap_or(-1),
                Ok(None) => {}
                Err(e) => {
                    // Recover the child rather than leaving the app permanently "running".
                    let _ = child.kill();
                    let _ = child.wait();
                    log(&mut d, format!("Failed to wait for imapsync: {e}"), "error");
                    break -1;
                }
            }
        };
        // Keep the run active and secrets intact until both pipes have been fully drained.
        let readers_ok =
            matches!(out_reader.join(), Ok(true)) & matches!(err_reader.join(), Ok(true));
        let mut d = shared.lock().unwrap();
        d.exit = Some(if readers_ok && !d.auth_failed {
            code
        } else {
            -1
        });
        if d.exit == Some(0) && !d.cancelled {
            if let Some(path) = journal.as_ref() {
                let result = if recovering {
                    recovery::resolve(path)
                } else {
                    std::fs::remove_file(path).map_err(|e| e.to_string())
                };
                if let Err(error) = result {
                    log(
                        &mut d,
                        format!("Recovery journal retained: {error}"),
                        "warning",
                    );
                }
            }
        }
        d.finished = Some(chrono::Utc::now().to_rfc3339());
        d.child = None;
        d.stopping = None;
        let cancelled = d.cancelled;
        let code = d.exit.unwrap();
        log(
            &mut d,
            if cancelled {
                "--- Sync stopped ---".into()
            } else {
                format!("--- Sync finished with exit code {code} ---")
            },
            if cancelled {
                "warning"
            } else if code == 0 {
                "success"
            } else {
                "error"
            },
        );
        d.run_dir = None;
        d.pair_lock = None;
        let transferred = d.transferred;
        let errors = d.errors;
        let interrupted = cancelled || code < 0 || d.auth_failed;
        if d.batch.is_none() {
            let saved = d
                .single_history
                .as_mut()
                .map(|run| run.finish(code, cancelled, interrupted, transferred, errors));
            if let Some(Err(error)) = saved {
                log(
                    &mut d,
                    format!("Single-account result could not be saved: {error}"),
                    "error",
                );
            }
        }
        if let Some(batch) = d.batch.as_mut() {
            batch.completed += 1;
            if code != 0 && !cancelled {
                batch.failed += 1;
            }
            batch.transferred += transferred;
            batch.errors = batch
                .errors
                .zip(errors)
                .map(|(total, current)| total + current);
            batch
                .results
                .push(json!({"row":batch.row,"exitCode":code,"cancelled":cancelled,"interrupted":interrupted,"transferred":transferred,"errors":errors}));
            if interrupted {
                batch.pending.clear();
            }
            d.transferred = 0;
            d.errors = Some(0);
        }
        let resolved = d
            .batch
            .as_ref()
            .is_some_and(|b| b.pending.is_empty() && !interrupted);
        if let Err(error) = persist_batch(&mut d, resolved) {
            log(&mut d, format!("Batch checkpoint failed: {error}"), "error");
            if let Some(b) = d.batch.as_mut() {
                b.pending.clear();
            }
            d.exit = Some(-1);
        }
        if d.batch.as_ref().is_some_and(|b| !b.pending.is_empty()) {
            match next_account(&mut d, shared.clone()) {
                Ok(()) => return,
                Err(e) => {
                    log(
                        &mut d,
                        format!("Account queue could not start: {e}"),
                        "error",
                    );
                    let batch = d.batch.as_mut().unwrap();
                    batch.completed += 1;
                    batch.failed += 1;
                    batch
                        .results
                        .push(json!({"row":batch.row,"exitCode":-1,"cancelled":false}));
                    batch.pending.clear();
                    d.exit = Some(-1);
                }
            }
        }
        if let Some(batch) = d.batch.as_ref() {
            let failed = batch.failed;
            let message = format!(
                "--- Account queue finished: {}/{} processed, {} failed ---",
                batch.completed, batch.total, batch.failed
            );
            if failed > 0 && !cancelled {
                d.exit = Some(1);
            }
            let level = if cancelled {
                "warning"
            } else if d.exit == Some(0) {
                "success"
            } else {
                "error"
            };
            log(&mut d, message, level);
        }
        if let Some(b) = d.batch.as_mut() {
            b.checkpoint_lock = None;
        }
        d.secrets.clear();
    });
    Ok(())
}

fn connection(v: Value) -> Result<Value, String> {
    let (host, port, mode) = endpoint(&v, "")?;
    let start = Instant::now();
    let addresses = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?;
    let mut socket = None;
    for addr in addresses {
        if let Ok(s) = TcpStream::connect_timeout(&addr, Duration::from_secs(8)) {
            socket = Some(s);
            break;
        }
    }
    let mut stream = socket.ok_or("Connection failed or timed out")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .map_err(|e| e.to_string())?;
    if mode == "starttls" {
        let mut line = String::new();
        let mut reader = BufReader::new(&mut stream);
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if !line.starts_with("* OK") {
            return Err("IMAP greeting rejected".into());
        }
        reader
            .get_mut()
            .write_all(b"a001 STARTTLS\r\n")
            .map_err(|e| e.to_string())?;
        loop {
            line.clear();
            if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                return Err("Server closed connection".into());
            }
            if line.starts_with("a001 ") {
                if !line.starts_with("a001 OK") {
                    return Err("STARTTLS rejected".into());
                }
                break;
            }
        }
    }
    if mode != "plain" {
        native_tls::TlsConnector::new()
            .map_err(|e| e.to_string())?
            .connect(&host, stream)
            .map_err(|e| e.to_string())?;
    }
    Ok(
        json!({"ok":true,"endpoint":format!("{host}:{port}"),"latencyMs":start.elapsed().as_millis()}),
    )
}
#[tauri::command]
async fn api(
    app: tauri::AppHandle,
    path: String,
    body: Value,
    state: tauri::State<'_, Shared>,
) -> Result<Value, String> {
    let shared = state.inner().clone();
    if matches!(
        path.as_str(),
        "/api/results/policy" | "/api/results/cleanup/preview" | "/api/results/cleanup"
    ) {
        let dir = shared
            .lock()
            .map_err(|_| "RESULTS_UNAVAILABLE")?
            .journal_dir
            .clone()
            .ok_or("RESULTS_UNAVAILABLE")?;
        let operation = path.clone();
        let request = body.clone();
        let mut response = tauri::async_runtime::spawn_blocking(move || match operation.as_str() {
            "/api/results/policy" => Ok(json!({"days":history_policy::load(&dir)?})),
            "/api/results/cleanup/preview" => {
                let days = history_policy::days(&request["days"])?;
                if request["savePolicy"] == true && days == 0 {
                    Ok(json!({"days":0,"ids":[],"count":0}))
                } else {
                    Ok(history_policy::preview(&dir, days, chrono::Utc::now()))
                }
            }
            _ => history_policy::commit(&dir, &request, chrono::Utc::now()),
        })
        .await
        .map_err(|_| "HISTORY_POLICY_FAILED".to_string())??;
        // A batch dry run is session-only. Clear the exact previewed attempt,
        // never a newer dry run that happened to reuse the synthetic ID.
        if body["savePolicy"] != true && body["days"] == 0 {
            let mut d = shared.lock().map_err(|_| "RESULTS_UNAVAILABLE")?;
            session_cleanup(&mut d, &body, path.ends_with("/preview"), &mut response);
        }
        return Ok(response);
    }
    if path == "/api/results/search" {
        let (current, dir, running) = {
            let d = shared.lock().map_err(|e| e.to_string())?;
            (current_result(&d), d.journal_dir.clone(), d.child.is_some())
        };
        return tauri::async_runtime::spawn_blocking(move || {
            results::search(reports_from_parts(current, dir.as_deref(), running), &body)
        })
        .await
        .map_err(|_| "RESULTS_UNAVAILABLE".to_string())?;
    }
    if matches!(
        path.as_str(),
        "/api/results/export" | "/api/results/export-matching"
    ) {
        let format = string(&body, "format");
        let multiple = path.ends_with("export-matching");
        let (bytes, count) = if multiple {
            let (current, dir, running) = {
                let d = shared.lock().map_err(|e| e.to_string())?;
                (current_result(&d), d.journal_dir.clone(), d.child.is_some())
            };
            let filters = body.clone();
            let export_format = format.clone();
            tauri::async_runtime::spawn_blocking(move || {
                results::export_matching(
                    reports_from_parts(current, dir.as_deref(), running),
                    &filters,
                    &export_format,
                )
            })
            .await
            .map_err(|_| "RESULTS_EXPORT_FAILED".to_string())??
        } else {
            let record = {
                let d = shared.lock().map_err(|e| e.to_string())?;
                result_record(&d, &string(&body, "id"))?
            };
            (results::export(&record, &format)?, 1)
        };
        let title = match string(&body, "lang").as_str() {
            "ja" => "転送結果を保存",
            "zh" => "保存迁移结果",
            _ => "Save migration results",
        };
        return tauri::async_runtime::spawn_blocking(move || {
            let mut dialog = app
                .dialog()
                .file()
                .set_title(title)
                .set_file_name(if multiple {
                    format!("imapsync-history-{count}.{}", format)
                } else {
                    format!("imapsync-results.{}", format)
                })
                .add_filter(format.to_uppercase(), &[format.as_str()]);
            if let Some(window) = app.get_webview_window("main") {
                dialog = dialog.set_parent(&window);
            }
            let file = dialog.blocking_save_file();
            if let Some(file) = file {
                let path = file.into_path().map_err(|_| "RESULTS_EXPORT_FAILED")?;
                oauth::write_private(&path, &bytes).map_err(|_| "RESULTS_EXPORT_FAILED")?;
                Ok(json!({"saved":true,"runCount":count}))
            } else {
                Ok(json!({"saved":false}))
            }
        })
        .await
        .map_err(|_| "RESULTS_EXPORT_FAILED".to_string())?;
    }
    if path.starts_with("/api/oauth/") {
        let provider = match body["provider"].as_str().unwrap_or("google") {
            "google" => oauth::Provider::Google,
            "microsoft" => oauth::Provider::Microsoft,
            _ => return Err("Invalid OAuth provider".into()),
        };
        let manager = {
            let mut d = shared.lock().map_err(|e| e.to_string())?;
            if d.child.is_some()
                && !["/api/oauth/status", "/api/oauth/cancel"].contains(&path.as_str())
            {
                return Err("GMAIL_OAUTH_BUSY".into());
            }
            if provider == oauth::Provider::Google {
                d.oauth.clone()
            } else {
                d.microsoft_oauth
                    .get_or_insert_with(|| Arc::new(oauth::Manager::microsoft()))
                    .clone()
            }
        };
        return tauri::async_runtime::spawn_blocking(move || {
            let _document = if !["/api/oauth/status", "/api/oauth/cancel"].contains(&path.as_str())
            {
                let d = shared.lock().map_err(|e| e.to_string())?;
                if d.child.is_some() {
                    return Err("GMAIL_OAUTH_BUSY".into());
                }
                Some(d)
            } else {
                None
            };
            if path == "/api/oauth/connect"
                && _document.as_ref().is_some_and(|d| {
                    if provider == oauth::Provider::Google {
                        d.microsoft_oauth
                            .as_ref()
                            .is_some_and(|m| m.view()["attempt"]["status"] == "waiting")
                    } else {
                        d.oauth.view()["attempt"]["status"] == "waiting"
                    }
                })
            {
                return Err("GMAIL_OAUTH_BUSY".into());
            }
            match path.as_str() {
                "/api/oauth/status" => Ok(manager.view()),
                "/api/oauth/configure" => manager.configure(&body),
                "/api/oauth/connect" => {
                    manager.begin(&string(&body, "email"), &string(&body, "lang"))
                }
                "/api/oauth/cancel" => {
                    manager.cancel();
                    Ok(json!({}))
                }
                "/api/oauth/disconnect" => {
                    manager.forget(&string(&body, "email"))?;
                    Ok(manager.view())
                }
                _ => Err("Unknown OAuth command".into()),
            }
        })
        .await
        .map_err(|_| provider.error("GMAIL_OAUTH_INTERNAL".to_string()))?
        .map_err(|e| provider.error(e));
    }
    if path == "/api/test-connection" {
        return tauri::async_runtime::spawn_blocking(move || connection(body))
            .await
            .map_err(|e| e.to_string())?;
    }
    if path == "/api/sync" {
        return tauri::async_runtime::spawn_blocking(move || {
            let mut d = shared.lock().map_err(|e| e.to_string())?;
            let cache = std::env::var_os("HOME")
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .map(|p| p.join("tmp"))
                .unwrap_or(
                    app.path()
                        .app_local_data_dir()
                        .map_err(|e| e.to_string())?
                        .join("imapsync"),
                );
            if body["multipleAccounts"] == true {
                start_batch(&mut d, shared.clone(), &body, &binary()?, &cache)?;
            } else {
                if d.child.is_some() {
                    return Err("Sync already running".into());
                }
                d.batch = None;
                validate_oauth_accounts(&d, &[(1, body.clone())])?;
                launch_sync(
                    &mut d,
                    shared.clone(),
                    &body,
                    Command::new(binary()?),
                    &cache,
                )?;
            }
            Ok(json!({"status":status(&d)}))
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    let mut d = shared.lock().map_err(|e| e.to_string())?;
    match path.as_str() {
        "/api/status" => Ok(status(&d)),
        "/api/draft" => Ok(d.draft.clone()),
        "/api/recovery" => Ok(json!(recovery_records(&d))),
        "/api/results" => Ok(json!(result_reports(&d))),
        "/api/results/delete" => delete_result(&mut d, &string(&body, "id")),
        "/api/results/retry" => prepare_retry(&d, &body),
        "/api/save-draft" => {
            d.draft = body;
            Ok(json!({}))
        }
        "/api/preview" => {
            let jobs = resumable_jobs(&d, &body)?;
            validate_oauth_accounts(&d, &jobs)?;
            d.preview = if body["multipleAccounts"] == true {
                job_preview(&d, &jobs)?
            } else {
                preview(&execution_args(&d, &body)?)
            };
            Ok(json!({"commandPreview":d.preview}))
        }
        "/api/stop" => {
            request_stop(&mut d)?;
            Ok(json!({"status":status(&d)}))
        }
        _ => Err("Unknown command".into()),
    }
}
#[tauri::command]
fn logs(state: tauri::State<Shared>) -> Vec<Value> {
    state.lock().unwrap().logs.iter().cloned().collect()
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Shared::default())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            app.state::<Shared>().lock().unwrap().journal_dir = Some(data_dir.join("recovery"));
            let history_dir = data_dir.join("recovery");
            std::thread::spawn(move || loop {
                if let Err(error) = history_policy::sweep(&history_dir, chrono::Utc::now()) {
                    eprintln!("History retention: {error}");
                }
                std::thread::sleep(Duration::from_secs(60 * 60));
            });
            let path = data_dir.join("google-oauth.json");
            app.state::<Shared>().lock().unwrap().oauth = Arc::new(oauth::Manager::load(path));
            app.state::<Shared>().lock().unwrap().microsoft_oauth = Some(Arc::new(
                oauth::Manager::load_microsoft(data_dir.join("microsoft-oauth.json")),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api, logs])
        .build(tauri::generate_context!())
        .expect("Tauri initialization failed")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let state = app.state::<Shared>();
                shutdown(state.inner());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Value {
        json!({"host1":"source.example","host2":"dest.example","user1":"a","user2":"b","pass1":" secret ","pass2":"p'word","security1":"ssl","security2":"starttls","dryRun":true,"providerSafeMode":true})
    }
    #[test]
    fn google_tokens_are_private_files_and_endpoint_is_fixed() {
        let mut v = config();
        v["host1"] = json!("imap.gmail.com");
        v["security1"] = json!("ssl");
        v["port1"] = json!("993");
        v["user1"] = json!("a@gmail.com");
        v["auth1"] = json!("google");
        v["pass1"] = json!("");
        let a = args(&v).unwrap();
        assert!(!a.contains(&"--password1".into()));
        assert!(!a.contains(&"--gmail1".into()));
        assert!(!a.contains(&"--expunge1".into()));
        assert!(preview(&a).contains("<Google OAuth token file>"));
        let mut same = v.clone();
        same["host2"] = json!("imap.gmail.com");
        same["port2"] = json!("993");
        same["security2"] = json!("ssl");
        same["auth2"] = json!("google");
        same["user2"] = json!("A@gmail.com");
        assert!(args(&same).is_err());
        let manager = oauth::Manager::with_test_accounts(&["a@gmail.com"]);
        let cache = tempfile::tempdir().unwrap();
        let (dir, actual) = prepare_run_authenticated(&a, cache.path(), Some(&manager)).unwrap();
        let index = actual
            .iter()
            .position(|a| a == "--oauthaccesstoken1")
            .unwrap()
            + 1;
        assert!(Path::new(&actual[index]).starts_with(dir.path()));
        assert_eq!(
            std::fs::read_to_string(&actual[index]).unwrap(),
            "test-access-token"
        );
        assert!(!actual.iter().any(|v| v.contains("test-access-token")));
        let mut d = Document::default();
        redact_oauth_token(&mut d, "a@gmail.com", "test-access-token");
        log(&mut d, "test-access-token".into(), "info");
        assert!(!d.logs.back().unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("test-access-token"));
        for (field, value) in [
            ("host1", "evil.example"),
            ("port1", "143"),
            ("security1", "plain"),
        ] {
            let mut bad = v.clone();
            bad[field] = json!(value);
            assert!(args(&bad).is_err());
        }
    }

    #[test]
    fn mixed_google_microsoft_uses_distinct_token_files_and_fixed_endpoints() {
        let mut body = config();
        body["host1"] = json!("imap.gmail.com");
        body["host2"] = json!("outlook.office365.com");
        body["port1"] = json!("993");
        body["port2"] = json!("993");
        body["security2"] = json!("ssl");
        body["auth1"] = json!("google");
        body["auth2"] = json!("microsoft");
        body["user1"] = json!("same@example.test");
        body["user2"] = json!("same@example.test");
        let google = oauth::Manager::with_test_accounts(&["same@example.test"]);
        let microsoft = oauth::Manager::with_microsoft_test_accounts(&["same@example.test"]);
        let args = args(&body).unwrap();
        let cache = tempfile::tempdir().unwrap();
        let (_dir, actual) =
            prepare_run_oauth(&args, cache.path(), Some(&google), Some(&microsoft)).unwrap();
        for (index, _, provider) in &args.oauth_accounts {
            let text = std::fs::read_to_string(&actual[*index]).unwrap();
            assert_eq!(
                text,
                if *provider == oauth::Provider::Google {
                    "test-access-token"
                } else {
                    "test-microsoft-access-token"
                }
            );
        }
        let preview = preview(&args);
        assert!(preview.contains("<Microsoft OAuth token file>"));
        assert!(!preview.contains("test-access-token"));
        assert!(prepare_run_oauth(&args, cache.path(), Some(&google), None).is_err());
        for (key, value) in [
            ("host2", "attacker.test"),
            ("security2", "starttls"),
            ("port2", "143"),
        ] {
            let mut changed = body.clone();
            changed[key] = json!(value);
            assert!(super::args(&changed)
                .err()
                .unwrap()
                .contains("MICROSOFT_OAUTH_ENDPOINT"));
        }
        body["host1"] = json!("outlook.office365.com");
        body["auth1"] = json!("microsoft");
        body["user2"] = json!("SAME@EXAMPLE.TEST");
        assert!(super::args(&body).is_err());
    }
    #[test]
    fn microsoft_batch_checks_each_row_and_provider_independently() {
        let mut body = batch_config();
        body["host1"] = json!("outlook.office365.com");
        body["auth1"] = json!("microsoft");
        body["security1"] = json!("ssl");
        body["port1"] = json!("993");
        let jobs = account_jobs(&body).unwrap();
        let emails = jobs
            .iter()
            .map(|(_, b)| b["user1"].as_str().unwrap())
            .collect::<Vec<_>>();
        let mut d = Document {
            oauth: Arc::new(oauth::Manager::with_test_accounts(&emails)),
            ..Document::default()
        };
        assert!(validate_oauth_accounts(&d, &jobs)
            .unwrap_err()
            .contains("MICROSOFT_OAUTH_RECONNECT"));
        d.microsoft_oauth = Some(Arc::new(oauth::Manager::with_microsoft_test_accounts(
            &emails[..1],
        )));
        assert!(validate_oauth_accounts(&d, &jobs).is_err());
        d.microsoft_oauth = Some(Arc::new(oauth::Manager::with_microsoft_test_accounts(
            &emails,
        )));
        assert!(validate_oauth_accounts(&d, &jobs).is_ok());
        body["host2"] = json!("outlook.office365.com");
        body["auth2"] = json!("microsoft");
        body["security2"] = json!("ssl");
        body["port2"] = json!("993");
        body["accounts2"][1]["user"] = json!(body["accounts2"][0]["user"]
            .as_str()
            .unwrap()
            .to_uppercase());
        assert!(account_jobs(&body)
            .err()
            .unwrap()
            .contains("duplicate destination"));
    }
    #[test]
    fn google_batch_requires_every_account_before_starting() {
        let mut v = config();
        for side in ["1", "2"] {
            v[format!("host{side}")] = json!("imap.gmail.com");
            v[format!("security{side}")] = json!("ssl");
            v[format!("port{side}")] = json!("993");
            v[format!("auth{side}")] = json!("google");
        }
        v["multipleAccounts"] = json!(true);
        v["accounts1"] = json!([{"user":"a@gmail.com"},{"user":"b@gmail.com"}]);
        v["accounts2"] = json!([{"user":"c@gmail.com"},{"user":"d@gmail.com"}]);
        let jobs = account_jobs(&v).unwrap();
        assert_eq!(jobs.len(), 2);
        let mut d = Document {
            oauth: Arc::new(oauth::Manager::with_test_accounts(&[
                "a@gmail.com",
                "b@gmail.com",
                "c@gmail.com",
            ])),
            ..Document::default()
        };
        assert!(validate_oauth_accounts(&d, &jobs)
            .unwrap_err()
            .contains("row 2, side 2"));
        d.oauth = Arc::new(oauth::Manager::with_test_accounts(&[
            "a@gmail.com",
            "b@gmail.com",
            "c@gmail.com",
            "d@gmail.com",
        ]));
        assert!(validate_oauth_accounts(&d, &jobs).is_ok());
    }
    #[test]
    fn command_preserves_secrets_and_redacts_preview() {
        let a = args(&config()).unwrap();
        assert!(a.contains(&" secret ".to_string()));
        let p = preview(&a);
        assert!(!p.contains("secret"));
        assert!(!p.contains("p'word"));
        assert!(a.contains(&"--dry".into()));
        assert!(a.contains(&"--tls2".into()));
        assert!(a.contains(&"0.5".into()));
    }
    #[test]
    fn rejects_invalid_endpoints_and_executable_options() {
        let mut v = config();
        v["port1"] = json!("70000");
        assert!(args(&v).is_err());
        v = config();
        v["advancedArgs"] = json!("--exec dangerous");
        assert!(args(&v).is_err());
        v["advancedArgs"] = json!("--password1 leaked");
        assert!(args(&v).is_err());
        v["advancedArgs"] = json!("--folder 'unclosed");
        assert!(args(&v).is_err());
    }
    #[test]
    fn applies_limits_exclusions_and_redacts_logs() {
        let mut v = config();
        v["bandwidthKb"] = json!("512");
        v["excludePattern"] = json!("^Spam$\n^Trash$");
        let a = args(&v).unwrap();
        assert!(a.contains(&"524288".into()));
        assert_eq!(a.iter().filter(|s| *s == "--exclude").count(), 2);
        let mut d = Document::default();
        d.secrets.push("sensitive".into());
        log(&mut d, "a sensitive string".into(), "info");
        assert!(!d.logs[0]["message"].as_str().unwrap().contains("sensitive"));
    }

    #[test]
    fn transport_fast_mode_and_subscription_match_selected_settings() {
        let mut v = config();
        v["fastMode"] = json!(true);
        v["subscribeFolders"] = json!(false);
        let a = args(&v).unwrap();
        for expected in [
            "--ssl1",
            "--notls1",
            "--nossl2",
            "--tls2",
            "--nosubscribe",
            "--nofoldersizes",
            "--nofoldersizesatend",
        ] {
            assert!(a.contains(&expected.into()), "missing {expected}");
        }
        assert!(!a.contains(&"--fast".into()));
        assert_eq!(a.iter().filter(|s| *s == "SSL_verify_mode=1").count(), 2);
        v["security1"] = json!("plain");
        let a = args(&v).unwrap();
        assert!(a.contains(&"--nossl1".into()));
        assert!(a.contains(&"--notls1".into()));
        assert!(!a.contains(&"--sslargs1".into()));
    }

    #[test]
    fn rejects_truncated_passwords_invalid_counts_and_malformed_extra_args() {
        let mut v = config();
        v["pass1"] = json!("first\nsecond");
        assert!(args(&v).is_err());
        v = config();
        v["errorsMax"] = json!("1.5");
        assert!(args(&v).is_err());
        for extra in [
            "--folder",
            "--folder --usecache",
            "stray",
            "--regextrans1 's/a/b/'",
            "--usecache stray",
        ] {
            v = config();
            v["advancedArgs"] = json!(extra);
            assert!(args(&v).is_err(), "accepted {extra}");
        }
        v = config();
        v["advancedArgs"] = json!("--folder 'Sent Mail' --regextrans2 's,^INBOX/,,' --usecache");
        assert!(args(&v).unwrap().contains(&"Sent Mail".into()));
        v["host2"] = json!("SOURCE.EXAMPLE");
        v["security2"] = json!("ssl");
        v["user2"] = json!("a");
        assert!(args(&v).is_err());
    }

    #[test]
    fn integer_byte_limits_and_regex_escapes_are_preserved() {
        let mut v = config();
        v["bandwidthKb"] = json!("0.1");
        v["excludePattern"] = json!(r"^Folder\name$");
        let a = args(&v).unwrap();
        let i = a.iter().position(|s| s == "--maxbytespersecond").unwrap();
        assert_eq!(a[i + 1], "102");
        assert!(a.contains(&r"^Folder\name$".into()));
    }

    #[test]
    fn private_run_files_are_unique_and_cache_survives_cleanup() {
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(cache.path().join("cached-uid"), b"mapping").unwrap();
        let a = args(&config()).unwrap();
        let (dir1, actual) = prepare_run(&a, cache.path()).unwrap();
        let (dir2, _) = prepare_run(&a, cache.path()).unwrap();
        assert_ne!(dir1.path(), dir2.path());
        assert!(!actual.contains(&" secret ".into()));
        assert_eq!(
            std::fs::read_to_string(dir1.path().join("pass1")).unwrap(),
            " secret "
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(dir1.path()).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(dir1.path().join("pass1"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let path = dir1.path().to_path_buf();
        drop(dir1);
        assert!(!path.exists());
        assert!(dir2.path().join("pass2").exists());
        assert!(cache.path().join("cached-uid").exists());
    }

    #[test]
    fn passwords_that_look_like_flags_are_not_reinterpreted() {
        let mut v = config();
        v["pass1"] = json!("--password2");
        v["pass2"] = json!("--ssl1");
        let a = args(&v).unwrap();
        let cache = tempfile::tempdir().unwrap();
        let (dir, actual) = prepare_run(&a, cache.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("pass1")).unwrap(),
            "--password2"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("pass2")).unwrap(),
            "--ssl1"
        );
        assert_eq!(actual.iter().filter(|s| *s == "--passfile2").count(), 1);
        assert!(actual.contains(&"--ssl1".into()));
    }

    #[test]
    fn reversed_age_range_is_rejected_instead_of_syncing_a_union() {
        let mut v = config();
        v["minAgeDays"] = json!("365");
        v["maxAgeDays"] = json!("30");
        assert!(args(&v).is_err());
    }

    fn batch_config() -> Value {
        let mut v = config();
        v["multipleAccounts"] = json!(true);
        v["accounts1"] = json!([
            {"user":"first", "password":"first-password"},
            {"user":"second", "password":"second-password"},
            {"user":"", "password":""}
        ]);
        v["accounts2"] = json!([
            {"user":"target-one", "password":"target-one-password"},
            {"user":"target-two", "password":"target-two-password"},
            {"user":"", "password":""}
        ]);
        v
    }

    #[test]
    fn sheet_pairs_validate_all_rows_and_redact_each_preview() {
        let v = batch_config();
        let jobs = account_jobs(&v).unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[1].1["user1"], "second");
        assert_eq!(jobs[1].1["user2"], "target-two");
        assert!(jobs.iter().all(|(_, v)| v["accounts1"].is_null()));
        let preview = job_preview(&Document::default(), &jobs).unwrap();
        assert_eq!(preview.matches("imapsync ").count(), 2);
        assert!(!preview.contains("password'"));
        for (_, job) in &jobs {
            assert!(!preview.contains(&string(job, "pass1")));
            assert!(!preview.contains(&string(job, "pass2")));
        }
        let mut invalid = v.clone();
        invalid["accounts2"][1]["password"] = json!("");
        assert!(account_jobs(&invalid).unwrap_err().contains("Row 2"));
        invalid = v.clone();
        invalid["accounts2"][1]["user"] = json!("target-one");
        assert!(account_jobs(&invalid).unwrap_err().contains("duplicate"));
        invalid["accounts2"] = json!([]);
        assert!(account_jobs(&invalid).is_err());
        invalid["accounts1"] = json!([]);
        assert!(account_jobs(&invalid).is_err());
    }

    #[cfg(unix)]
    fn batch_executable(dir: &Path, script: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let executable = dir.join("fixture-imapsync");
        std::fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        executable
    }

    #[test]
    #[cfg(unix)]
    fn single_attempts_persist_success_failure_dry_run_and_stop_with_one_export_row() {
        let cache = tempfile::tempdir().unwrap();
        let journal = cache.path().join("history");
        let shared = Shared::default();
        shared.lock().unwrap().journal_dir = Some(journal.clone());
        let mut body = config();
        body["pass1"] = json!("single-source-secret");
        body["pass2"] = json!("single-dest-secret");
        body["dryRun"] = json!(false);
        for (code, dry) in [(0, false), (9, false), (0, true)] {
            body["dryRun"] = json!(dry);
            let script =
                format!("echo 'Messages transferred : 4';echo 'Detected 2 errors';exit {code}");
            launch_sync(
                &mut shared.lock().unwrap(),
                shared.clone(),
                &body,
                fake_command(&script),
                cache.path(),
            )
            .unwrap();
            wait_finished(&shared);
            let d = shared.lock().unwrap();
            assert!(d.batch.is_none());
            let record = current_result(&d).unwrap();
            let report = results::report(&record, false);
            assert_eq!(report["accountMode"], "single");
            assert_eq!(report["rows"].as_array().unwrap().len(), 1);
            assert_eq!(report["rows"][0]["exitCode"], code);
            assert_eq!(report["rows"][0]["transferred"], 4);
            assert_eq!(report["rows"][0]["errors"], 2);
            assert_eq!(report["dryRun"], dry);
            for format in ["json", "csv"] {
                let data = String::from_utf8(results::export(&record, format).unwrap()).unwrap();
                assert!(!data.contains("single-source-secret"));
                assert!(!data.contains("single-dest-secret"));
            }
        }
        body["dryRun"] = json!(false);
        launch_sync(
            &mut shared.lock().unwrap(),
            shared.clone(),
            &body,
            fake_command(
                "echo ready;while [ ! -f imapsync.pidabort$$ ]; do sleep 0.01;done;exit 0",
            ),
            cache.path(),
        )
        .unwrap();
        request_stop(&mut shared.lock().unwrap()).unwrap();
        wait_finished(&shared);
        let restarted = Document {
            journal_dir: Some(journal),
            ..Document::default()
        };
        let reports = result_reports(&restarted);
        assert_eq!(reports.len(), 4);
        assert_eq!(reports.iter().filter(|r| r["dryRun"] == true).count(), 1);
        assert!(reports.iter().any(|r| r["rows"][0]["status"] == "stopped"));
        assert!(reports.iter().any(|r| r["rows"][0]["status"] == "failed"));
        let (bytes, count) = results::export_matching(reports.clone(), &json!({}), "json").unwrap();
        assert_eq!(count, 4);
        let exported: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(exported["rowCount"], 4);
        let (csv, count) =
            results::export_matching(reports.clone(), &json!({"mode":"dry"}), "csv").unwrap();
        assert_eq!(count, 1);
        assert_eq!(String::from_utf8(csv).unwrap().split("\r\n").count(), 3);
        let page = results::search(reports, &json!({"mode":"dry"})).unwrap();
        assert_eq!(page["total"], 1);
    }

    #[test]
    fn deleting_history_preserves_recovery_state_and_hides_current_result() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("recovery");
        let config = batch_config();
        let (path, mut saved) = batch_recovery::create(&journal, &config).unwrap();
        let id = saved["id"].as_str().unwrap().to_string();
        let mut d = Document {
            journal_dir: Some(journal.clone()),
            ..Document::default()
        };
        assert!(delete_result(&mut d, &id).is_err());
        assert!(path.exists());
        saved["resolved"] = json!(true);
        saved["results"] = json!([{"row":1,"exitCode":0},{"row":2,"exitCode":161}]);
        batch_recovery::save(&path, &saved).unwrap();
        let guard = batch_recovery::lock(&path).unwrap();
        assert!(delete_result(&mut d, &id).is_err());
        assert!(path.exists());
        guard.unlock().unwrap();
        drop(guard);
        let pair = recovery::mark(&journal, &config).unwrap();
        let (_other, unfinished) = batch_recovery::create(&journal, &config).unwrap();
        delete_result(&mut d, &id).unwrap();
        assert!(!path.exists());
        assert!(pair.exists());
        assert!(batch_recovery::load(&journal, unfinished["id"].as_str().unwrap()).is_ok());
        assert!(batch_recovery::delete_history(&journal, "../../outside").is_err());
        #[cfg(unix)]
        {
            let shared = Shared::default();
            let cache = tempfile::tempdir().unwrap();
            let executable = batch_executable(cache.path(), "echo 'Detected 0 errors'");
            let mut body = config.clone();
            body["dryRun"] = json!(true);
            start_batch(
                &mut shared.lock().unwrap(),
                shared.clone(),
                &body,
                &executable,
                cache.path(),
            )
            .unwrap();
            wait_finished(&shared);
            let mut d = shared.lock().unwrap();
            assert_eq!(result_reports(&d).len(), 1);
            let mut preview = json!({"count":0});
            session_cleanup(&mut d, &json!({}), true, &mut preview);
            assert_eq!(preview["count"], 1);
            let mut result = json!({"deleted":0,"skipped":0});
            session_cleanup(
                &mut d,
                &json!({"sessionStartedAt":"different-attempt"}),
                false,
                &mut result,
            );
            assert_eq!(result["skipped"], 1);
            assert_eq!(result_reports(&d).len(), 1);
            session_cleanup(&mut d, &preview, false, &mut result);
            assert_eq!(result["deleted"], 1);
            assert!(result_reports(&d).is_empty());
            assert!(d.batch.is_some());
        }
    }

    #[test]
    fn selected_retry_creates_a_separate_queue_and_keeps_original_results() {
        let dir = tempfile::tempdir().unwrap();
        let mut d = Document {
            journal_dir: Some(dir.path().into()),
            ..Document::default()
        };
        let config = batch_config();
        let (path, mut original) = batch_recovery::create(dir.path(), &config).unwrap();
        original["resolved"] = json!(true);
        original["results"] = json!([{"row":1,"exitCode":0},{"row":2,"exitCode":161}]);
        batch_recovery::save(&path, &original).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(prepare_retry(&d, &json!({"id":original["id"],"rows":[1]})).is_err());
        let retry = prepare_retry(&d, &json!({"id":original["id"],"rows":[2]})).unwrap();
        assert_ne!(retry["id"], original["id"]);
        assert_eq!(retry["retryOf"], original["id"]);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut body = retry["settings"].clone();
        body["resumeBatch"] = retry["id"].clone();
        body["accounts1"][1]["password"] = config["accounts1"][1]["password"].clone();
        body["accounts2"][1]["password"] = config["accounts2"][1]["password"].clone();
        let jobs = resumable_jobs(&d, &body).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].0, 2);
        assert_eq!(jobs[0].1["user1"], "second");
        d.draft = body;
        let reports = result_reports(&d);
        assert_eq!(reports.len(), 2);
        assert_eq!(
            reports.iter().find(|v| v["id"] == original["id"]).unwrap()["rows"][0]["status"],
            "success"
        );
    }

    #[test]
    #[cfg(unix)]
    fn interrupted_queue_restores_completed_rows_without_passwords_or_replay() {
        let cache = tempfile::tempdir().unwrap();
        let journal = cache.path().join("recovery");
        let shared = Shared::default();
        shared.lock().unwrap().journal_dir = Some(journal.clone());
        let executable = batch_executable(
            cache.path(),
            r#"while [ "$#" -gt 0 ]; do case "$1" in --user1) account="$2";shift;; esac;shift;done;echo "running:$account";if [ "$account" = second ];then echo waiting;while :;do sleep 0.1;done;fi;echo 'Messages transferred : 3';echo 'Detected 0 errors'"#,
        );
        let mut config = batch_config();
        config["dryRun"] = json!(false);
        start_batch(
            &mut shared.lock().unwrap(),
            shared.clone(),
            &config,
            &executable,
            cache.path(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let mut d = shared.lock().unwrap();
            if d.logs.iter().any(|v| v["message"] == "waiting") {
                d.child.as_mut().unwrap().kill().unwrap();
                break;
            }
            drop(d);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
        wait_finished(&shared);
        let saved = batch_recovery::pending(&journal).remove(0);
        assert_eq!(
            batch_recovery::completed(&saved),
            std::collections::HashSet::from([1])
        );
        let restarted = Shared::default();
        restarted.lock().unwrap().journal_dir = Some(journal.clone());
        let mut resume = saved["settings"].clone();
        resume["resumeBatch"] = saved["id"].clone();
        resume["accounts1"][1]["password"] = config["accounts1"][1]["password"].clone();
        resume["accounts2"][1]["password"] = config["accounts2"][1]["password"].clone();
        assert_eq!(
            resumable_jobs(&restarted.lock().unwrap(), &resume)
                .unwrap()
                .len(),
            1
        );
        let before = std::fs::read(
            journal
                .join("batches")
                .join(format!("{}.json", saved["id"].as_str().unwrap())),
        )
        .unwrap();
        let executable = batch_executable(
            cache.path(),
            "echo 'Messages transferred : 2';echo 'Detected 0 errors'",
        );
        resume["dryRun"] = json!(true);
        start_batch(
            &mut restarted.lock().unwrap(),
            restarted.clone(),
            &resume,
            &executable,
            cache.path(),
        )
        .unwrap();
        wait_finished(&restarted);
        assert_eq!(
            std::fs::read(
                journal
                    .join("batches")
                    .join(format!("{}.json", saved["id"].as_str().unwrap()))
            )
            .unwrap(),
            before
        );
        resume["dryRun"] = json!(false);
        start_batch(
            &mut restarted.lock().unwrap(),
            restarted.clone(),
            &resume,
            &executable,
            cache.path(),
        )
        .unwrap();
        wait_finished(&restarted);
        let d = restarted.lock().unwrap();
        assert_eq!(status(&d)["batch"]["completed"], 2);
        assert_eq!(status(&d)["batch"]["failed"], 0);
        assert_eq!(status(&d)["transferred"], 5);
        assert!(batch_recovery::pending(&journal).is_empty());
        assert!(d
            .logs
            .iter()
            .all(|v| !string(v, "message").contains("running:first")));
    }

    #[test]
    #[cfg(unix)]
    fn queue_runs_in_order_and_aggregates_without_stopping_after_failure() {
        let shared = Shared::default();
        let cache = tempfile::tempdir().unwrap();
        let executable = batch_executable(
            cache.path(),
            r#"
            while [ "$#" -gt 0 ]; do
                case "$1" in --user1) account="$2"; shift;; esac
                shift
            done
            echo "running:$account"
            echo 'Messages transferred : 3'
            if [ "$account" = first ]; then echo 'Detected 1 errors'; exit 7; fi
            echo 'Detected 0 errors'
        "#,
        );
        {
            let mut d = shared.lock().unwrap();
            start_batch(
                &mut d,
                shared.clone(),
                &batch_config(),
                &executable,
                cache.path(),
            )
            .unwrap();
            assert_eq!(status(&d)["batch"]["total"], 2);
            assert!(start_batch(
                &mut d,
                shared.clone(),
                &batch_config(),
                &executable,
                cache.path()
            )
            .is_err());
        }
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        let snapshot = status(&d);
        assert_eq!(snapshot["batch"]["completed"], 2);
        assert_eq!(snapshot["batch"]["failed"], 1);
        assert_eq!(snapshot["transferred"], 6);
        assert_eq!(snapshot["errors"], 1);
        assert_ne!(d.exit, Some(0));
        let logs = d
            .logs
            .iter()
            .map(|v| string(v, "message"))
            .collect::<Vec<_>>();
        let first = logs.iter().position(|s| s == "running:first").unwrap();
        let second = logs.iter().position(|s| s == "running:second").unwrap();
        assert!(first < second);
        assert!(d.secrets.is_empty());
        assert!(d.run_dir.is_none());
        assert!(d.batch.as_ref().unwrap().pending.is_empty());
        drop(d);
        // A new batch must reset prior failures, logs and aggregate counters.
        let mut next = batch_config();
        next["accounts1"][0]["user"] = json!("third");
        start_batch(
            &mut shared.lock().unwrap(),
            shared.clone(),
            &next,
            &executable,
            cache.path(),
        )
        .unwrap();
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        assert_eq!(d.exit, Some(0));
        assert_eq!(status(&d)["batch"]["failed"], 0);
        assert_eq!(status(&d)["transferred"], 6);
        assert_eq!(status(&d)["errors"], 0);
    }

    #[test]
    #[cfg(unix)]
    fn stop_cancels_pending_accounts_and_invalid_sheet_starts_nothing() {
        let shared = Shared::default();
        let cache = tempfile::tempdir().unwrap();
        let executable = batch_executable(
            cache.path(),
            "echo ready; while [ ! -f imapsync.pidabort$$ ]; do sleep 0.1; done; exit 0",
        );
        {
            let mut d = shared.lock().unwrap();
            let mut invalid = batch_config();
            invalid["accounts2"][1]["password"] = json!("");
            assert!(
                start_batch(&mut d, shared.clone(), &invalid, &executable, cache.path()).is_err()
            );
            assert!(d.child.is_none());
            assert!(d.batch.is_none());
            start_batch(
                &mut d,
                shared.clone(),
                &batch_config(),
                &executable,
                cache.path(),
            )
            .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if shared
                .lock()
                .unwrap()
                .logs
                .iter()
                .any(|v| v["message"] == "ready")
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
        request_stop(&mut shared.lock().unwrap()).unwrap();
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        let batch = d.batch.as_ref().unwrap();
        assert_eq!(batch.completed, 1);
        assert_eq!(batch.current, 1);
        assert!(batch.pending.is_empty());
        assert_eq!(batch.results[0]["cancelled"], true);
        assert!(d.cancelled);
    }

    #[test]
    fn non_utf8_output_is_drained_and_metrics_survive_log_truncation() {
        let shared = Shared::default();
        shared.lock().unwrap().secrets = vec!["secret".into()];
        let mut bytes = b"invalid \xff secret\n".to_vec();
        for _ in 0..1300 {
            bytes.extend_from_slice(b"msg 1/INBOX copied to 2/INBOX\n");
        }
        bytes.extend_from_slice(b"Messages transferred : 1300\nDetected 0 errors\nlast line");
        read_output(bytes.as_slice(), shared.clone(), "info");
        let d = shared.lock().unwrap();
        assert_eq!(d.transferred, 1300);
        assert_eq!(d.errors, Some(0));
        assert_eq!(d.logs.len(), 1200);
        assert_eq!(d.logs.back().unwrap()["message"], "last line");
    }

    #[cfg(unix)]
    fn fake_command(script: &str) -> Command {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", script, "fake-imapsync"]);
        cmd
    }

    #[cfg(unix)]
    fn wait_finished(shared: &Shared) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while shared.lock().unwrap().child.is_some() {
            assert!(Instant::now() < deadline, "child did not finish");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    #[cfg(unix)]
    fn process_completion_drains_logs_cleans_secrets_and_can_restart() {
        let shared = Shared::default();
        let cache = tempfile::tempdir().unwrap();
        let run_path;
        {
            let mut d = shared.lock().unwrap();
            launch_sync(&mut d, shared.clone(), &config(), fake_command("printf 'secret late output\\nMessages transferred : 42\\nDetected 0 errors\\n'; exit 0"), cache.path()).unwrap();
            run_path = d.run_dir.as_ref().unwrap().path().to_path_buf();
            assert!(launch_sync(
                &mut d,
                shared.clone(),
                &config(),
                fake_command("exit 0"),
                cache.path()
            )
            .is_err());
        }
        wait_finished(&shared);
        {
            let mut d = shared.lock().unwrap();
            assert_eq!(d.exit, Some(0));
            assert_eq!(d.transferred, 42);
            assert_eq!(d.errors, Some(0));
            assert!(d.finished.is_some());
            assert!(d.secrets.is_empty());
            assert!(!run_path.exists());
            assert!(d.logs.back().unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("finished"));
            launch_sync(
                &mut d,
                shared.clone(),
                &config(),
                fake_command("printf 'second run\\n'; exit 7"),
                cache.path(),
            )
            .unwrap();
        }
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        assert_eq!(d.exit, Some(7));
        assert!(!d
            .logs
            .iter()
            .any(|v| v["message"].as_str().unwrap().contains("late output")));
    }

    #[test]
    #[cfg(unix)]
    fn stop_remains_running_until_graceful_exit() {
        let shared = Shared::default();
        let cache = tempfile::tempdir().unwrap();
        {
            let mut d = shared.lock().unwrap();
            launch_sync(
                &mut d,
                shared.clone(),
                &config(),
                fake_command(
                    "echo ready; while [ ! -f imapsync.pidabort$$ ]; do sleep 0.1; done; echo saved-cache; exit 0",
                ),
                cache.path(),
            )
            .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if shared
                .lock()
                .unwrap()
                .logs
                .iter()
                .any(|v| v["message"] == "ready")
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
        {
            let mut d = shared.lock().unwrap();
            request_stop(&mut d).unwrap();
            request_stop(&mut d).unwrap(); // idempotent
            assert_eq!(status(&d)["running"], true);
            assert_eq!(status(&d)["stopping"], true);
        }
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        assert!(d.cancelled);
        assert!(d.logs.iter().any(|v| v["message"] == "saved-cache"));
        assert_eq!(d.logs.back().unwrap()["message"], "--- Sync stopped ---");
    }

    #[test]
    fn spawn_failure_does_not_leave_a_running_job() {
        let shared = Shared::default();
        let cache = tempfile::tempdir().unwrap();
        let mut d = shared.lock().unwrap();
        let missing = cache.path().join("missing-executable");
        assert!(launch_sync(
            &mut d,
            shared.clone(),
            &config(),
            Command::new(missing),
            cache.path()
        )
        .is_err());
        assert!(d.child.is_none());
        assert!(d.run_dir.is_none());
    }

    #[test]
    #[cfg(unix)]
    fn unresponsive_process_is_force_stopped_after_grace_period() {
        let shared = Shared::default();
        let cache = tempfile::tempdir().unwrap();
        {
            let mut d = shared.lock().unwrap();
            launch_sync(
                &mut d,
                shared.clone(),
                &config(),
                fake_command("trap '' TERM; echo ready; while :; do sleep 0.1; done"),
                cache.path(),
            )
            .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if shared
                .lock()
                .unwrap()
                .logs
                .iter()
                .any(|v| v["message"] == "ready")
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
        {
            let mut d = shared.lock().unwrap();
            request_stop(&mut d).unwrap();
            d.stopping = Some(Instant::now() - Duration::from_secs(31));
        }
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        assert!(d.cancelled);
        assert_eq!(d.exit, Some(-1));
        assert!(d.run_dir.is_none());
    }

    #[test]
    fn bundled_imapsync_accepts_generated_options() {
        let cache = tempfile::tempdir().unwrap();
        let mut v = config();
        v["fastMode"] = json!(true);
        v["subscribeFolders"] = json!(false);
        v["useUid"] = json!(true);
        v["bandwidthKb"] = json!("0.1");
        v["stopAfterMb"] = json!("1.1");
        v["advancedArgs"] = json!("--folder 'Sent Mail' --regextrans2 's,^INBOX/,,' --useheader Message-Id --syncinternaldates --addheader --skipemptyfolders --skipcrossduplicates");
        let (dir, actual) = prepare_run(&args(&v).unwrap(), cache.path()).unwrap();
        let mut command = Command::new(binary().unwrap());
        engine::sanitize(&mut command);
        let output = command
            .args(actual)
            .arg("--justbanner")
            .current_dir(dir.path())
            .output()
            .unwrap();
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{text}");
        assert!(text.contains("Exiting because of --justbanner"), "{text}");
        assert!(!text.contains("Unknown option"), "{text}");
    }

    #[test]
    #[ignore = "Run through tests/imap/run.py with disposable loopback IMAP servers"]
    fn local_imap_integration_driver() {
        let input = std::env::var("IMAPSYNC_TEST_INPUT").expect("fixture input path required");
        let fixture: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
        let mut config = fixture["config"].clone();
        if fixture["retryLatestFailed"] == true {
            let journal = PathBuf::from(string(&fixture, "cache")).join("recovery");
            let record = batch_recovery::history(&journal)
                .into_iter()
                .find(|v| {
                    v["results"]
                        .as_array()
                        .is_some_and(|r| r.iter().any(|row| row["exitCode"] != 0))
                })
                .expect("failed source report missing");
            let d = Document {
                journal_dir: Some(journal),
                ..Document::default()
            };
            let retry =
                prepare_retry(&d, &json!({"id":record["id"],"rows":fixture["retryRows"]})).unwrap();
            let mut selected = retry["settings"].clone();
            for side in ["1", "2"] {
                for (index, account) in selected[format!("accounts{side}")]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .enumerate()
                {
                    if account["user"].as_str().is_some_and(|s| !s.is_empty()) {
                        account["password"] =
                            config[format!("accounts{side}")][index]["password"].clone();
                    }
                }
            }
            selected["resumeBatch"] = retry["id"].clone();
            selected["dryRun"] = config["dryRun"].clone();
            config = selected;
        }
        if fixture["resumeBatchLatest"] == true {
            let journal = PathBuf::from(string(&fixture, "cache")).join("recovery");
            config["resumeBatch"] = batch_recovery::pending(&journal)
                .first()
                .expect("batch recovery missing")["id"]
                .clone();
        }
        let body = &config;
        // This test must never target an external mailbox, even with a mistaken fixture.
        for suffix in ["1", "2"] {
            assert!(matches!(
                string(body, &format!("host{suffix}")).as_str(),
                "127.0.0.1" | "localhost"
            ));
        }
        if fixture["operation"] == "connection" {
            let result = connection(
                json!({"host":body["host1"], "port":body["port1"], "security":body["security1"]}),
            );
            let error = result.expect_err("untrusted fixture certificate was accepted");
            assert!(
                error.to_lowercase().contains("cert"),
                "TLS failed for an unrelated reason: {error}"
            );
            std::fs::write(
                string(&fixture, "output"),
                b"{\"certificateRejected\":true}",
            )
            .unwrap();
            return;
        }
        let shared = Shared::default();
        let cache = PathBuf::from(string(&fixture, "cache"));
        shared.lock().unwrap().journal_dir = Some(if fixture["isolatedJournal"] == true {
            cache.join("isolated-recovery")
        } else {
            cache.join("recovery")
        });
        if fixture["operation"] == "history" {
            let d = shared.lock().unwrap();
            let id = string(&fixture, "reportId");
            let record = result_record(&d, &id).unwrap();
            let output = json!({"reports":result_reports(&d),"csv":String::from_utf8(results::export(&record,"csv").unwrap()).unwrap(),"json":String::from_utf8(results::export(&record,"json").unwrap()).unwrap()});
            std::fs::write(
                string(&fixture, "output"),
                serde_json::to_vec(&output).unwrap(),
            )
            .unwrap();
            return;
        }
        {
            let mut d = shared.lock().unwrap();
            if body["multipleAccounts"] == true {
                start_batch(&mut d, shared.clone(), body, &binary().unwrap(), &cache).unwrap();
            } else {
                launch_sync(
                    &mut d,
                    shared.clone(),
                    body,
                    Command::new(binary().unwrap()),
                    &cache,
                )
                .unwrap();
            }
        }
        let started = Instant::now();
        let cancel_after = fixture["cancelAfterMs"].as_u64();
        let mut cancel_sent = false;
        while shared.lock().unwrap().child.is_some() {
            if let Some(ms) = cancel_after {
                let copied_enough = shared.lock().unwrap().transferred
                    >= fixture["cancelAfterMessages"].as_u64().unwrap_or(0);
                let row_matches = fixture["cancelAfterRow"].as_u64().is_none_or(|row| {
                    shared
                        .lock()
                        .unwrap()
                        .batch
                        .as_ref()
                        .is_some_and(|b| b.row == row as usize)
                });
                if !cancel_sent
                    && copied_enough
                    && row_matches
                    && started.elapsed() >= Duration::from_millis(ms)
                {
                    if fixture["hardKill"] == true {
                        shared
                            .lock()
                            .unwrap()
                            .child
                            .as_mut()
                            .unwrap()
                            .kill()
                            .unwrap();
                    } else {
                        request_stop(&mut shared.lock().unwrap()).unwrap();
                    }
                    cancel_sent = true;
                }
            }
            if started.elapsed() > Duration::from_secs(60) {
                shutdown(&shared);
                panic!("local IMAP integration timed out");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let d = shared.lock().unwrap();
        let mut result = json!({"status":status(&d), "logs":d.logs});
        if fixture["includeExports"] == true {
            let record = current_result(&d).unwrap();
            result["reports"] = json!(result_reports(&d));
            result["csv"] =
                json!(String::from_utf8(results::export(&record, "csv").unwrap()).unwrap());
            result["json"] =
                json!(String::from_utf8(results::export(&record, "json").unwrap()).unwrap());
        }
        let serialized = serde_json::to_vec_pretty(&result).unwrap();
        for secret in [
            "fixture-alice",
            "fixture-bob",
            "fixture-carol",
            "wrong-fixture-password",
        ] {
            assert!(
                !String::from_utf8_lossy(&serialized).contains(secret),
                "fixture secret leaked"
            );
        }
        assert!(d.run_dir.is_none());
        std::fs::write(string(&fixture, "output"), serialized).unwrap();
    }
}
