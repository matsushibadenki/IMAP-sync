# Local IMAP integration tests

These tests run the **real Rust process manager and bundled imapsync** against two disposable, loopback-only IMAP servers. The servers use [pymap's in-memory backend](https://github.com/icgood/pymap), not a simulated imapsync executable. No external mailbox credentials are needed, and no production mailboxes are accessed.

## Requirements

- Python 3.11 or newer, Rust, npm and OpenSSL for the disposable TLS fixture.
- The normal Tauri build prerequisites for your platform.
- A prepared bundled engine (`npm run engine:prepare`, after the toolchain/dependency setup in the main README) and built frontend (`npm run build`).

From the repository root, create an isolated test environment (macOS/Linux):

```sh
python3.12 -m venv .venv
.venv/bin/python -m pip install -r tests/imap/requirements.txt
.venv/bin/python tests/imap/run.py --output test-results/imap-integration.json
```

On Windows, use `.venv\Scripts\python.exe` instead. Windows execution remains unverified.

The runner builds the Rust test driver once, starts servers on automatically assigned ports bound to `127.0.0.1`, and creates only synthetic accounts and messages. Each scenario uses the production argument builder and process manager. Interruption journals and mailbox-pair locks are also isolated under the fixture cache. Mail and UID caches are confined to a temporary directory; servers and files are removed on completion or failure. The optional output contains scenario names, not passwords or message contents. A failed resume check writes an adjacent diagnostic containing synthetic message hashes and redacted logs.

## Scenarios

1. Authenticate in dry-run mode without modifying the destination.
2. Reject an incorrect password without modifying either mailbox.
3. Transfer exact MIME bytes, including a binary attachment and multilingual text; preserve message flags, internal dates and folders.
4. Reopen single-account success, failure and dry-run history in a new Rust process and verify one-row CSV/JSON exports exclude credentials.
5. Re-run with the same UID cache and transfer zero messages without duplicates.
6. Continue a batch after an actual authentication failure, retaining the overall failure result.
7. Complete a successful multi-account batch.
8. Retry a selected failed row in a separate queue, preserve the original failed report, skip the successful account and parse matching credential-free CSV/JSON exports.
9. Verify a deletion dry run is inert and actual deletion removes only an extra destination message.
10. Stop after at least one real message copy, then resume without missing or duplicate messages.
11. Force-kill imapsync after a real APPEND, reopen the persisted journal in a new Rust driver process, verify a recovery dry run is inert, then recover and re-run with exact mail integrity and no duplicates.
12. Interrupt the second row of a three-account queue, restart the Rust driver, restore completed results without completed-row passwords, run an inert dry run, then resume the interrupted and pending rows and compare exact mailbox integrity.
13. Reject an untrusted STARTTLS certificate in both the Rust connection probe and real imapsync.

The TLS fixture uses a temporary self-signed certificate solely to verify rejection. It does not add trust roots or disable certificate validation in the product. Successful TLS with real provider certificate chains, provider throttling, OAuth and platform-specific behavior require separate verification.

The Rust bridge is opt-in and intentionally refuses non-loopback hosts. Do not run it directly: the Python runner supplies its temporary configuration and verifies the results independently using Python's IMAP client.

## Harness regression checks

```sh
.venv/bin/python -m unittest discover -s tests/imap -p 'test_*.py'
```

The fixture uses the fixed greeting hostname `imap-fixture.test` so hosted-runner reverse DNS cannot block initial IMAP responses. Readiness requires a greeting and successful NOOP, not only a bound socket. With `--output`, a summary is written on success or failure; `status`, completed `scenarios`, and (on failure) `errorType` are recorded. Failures still return a nonzero exit code and print their traceback in the job log. Exception messages are excluded from uploaded summaries.
