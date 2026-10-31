#!/usr/bin/env python3
"""Exercise the real Rust process manager + bundled imapsync on isolated IMAP servers."""
import argparse
import contextlib
import csv
import io
import hashlib
import imaplib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import datetime
from email.message import EmailMessage
from email.policy import SMTP
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def check(condition, message):
    if not condition:
        raise AssertionError(message)


@contextlib.contextmanager
def server(directory, name, cert=None, key=None):
    ready = directory / f"{name}-ready.json"
    command = [sys.executable, str(Path(__file__).with_name("server.py")), "--ready", str(ready)]
    if cert:
        command += ["--cert", str(cert), "--key", str(key)]
    with (directory / f"{name}.log").open("wb") as log:
        process = subprocess.Popen(command, stdout=log, stderr=log)
        try:
            deadline = time.monotonic() + 15
            while not ready.exists():
                check(process.poll() is None, f"{name} IMAP server exited:\n{Path(log.name).read_text()}")
                check(time.monotonic() < deadline, f"{name} IMAP server startup timed out")
                time.sleep(0.05)
            port = json.loads(ready.read_text())["port"]
            # A bound port is not proof that the IMAP greeting/command loop works.
            with imaplib.IMAP4("127.0.0.1", port, timeout=10) as probe:
                status, _ = probe.noop()
                check(status == "OK", f"{name} IMAP readiness check failed")
            yield port
        except BaseException:
            sys.stderr.write(Path(log.name).read_text())
            raise
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


@contextlib.contextmanager
def mailbox(port, user="alice"):
    client = imaplib.IMAP4("127.0.0.1", port, timeout=10)
    try:
        client.login(user, "fixture-" + user)
        yield client
    finally:
        with contextlib.suppress(imaplib.IMAP4.error, OSError):
            client.logout()


def message(index, user):
    msg = EmailMessage(policy=SMTP)
    msg["From"] = "sender@example.test"
    msg["To"] = f"{user}@example.test"
    msg["Date"] = "Thu, 02 Jan 2020 03:04:05 +0000"
    msg["Message-ID"] = f"<{user}-{index}@imap-sync.test>"
    msg["Subject"] = f"Migration fixture {index}: 日本語 / 简体中文"
    msg.set_content("Preserve Unicode: 日本語 / 简体中文\n" + str(index))
    msg.add_attachment(bytes(range(256)) * 16, maintype="application", subtype="octet-stream", filename=f"attachment-{index}.bin")
    # EmailMessage generates a random MIME boundary once; snapshots verify those exact bytes.
    return msg.as_bytes()


def append(port, user, folder, index):
    with mailbox(port, user) as client:
        if folder != "INBOX":
            client.create(folder)
        status, _ = client.append(folder, r"(\Seen \Flagged)", 1577934245.0, message(index, user))
        check(status == "OK", "fixture APPEND failed")


def snapshot(port, user="alice"):
    result = {}
    with mailbox(port, user) as client:
        status, folders = client.list()
        check(status == "OK", "LIST failed")
        for line in folders:
            match = re.search(rb'\) (?:"[^"]*"|NIL) (.*)$', line)
            check(match is not None, "unexpected fixture LIST response")
            folder = match[1].decode().strip('"')
            status, _ = client.select('"' + folder + '"', readonly=True)
            if status != "OK":
                continue
            status, found = client.uid("search", None, "ALL")
            check(status == "OK", "SEARCH failed")
            messages = []
            for uid in found[0].split():
                status, parts = client.uid("fetch", uid, "(BODY.PEEK[] FLAGS INTERNALDATE)")
                check(status == "OK", "FETCH failed")
                record = next(part for part in parts if isinstance(part, tuple))
                content = record[1]
                metadata = b" ".join(part[0] if isinstance(part, tuple) else part for part in parts if part)
                date = re.search(rb'INTERNALDATE "([^"]+)"', metadata)
                check(date is not None, "missing INTERNALDATE")
                messages.append({
                    "sha256": hashlib.sha256(content).hexdigest(),
                    "flags": sorted(flag.decode() for flag in imaplib.ParseFlags(metadata) if flag != b"\\Recent"),
                    "internaldate": datetime.strptime(date[1].decode(), "%d-%b-%Y %H:%M:%S %z").timestamp(),
                })
            result[folder] = sorted(messages, key=lambda m: m["sha256"])
    return result


