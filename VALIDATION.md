# Verification

2026-10-22 / macOS

- 🟢 [Done] `npm run build` passed.
- 🟢 [Done] `cargo test --offline --manifest-path src-tauri/Cargo.toml`: 3 passed.
- 🟢 [Done] `npm run tauri build -- --debug --bundles app`: generated `Imapsync Studio.app`.
- 🟢 [Done] Launched the generated native application and inspected its accessibility tree and screenshot.
- 🟢 [Done] Connections → Migration Options → Command Preview switches the visible panel.
- 🟢 [Done] Japanese → English → Simplified Chinese → Japanese updates navigation and form labels.
- 🟢 [Done] Floating/docking controls and Rust commands were removed at user request; all five tabs use the main window.
- 🟢 [Done] Sample settings + disposable test passwords produce a Rust-generated command with both passwords redacted, dry-run and provider-safe limits included.
- 🟢 [Done] Restart restores non-password draft fields; password inputs remain empty.
- 🟢 [Done] Final screenshot confirms compact option rows and tooltip information icons.
- ⭕️ [Pending] Live IMAP authentication, actual transfers, delete synchronization, cancellation during transfer: no test mailboxes supplied.
- ⭕️ [Pending] Narrow-window interaction testing, Windows/Linux packaging, signed/notarized release packaging.

No real mailbox migration was run during verification. The generated app is a local unsigned debug build; imapsync is installed separately.


## Migration reliability review — 2026-10-22

- 🟢 [Done] Compared generated arguments with the installed imapsync 2.314 source and upstream implementation. Replaced unsupported `--fast`, removed unsupported `--regextrans1`, made transport and subscription switches explicit, and enabled TLS peer/hostname verification.
- 🟢 [Done] `cargo test --offline --manifest-path src-tauri/Cargo.toml`: 14 passed; one installed-binary check is opt-in. Covers argument validation, reversed date ranges, literal password values, private-file permissions, cache retention, non-UTF-8 output, bounded-log metrics, normal/failed completion, repeat runs, double-start rejection, graceful stop and forced-stop fallback.
- 🟢 [Done] `cargo test --offline --manifest-path src-tauri/Cargo.toml installed_imapsync_accepts_generated_options -- --ignored --nocapture`: passed with imapsync 2.314 and `--justbanner`; no mailbox connection.
- 🟢 [Done] `npm run build` passed.
- 🟢 [Done] `cargo clippy --offline --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passed.
- 🟢 [Done] Playwright + local Chrome, 1280×850, with a mock Tauri bridge: double-submit prevention, running → stopping → stopped, final elapsed time, reported transfer/error counts, dry-run vs failure status, and Japanese/English/Simplified Chinese status labels passed; no JavaScript runtime errors. Browser skill unavailable; used bundled Playwright.
- ⭕️ [Pending] Real-server authentication, message/attachment integrity, duplicate suppression after resume, certificate-chain compatibility, and deletion semantics require disposable test mailboxes. No live mail transfer or deletion was performed.
- ⭕️ [Pending] Windows/Linux runtime and packaging; graceful TERM handling was exercised on macOS with fixture processes.


## Multiple-account mode — 2026-10-22

- 🟢 [Done] Rust tests: 17 passed, one installed-imapsync check remains opt-in. Batch coverage includes row pairing, blank-row skipping, all-row validation before launch, duplicate destinations, per-account preview redaction, sequential execution, continuation after failure, aggregate metrics, fresh-batch reset and cancellation of remaining accounts.
- 🟢 [Done] `npm run build` and `cargo clippy --offline --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passed.
- 🟢 [Done] Playwright + local Chrome at 1280×900 and 375×900, mock Tauri bridge: button/mode switching, two sheets with two credential columns, paired add/remove, TSV paste with password whitespace preserved, password masking, localStorage redaction, all-account preview/start payloads, result-row states, username restoration without passwords, and Japanese/English/Simplified Chinese layout passed. Screenshots inspected; no page errors or horizontal overflow.
- ⭕️ [Pending] Live batch migration, attachment integrity, authentication and provider rate limits require disposable test accounts; no real mailboxes were accessed.
