#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Default)]
struct Document {
    child: Option<Child>,
    started: Option<String>,
    exit: Option<i32>,
    preview: String,
    logs: VecDeque<Value>,
    secrets: Vec<String>,
    draft: Value,
}
type Shared = Arc<Mutex<Document>>;
fn string(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}
fn required(v: &Value, key: &str) -> Result<String, String> {
    let s = string(v, key);
    if s.trim().is_empty() {
        Err(format!("Required: {key}"))
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
fn args(v: &Value) -> Result<Vec<String>, String> {
    let mut a = Vec::new();
    for i in ["1", "2"] {
        let (host, port, mode) = endpoint(v, i)?;
        a.extend([
            format!("--host{i}"),
            host,
            format!("--port{i}"),
            port.to_string(),
            format!("--user{i}"),
            required(v, &format!("user{i}"))?,
            format!("--password{i}"),
            required(v, &format!("pass{i}"))?,
        ]);
        if mode == "ssl" {
            a.push(format!("--ssl{i}"));
        }
        if mode == "starttls" {
            a.push(format!("--tls{i}"));
        }
    }
    for (key, flag) in [
        ("dryRun", "--dry"),
        ("syncDeletes", "--delete2"),
        ("subscribeFolders", "--subscribe"),
        ("automapFolders", "--automap"),
        ("useUid", "--useuid"),
        ("fastMode", "--fast"),
    ] {
        if v[key] == true {
            a.push(flag.into())
        }
    }
    let safe = v["providerSafeMode"] == true;
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
            let val = (n * scale).to_string();
            a.extend([flag.into(), val.clone()]);
            if key == "timeoutSeconds" {
                a.extend(["--timeout2".into(), val]);
            }
        }
    }
    for line in string(v, "excludePattern")
        .replace("\\n", "\n")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        a.extend(["--exclude".into(), line.into()]);
    }
    let extra = shell_words::split(&string(v, "advancedArgs")).map_err(|e| e.to_string())?;
    // Prevent arbitrary Perl execution, alternate credentials and logging secrets via expert options.
    let allowed = [
        "--nofoldersizes",
        "--nofoldersizesatend",
        "--regextrans2",
        "--regextrans1",
        "--folder",
        "--folderrec",
        "--skipcrossduplicates",
        "--skipemptyfolders",
        "--addheader",
        "--useheader",
        "--usecache",
        "--syncinternaldates",
    ];
    for token in &extra {
        if token.starts_with('-') && !allowed.contains(&token.as_str()) {
            return Err(format!("Unsupported advanced option: {token}"));
        }
    }
    a.extend(extra);
    a.extend(["--nolog".into()]);
    Ok(a)
}
fn preview(a: &[String]) -> String {
    let mut redacted = a.to_vec();
    for i in 1..a.len() {
        if a[i - 1].starts_with("--password") {
            redacted[i] = "••••••••".into()
        }
    }
    format!("imapsync {}", shell_words::join(redacted))
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
    json!({"running":d.child.is_some(),"startedAt":d.started,"lastExitCode":d.exit,"commandPreview":d.preview})
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
async fn api(path: String, body: Value, state: tauri::State<'_, Shared>) -> Result<Value, String> {
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
        "/api/preview" => Ok(json!({"commandPreview":preview(&args(&body)?)})),
        "/api/sync" => {
            if d.child.is_some() {
                return Err("Sync already running".into());
            }
            let a = args(&body)?;
            let mut cmd = Command::new(binary());
            // Private temporary password files keep credentials out of process listings.
            let dir = std::env::temp_dir().join(format!("imapsync-{}", std::process::id()));
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                    .map_err(|e| e.to_string())?;
            }
            let mut actual = a.clone();
            for i in 1..actual.len() {
                if a[i - 1].starts_with("--password") {
                    let suffix = a[i - 1].trim_start_matches("--password");
                    let file = dir.join(format!("pass{suffix}"));
                    std::fs::write(&file, &a[i]).map_err(|e| e.to_string())?;
                    actual[i - 1] = format!("--passfile{suffix}");
                    actual[i] = file.to_string_lossy().into_owned();
                }
            }
            cmd.args(actual)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(format!("imapsync: {e}"));
                }
            };
            d.logs.clear();
            d.preview = preview(&a);
            d.secrets = vec![string(&body, "pass1"), string(&body, "pass2")];
            d.started = Some(chrono::Utc::now().to_rfc3339());
            d.exit = None;
            log(&mut d, "--- Sync started ---".into(), "info");
            let p = d.preview.clone();
            log(&mut d, p, "command");
            for (pipe, level) in [
                (
                    child
                        .stdout
                        .take()
                        .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
                    "info",
                ),
                (
                    child
                        .stderr
                        .take()
                        .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
                    "error",
                ),
            ] {
                let shared = shared.clone();
                let secrets = d.secrets.clone();
                std::thread::spawn(move || {
                    if let Some(pipe) = pipe {
                        for line in BufReader::new(pipe).lines() {
                            match line {
                                Ok(mut line) => {
                                    for secret in &secrets {
                                        if !secret.is_empty() {
                                            line = line.replace(secret, "••••••••");
                                        }
                                    }
                                    log(&mut shared.lock().unwrap(), line, level)
                                }
                                Err(_) => break,
                            }
                        }
                    }
                });
            }
            d.child = Some(child);
            let monitor = shared.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(150));
                let mut d = monitor.lock().unwrap();
                let done = d.child.as_mut().map(|c| c.try_wait());
                match done {
                    Some(Ok(Some(s))) => {
                        d.child = None;
                        d.exit = Some(s.code().unwrap_or(-1));
                        let code = d.exit.unwrap();
                        log(
                            &mut d,
                            format!("--- Sync finished with exit code {code} ---"),
                            if code == 0 { "success" } else { "error" },
                        );
                        d.secrets.clear();
                        let _ = std::fs::remove_dir_all(&dir);
                        break;
                    }
                    None => {
                        let _ = std::fs::remove_dir_all(&dir);
                        break;
                    }
                    _ => {}
                }
            });
            Ok(json!({"status":status(&d)}))
        }
        "/api/stop" => {
            if let Some(c) = d.child.as_mut() {
                c.kill().map_err(|e| e.to_string())?;
                Ok(json!({"status":status(&d)}))
            } else {
                Err("No process running".into())
            }
        }
        _ => Err("Unknown command".into()),
    }
}
#[tauri::command]
fn logs(state: tauri::State<Shared>) -> Vec<Value> {
    state.lock().unwrap().logs.iter().cloned().collect()
}
#[tauri::command]
fn float_panel(app: tauri::AppHandle, panel: usize) -> Result<(), String> {
    if panel > 5 {
        return Err("Invalid panel".into());
    }
    let label = format!("panel-{panel}");
    if let Some(w) = app.get_webview_window(&label) {
        w.set_focus().map_err(|e| e.to_string())?;
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            label,
            tauri::WebviewUrl::App(format!("index.html?panel={panel}").into()),
        )
        .title("Imapsync Studio")
        .inner_size(900., 700.)
        .min_inner_size(480., 560.)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
fn dock_panel(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        app.get_webview_window("main")
            .ok_or("Main window missing")?
            .set_focus()
            .map_err(|e| e.to_string())?;
        if let Some(panel) = window
            .label()
            .strip_prefix("panel-")
            .and_then(|s| s.parse::<usize>().ok())
        {
            app.emit_to("main", "dock-panel", panel)
                .map_err(|e| e.to_string())?;
        }
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() {
    tauri::Builder::default()
        .manage(Shared::default())
        .invoke_handler(tauri::generate_handler![api, logs, float_panel, dock_panel])
        .build(tauri::generate_context!())
        .expect("Tauri initialization failed")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let state = app.state::<Shared>();
                let mut d = state.lock().unwrap();
                if let Some(c) = d.child.as_mut() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                let _ = std::fs::remove_dir_all(
                    std::env::temp_dir().join(format!("imapsync-{}", std::process::id())),
                );
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
}