def run(args, passed):
    check(shutil.which("cargo") is not None, "cargo is required")
    check(list((ROOT/"src-tauri/binaries").glob("imapsync-engine-*")), "run npm run engine:prepare first")
    print("Building the Rust integration driver once...", flush=True)
    build = subprocess.run(["cargo", "test", "--locked", "--no-run", "--message-format=json", "--manifest-path", str(ROOT/"src-tauri/Cargo.toml")], cwd=ROOT, capture_output=True, text=True, timeout=600)
    check(build.returncode == 0, f"Rust build failed:\n{build.stderr}")
    artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
    executable = next(item["executable"] for item in artifacts if item.get("reason")=="compiler-artifact" and item.get("profile",{}).get("test") and item.get("executable") and item["target"]["name"]=="imapsync-studio")
    with tempfile.TemporaryDirectory(prefix="imap-sync-e2e-") as temporary:
        directory = Path(temporary)
        def run(name, config, **extra):
            output = directory / f"{name}-result.json"
            fixture = directory / f"{name}-input.json"
            fixture.write_text(json.dumps({"config": config, "cache": str(directory / "cache"), "output": str(output), **extra}))
            fixture.chmod(0o600)
            env = {**os.environ, "IMAPSYNC_TEST_INPUT": str(fixture)}
            completed = subprocess.run([executable, "tests::local_imap_integration_driver", "--ignored", "--exact", "--nocapture"], cwd=ROOT, env=env, capture_output=True, text=True, timeout=90)
            check(completed.returncode == 0 and output.exists(), f"{name} Rust driver failed:\n{completed.stdout}\n{completed.stderr}")
            return json.loads(output.read_text())
        def record(name):
            passed.append(name)
            print(f"PASS: {name}", flush=True)
        with server(directory, "source") as source, server(directory, "destination") as destination:
            config = {"host1":"127.0.0.1", "port1":str(source), "security1":"plain", "user1":"alice", "pass1":"fixture-alice", "host2":"127.0.0.1", "port2":str(destination), "security2":"plain", "user2":"alice", "pass2":"fixture-alice", "dryRun":False, "useUid":True, "fastMode":True, "subscribeFolders":True, "timeoutSeconds":"3", "errorsMax":"3"}
            for index, folder in [(1,"INBOX"),(2,"INBOX"),(3,"Archive")]:
                append(source, "alice", folder, index)
            append(source, "bob", "INBOX", 10)
            original = snapshot(source)
            empty = snapshot(destination)
            result = run("dry-run", {**config, "dryRun":True}, includeExports=True)
            check(result["status"]["lastExitCode"] == 0 and snapshot(destination) == empty, "dry run modified destination or failed")
            dry_id=json.loads(result["json"])["id"]
            check(json.loads(result["json"])["accountMode"]=="single" and len(list(csv.DictReader(io.StringIO(result["csv"].lstrip("\ufeff")))))==1,"single dry-run export mismatch")
            record("dry run authenticates without changing destination")
            result = run("bad-password", {**config, "pass1":"wrong-fixture-password"}, isolatedJournal=True, includeExports=True)
            check(result["status"]["lastExitCode"] != 0 and snapshot(destination) == empty, "failed authentication was reported as success")
            check(json.loads(result["json"])["rows"][0]["status"]=="failed","single authentication failure was not retained in history")
            record("invalid authentication fails without mailbox changes")
            result = run("transfer", config, includeExports=True)
            check(result["status"]["lastExitCode"] == 0, json.dumps(result))
            check(snapshot(destination) == original and snapshot(source) == original, "MIME bytes, flags, dates, folders, or source changed")
            record("real transfer preserves MIME/attachment bytes, flags, internal dates and folders")
            view=json.loads(result["json"]);rows=list(csv.DictReader(io.StringIO(result["csv"].lstrip("\ufeff"))))
            check(view["accountMode"]=="single" and len(view["rows"])==1 and view["rows"][0]["status"]=="success" and len(rows)==1 and rows[0]["account_mode"]=="single","single transfer exports were not one-row CSV/JSON")
            check(view["rows"][0]["transferred"]==3 and rows[0]["source_user"]=="alice","single transfer history lost account/counter fields")
            restarted=run("single-history-restart",config,operation="history",reportId=view["id"])
            check(json.loads(restarted["json"])["rows"]==view["rows"],"single result changed after process restart")
            check(any(r["id"]==dry_id and r["dryRun"] for r in restarted["reports"]),"single dry-run history did not survive restart")
            for exported in [restarted["json"],restarted["csv"]]:
                check(not any(secret in exported for secret in ["fixture-alice","fixture-bob","password","access_token"]),"single export leaked credentials")
            record("single-account success/failure/dry-run history persists across process restart with one-row credential-free CSV/JSON")
            result = run("repeat", config)
            check(result["status"]["lastExitCode"] == 0 and result["status"]["transferred"] == 0 and snapshot(destination) == original, "UID-cache rerun duplicated messages")
            record("UID-cache rerun transfers zero messages without duplicates")
            batch = {**config, "multipleAccounts":True, "accounts1":[{"user":"alice","password":"wrong-fixture-password"},{"user":"bob","password":"fixture-bob"}], "accounts2":[{"user":"alice","password":"fixture-alice"},{"user":"bob","password":"fixture-bob"}]}
            result = run("batch-failure", batch)
            check(result["status"]["batch"]["failed"] == 1 and result["status"]["batch"]["completed"] == 2 and result["status"]["lastExitCode"] != 0, "batch did not preserve failure")
            check(snapshot(destination,"bob") == snapshot(source,"bob"), "batch failed to continue to second account")
            record("batch continues after a real authentication failure and reports partial failure")
            batch["accounts1"][0]["password"] = "fixture-alice"
            retry_result=run("selected-retry",batch,retryLatestFailed=True,retryRows=[1],includeExports=True)
            check(retry_result["status"]["lastExitCode"]==0 and retry_result["status"]["batch"]["total"]==1 and retry_result["status"]["batch"]["results"][0]["row"]==1,"selected retry ran the wrong rows")
            parsed=json.loads(retry_result["json"])
            exported=list(csv.DictReader(io.StringIO(retry_result["csv"].lstrip("\ufeff"))))
            check(len(parsed["rows"])==1 and parsed["rows"][0]["status"]=="success" and len(exported)==1 and exported[0]["row"]=="1","exported retry result mismatch")
            check(any(any(row["status"]=="failed" for row in report["rows"]) for report in retry_result["reports"]),"retry overwrote the original failed report")
            check(snapshot(destination)==original and snapshot(destination,"bob")==snapshot(source,"bob"),"selected retry damaged or duplicated mail")
            record("selected failed-row retry preserves original results, skips successful rows, and exports matching CSV/JSON without credentials")
            result = run("batch-success", batch)
            check(result["status"]["lastExitCode"] == 0 and result["status"]["batch"]["failed"] == 0, "valid batch failed")
            record("all-success batch completes both accounts")
            append(destination,"alice","INBOX",99)
            extra_snapshot = snapshot(destination)
            result = run("delete-dry", {**config,"syncDeletes":True,"dryRun":True})
            check(result["status"]["lastExitCode"] == 0 and snapshot(destination) == extra_snapshot, "delete dry run removed a message")
            result = run("delete", {**config,"syncDeletes":True})
            check(result["status"]["lastExitCode"] == 0 and snapshot(destination) == original and snapshot(source) == original, "delete sync damaged source or retained extra destination message")
            record("delete dry run is inert; real delete removes only the extra destination message")
            for index in range(20,30):
                append(source,"alice","INBOX",index)
            expected = snapshot(source)
            result = run("cancel", {**config,"maxMessagesPerSecond":"1","maxSleepSeconds":"1"}, cancelAfterMs=1000, cancelAfterMessages=1, includeExports=True)
            check(result["status"]["cancelled"] and not result["status"]["running"], "real transfer did not stop")
            check(json.loads(result["json"])["rows"][0]["status"]=="stopped","single stopped history was not saved")
            result = run("resume", config)
            actual = snapshot(destination)
            unchanged = snapshot(source)
            if result["status"]["lastExitCode"] != 0 or actual != expected or unchanged != expected:
                diagnostic = {"run":result,"expected":expected,"destination":actual,"source":unchanged}
                report = args.output.with_name("imap-resume-diagnostic.json") if args.output else Path(tempfile.gettempdir())/"imap-resume-diagnostic.json"
                report.parent.mkdir(parents=True,exist_ok=True)
                report.write_text(json.dumps(diagnostic,indent=2))
                raise AssertionError(f"resumed transfer lost/duplicated messages or changed metadata; see {report}")
            record("interrupted transfer resumes with exact content and no duplicates")
            # Kill during imapsync's post-APPEND sleep, before its UID-cache update.
            for index in range(100,106):
                append(source,"bob","INBOX",index)
            bob_config={**config,"user1":"bob","pass1":"fixture-bob","user2":"bob","pass2":"fixture-bob"}
            bob_expected=snapshot(source,"bob")
            result=run("forced-kill",{**bob_config,"maxMessagesPerSecond":"0.2","maxSleepSeconds":"5"},cancelAfterMs=100,cancelAfterMessages=1,hardKill=True)
            check(result["status"]["lastExitCode"] != 0 and result["status"]["recoveryPending"] >= 1,"forced termination did not retain recovery journal")
            partial=snapshot(destination,"bob")
            check(partial != bob_expected,"forced kill did not interrupt an unfinished transfer")
            result=run("forced-dry",{**bob_config,"dryRun":True})
            check(result["status"]["lastExitCode"] == 0 and snapshot(destination,"bob")==partial and result["status"]["recoveryPending"]>=1,"recovery dry run changed mail or cleared journal")
            check("--nouseuid" in result["status"]["commandPreview"] and "--nousecache" in result["status"]["commandPreview"],"recovery kept UID-cache matching")
            result=run("forced-resume",bob_config)
            check(result["status"]["lastExitCode"] == 0 and snapshot(destination,"bob")==bob_expected and snapshot(source,"bob")==bob_expected,"forced recovery lost or duplicated MIME/attachments/flags/dates")
            result=run("forced-repeat",bob_config)
            check(result["status"]["lastExitCode"] == 0 and result["status"]["transferred"]==0 and snapshot(destination,"bob")==bob_expected and "--nouseuid" in result["status"]["commandPreview"],"post-recovery rerun reused stale UID cache or duplicated messages")
            record("forced termination persists across driver restarts; dry run, recovery and subsequent rerun preserve exact mail without duplicates")
            for index in range(300,306):
                append(source,"bob","INBOX",index)
            append(source,"carol","INBOX",400)
            queue={**config,"multipleAccounts":True,"maxMessagesPerSecond":"0.2","maxSleepSeconds":"5","accounts1":[{"user":u,"password":"fixture-"+u} for u in ["alice","bob","carol"]],"accounts2":[{"user":u,"password":"fixture-"+u} for u in ["alice","bob","carol"]]}
            carol_empty=snapshot(destination,"carol")
            result=run("queue-interrupt",queue,cancelAfterMs=100,cancelAfterMessages=1,cancelAfterRow=2,hardKill=True)
            check(result["status"]["batch"]["results"][0]["exitCode"]==0 and result["status"]["batch"]["completed"]==2,"queue did not interrupt after first completed row")
            check(snapshot(destination,"carol")==carol_empty,"pending row was run after interruption")
            queue["accounts1"][0]["password"]=""
            queue["accounts2"][0]["password"]=""
            partial=snapshot(destination,"bob")
            result=run("queue-resume-dry",{**queue,"dryRun":True},resumeBatchLatest=True)
            check(result["status"]["lastExitCode"]==0 and snapshot(destination,"bob")==partial and snapshot(destination,"carol")==carol_empty,"queue recovery dry run changed mail")
            result=run("queue-resume-real",queue,resumeBatchLatest=True)
            check(result["status"]["lastExitCode"]==0 and result["status"]["batch"]["completed"]==3 and result["status"]["batch"]["failed"]==0,"queue recovery failed or lost completed result")
            check(not any("Account 2/3 (row 1)" in v["message"] for v in result["logs"]),"completed row was replayed")
            for user in ["alice","bob","carol"]:
                check(snapshot(destination,user)==snapshot(source,user),"queue recovery changed exact mail integrity")
            record("interrupted batch restores completed results and resumes interrupted/pending rows without completed-row passwords")
            openssl = shutil.which("openssl")
            check(openssl is not None, "openssl is required for the untrusted-certificate test")
            cert, key = directory/"fixture.crt", directory/"fixture.key"
            subprocess.run([openssl,"req","-x509","-newkey","rsa:2048","-nodes","-days","1","-subj","/CN=localhost","-keyout",str(key),"-out",str(cert)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            with server(directory,"untrusted-tls",cert,key) as tls_port:
                tls_config={**config,"host1":"localhost","port1":str(tls_port),"security1":"starttls"}
                result=run("tls-probe",tls_config,operation="connection")
                check(result["certificateRejected"],"connection probe accepted an untrusted certificate")
                result=run("tls-transfer",tls_config)
                check(result["status"]["lastExitCode"] != 0 and snapshot(destination) == expected,"transfer accepted an untrusted certificate or changed destination")
            record("connection probe and real imapsync reject an untrusted STARTTLS certificate")
    print(f"{len(passed)} integration scenarios passed; all servers and fixture data removed.",flush=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="save a non-secret scenario summary, including failures")
    args = parser.parse_args(argv)
    passed = []
    summary = {"server": "pymap 0.36.7", "scenarios": passed, "externalMailboxes": False, "status": "failed"}
    try:
        run(args, passed)
        summary["status"] = "passed"
    except BaseException as error:
        # Keep arbitrary exception text/configuration out of uploaded summaries.
        summary["errorType"] = type(error).__name__
        raise
    finally:
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(summary, indent=2) + "\n")


if __name__ == "__main__":
    main()
