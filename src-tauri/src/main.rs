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
    finished: Option<String>,
    cancelled: bool,
    dry_run: bool,
    transferred: u64,
    errors: Option<u64>,
    batch: Option<Batch>,
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
    let source = endpoint(v, "1")?;
    let destination = endpoint(v, "2")?;
    if source.0.eq_ignore_ascii_case(&destination.0)
        && source.1 == destination.1
        && string(v, "user1") == string(v, "user2")
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
            format!("--password{i}"),
            required(v, &format!("pass{i}"))?,
        ]);
        passwords.push(a.len() - 1);
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
    })
}
fn preview(a: &SyncArgs) -> String {
    let mut redacted = a.to_vec();
    for &i in &a.passwords {
        redacted[i] = "••••••••".into();
    }
    format!("imapsync {}", shell_words::join(redacted))
}

fn account_jobs(body: &Value) -> Result<Vec<(usize, Value)>, String> {
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
        if !used_destinations.insert(string(&job, "user2")) {
            return Err(format!("Row {}: duplicate destination account", index + 1));
        }
        jobs.push((index + 1, job));
    }
    if jobs.is_empty() {
        return Err("Enter at least one account pair".into());
    }
    Ok(jobs)
}

fn job_preview(jobs: &[(usize, Value)]) -> Result<String, String> {
    jobs.iter()
        .map(|(row, body)| Ok(format!("# {row}\n{}", preview(&args(body)?))))
        .collect::<Result<Vec<_>, String>>()
        .map(|s| s.join("\n\n"))
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
    let jobs = account_jobs(body)?; // Validate every row before starting any migration.
    let command_preview = job_preview(&jobs)?;
    d.secrets = jobs
        .iter()
        .flat_map(|(_, v)| [string(v, "pass1"), string(v, "pass2")])
        .collect();
    d.secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    d.batch = Some(Batch {
        total: jobs.len(),
        pending: jobs.into(),
        current: 0,
        row: 0,
        completed: 0,
        failed: 0,
        transferred: 0,
        errors: Some(0),
        results: Vec::new(),
        executable: executable.into(),
        cache: cache.into(),
    });
    d.preview = command_preview;
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
        "batch":batch.map(|b| json!({"total":b.total,"current":b.current,"row":b.row,"completed":b.completed,"failed":b.failed,"results":b.results}))})
}
fn binary() -> String {
    for p in [
        "/opt/homebrew/bin/imapsync",
        "/usr/local/bin/imapsync",
        "/usr/bin/imapsync",
    ] {
        if std::path::Path::new(p).is_file() {
            return p.into();
        }
    }
    "imapsync".into()
}

fn prepare_run(a: &SyncArgs, cache: &Path) -> Result<(tempfile::TempDir, Vec<String>), String> {
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
    #[cfg(unix)]
    {
        // imapsync handles TERM by closing its connections and saving its cache.
        let result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
        if result != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(unix))]
    child.kill().map_err(|e| e.to_string())?;
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
    let (child, dir) = {
        let mut d = shared.lock().unwrap();
        let _ = request_stop(&mut d);
        if let Some(batch) = d.batch.as_mut() {
            batch.pending.clear();
        }
        (d.child.take(), d.run_dir.take())
    };
    // Release the document lock so readers can drain output during shutdown.
    if let Some(mut child) = child {
        let deadline = Instant::now() + Duration::from_secs(5);
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
    drop(dir);
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
    let a = args(body)?;
    let (dir, actual) = prepare_run(&a, cache)?;
    cmd.args(actual)
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("imapsync: {e}"))?;
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
    d.finished = None;
    d.exit = None;
    d.stopping = None;
    d.cancelled = false;
    d.dry_run = body["dryRun"] == true;
    d.transferred = 0;
    d.errors = None;
    d.run_dir = Some(dir);
    let message = d.batch.as_ref().map_or_else(
        || "--- Sync started ---".into(),
        |b| format!("--- Account {}/{} (row {}) ---", b.current, b.total, b.row),
    );
    log(d, message, "info");
    log(d, preview(&a), "command");
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out_state = shared.clone();
    let err_state = shared.clone();
    let out_reader = std::thread::spawn(move || read_output(stdout, out_state, "info"));
    let err_reader = std::thread::spawn(move || read_output(stderr, err_state, "error"));
    d.child = Some(child);
    std::thread::spawn(move || {
        let code = loop {
            std::thread::sleep(Duration::from_millis(100));
            let mut d = shared.lock().unwrap();
            let force_stop = d
                .stopping
                .is_some_and(|t| t.elapsed() >= Duration::from_secs(5));
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
        d.exit = Some(if readers_ok { code } else { -1 });
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
        let transferred = d.transferred;
        let errors = d.errors;
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
                .push(json!({"row":batch.row,"exitCode":code,"cancelled":cancelled}));
            if cancelled {
                batch.pending.clear();
            }
            d.transferred = 0;
            d.errors = Some(0);
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
    if path == "/api/test-connection" {
        return tauri::async_runtime::spawn_blocking(move || connection(body))
            .await
            .map_err(|e| e.to_string())?;
    }
    let mut d = shared.lock().map_err(|e| e.to_string())?;
    match path.as_str() {
        "/api/status" => Ok(status(&d)),
        "/api/draft" => Ok(d.draft.clone()),
        "/api/save-draft" => {
            d.draft = body;
            Ok(json!({}))
        }
        "/api/preview" => {
            let jobs = account_jobs(&body)?;
            d.preview = if body["multipleAccounts"] == true {
                job_preview(&jobs)?
            } else {
                preview(&args(&body)?)
            };
            Ok(json!({"commandPreview":d.preview}))
        }
        "/api/sync" => {
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
                start_batch(&mut d, shared.clone(), &body, Path::new(&binary()), &cache)?;
            } else {
                if d.child.is_some() {
                    return Err("Sync already running".into());
                }
                d.batch = None;
                launch_sync(
                    &mut d,
                    shared.clone(),
                    &body,
                    Command::new(binary()),
                    &cache,
                )?;
            }
            Ok(json!({"status":status(&d)}))
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
        .manage(Shared::default())
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
        let preview = job_preview(&jobs).unwrap();
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
            "trap 'exit 0' TERM; echo ready; while :; do sleep 0.1; done",
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
                    "trap 'echo saved-cache; exit 0' TERM; echo ready; while :; do sleep 0.1; done",
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
            d.stopping = Some(Instant::now() - Duration::from_secs(6));
        }
        wait_finished(&shared);
        let d = shared.lock().unwrap();
        assert!(d.cancelled);
        assert_eq!(d.exit, Some(-1));
        assert!(d.run_dir.is_none());
    }

    #[test]
    #[ignore = "Requires installed imapsync; validates options without connecting to mail servers"]
    fn installed_imapsync_accepts_generated_options() {
        let cache = tempfile::tempdir().unwrap();
        let mut v = config();
        v["fastMode"] = json!(true);
        v["subscribeFolders"] = json!(false);
        v["useUid"] = json!(true);
        v["bandwidthKb"] = json!("0.1");
        v["stopAfterMb"] = json!("1.1");
        v["advancedArgs"] = json!("--folder 'Sent Mail' --regextrans2 's,^INBOX/,,' --useheader Message-Id --syncinternaldates --addheader --skipemptyfolders --skipcrossduplicates");
        let (dir, actual) = prepare_run(&args(&v).unwrap(), cache.path()).unwrap();
        let output = Command::new(binary())
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
}
