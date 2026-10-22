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

進捗と実行ボタンを上部に常時表示し、接続設定、移行オプション、ログ、実行前チェック、コマンドプレビューをタブで切り替えます。タブは矢印キーでも切り替えられます。説明は ⓘ にマウスを重ねて表示します。

日本語・English・简体中文を右上で切り替えられます。接続設定を入力してプレビューし、ドライランで確認してから実行してください。接続テストは TCP/TLS/STARTTLS の疎通確認です。認証の確認は実際のドライランで行います。

### 複数アカウント

接続設定見出しの右端にある「複数アカウント」で切り替えます。転送元・転送先のホスト／ポート／暗号化方式は共通とし、それぞれのシートへユーザー名とパスワードを入力します。左右の同じ行が転送ペアです。

- 行の追加・削除は左右のシートに同時適用します。行番号にマウスを重ねると削除ボタンになります。Excelなどからユーザー名／パスワードの2列を貼り付けできます。最大1000行です。
- プレビューは全ペアのコマンドを表示します。開始前に全行を検証し、片側の不足や転送先アカウントの重複があれば実行を開始しません。両側とも空の行はスキップします。
- 上から1件ずつ実行します。アカウントの失敗は記録して次を続け、全体の結果はエラーとして表示します。停止は実行中の処理と未実行の残りを中止します。起動そのものに失敗した場合も残りを中止します。
- 行番号の色で実行中・成功・失敗を表示し、ログに行番号と個別の終了コードを残します。上部の件数はバッチ全体の合計です。
- ユーザー名とモードは保存します。単一入力とシートのパスワードはローカル保存せず、再起動後は再入力します。

## 構成

- WebView: 既存画面を引き継いだ HTML/CSS/JavaScript。Vite でビルド。
- Rust Document: 設定、子プロセス、終了コード、最大1200行のログ。
- Rust: 引数生成、検証、TCP/TLS/STARTTLS、imapsync の起動・停止・終了回収。
- ウインドウ: タブで画面を切り替え、入力・プロセス情報は Tauri IPC でRustと連携。

パスワードは設定のローカル保存から除外し、Rust共有メモリに保持します。実行時は権限を制限した一時ディレクトリのパスワードファイルを使い、終了時に削除します。異常強制終了後は一時ファイルが残る場合があります。ログとプレビューはパスワードを伏せ字にし、imapsync のディスクログを無効にします。

追加引数は安全に扱えるオプションの許可リスト方式です。任意の実行オプションや認証引数の上書きは拒否します。同期・進捗は実際の出力を表示し、実行中の推測パーセントは表示しません。

このツールはメール移行を扱い、削除同期は転送先のメールを削除します。初回はドライランを使用してください。

### 移行の実行と再開

- 暗号化方式は選択値を明示して実行し、TLS接続では証明書とホスト名を検証します。高速モードは開始・終了時のフォルダ容量集計だけを省略します。
- 停止要求後もプロセスが終了するまで実行中として扱います。macOS/Linuxでは終了シグナルを送り、5秒後も応答しない場合は強制終了します。Windowsではプロセス終了を要求します。
- パスワードとPIDファイルは実行ごとの一時ディレクトリに置き、終了時に削除します。再開に必要なUIDキャッシュは削除しません。既存のimapsyncと同じ `$HOME/tmp` を使用し、HOMEがない環境のみアプリのローカルデータ領域を使用します。UID比較を使用する場合はキャッシュを保持し、同じメールボックスへの同時実行を避けてください。
- 失敗・停止後はログを確認し、同じ設定で再実行できます。自動再試行は行いません。ドライラン完了・停止・異常終了は別の状態として表示します。
- 転送件数はコピー結果と終了時の集計から取得します。エラー数はimapsyncの集計が届くまで「—」です。1200行を超えてログが省略されても件数を維持し、終了後の経過時間は固定します。


## 検証

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## Roadmap

- 🟢 [Done] Tauri 2 + Rust、5タブ、共有設定、ログ・起動・停止、3言語
- 🟢 [Done] TLS証明書検証、秘密情報の伏せ字、パスワードを除く下書き保存
- 🟢 [Done] 複数アカウントシート、2列貼り付け、逐次転送キュー、アカウントごとの結果表示
- 🔴 [Later] 署名・公証、imapsync の同梱配布
- ⭕️ [Pending] 実メールサーバーでの認証・転送・削除同期検証（テストアカウント未提供）

## English

A Tauri 2 / Rust desktop port of imapsync-web. Five workspace tabs share one application window. Rust owns the shared document, process and logs. Run `npm install`, then `npm run tauri dev`; package with `npm run tauri build`. Install imapsync separately. Connection tests check reachability and TLS, not authentication. Start with dry run. Passwords are excluded from saved drafts.

## 简体中文

基于 imapsync-web 的 Tauri 2 / Rust 桌面应用。五个工作区标签在同一窗口内切换，Rust 管理共享设置、进程和日志。运行 `npm install` 和 `npm run tauri dev`，使用 `npm run tauri build` 打包。需另行安装 imapsync。连接测试检查网络和 TLS，不验证账户密码。请先试运行。保存草稿时不保存密码。


Execution notes: TLS verifies certificates and hostnames. Fast mode skips folder-size scans. Stop waits for process exit (TERM, then a five-second force-stop fallback on macOS/Linux). Per-run password files are removed; the existing `$HOME/tmp` UID cache is retained for resuming. Keep that cache and avoid concurrent runs against the same mailbox. Dry-run completion, cancellation and failure have distinct states. Error counts remain unknown until imapsync reports them.

执行说明：TLS 验证证书及主机名；快速模式跳过文件夹容量统计。停止操作会等待进程退出（macOS/Linux 先发送 TERM，五秒后仍未退出则强制结束）。每次执行的密码文件会删除，原有 `$HOME/tmp` UID 缓存会保留以便继续迁移。请保留缓存，并避免同时迁移同一邮箱。试运行完成、停止和失败分别显示；imapsync 尚未报告错误总数时显示「—」。

Multiple accounts: Toggle the button in the Connections heading. Enter usernames and passwords in the two sheets; matching row numbers form a pair. Hosts and migration options are shared. Paste two columns from a spreadsheet, add/remove paired rows, then preview or start. All rows are validated first and run sequentially. Account failures are recorded while remaining pairs continue; Stop cancels the rest. Passwords are never persisted locally.

多个账户：点击连接设置标题右侧的按钮，在来源和目标表格中填写用户名与密码。相同行号为一组，共用服务器与迁移选项。支持粘贴两列及成对增删行。开始前验证所有行，再按顺序执行；单个账户失败会记录并继续，停止会取消剩余任务。密码不保存到本地。
