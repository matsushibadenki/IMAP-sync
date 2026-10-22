# Imapsync Studio Desktop

`../imapsync-web` を元にした Tauri 2 + Rust デスクトップアプリです。Node.js HTTP サーバーは使用しません。

## 開発・ビルド

Rust、Node.js、Tauri 2 のOS別ビルド要件、`imapsync` が必要です。

```sh
npm install
npm run tauri dev
npm run tauri build
```

macOS の生成物: `src-tauri/target/release/bundle/`。
`imapsync` は Homebrew 等で別途インストールしてください。アプリは `/opt/homebrew/bin`、`/usr/local/bin`、`/usr/bin`、PATH の順に検索します。

## 操作

接続設定、移行オプション、ログ、進捗、実行前チェック、コマンドプレビューをタブで切り替えます。↗ でパネルを別ウインドウへ切り離し、↙ でメインへ戻します。タブは矢印キーでも切り替えられます。説明は ⓘ にマウスを重ねて表示します。

日本語・English・简体中文を右上で切り替えられます。接続設定を入力してプレビューし、ドライランで確認してから実行してください。接続テストは TCP/TLS/STARTTLS の疎通確認です。認証の確認は実際のドライランで行います。

## 構成

- WebView: 既存画面を引き継いだ HTML/CSS/JavaScript。Vite でビルド。
- Rust Document: すべてのウインドウで共有する設定、子プロセス、終了コード、最大1200行のログ。
- Rust: 引数生成、検証、TCP/TLS/STARTTLS、imapsync の起動・停止・終了回収。
- ウインドウ: 共有設定の表示を担当。入力・プロセス情報は Tauri IPC で共有。

パスワードは設定のローカル保存から除外し、Rust共有メモリに保持します。実行時は権限を制限した一時ディレクトリのパスワードファイルを使い、終了時に削除します。異常強制終了後は一時ファイルが残る場合があります。ログとプレビューはパスワードを伏せ字にし、imapsync のディスクログを無効にします。

追加引数は安全に扱えるオプションの許可リスト方式です。任意の実行オプションや認証引数の上書きは拒否します。同期・進捗は実際の出力を表示し、実行中の推測パーセントは表示しません。

このツールはメール移行を扱い、削除同期は転送先のメールを削除します。初回はドライランを使用してください。

## 検証

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## Roadmap

- 🟢 [Done] Tauri 2 + Rust、6タブ、切り離し・再ドッキング、共有設定、ログ・起動・停止、3言語
- 🟢 [Done] TLS証明書検証、秘密情報の伏せ字、パスワードを除く下書き保存
- 🟠 [Next] ドラッグ操作による自由なパネル配置、ドッキング位置の永続化
- 🔴 [Later] 署名・公証、imapsync の同梱配布
- ⭕️ [Pending] 実メールサーバーでの認証・転送・削除同期検証（テストアカウント未提供）

## English

A Tauri 2 / Rust desktop port of imapsync-web. Six workspace tabs support floating windows. Rust owns the shared document, process and logs. Run `npm install`, then `npm run tauri dev`; package with `npm run tauri build`. Install imapsync separately. Connection tests check reachability and TLS, not authentication. Start with dry run. Passwords are excluded from saved drafts.

## 简体中文

基于 imapsync-web 的 Tauri 2 / Rust 桌面应用。六个工作区标签支持独立窗口，Rust 管理共享设置、进程和日志。运行 `npm install` 和 `npm run tauri dev`，使用 `npm run tauri build` 打包。需另行安装 imapsync。连接测试检查网络和 TLS，不验证账户密码。请先试运行。保存草稿时不保存密码。
