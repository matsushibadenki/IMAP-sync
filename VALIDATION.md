# Verification

2026-10-22 / macOS

- 🟢 [Done] `npm run build` passed.
- 🟢 [Done] `cargo test --offline --manifest-path src-tauri/Cargo.toml`: 3 passed.
- 🟢 [Done] `npm run tauri build -- --debug --bundles app`: generated `Imapsync Studio.app`.
- 🟢 [Done] Launched the generated native application and inspected its accessibility tree and screenshot.
- 🟢 [Done] Connections → Migration Options → Command Preview switches the visible panel.
- 🟢 [Done] Japanese → English → Simplified Chinese → Japanese updates navigation and form labels.
- 🟢 [Done] Floating a command panel opens `index.html?panel=5`; docking closes the floating window and returns to the main window.
- 🟢 [Done] Sample settings + disposable test passwords produce a Rust-generated command with both passwords redacted, dry-run and provider-safe limits included.
- 🟢 [Done] Restart restores non-password draft fields; password inputs remain empty.
- 🟢 [Done] Final screenshot confirms compact option rows and tooltip information icons.
- ⭕️ [Pending] Live IMAP authentication, actual transfers, delete synchronization, cancellation during transfer: no test mailboxes supplied.
- ⭕️ [Pending] Narrow-window interaction testing, Windows/Linux packaging, signed/notarized release packaging.
- 🟠 [Next] Drag-based docking and persistent panel layout. Current floating/docking uses toolbar buttons.

No real mailbox migration was run during verification. The generated app is a local unsigned debug build; imapsync is installed separately.
