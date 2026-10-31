import './style.css';
import body from './body';
import {PRESETS} from './presets';
document.body.innerHTML=body;
const invoke=(command,args={})=>window.__TAURI__ ? window.__TAURI__.core.invoke(command,args) : Promise.reject('Please open this interface in the Tauri desktop app.');

(async()=>{
        const i18n = {
            ja: {
                tagline: 'メール移行を設計、確認、実行するローカルGUI',
                statusIdle: '待機中', statusRunning: '同期中', statusSuccess: '完了', statusError: 'エラー終了',
                loadSample: 'サンプル', connections: '接続設定', connectionsSub: '転送元と転送先のIMAP情報を入力し、実行前に疎通を確認します。',
                source: '転送元メールボックス', destination: '転送先メールボックス', test: 'テスト',
                host: 'ホスト', port: 'ポート', security: '暗号化', user: 'ユーザー', password: 'パスワード',
                options: '移行オプション', optionsSub: 'まずドライランで確認し、必要な同期ポリシーだけを有効にします。',
                dryRun: 'ドライラン', dryRunHelp: '変更せずに差分と見積もりだけ確認します。',
                syncDeletes: '削除を同期', syncDeletesHelp: '転送元にないメールを転送先から削除します。',
                subscribe: '購読状態を反映', subscribeHelp: 'フォルダ購読情報も移行します。',
                automap: '標準フォルダを自動対応', automapHelp: 'Sent、Trashなどの名称差を吸収します。',
                useUid: 'UIDベースで比較', useUidHelp: '同一サーバー間などの高速比較に使います。',
                fastMode: '高速モード', fastModeHelp: '安全な範囲でimapsyncの高速化オプションを使います。',
                providerSafeMode: 'プロバイダ配慮モード', providerSafeModeHelp: '大量アクセスに見えないよう、速度・エラー停止・タイムアウトに保守的な既定値を使います。',
                maxMessagesPerSecond: '最大メッセージ/秒', maxSleepSeconds: '最大待機秒', maxBytesAfter: '制限開始 MB', errorsMax: 'エラー停止数', timeoutSeconds: '接続タイムアウト秒', stopAfter: '今回の上限 MB',
                maxAge: '直近N日だけ', minAge: 'N日より古い', bandwidth: '帯域制限 KB/s', exclude: '除外フォルダ/正規表現', advanced: '追加imapsync引数',
                logs: 'ログ', logsSub: '実行ログ、エラー、生成されたコマンドを時系列で確認できます。',
                all: 'すべて', errors: 'エラー', commands: 'コマンド', autoscroll: '自動スクロール ON', autoscrollOff: '自動スクロール OFF', clear: 'クリア',
                emptyLog: 'まだログはありません。設定を入力してプレビューまたは同期を開始してください。',
                progress: '進捗', messages: 'messages', errorCount: 'errors', elapsed: 'elapsed',
                readiness: '実行前チェック', checkSource: '転送元の必須項目', checkDestination: '転送先の必須項目', checkSafe: '初回はドライラン推奨', checkRate: '速度制限または配慮モード', checkPreview: 'コマンドプレビュー生成',
                commandPreview: 'コマンドプレビュー', redacted: 'パスワードは伏せて表示します。', preview: 'プレビュー', copy: 'コピー', start: '同期開始', stop: '停止',
                previewReady: 'コマンドプレビューを更新しました。', started: '同期を開始しました。', stopped: '同期を停止しました。', copied: 'コマンドをコピーしました。', sampleLoaded: 'サンプル設定を入力しました。',
                testOk: '接続OK', testFail: '接続失敗'
            },
            en: {
                tagline: 'A local GUI to design, verify, run, and monitor mail migrations',
                statusIdle: 'Idle', statusRunning: 'Running', statusSuccess: 'Finished', statusError: 'Finished with error',
                loadSample: 'Sample', connections: 'Connections', connectionsSub: 'Enter source and destination IMAP details, then check reachability before running.',
                source: 'Source mailbox', destination: 'Destination mailbox', test: 'Test',
                host: 'Host', port: 'Port', security: 'Security', user: 'User', password: 'Password',
                options: 'Migration Options', optionsSub: 'Start with a dry run, then enable only the policy you need.',
                dryRun: 'Dry run', dryRunHelp: 'Preview differences and estimates without changing mailboxes.',
                syncDeletes: 'Sync deletes', syncDeletesHelp: 'Delete destination messages missing from the source.',
                subscribe: 'Copy subscriptions', subscribeHelp: 'Migrate folder subscription state.',
                automap: 'Auto-map folders', automapHelp: 'Map common Sent, Trash, and Draft folder names.',
                useUid: 'Compare by UID', useUidHelp: 'Useful for fast comparisons on compatible servers.',
                fastMode: 'Fast mode', fastModeHelp: 'Enable safe imapsync speed-oriented options.',
                providerSafeMode: 'Provider-safe mode', providerSafeModeHelp: 'Use conservative defaults for pacing, error stop, and connection timeouts so the migration looks less abusive.',
                maxMessagesPerSecond: 'Max messages/sec', maxSleepSeconds: 'Max sleep sec', maxBytesAfter: 'Limit after MB', errorsMax: 'Stop after errors', timeoutSeconds: 'Timeout seconds', stopAfter: 'Stop after MB',
                maxAge: 'Only last N days', minAge: 'Older than N days', bandwidth: 'Bandwidth KB/s', exclude: 'Excluded folders / regex', advanced: 'Extra imapsync arguments',
                logs: 'Logs', logsSub: 'Review runtime output, errors, and generated commands chronologically.',
                all: 'All', errors: 'Errors', commands: 'Commands', autoscroll: 'Auto-scroll ON', autoscrollOff: 'Auto-scroll OFF', clear: 'Clear',
                emptyLog: 'No logs yet. Enter settings, preview the command, or start a migration.',
                progress: 'Progress', messages: 'messages', errorCount: 'errors', elapsed: 'elapsed',
                readiness: 'Readiness', checkSource: 'Source required fields', checkDestination: 'Destination required fields', checkSafe: 'Dry run is recommended first', checkRate: 'Rate limit or safe mode', checkPreview: 'Command preview generated',
                commandPreview: 'Command Preview', redacted: 'Passwords are hidden.', preview: 'Preview', copy: 'Copy', start: 'Start Sync', stop: 'Stop',
                previewReady: 'Command preview updated.', started: 'Sync started.', stopped: 'Sync stopped.', copied: 'Command copied.', sampleLoaded: 'Sample settings loaded.',
                testOk: 'Connection OK', testFail: 'Connection failed'
            }
        };

        i18n.zh = {"tagline": "本地邮件迁移工具", "statusIdle": "待机", "statusRunning": "同步中", "statusSuccess": "已完成", "statusError": "异常结束", "loadSample": "示例", "connections": "连接设置", "connectionsSub": "填写来源和目标 IMAP 信息并测试连接。", "source": "来源邮箱", "destination": "目标邮箱", "test": "测试", "host": "主机", "port": "端口", "security": "加密", "user": "用户", "password": "密码", "options": "迁移选项", "optionsSub": "先试运行，再启用所需选项。", "dryRun": "试运行", "dryRunHelp": "只检查差异，不修改邮箱。", "syncDeletes": "同步删除", "syncDeletesHelp": "删除目标中来源不存在的邮件。", "subscribe": "复制订阅", "subscribeHelp": "迁移文件夹订阅状态。", "automap": "自动映射文件夹", "automapHelp": "自动匹配已发送、垃圾箱等文件夹。", "useUid": "按 UID 比较", "useUidHelp": "用于兼容服务器之间的快速比较。", "fastMode": "快速模式", "fastModeHelp": "使用 imapsync 加速选项。", "providerSafeMode": "服务商友好模式", "providerSafeModeHelp": "使用保守的速率、错误上限和超时。", "maxMessagesPerSecond": "每秒最大邮件数", "maxSleepSeconds": "最长等待秒数", "maxBytesAfter": "限速起点 MB", "errorsMax": "错误上限", "timeoutSeconds": "连接超时秒数", "stopAfter": "本次上限 MB", "maxAge": "最近 N 天", "minAge": "早于 N 天", "bandwidth": "带宽 KB/s", "exclude": "排除文件夹 / 正则表达式", "advanced": "附加 imapsync 参数", "logs": "日志", "logsSub": "查看执行输出、错误和命令。", "all": "全部", "errors": "错误", "commands": "命令", "autoscroll": "自动滚动 开", "autoscrollOff": "自动滚动 关", "clear": "清空", "emptyLog": "暂无日志。填写设置后预览或开始同步。", "progress": "进度", "messages": "邮件", "errorCount": "错误", "elapsed": "用时", "readiness": "执行前检查", "checkSource": "来源必填项", "checkDestination": "目标必填项", "checkSafe": "建议先试运行", "checkRate": "已限速或启用友好模式", "checkPreview": "已生成命令预览", "commandPreview": "命令预览", "redacted": "密码已隐藏。", "preview": "预览", "copy": "复制", "start": "开始同步", "stop": "停止", "previewReady": "已更新命令预览。", "started": "同步已开始。", "stopped": "同步已停止。", "copied": "命令已复制。", "sampleLoaded": "已载入示例设置。", "testOk": "连接成功", "testFail": "连接失败"};
        Object.assign(i18n.ja, {statusStopping:'停止処理中', statusStopped:'停止済み', statusDrySuccess:'ドライラン完了', stopRequested:'停止を要求しました。終了を待っています。', fastModeHelp:'開始・終了時のフォルダ容量集計を省略します。', useUidHelp:'UID対応キャッシュを使って比較します。再実行時も同じキャッシュを保持してください。'});
        Object.assign(i18n.en, {statusStopping:'Stopping', statusStopped:'Stopped', statusDrySuccess:'Dry run finished', stopRequested:'Stop requested. Waiting for imapsync to exit.', fastModeHelp:'Skip folder size scans at the start and end.', useUidHelp:'Compare using a UID mapping cache. Keep the same cache when resuming.'});
        Object.assign(i18n.zh, {statusStopping:'正在停止', statusStopped:'已停止', statusDrySuccess:'试运行完成', stopRequested:'已请求停止，正在等待进程退出。', fastModeHelp:'跳过开始和结束时的文件夹容量统计。', useUidHelp:'使用 UID 映射缓存进行比较。再次运行时请保留同一缓存。'});
        Object.assign(i18n.ja, {multipleAccounts:'複数アカウント', accountUser:'ユーザー名', addRow:'行を追加', removeRow:'この行のペアを削除', rows:'行', accountSheetHelp:'左右の同じ行が転送ペアです。上から順番に実行し、失敗した行は記録して次へ進みます。停止で残りも中止します。Excelなどからユーザー名・パスワードの2列を貼り付けできます。', rowLimit:'最大1000行まで入力できます。', sheetSource:'転送元アカウント', sheetDestination:'転送先アカウント', batchResult:'アカウント結果'});
        Object.assign(i18n.en, {multipleAccounts:'Multiple accounts', accountUser:'Username', addRow:'Add row', removeRow:'Remove this account pair', rows:'rows', accountSheetHelp:'Matching rows form a source/destination pair. Pairs run in order; failures are recorded and remaining pairs continue. Stop cancels the rest. Paste two columns of usernames and passwords from a spreadsheet.', rowLimit:'Up to 1000 rows are supported.', sheetSource:'Source accounts', sheetDestination:'Destination accounts', batchResult:'Account results'});
        Object.assign(i18n.zh, {multipleAccounts:'多个账户', accountUser:'用户名', addRow:'添加行', removeRow:'删除此账户对', rows:'行', accountSheetHelp:'左右相同行为一组，按顺序迁移。失败后记录结果并继续，停止会取消剩余任务。可从电子表格粘贴用户名和密码两列。', rowLimit:'最多支持1000行。', sheetSource:'来源账户', sheetDestination:'目标账户', batchResult:'账户结果'});
        Object.assign(i18n.ja,{"provider": "接続プリセット", "customProvider": "カスタム IMAP", "authentication": "認証", "passwordAuth": "パスワード", "googleAuth": "Google OAuth", "connectGoogle": "Googleで認証", "googleConnected": "認証済み", "disconnectGoogle": "認証を解除", "oauthSettings": "Google OAuth設定", "oauthClientHelp": "Google Cloudで作成した「デスクトップアプリ」のクライアントIDを設定してください。Gmailへのアクセス権とテストユーザーの設定が必要です。", "oauthImportHelp": "Google Cloudからダウンロードしたデスクトップ用クライアントJSONを読み込みます。", "rememberGoogle": "OSの資格情報ストアに保存", "optional": "任意", "save": "保存", "cancel": "キャンセル", "oauthWaiting": "ブラウザでGoogle認証を完了してください", "oauthSession": "このセッションのみ有効です。資格情報ストアに保存できませんでした。", "oauthFailure": "Google認証に失敗しました。設定とアクセス許可を確認してください。", "oauthMismatch": "選択したGoogleアカウントが入力したユーザー名と一致しません。", "oauthReconnect": "Googleアカウントを再認証してください。", "oauthDenied": "Google認証が拒否またはキャンセルされました。", "oauthCredential": "Google認証", "oauthImport": "JSONを読み込む"});
        Object.assign(i18n.en,{"provider": "Connection preset", "customProvider": "Custom IMAP", "authentication": "Authentication", "passwordAuth": "Password", "googleAuth": "Google OAuth", "connectGoogle": "Sign in with Google", "googleConnected": "Connected", "disconnectGoogle": "Disconnect", "oauthSettings": "Google OAuth settings", "oauthClientHelp": "Enter a Desktop app client ID from Google Cloud. Configure Gmail access and OAuth test users in that project.", "oauthImportHelp": "Import the Desktop app client JSON downloaded from Google Cloud.", "rememberGoogle": "Save in the OS credential store", "optional": "Optional", "save": "Save", "cancel": "Cancel", "oauthWaiting": "Complete Google sign-in in your browser", "oauthSession": "Available for this session only. The credential store could not save this account.", "oauthFailure": "Google sign-in failed. Check client settings and permissions.", "oauthMismatch": "The selected Google account does not match the entered username.", "oauthReconnect": "Sign in to the Google account again.", "oauthDenied": "Google authorization was denied or cancelled.", "oauthCredential": "Google sign-in", "oauthImport": "Import JSON"});
        Object.assign(i18n.zh,{"provider": "连接预设", "customProvider": "自定义 IMAP", "authentication": "认证", "passwordAuth": "密码", "googleAuth": "Google OAuth", "connectGoogle": "使用 Google 登录", "googleConnected": "已认证", "disconnectGoogle": "断开认证", "oauthSettings": "Google OAuth 设置", "oauthClientHelp": "输入 Google Cloud 中桌面应用的客户端 ID，并配置 Gmail 权限及测试用户。", "oauthImportHelp": "导入从 Google Cloud 下载的桌面应用客户端 JSON。", "rememberGoogle": "保存到系统凭据存储", "optional": "可选", "save": "保存", "cancel": "取消", "oauthWaiting": "请在浏览器中完成 Google 登录", "oauthSession": "仅当前会话有效，无法保存到系统凭据存储。", "oauthFailure": "Google 认证失败，请检查客户端设置和权限。", "oauthMismatch": "选择的 Google 账户与输入的用户名不一致。", "oauthReconnect": "请重新登录 Google 账户。", "oauthDenied": "Google 授权已被拒绝或取消。", "oauthCredential": "Google 认证", "oauthImport": "导入 JSON"});
        Object.assign(i18n.ja,{recovery:'中断した転送',recoveryHelp:'未完了の転送設定を呼び戻します。パスワードを再入力し、ドライランで確認してください。復旧した組はUIDキャッシュを使わず既存メールを照合します。',recoveryUnreadable:'中断記録を読み込めません。接続設定を再入力してください。復旧用の照合は維持されます。',recoveryLoaded:'中断した転送をドライランで復元しました。認証情報を確認してください。'});
        Object.assign(i18n.en,{recovery:'Interrupted transfer',recoveryHelp:'Restore unfinished transfer settings. Re-enter passwords and verify with a dry run. Recovered pairs compare existing messages without the UID cache.',recoveryUnreadable:'The interruption record cannot be restored. Re-enter connection settings; recovery matching remains enabled.',recoveryLoaded:'Restored interrupted settings as a dry run. Check credentials before starting.'});
        Object.assign(i18n.zh,{recovery:'中断的迁移',recoveryHelp:'恢复未完成的迁移设置，重新输入密码并先试运行。恢复后的账户对不使用 UID 缓存来匹配已有邮件。',recoveryUnreadable:'无法恢复中断记录，请重新输入连接设置。仍将使用恢复匹配。',recoveryLoaded:'已将中断的设置恢复为试运行，请检查认证信息。'});
        Object.assign(i18n.ja,{pairBusy:'同じメールボックスの転送プロセスがまだ動作しています。終了してから再実行してください。',processCheck:'前回の転送プロセスを確認できませんでした。再実行を保留しました。'});
        Object.assign(i18n.en,{pairBusy:'A transfer for this mailbox pair is still running. Wait for it to exit before retrying.',processCheck:'Could not verify the previous transfer process. Restart was blocked.'});
        Object.assign(i18n.zh,{pairBusy:'此邮箱对的迁移进程仍在运行，请等待结束后再重试。',processCheck:'无法确认上次的迁移进程，已阻止重新启动。'});
        Object.assign(i18n.ja,{resumeBatch:'複数転送を再開',resumeBatchHelp:'完了した行を保持し、中断・未実行の行だけを再開します。設定変更は×で再開モードを終了してから行ってください。',resumeExit:'再開モードを終了',batchRecoveryChanged:'復旧記録と設定が一致しません。記録を再度呼び戻すか、再開モードを終了してください。',batchRecoveryInvalid:'復旧記録を使用できません。完了済み、または記録が変更されています。'});
        Object.assign(i18n.en,{resumeBatch:'Resume account queue',resumeBatchHelp:'Keep completed rows and resume only interrupted/pending rows. Exit resume mode with × before changing settings.',resumeExit:'Exit resume mode',batchRecoveryChanged:'Settings no longer match the saved queue. Restore again or exit resume mode.',batchRecoveryInvalid:'The saved queue is unavailable, already complete or has changed.'});
        Object.assign(i18n.zh,{resumeBatch:'恢复账户队列',resumeBatchHelp:'保留已完成的行，仅恢复中断及未执行的行。更改设置前请用 × 退出恢复模式。',resumeExit:'退出恢复模式',batchRecoveryChanged:'设置与保存的队列不一致，请重新恢复或退出恢复模式。',batchRecoveryInvalid:'队列记录不可用、已完成或已更改。'});
        Object.assign(i18n.ja,{"resultsTitle": "転送結果", "resultsHelp": "行ごとの結果を確認し、失敗行だけを再試行するかCSV・JSONで保存します。資格情報は含めません。", "resultsRun": "実行履歴", "resultsSelectFailed": "失敗行を選択", "resultsRetry": "選択行を再試行", "resultsRetryHelp": "選択した失敗・停止行を別キューとして準備します。元の結果は保持し、ドライランとパスワード再入力から始めます。", "resultsEmpty": "転送結果はまだありません。", "resultsRow": "行", "resultsState": "結果", "resultsExit": "終了コード", "resultsSaved": "転送結果を保存しました。", "resultsPrepared": "選択した行をドライランで準備しました。認証情報を確認してください。", "resultsSelectionError": "失敗または停止した行を選択してください。", "resultsUnavailable": "転送結果を読み込めません。", "resultsExportError": "結果を保存できませんでした。", "resultsPending": "未実行", "resultsRunning": "実行中", "resultsSuccess": "成功", "resultsFailed": "失敗", "resultsStopped": "停止", "resultsClose": "閉じる"});
        Object.assign(i18n.en,{"resultsTitle": "Migration results", "resultsHelp": "Inspect row results, retry selected failed rows, or save CSV/JSON. Credentials are excluded.", "resultsRun": "Run history", "resultsSelectFailed": "Select failed rows", "resultsRetry": "Retry selected", "resultsRetryHelp": "Prepare failed/stopped rows as a separate queue. Keep the original results, re-enter credentials and start with a dry run.", "resultsEmpty": "No migration results yet.", "resultsRow": "Row", "resultsState": "Result", "resultsExit": "Exit code", "resultsSaved": "Migration results saved.", "resultsPrepared": "Selected rows prepared as a dry run. Check credentials before starting.", "resultsSelectionError": "Select failed or stopped rows.", "resultsUnavailable": "Could not read migration results.", "resultsExportError": "Could not save migration results.", "resultsPending": "Pending", "resultsRunning": "Running", "resultsSuccess": "Success", "resultsFailed": "Failed", "resultsStopped": "Stopped", "resultsClose": "Close"});
        Object.assign(i18n.zh,{"resultsTitle": "迁移结果", "resultsHelp": "查看各行结果、重试选中的失败行，或保存 CSV/JSON。不包含认证信息。", "resultsRun": "执行历史", "resultsSelectFailed": "选择失败行", "resultsRetry": "重试所选行", "resultsRetryHelp": "将失败或停止的行准备为独立队列。保留原结果，重新输入认证信息并先试运行。", "resultsEmpty": "暂无迁移结果。", "resultsRow": "行", "resultsState": "结果", "resultsExit": "退出码", "resultsSaved": "已保存迁移结果。", "resultsPrepared": "所选行已准备为试运行，请检查认证信息。", "resultsSelectionError": "请选择失败或停止的行。", "resultsUnavailable": "无法读取迁移结果。", "resultsExportError": "无法保存迁移结果。", "resultsPending": "未执行", "resultsRunning": "执行中", "resultsSuccess": "成功", "resultsFailed": "失败", "resultsStopped": "停止", "resultsClose": "关闭"});
        Object.assign(i18n.ja,{"historySearch": "履歴を検索", "historySearchHelp": "アカウント・サーバー・実行ID・日時を空白区切りで検索します。条件は履歴単位で適用し、選んだ履歴の全行を表示・出力します。", "historyAll": "すべての結果", "historySuccess": "すべて成功", "historyFailed": "失敗・停止あり", "historyUnfinished": "未完了", "historyRunning": "実行中", "historyAllModes": "すべての実行", "historySync": "本転送", "historyDry": "ドライラン", "historyNoMatches": "条件に一致する履歴はありません。", "historyPrevious": "前のページ", "historyNext": "次のページ", "historyRefresh": "履歴を更新", "historyDelete": "この履歴を削除", "historyDeleteHelp": "終了した実行履歴を削除できます。実行中と未完了キューは保護され、メールボックスの復旧情報は保持されます。", "historyDeleteQuestion": "この履歴を削除しますか？", "historyDeleteConfirm": "削除", "historyCancel": "キャンセル", "historyDeleted": "履歴を削除しました。", "historyProtected": "未完了または使用中の履歴は削除できません。", "historyDeleteFailed": "履歴を削除できませんでした。更新して再確認してください。", "historyFilterError": "検索条件を確認してください。"});
        Object.assign(i18n.en,{"historySearch": "Search history", "historySearchHelp": "Search accounts, servers, run IDs and timestamps with space-separated terms. Filters apply to runs; all rows of the selected run remain visible and exportable.", "historyAll": "All results", "historySuccess": "All successful", "historyFailed": "Failed or stopped", "historyUnfinished": "Unfinished", "historyRunning": "Running", "historyAllModes": "All runs", "historySync": "Migration", "historyDry": "Dry run", "historyNoMatches": "No history matches these filters.", "historyPrevious": "Previous page", "historyNext": "Next page", "historyRefresh": "Refresh history", "historyDelete": "Delete this history", "historyDeleteHelp": "Delete finished attempt history. Running attempts and unfinished queues are protected; mailbox recovery information is retained.", "historyDeleteQuestion": "Delete this run history?", "historyDeleteConfirm": "Delete", "historyCancel": "Cancel", "historyDeleted": "History deleted.", "historyProtected": "Unfinished or in-use history cannot be deleted.", "historyDeleteFailed": "Could not delete history. Refresh and check again.", "historyFilterError": "Check the search filters."});
        Object.assign(i18n.zh,{"historySearch": "搜索历史", "historySearchHelp": "用空格分隔的词搜索账户、服务器、执行 ID 和时间。条件针对整个执行记录，选中记录的所有行仍可查看和导出。", "historyAll": "所有结果", "historySuccess": "全部成功", "historyFailed": "包含失败或停止", "historyUnfinished": "未完成", "historyRunning": "执行中", "historyAllModes": "所有执行", "historySync": "正式迁移", "historyDry": "试运行", "historyNoMatches": "没有符合条件的历史记录。", "historyPrevious": "上一页", "historyNext": "下一页", "historyRefresh": "刷新历史", "historyDelete": "删除此历史记录", "historyDeleteHelp": "可以删除已结束的执行历史。执行中的记录和未完成队列受保护，并保留邮箱恢复信息。", "historyDeleteQuestion": "删除此执行历史？", "historyDeleteConfirm": "删除", "historyCancel": "取消", "historyDeleted": "已删除历史记录。", "historyProtected": "无法删除未完成或正在使用的历史记录。", "historyDeleteFailed": "无法删除历史，请刷新后重试。", "historyFilterError": "请检查搜索条件。"});
        Object.assign(i18n.ja,{historyStatusLabel:'結果で絞り込み',historyModeLabel:'実行方法で絞り込み'});
        Object.assign(i18n.en,{historyStatusLabel:'Filter by result',historyModeLabel:'Filter by run mode'});
        Object.assign(i18n.zh,{historyStatusLabel:'按结果筛选',historyModeLabel:'按执行方式筛选'});
        Object.assign(i18n.ja,{resultsSingle:'単一アカウント',resultsMultiple:'複数アカウント'});
        Object.assign(i18n.en,{resultsSingle:'Single account',resultsMultiple:'Multiple accounts'});
        Object.assign(i18n.zh,{resultsSingle:'单个账户',resultsMultiple:'多个账户'});
        const fields = ['provider1','auth1','provider2','auth2','host1','port1','security1','user1','pass1','host2','port2','security2','user2','pass2','dryRun','syncDeletes','subscribeFolders','automapFolders','useUid','fastMode','providerSafeMode','maxMessagesPerSecond','maxSleepSeconds','maxBytesAfterMb','errorsMax','timeoutSeconds','stopAfterMb','maxAgeDays','minAgeDays','bandwidthKb','excludePattern','advancedArgs'];
        Object.assign(i18n.ja,{"historyRetention": "保存期間", "historyForever": "無期限", "history30": "30日", "history90": "90日", "history180": "180日", "history365": "1年", "historyRetentionHelp": "完了からの保存期間。起動時と1時間ごとに期限切れの保存済み履歴を削除します。実行中・未完了キュー・メールボックス復旧情報は保護します。古い履歴はファイル更新日時を基準にします。", "historyPolicySave": "保存期間を適用", "historyBulk": "完了履歴を一括削除", "historyBulkHelp": "検索条件やページに関係なく、完了履歴を対象にします。セッション内の複数アカウントのドライランも含みます。失敗・停止した単一転送も含みます。実行中と未完了キュー、復旧情報は保持します。", "historyBulkQuestion": "完了履歴 {count} 件を削除しますか？検索条件に関係なく、失敗履歴も含みます。", "historyPolicyQuestion": "保存期間を「{period}」に変更しますか？期限切れの完了履歴 {count} 件を削除し、以後も自動整理します。", "historyForeverQuestion": "保存期間を無期限に変更しますか？履歴は自動削除されません。", "historyMaintenanceDone": "削除 {deleted} 件・保護／削除済み {skipped} 件・失敗 {failed} 件", "historyPolicyError": "履歴設定または整理を完了できませんでした。更新して再確認してください。", "historyNoCompleted": "削除できる保存済み完了履歴はありません。", "historyPolicyApply": "適用"});
        Object.assign(i18n.en,{"historyRetention": "Keep history", "historyForever": "Forever", "history30": "30 days", "history90": "90 days", "history180": "180 days", "history365": "1 year", "historyRetentionHelp": "Age is measured from completion. Expired saved reports are removed at startup and hourly. Running attempts, unfinished queues and mailbox recovery information are protected. Legacy reports use file modification time.", "historyPolicySave": "Apply retention period", "historyBulk": "Clear completed history", "historyBulkHelp": "Includes all completed runs across pages and search filters, including the current session batch dry run and failed or stopped single-account attempts. Running attempts, unfinished queues and recovery information are retained.", "historyBulkQuestion": "Delete {count} completed runs? This includes failed runs across all pages and filters.", "historyPolicyQuestion": "Keep history for {period}? Delete {count} expired completed runs and automatically clean up future expired reports.", "historyForeverQuestion": "Keep history forever? Reports will not be deleted automatically.", "historyMaintenanceDone": "Deleted {deleted} · Protected / already removed {skipped} · Failed {failed}", "historyPolicyError": "Could not complete history settings or cleanup. Refresh and check again.", "historyNoCompleted": "No saved completed history can be deleted.", "historyPolicyApply": "Apply"});
        Object.assign(i18n.zh,{"historyRetention": "保留期限", "historyForever": "永久", "history30": "30天", "history90": "90天", "history180": "180天", "history365": "1年", "historyRetentionHelp": "从执行结束时计算。启动时及每小时删除过期的已保存历史。执行中的记录、未完成队列和邮箱恢复信息受保护。旧记录使用文件修改时间。", "historyPolicySave": "应用保留期限", "historyBulk": "批量删除已完成历史", "historyBulkHelp": "针对所有页面和搜索条件之外的完成记录，包含当前会话的多账户试运行、失败记录及已停止的单账户记录。保留执行中的记录、未完成队列和恢复信息。", "historyBulkQuestion": "删除 {count} 条完成记录？包含所有页面和筛选条件之外的失败记录。", "historyPolicyQuestion": "将保留期限设为{period}？删除 {count} 条过期完成记录，并自动清理今后过期的记录。", "historyForeverQuestion": "永久保留历史？不会自动删除记录。", "historyMaintenanceDone": "已删除 {deleted} · 受保护／已移除 {skipped} · 失败 {failed}", "historyPolicyError": "无法完成历史设置或清理。请刷新后重试。", "historyNoCompleted": "没有可删除的已保存完成记录。", "historyPolicyApply": "应用"});
        Object.assign(i18n.ja,{"historyExportCount": "条件一致：{count}件を一括出力", "historyExportHelp": "現在の検索・結果・実行種別に一致する全ページの履歴を保存します。履歴ごとの全アカウント行を含み、認証情報は含めません。保存ダイアログを開いた時点の結果です。", "historyExportEmpty": "条件に一致する履歴はありません。更新して再確認してください。", "historyExportSaved": "{count}件の履歴を書き出しました。"});
        Object.assign(i18n.en,{"historyExportCount": "Export all {count} matching runs", "historyExportHelp": "Save matching runs across all pages using the current search, result and run-mode filters. Includes every account row in each matching run, without credentials. Results are captured before the save dialog opens.", "historyExportEmpty": "No history matches these filters. Refresh and check again.", "historyExportSaved": "Exported {count} runs."});
        Object.assign(i18n.zh,{"historyExportCount": "批量导出符合条件的 {count} 条记录", "historyExportHelp": "按当前搜索、结果及执行类型导出所有页面中符合条件的记录。包含每条记录的所有账户行，不含认证信息。结果在打开保存对话框前获取。", "historyExportEmpty": "没有符合条件的记录，请刷新后重试。", "historyExportSaved": "已导出 {count} 条记录。"});
        Object.assign(i18n.ja,{bundledEngineMissing:'同梱の転送エンジンが見つかりません。アプリを再インストールしてください。'});
        Object.assign(i18n.en,{bundledEngineMissing:'The bundled migration engine is missing. Reinstall the application.'});
        Object.assign(i18n.zh,{bundledEngineMissing:'找不到内置迁移引擎，请重新安装应用。'});
        Object.assign(i18n.ja,{"presetCustom": "接続先・ポート・暗号化を手動で設定します。", "presetIcloud": "iCloud IMAP：993・SSL/TLS。Appleのアプリ用パスワードを使用してください。", "presetExchange": "Microsoft 365 / Exchange Online用。社内ExchangeはカスタムIMAPで指定してください。Microsoft OAuthは未対応のため、OAuth必須のアカウントでは現在転送できません。", "presetGoogle": "Google IMAP：993・SSL/TLS。Google OAuthを選択します。アプリ用パスワードを使う場合は認証方式を変更できます。", "presetYahoo": "海外版Yahoo Mail用。アプリ用パスワードが必要です。日本のアカウントはYahoo! JAPANを選択してください。", "presetYahooJapan": "Yahoo! JAPAN用。Yahoo側でIMAP／外部メールソフトからのアクセスを有効にしてください。海外版Yahoo Mailとは接続先が異なります。", "presetAol": "AOL IMAP：993・SSL/TLS。アプリ用パスワードを使用してください。", "presetOutlook": "Outlook.com / Hotmail / Live用。Microsoft OAuthは未対応のため、OAuth必須のアカウントでは現在転送できません。"});
        Object.assign(i18n.en,{"presetCustom": "Enter the IMAP host, port and encryption manually.", "presetIcloud": "iCloud IMAP: 993, SSL/TLS. Use an Apple app-specific password.", "presetExchange": "For Microsoft 365 / Exchange Online. Use Custom IMAP for an on-premises Exchange server. Microsoft OAuth is not implemented, so accounts requiring OAuth cannot currently migrate.", "presetGoogle": "Google IMAP: 993, SSL/TLS. Selects Google OAuth; switch authentication if using an app password.", "presetYahoo": "For international Yahoo Mail. Use an app password. Select Yahoo! JAPAN for Japanese accounts.", "presetYahooJapan": "For Yahoo! JAPAN. Enable IMAP/external mail client access in Yahoo settings. Its endpoint differs from international Yahoo Mail.", "presetAol": "AOL IMAP: 993, SSL/TLS. Use an app password.", "presetOutlook": "For Outlook.com / Hotmail / Live. Microsoft OAuth is not implemented, so accounts requiring OAuth cannot currently migrate."});
        Object.assign(i18n.zh,{"presetCustom": "手动输入 IMAP 主机、端口和加密方式。", "presetIcloud": "iCloud IMAP：993、SSL/TLS。请使用 Apple 应用专用密码。", "presetExchange": "用于 Microsoft 365 / Exchange Online。本地 Exchange 服务器请使用自定义 IMAP。尚未实现 Microsoft OAuth，目前无法迁移要求 OAuth 的账户。", "presetGoogle": "Google IMAP：993、SSL/TLS。自动选择 Google OAuth；使用应用专用密码时可切换认证方式。", "presetYahoo": "用于国际版 Yahoo Mail，请使用应用专用密码。日本账户请选择 Yahoo! JAPAN。", "presetYahooJapan": "用于 Yahoo! JAPAN。请在 Yahoo 设置中启用 IMAP／外部邮件客户端访问。服务器与国际版 Yahoo Mail 不同。", "presetAol": "AOL IMAP：993、SSL/TLS。请使用应用专用密码。", "presetOutlook": "用于 Outlook.com / Hotmail / Live。尚未实现 Microsoft OAuth，目前无法迁移要求 OAuth 的账户。"});
        Object.assign(i18n.ja,{"microsoftAuth": "Microsoft OAuth", "connectMicrosoft": "Microsoftで認証", "microsoftCredential": "Microsoft認証", "microsoftWaiting": "ブラウザーでMicrosoft認証を完了してください", "microsoftFailure": "Microsoft認証に失敗しました。アプリ登録・同意・IMAPの設定を確認してください。", "microsoftMismatch": "選択したMicrosoftアカウントが入力したユーザー名と一致しません。", "microsoftReconnect": "Microsoftアカウントを再認証してください。", "microsoftDenied": "Microsoft認証が拒否またはキャンセルされました。", "oauthSettings": "OAuth設定", "oauthProvider": "認証サービス", "microsoftClientHelp": "Microsoft Entraでアプリを登録し、組織と個人のアカウントを許可します。モバイル／デスクトップのリダイレクトURIを http://localhost/oauth/callback に設定し、Exchange Onlineの委任アクセス許可 IMAP.AccessAsUser.All を追加してください。クライアントシークレットは不要です。", "presetExchange": "Microsoft 365 / Exchange Online用。Microsoft OAuthを選択します。管理者によるIMAPの有効化・アプリへの同意が必要な場合があります。社内ExchangeはカスタムIMAPで指定してください。", "presetOutlook": "Outlook.com / Hotmail / Live用。Microsoft OAuthを選択します。Outlook側のIMAPアクセス設定も確認してください。"});
        Object.assign(i18n.en,{"microsoftAuth": "Microsoft OAuth", "connectMicrosoft": "Sign in with Microsoft", "microsoftCredential": "Microsoft sign-in", "microsoftWaiting": "Complete Microsoft sign-in in your browser", "microsoftFailure": "Microsoft sign-in failed. Check app registration, consent and IMAP settings.", "microsoftMismatch": "The selected Microsoft account does not match the entered username.", "microsoftReconnect": "Sign in to the Microsoft account again.", "microsoftDenied": "Microsoft authorization was denied or cancelled.", "oauthSettings": "OAuth settings", "oauthProvider": "Provider", "microsoftClientHelp": "Register an Entra application supporting work/school and personal Microsoft accounts. Add the Mobile/Desktop redirect URI http://localhost/oauth/callback and Exchange Online delegated permission IMAP.AccessAsUser.All. No client secret is required.", "presetExchange": "For Microsoft 365 / Exchange Online. Selects Microsoft OAuth. Administrators may need to enable IMAP and consent to the app. Use Custom IMAP for on-premises Exchange.", "presetOutlook": "For Outlook.com / Hotmail / Live. Selects Microsoft OAuth. Also check IMAP access in Outlook settings."});
        Object.assign(i18n.zh,{"microsoftAuth": "Microsoft OAuth", "connectMicrosoft": "使用 Microsoft 登录", "microsoftCredential": "Microsoft 认证", "microsoftWaiting": "请在浏览器中完成 Microsoft 登录", "microsoftFailure": "Microsoft 认证失败，请检查应用注册、同意及 IMAP 设置。", "microsoftMismatch": "选择的 Microsoft 账户与输入的用户名不一致。", "microsoftReconnect": "请重新认证 Microsoft 账户。", "microsoftDenied": "Microsoft 认证被拒绝或已取消。", "oauthSettings": "OAuth 设置", "oauthProvider": "认证服务", "microsoftClientHelp": "在 Entra 注册支持工作／学校及个人 Microsoft 账户的应用。添加移动／桌面重定向 URI http://localhost/oauth/callback 和 Exchange Online 委托权限 IMAP.AccessAsUser.All。不需要客户端密钥。", "presetExchange": "用于 Microsoft 365 / Exchange Online，自动选择 Microsoft OAuth。可能需要管理员启用 IMAP 并同意应用。本地 Exchange 请使用自定义 IMAP。", "presetOutlook": "用于 Outlook.com / Hotmail / Live，自动选择 Microsoft OAuth。请同时检查 Outlook 的 IMAP 访问设置。"});
        const state = {
            lang: localStorage.getItem('imapsync-lang') || 'ja',
            resumeBatch: null, resumeResults: [], resultReports: [], resultsBusy:false, historyLoading:false, historyOffset:0, historyLimit:20, historyTotal:0, historyRequest:0, historyDeleteId:null, historyPolicyDays:null, historyMaintenance:null,
            oauth: {accounts:[]}, microsoftOauth:{accounts:[]}, oauthSettingsProvider:'google', oauthSettingsRequest:0, oauthBusy: null, customEndpoints: {}, appliedPresets: {},
            logs: [],
            filter: 'all',
            autoscroll: true,
            previewReady: false,
            startedAt: null,
            running: false,
            submitting: false,
            stopPending: false,
            multipleAccounts: false,
            accounts1: Array.from({length:3}, () => ({user:'', password:''})),
            accounts2: Array.from({length:3}, () => ({user:'', password:''}))
        };

        const $ = selector => document.querySelector(selector);
        const t = key => i18n[state.lang][key] || key;

        for(const side of [1,2]) {
            const select=$('#provider'+side);
            select.querySelectorAll('option:not([value="custom"])').forEach(option=>option.remove());
            for(const [id,preset] of Object.entries(PRESETS))select.add(new Option(preset.label,id));
        }
        function renderPresetInfo() {
            for(const side of [1,2]) {
                const info=$('#preset-info-'+side),preset=PRESETS[$('#provider'+side).value];
                info.title=t(preset?.help||'presetCustom');info.setAttribute('aria-label',info.title);
            }
        }
        function iconCheck(ok) {
            return ok ? '✓' : '•';
        }

        function applyTranslations() {
            document.documentElement.lang = state.lang;
            document.querySelectorAll('[data-i18n]').forEach(el => {
                el.textContent = t(el.dataset.i18n);
            });
            $('#lang-btn').textContent = ({ja:'English', en:'简体中文', zh:'日本語'})[state.lang];
            $('#autoscroll-btn').textContent = state.autoscroll ? t('autoscroll') : t('autoscrollOff');
            document.querySelectorAll('[data-help]').forEach(el=> {el.title=t(el.dataset.help); el.setAttribute('aria-label',el.title);});
            document.querySelectorAll('[data-account-side]').forEach(input => input.setAttribute('aria-label', `${t(input.dataset.accountField === 'user' ? 'accountUser' : 'password')} ${Number(input.dataset.accountRow) + 1}`));
            document.querySelectorAll('.sheet-row-remove').forEach(button => { button.title=t('removeRow'); button.setAttribute('aria-label', `${t('removeRow')} ${Number(button.dataset.removeRow)+1}`); });
            for (const side of [1,2]) $(`#account-sheet-${side} table`).setAttribute('aria-label', t(side===1?'sheetSource':'sheetDestination'));
            renderPresetInfo();
            captureAccounts(); renderAccounts();
            $('#history-search').placeholder=t('historySearch');
            if($('#results-dialog').open) renderResultRows();
            updateAccountCount();
            updateStatusUi();
        }

        function sheetAccounts(side) {
            return [...document.querySelectorAll(`#accounts-${side} tr`)].map(row => ({
                user:row.querySelector('[data-account-field="user"]').value.trim(),
                password:row.querySelector('[data-account-field="password"]')?.value || ''
            }));
        }

        function captureAccounts() {
            for (const side of [1,2]) if ($(`#accounts-${side}`).children.length) state[`accounts${side}`] = sheetAccounts(side);
        }

        function updateAccountCount() {
            $('#account-row-count').textContent = `${state.accounts1.length} ${t('rows')}`;
        }

        function renderAccounts() {
            for (const side of [1,2]) {
                const tbody = $(`#accounts-${side}`);
                tbody.innerHTML = state[`accounts${side}`].map((account,index) => `<tr data-row="${index+1}"><th scope="row" class="sheet-index"><button class="sheet-row-remove" type="button" data-remove-row="${index}" title="${escapeHtml(t('removeRow'))}" aria-label="${escapeHtml(t('removeRow'))} ${index+1}">${index+1}</button></th>${['user','password'].map(field => field==='password'&&usesOAuth(side)?`<td class="oauth-cell">${oauthControls(side,index,account.user)}</td>`: `<td><input type="${field==='password'?'password':'text'}" autocomplete="off" spellcheck="false" data-account-side="${side}" data-account-row="${index}" data-account-field="${field}" aria-label="${escapeHtml(t(field==='user'?'accountUser':'password'))} ${index+1}" value="${escapeHtml(account[field] || '')}"></td>`).join('')}</tr>`).join('');
            }
            renderAuthUi();
            updateAccountCount();
            updateStatusUi();
        }

        function setMultipleAccounts(enabled) {
            captureAccounts();
            if (enabled && state.accounts1.every(a=>!a.user&&!a.password) && state.accounts2.every(a=>!a.user&&!a.password)) {
                for (const side of [1,2]) state[`accounts${side}`][0] = {user:$(`#user${side}`).value,password:$(`#pass${side}`).value};
            }
            state.multipleAccounts = enabled;
            $('.connections-panel').classList.toggle('multiple-accounts', enabled);
            $('#multiple-accounts-btn').setAttribute('aria-pressed', String(enabled));
            for (const side of [1,2]) $(`#account-sheet-${side}`).hidden = !enabled;
            $('#account-sheet-tools').hidden = !enabled;
            renderAccounts();
        }

        function accountDraftChanged() {
            captureAccounts();
            state.previewReady=false;
            saveDraft();
            updateReadiness();
        }

        $('#multiple-accounts-btn').addEventListener('click', () => {
            if (state.running || state.submitting || state.oauthBusy) return;
            setMultipleAccounts(!state.multipleAccounts);
            accountDraftChanged();
        });
        $('#add-account-row').addEventListener('click', () => {
            if (state.running || state.submitting || state.oauthBusy) return;
            captureAccounts();
            if (state.accounts1.length >= 1000) return showToast(t('rowLimit'));
            for (const side of [1,2]) state[`accounts${side}`].push({user:'', password:''});
            renderAccounts(); accountDraftChanged();
        });
        for (const side of [1,2]) {
            const sheet = $(`#account-sheet-${side}`);
            sheet.querySelector('.sheet-scroll').addEventListener('scroll', event => {
                const other=$(`#account-sheet-${side===1?2:1} .sheet-scroll`);
                if(other.scrollTop!==event.target.scrollTop) other.scrollTop=event.target.scrollTop;
            });
            sheet.addEventListener('input', event=>{accountDraftChanged();if(usesOAuth(side)){const input=event.target.closest('[data-account-field="user"]');if(input)input.closest('tr').querySelector('.oauth-cell').innerHTML=oauthControls(side,Number(input.dataset.accountRow),input.value);}});
            sheet.addEventListener('click', event => {
                const button = event.target.closest('[data-remove-row]');
                if (!button || state.running || state.submitting || state.oauthBusy) return;
                captureAccounts();
                for (const side of [1,2]) {
                    state[`accounts${side}`].splice(Number(button.dataset.removeRow),1);
                    if (!state[`accounts${side}`].length) state[`accounts${side}`].push({user:'',password:''});
                }
                renderAccounts(); accountDraftChanged();
            });
            sheet.addEventListener('paste', event => {
                const input = event.target.closest('[data-account-field]');
                if (!input || state.running || state.submitting || state.oauthBusy) return;
                const text = event.clipboardData.getData('text/plain');
                if (!/[\t\n\r]/.test(text)) return;
                event.preventDefault();
                const rows=text.replace(/\r\n?/g,'\n').replace(/\n$/,'').split('\n').map(line=>line.split('\t'));
                const start=Number(input.dataset.accountRow);
                if (start+rows.length>1000) return showToast(t('rowLimit'));
                captureAccounts();
                while (state.accounts1.length<start+rows.length) for (const side of [1,2]) state[`accounts${side}`].push({user:'',password:''});
                rows.forEach((values,index)=> {
                    const account=state[`accounts${side}`][start+index];
                    if (input.dataset.accountField==='user') {account.user=values[0].trim();if(values.length>1) account.password=values[1];}
                    else account.password=values[0];
                });
                renderAccounts(); accountDraftChanged();
                $(`#accounts-${side} [data-account-row="${start}"][data-account-field="${input.dataset.accountField}"]`).focus();
            });
        }

        function oauthProvider(side) {const auth=$('#auth'+side).value;return ['google','microsoft'].includes(auth)?auth:null;}
        function usesOAuth(side) {return Boolean(oauthProvider(side));}
        function oauthView(provider) {return provider==='microsoft'?state.microsoftOauth:state.oauth;}
        function oauthReady(side,email) {return (oauthView(oauthProvider(side)).accounts||[]).some(a=>a.email===email.trim().toLowerCase()&&a.connected);}
        function oauthControls(side,row,email) {
            const provider=oauthProvider(side)||'google', ready=oauthReady(side,email), account=(oauthView(provider).accounts||[]).find(a=>a.email===email.trim().toLowerCase());
            return `<div class="oauth-account-actions"><button type="button" class="tiny-button google-connect ${ready?'connected':''}" data-oauth-provider="${provider}" data-google-connect="${side}" data-google-row="${row}" title="${escapeHtml(ready&&!account.persistent?t('oauthSession'):t(provider==='microsoft'?'connectMicrosoft':'connectGoogle'))}">${ready?'✓':provider==='microsoft'?'M':'G'} ${escapeHtml(t(ready?'googleConnected':provider==='microsoft'?'connectMicrosoft':'connectGoogle'))}</button>${ready?`<button type="button" class="tiny-button" data-oauth-provider="${provider}" data-google-disconnect="${escapeHtml(email)}" title="${escapeHtml(t('disconnectGoogle'))}" aria-label="${escapeHtml(t('disconnectGoogle'))}">×</button>`:''}</div>`;
        }
        function renderAuthUi() {
            for(const side of [1,2]) {
                const google=usesOAuth(side);
                $(`#password-field-${side}`).hidden=google;
                $(`#oauth-single-${side}`).hidden=!google;
                $(`#oauth-single-${side}`).innerHTML=oauthControls(side,-1,$(`#user${side}`).value);
                const header=$(`#account-sheet-${side} [data-i18n="password"],#account-sheet-${side} [data-i18n="oauthCredential"],#account-sheet-${side} [data-i18n="microsoftCredential"]`);
                if(header) {header.dataset.i18n=google?(oauthProvider(side)==='microsoft'?'microsoftCredential':'oauthCredential'):'password';header.textContent=t(header.dataset.i18n);}
            }
        }
        function changePreset(side) {
            captureAccounts();
            const ids=['host','port','security'],provider=$('#provider'+side).value,preset=PRESETS[provider];
            if((state.appliedPresets[side]||'custom')==='custom')state.customEndpoints[side]=ids.map(id=>$('#'+id+side).value);
            if(preset) {
                ids.forEach(id=>$('#'+id+side).value=preset[id]);
                $('#auth'+side).value=provider==='gmail'?'google':['exchange','outlook'].includes(provider)?'microsoft':'password';
            } else {
                (state.customEndpoints[side]||['','993','ssl']).forEach((value,i)=>$('#'+ids[i]+side).value=value);
                $('#auth'+side).value='password';
            }
            state.appliedPresets[side]=provider;
            renderPresetInfo();renderAccounts();
        }
        function oauthError(error,provider='google') {
            const value=String(error),ms=provider==='microsoft';
            const key=value.includes('IDENTITY_MISMATCH')?(ms?'microsoftMismatch':'oauthMismatch'):value.includes('RECONNECT')?(ms?'microsoftReconnect':'oauthReconnect'):value.includes('DENIED')||value.includes('CANCELLED')?(ms?'microsoftDenied':'oauthDenied'):(ms?'microsoftFailure':'oauthFailure');
            return `${t(key)} (${value.match(/(?:GMAIL|MICROSOFT)_OAUTH_[A-Z_]+/)?.[0]||'OAuth'})`;
        }
        async function refreshOAuth(provider='google') {
            const view=await api('/api/oauth/status',{body:JSON.stringify({provider})});
            if(provider==='microsoft')state.microsoftOauth=view||{accounts:[]};else state.oauth=view||{accounts:[]};
            return oauthView(provider);
        }
        async function openOAuthSettings(provider='google') {
            const request=++state.oauthSettingsRequest;
            try {await refreshOAuth(provider);}catch(error){showToast(oauthError(error,provider));}
            if(request!==state.oauthSettingsRequest)return;
            state.oauthSettingsProvider=provider;
            const view=oauthView(provider),ms=provider==='microsoft';
            $('#oauth-settings-provider').value=provider;
            $('#google-client-id').value=view.clientId||'';
            $('#google-client-id').placeholder=ms?'00000000-0000-0000-0000-000000000000':'…apps.googleusercontent.com';
            const help=$('#google-oauth-dialog [data-help="oauthClientHelp"],#google-oauth-dialog [data-help="microsoftClientHelp"]');
            help.dataset.help=ms?'microsoftClientHelp':'oauthClientHelp';help.title=t(help.dataset.help);help.setAttribute('aria-label',help.title);
            $('#google-client-secret').value='';$('#google-client-json').value='';$('#oauth-secret-field').hidden=ms;
            $('#google-oauth-dialog .oauth-import').hidden=ms;
            $('#google-remember').checked=view.remember!==false;
            $('#oauth-settings-error').textContent='';
            if(!$('#google-oauth-dialog').open)$('#google-oauth-dialog').showModal();
        }
        $('#google-settings-btn').addEventListener('click',()=>openOAuthSettings(oauthProvider(1)||oauthProvider(2)||'google'));
        $('#oauth-settings-provider').addEventListener('change',event=>openOAuthSettings(event.target.value));
        $('#oauth-settings-close').addEventListener('click',()=>{$('#google-client-secret').value='';$('#google-oauth-dialog').close();});
        $('#google-oauth-dialog').addEventListener('close',()=>{++state.oauthSettingsRequest;$('#google-client-secret').value='';$('#google-client-json').value='';});
        $('#google-client-json').addEventListener('change',async event=>{
            try {const file=event.target.files[0];if(!file||file.size>65536) throw Error('GMAIL_OAUTH_CLIENT');const data=JSON.parse(await file.text()).installed;if(!data?.client_id) throw Error('GMAIL_OAUTH_CLIENT');$('#google-client-id').value=data.client_id;$('#google-client-secret').value=data.client_secret||'';}
            catch(error){$('#oauth-settings-error').textContent=oauthError(error,state.oauthSettingsProvider);}
        });
        $('#google-oauth-form').addEventListener('submit',async event=>{
            event.preventDefault();
            const button=event.submitter;button.disabled=true;$('#oauth-settings-provider').disabled=true;$('#oauth-settings-close').disabled=true;
            try {const provider=state.oauthSettingsProvider,view=await api('/api/oauth/configure',{body:JSON.stringify({provider,clientId:$('#google-client-id').value.trim(),...(provider==='google'?{clientSecret:$('#google-client-secret').value}:{}),remember:$('#google-remember').checked})});if(provider==='microsoft')state.microsoftOauth=view;else state.oauth=view;$('#google-oauth-dialog').close();captureAccounts();renderAccounts();updateReadiness();if(view.storageAvailable===false)showToast(t('oauthSession'));}
            catch(error){$('#oauth-settings-error').textContent=oauthError(error,state.oauthSettingsProvider);}finally{button.disabled=false;$('#oauth-settings-provider').disabled=false;$('#oauth-settings-close').disabled=false;}
        });
        $('#oauth-cancel-btn').addEventListener('click',()=>api('/api/oauth/cancel',{body:JSON.stringify({provider:state.oauthBusy?.provider||'google'})}).catch(error=>showToast(oauthError(error,state.oauthBusy?.provider))));
        document.addEventListener('click',async event=>{
            const disconnect=event.target.closest('[data-google-disconnect]');
            if(disconnect&&!state.running&&!state.submitting&&!state.oauthBusy) {
                try {const provider=disconnect.dataset.oauthProvider,view=await api('/api/oauth/disconnect',{body:JSON.stringify({provider,email:disconnect.dataset.googleDisconnect})});if(provider==='microsoft')state.microsoftOauth=view;else state.oauth=view;captureAccounts();renderAccounts();updateReadiness();}catch(error){showToast(oauthError(error,disconnect.dataset.oauthProvider));}return;
            }
            const button=event.target.closest('[data-google-connect]');
            if(!button||state.running||state.submitting||state.oauthBusy) return;
            const side=Number(button.dataset.googleConnect),row=Number(button.dataset.googleRow),provider=button.dataset.oauthProvider;
            try {
                const initial=await refreshOAuth(provider);if(!initial.configured) {await openOAuthSettings(provider);return;}
                captureAccounts();const email=row<0?$(`#user${side}`).value:state[`accounts${side}`][row].user;
                state.oauthBusy={side,row,provider};updateStatusUi();
                const attempt=await api('/api/oauth/connect',{body:JSON.stringify({provider,email,lang:state.lang})});
                for(;;) {
                    await new Promise(resolve=>setTimeout(resolve,500));
                    const view=await refreshOAuth(provider), result=view.attempt;
                    if(!result||result.id!==attempt.id) throw Error('GMAIL_OAUTH_CANCELLED');
                    if(result.status==='waiting') continue;
                    if(result.status!=='connected') throw Error(result.error||'GMAIL_OAUTH_RESPONSE');
                    if(row<0) $(`#user${side}`).value=result.email;else state[`accounts${side}`][row].user=result.email;
                    if(!(view.accounts||[]).find(a=>a.email===result.email)?.persistent) showToast(t('oauthSession'));
                    break;
                }
                renderAccounts();saveDraft();updateReadiness();
            }catch(error){showToast(oauthError(error,provider));}
            finally{state.oauthBusy=null;renderAccounts();updateReadiness();}
        });

        function getFormData() {
            const data = {};
            fields.forEach(id => {
                const el = document.getElementById(id);
                data[id] = el.type === 'checkbox' ? el.checked : (el.type === 'password' ? el.value : el.value.trim());
            });
            for(const side of [1,2]) if(usesOAuth(side)) data[`pass${side}`]='';
            data.resumeBatch=state.resumeBatch||"";
            data.resumeResults=state.resumeResults;
            data.multipleAccounts = state.multipleAccounts;
            data.accounts1 = sheetAccounts(1);
            data.accounts2 = sheetAccounts(2);
            return data;
        }

        function resumeComplete(index) {return Boolean(state.resumeBatch)&&state.resumeResults.some(r=>r.row===index+1&&r.interrupted!==true);}
        function requiredComplete(prefix) {
            if (!state.multipleAccounts) return $(`#host${prefix}`).value.trim() && $(`#user${prefix}`).value.trim() && (usesOAuth(prefix)?oauthReady(prefix,$(`#user${prefix}`).value):$(`#pass${prefix}`).value.trim());
            const source=sheetAccounts(1), destination=sheetAccounts(2);
            const active=source.map((a,i)=>a.user||a.password||destination[i]?.user||destination[i]?.password);
            return $(`#host${prefix}`).value.trim() && active.some(Boolean) && sheetAccounts(prefix).every((a,i)=>!active[i]||resumeComplete(i)||(a.user&&(usesOAuth(prefix)?oauthReady(prefix,a.user):a.password)));
        }

        function updateReadiness() {
            const source = requiredComplete('1');
            const destination = requiredComplete('2');
            const safe = $('#dryRun').checked || confirmDeletesAcknowledged();
            const rate = $('#providerSafeMode').checked || Boolean($('#bandwidthKb').value.trim()) || Boolean($('#maxMessagesPerSecond').value.trim());
            const preview = state.previewReady;
            const checks = { source, destination, safe, rate, preview };
            Object.entries(checks).forEach(([key, ok]) => {
                const item = document.querySelector(`[data-check="${key}"]`);
                item.classList.toggle('ok', ok);
                item.querySelector('.check-dot').textContent = iconCheck(ok);
            });
        }

        function confirmDeletesAcknowledged() {
            return !$('#syncDeletes').checked || $('#dryRun').checked;
        }

        function showToast(message) {
            const toast = $('#toast');
            toast.textContent = message;
            toast.classList.add('show');
            clearTimeout(showToast.timer);
            showToast.timer = setTimeout(() => toast.classList.remove('show'), 2600);
        }

        async function api(path, options = {}) {
            try {return await invoke('api', {path, body:JSON.parse(options.body || '{}')});} catch(error) {if(!path.startsWith('/api/oauth/')&&String(error).includes('_OAUTH_'))throw Error(oauthError(error,String(error).includes('MICROSOFT')?'microsoft':'google'));if(String(error).includes('IMAPSYNC_BUNDLED_MISSING'))throw Error(t('bundledEngineMissing'));if(String(error).includes('RESULTS_EXPORT_EMPTY'))throw Error(t('historyExportEmpty'));if(String(error).includes('HISTORY_POLICY_'))throw Error(t('historyPolicyError'));if(String(error).includes('RESULTS_PROTECTED'))throw Error(t('historyProtected'));if(String(error).includes('RESULTS_DELETE_FAILED'))throw Error(t('historyDeleteFailed'));if(String(error).includes('RESULTS_FILTER'))throw Error(t('historyFilterError'));if(String(error).includes('RESULTS_SELECTION'))throw Error(t('resultsSelectionError'));if(String(error).includes('RESULTS_EXPORT_FAILED')||String(error).includes('RESULTS_FORMAT'))throw Error(t('resultsExportError'));if(String(error).includes('RESULTS_UNAVAILABLE'))throw Error(t('resultsUnavailable'));if(String(error).includes('BATCH_RECOVERY_CHANGED'))throw Error(t('batchRecoveryChanged'));if(String(error).includes('BATCH_RECOVERY_INVALID'))throw Error(t('batchRecoveryInvalid'));if(String(error).includes('RECOVERY_PAIR_BUSY'))throw Error(t('pairBusy'));if(String(error).includes('RECOVERY_PROCESS_CHECK'))throw Error(t('processCheck'));throw error;}
        }

        async function updatePreview(silent = false) {
            const payload = await api('/api/preview', {
                method: 'POST',
                body: JSON.stringify(getFormData())
            });
            $('#command-preview').textContent = payload.commandPreview;
            state.previewReady = true;
            updateReadiness();
            if (!silent) showToast(t('previewReady'));
        }

        async function testConnection(index) {
            const resultEl = $(`#test-result-${index}`);
            resultEl.textContent = '...';
            try {
                const payload = await api('/api/test-connection', {
                    method: 'POST',
                    body: JSON.stringify({
                        host: $(`#host${index}`).value.trim(),
                        port: $(`#port${index}`).value.trim(),
                        security: $(`#security${index}`).value
                    })
                });
                resultEl.textContent = `${t('testOk')} · ${payload.endpoint} · ${payload.latencyMs}ms`;
                resultEl.style.color = 'var(--success)';
            } catch (error) {
                resultEl.textContent = `${t('testFail')} · ${error.message}`;
                resultEl.style.color = 'var(--danger)';
            }
        }

        function renderLogs() {
            const consoleEl = $('#console');
            const visible = state.filter === 'all' ? state.logs : state.logs.filter(log => log.level === state.filter);
            if (!visible.length) {
                consoleEl.innerHTML = `<div class="empty-console">${t('emptyLog')}</div>`;
                return;
            }
            consoleEl.innerHTML = visible.map(log => {
                const date = new Date(log.time);
                const time = Number.isNaN(date.getTime()) ? '--:--:--' : date.toLocaleTimeString([], { hour12: false });
                return `<div class="log-row ${log.level}"><span class="log-time">${time}</span><span class="log-level">${log.level}</span><span>${escapeHtml(log.message)}</span></div>`;
            }).join('');
            if (state.autoscroll) consoleEl.scrollTop = consoleEl.scrollHeight;
        }

        function escapeHtml(text) {
            return String(text).replace(/[&<>"']/g, char => ({
                '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;'
            }[char]));
        }

        function updateMetrics() {
            const progress = state.lastExitCode === 0 && !state.running && !state.cancelled ? 100 : 0;
            $('#message-count').textContent = state.transferred ?? 0;
            $('#error-count').textContent = state.errors ?? (state.startedAt ? '—' : '0');
            $('#meter').style.setProperty('--progress', `${progress}%`);
            $('#meter-value').textContent = state.running ? '…' : `${progress}%`;
        }

        function updateElapsed() {
            if (!state.startedAt) {
                $('#elapsed').textContent = '00:00';
                return;
            }
            const seconds = Math.max(0, Math.floor(((state.finishedAt ? new Date(state.finishedAt).getTime() : Date.now()) - new Date(state.startedAt).getTime()) / 1000));
            const mm = String(Math.floor(seconds / 60)).padStart(2, '0');
            const ss = String(seconds % 60).padStart(2, '0');
            $('#elapsed').textContent = `${mm}:${ss}`;
        }

        function updateStatusUi() {
            renderPresetInfo();
            const locked=state.running||state.submitting||Boolean(state.oauthBusy);
            fields.forEach(id=>document.getElementById(id).disabled=locked);
            document.querySelectorAll('[data-google-connect],[data-google-disconnect],#google-settings-btn,[id^=provider],[id^=auth]').forEach(el=>el.disabled=locked);
            for(const side of [1,2]) {for(const field of ['host','port','security']) $(`#${field}${side}`).disabled=locked||Boolean(PRESETS[$(`#provider${side}`).value]);}
            for(const side of [1,2]) {
                $(`#user${side}`).disabled=locked;
                $(`#auth${side} option[value="password"]`).disabled=['exchange','outlook'].includes($(`#provider${side}`).value);
            }
            $('#sample-btn').disabled=locked;
            $('#oauth-progress').hidden=!state.oauthBusy;
            $('#oauth-progress-text').textContent=t(state.oauthBusy?.provider==='microsoft'?'microsoftWaiting':'oauthWaiting');
            $('#recovery-btn').hidden=!(state.recoveryPending>0);
            $('#recovery-btn').disabled=locked;
            $('#results-btn').disabled=state.resultsBusy||state.historyLoading;
            if($('#results-dialog').open) updateResultActions();
            $('#resume-clear-btn').hidden=!state.resumeBatch;
            $('#resume-clear-btn').disabled=locked;
            const dot = $('#status-dot');
            const label = $('#status-text');
            dot.className = 'dot';
            $('#start-btn').disabled = state.running || state.submitting || Boolean(state.oauthBusy);
            $('#stop-btn').disabled = !state.running || state.stopping || state.stopPending;
            $('#multiple-accounts-btn').disabled = state.running || state.submitting || Boolean(state.oauthBusy);
            $('#add-account-row').disabled = state.running || state.submitting || Boolean(state.oauthBusy);
            document.querySelectorAll('[data-account-side], [data-remove-row]').forEach(el=>el.disabled=state.running||state.submitting||Boolean(state.oauthBusy));
            if(state.resumeBatch) {
                fields.forEach(id=>{if(!['dryRun','pass1','pass2'].includes(id)) document.getElementById(id).disabled=true;});
                document.querySelectorAll('[data-remove-row],[data-account-field="user"],#multiple-accounts-btn,#add-account-row,#sample-btn').forEach(el=>el.disabled=true);
                document.querySelectorAll('[data-account-field="password"]').forEach(el=>el.disabled=locked||resumeComplete(Number(el.dataset.accountRow)));
            }
            $('#batch-progress').hidden = !state.batch&&!state.resumeBatch;
            if(state.resumeBatch&&!state.resumeActive) $('#batch-progress').textContent=`${state.resumeResults.filter(r=>r.interrupted!==true).length}/${state.accounts1.filter((a,i)=>a.user||state.accounts2[i]?.user).length}`;
            else if (state.batch) $('#batch-progress').textContent = `${state.batch.completed}/${state.batch.total}`;
            document.querySelectorAll('.account-sheet tr[data-row]').forEach(row => {
                const result=(state.resumeBatch&&!state.resumeActive?state.resumeResults:state.batch?.results)?.find(r=>r.row===Number(row.dataset.row));
                row.dataset.result=result ? (result.cancelled?'stopped':result.exitCode===0?'success':'error') : state.running&&state.batch?.row===Number(row.dataset.row)?'running':'';
                const button=row.querySelector('[data-remove-row]');
                const resultLabel=result ? t(result.cancelled?'statusStopped':result.exitCode===0?'statusSuccess':'statusError') : row.dataset.result==='running'?t('statusRunning'):'';
                button.title=[t('removeRow'),resultLabel].filter(Boolean).join(' · ');
            });
            if (state.running) {
                dot.classList.add('running');
                label.textContent = t(state.stopping ? 'statusStopping' : 'statusRunning');
            } else if (state.cancelled) {
                label.textContent = t('statusStopped');
            } else if (state.lastExitCode === 0) {
                dot.classList.add('success');
                label.textContent = t(state.dryRun ? 'statusDrySuccess' : 'statusSuccess');
            } else if (typeof state.lastExitCode === 'number') {
                dot.classList.add('error');
                label.textContent = t('statusError');
            } else {
                label.textContent = t('statusIdle');
            }
        }

        function applyStatus(status) {
            Object.assign(state, status);
            if(state.resumeBatch&&state.resumeActive&&!status.running&&!status.dryRun&&typeof status.lastExitCode==='number'&&status.batch?.completed===status.batch?.total) {
                state.resumeBatch=null;state.resumeResults=[];state.resumeActive=false;saveDraft();
            }
            if (status.commandPreview) $('#command-preview').textContent = status.commandPreview;
            updateStatusUi();
            updateMetrics();
            updateElapsed();
            updateReadiness();
        }

        async function refreshStatus() {
            try { applyStatus(await api('/api/status')); }
            catch (error) { showToast(String(error)); }
        }

        let draftSignature='';
        function saveDraft() {
            const data = getFormData();
            data.pass1 = '';
            data.pass2 = '';
            for (const side of [1,2]) data[`accounts${side}`] = data[`accounts${side}`].map(a=>({...a,password:''}));
            localStorage.setItem('imapsync-draft', JSON.stringify(data));
            api('/api/save-draft',{body:JSON.stringify(getFormData())}).catch(e=>showToast(String(e)));
            draftSignature=JSON.stringify(getFormData());
        }

        function applyDraft(data) {
            state.resumeActive=false;
            state.resumeBatch=data.resumeBatch||null;
            state.resumeResults=Array.isArray(data.resumeResults)?data.resumeResults:[];
            fields.forEach(key=> {
                if (!(key in data)) return;
                const el=document.getElementById(key);
                if (el.type==='checkbox') el.checked=Boolean(data[key]); else el.value=data[key];
            });
            for (const side of [1,2]) if (Array.isArray(data[`accounts${side}`])) {
                state[`accounts${side}`] = data[`accounts${side}`].slice(0,1000).map(a=>({user:String(a.user||''),password:String(a.password||'')}));
            }
            const length=Math.max(1,state.accounts1.length,state.accounts2.length);
            for(const side of [1,2]) while(state[`accounts${side}`].length<length) state[`accounts${side}`].push({user:'',password:''});
            for(const side of [1,2]) {
                const provider=$('#provider'+side),preset=PRESETS[provider.value];
                if(preset) {for(const id of ['host','port','security'])$('#'+id+side).value=preset[id];if(['exchange','outlook'].includes(provider.value))$('#auth'+side).value='microsoft';}
                else {provider.value='custom';state.customEndpoints[side]=['host','port','security'].map(id=>$('#'+id+side).value);}
                state.appliedPresets[side]=provider.value;
            }
            renderPresetInfo();
            // Rebuild before toggling so old DOM values cannot overwrite the restored sheet.
            renderAccounts();
            setMultipleAccounts(Boolean(data.multipleAccounts));
        }

        function restoreDraft() {
            let data; try {data=JSON.parse(localStorage.getItem('imapsync-draft')||'{}');} catch {data={};}
            applyDraft(data);
        }

        function loadSample() {
            for(const side of [1,2]) {$(`#provider${side}`).value='custom';$(`#auth${side}`).value='password';state.appliedPresets[side]='custom';}
            renderAuthUi();
            const sample = {
                host1: 'imap.old-mail.example', port1: '993', user1: 'archive@example.com',
                host2: 'imap.new-mail.example', port2: '993', user2: 'archive@example.com',
                maxAgeDays: '365', bandwidthKb: '512', maxMessagesPerSecond: '0.5', maxSleepSeconds: '8', maxBytesAfterMb: '50', errorsMax: '10', timeoutSeconds: '180', excludePattern: '^Spam$\n^Trash/Old', advancedArgs: '--nofoldersizes'
            };
            Object.entries(sample).forEach(([key, value]) => {
                const el = document.getElementById(key);
                if (el) el.value = value;
            });
            $('#dryRun').checked = true;
            $('#providerSafeMode').checked = true;
            $('#automapFolders').checked = true;
            $('#subscribeFolders').checked = true;
            if(state.multipleAccounts) {
                for(const side of [1,2]) state[`accounts${side}`]=Array.from({length:3},(_,i)=>({user:`account${i+1}@example.com`,password:''}));
                renderAccounts();
            }
            state.previewReady = false;
            updateReadiness();
            saveDraft();
            showToast(t('sampleLoaded'));
        }

        $('#lang-btn').addEventListener('click', () => {
            state.lang = ({ja:'en', en:'zh', zh:'ja'})[state.lang];
            localStorage.setItem('imapsync-lang', state.lang);
            applyTranslations();
            renderLogs();
        });

        function historyFilters() {return {query:$('#history-search').value,status:$('#history-status').value,mode:$('#history-mode').value};}
        function selectedReport() {return state.resultReports.find(r=>r.id===$('#results-run').value);}
        function updateResultActions() {
            const report=selectedReport(),busy=state.resultsBusy||state.historyLoading;
            $('#results-retry').disabled=busy||state.running||state.submitting||Boolean(state.oauthBusy)||!document.querySelector('#results-rows input:checked');
            for(const format of ['csv','json']) $(`#results-${format}`).disabled=busy||!report;
            for(const format of ['csv','json']) $('#history-export-'+format).disabled=busy||state.historyTotal===0;
            $('#history-export-count').textContent=t('historyExportCount').replace('{count}',String(state.historyTotal));
            $('#results-run').disabled=busy;
            $('#results-btn').disabled=busy;
            $('#history-prev').disabled=busy||state.historyOffset===0;
            $('#history-next').disabled=busy||state.historyOffset+state.historyLimit>=state.historyTotal;
            $('#history-refresh').disabled=busy;
            $('#history-delete').disabled=busy||!report?.canDelete||report.running;
            $('#history-delete-yes').disabled=busy;
            $('#history-delete-no').disabled=busy;
            $('#history-retention').disabled=busy||state.historyPolicyDays===null;
            $('#history-policy-save').disabled=busy||state.historyPolicyDays===null||Number($('#history-retention').value)===state.historyPolicyDays;
            $('#history-bulk').disabled=busy;
            $('#history-maintenance-yes').disabled=busy;
            $('#history-maintenance-no').disabled=busy;
            $('#results-close').disabled=state.resultsBusy;
            for(const id of ['history-search','history-status','history-mode']) $('#'+id).disabled=state.resultsBusy;
            $('#results-dialog').setAttribute('aria-busy',String(busy));
            $('#history-count').textContent=`${state.historyTotal?state.historyOffset+1:0}–${Math.min(state.historyOffset+state.historyLimit,state.historyTotal)} / ${state.historyTotal}`;
            $('#results-select-failed').disabled=busy||state.running||!report?.rows?.some(r=>['failed','stopped'].includes(r.status));
            const boxes=[...document.querySelectorAll('#results-rows input')];
            boxes.forEach(el=>el.disabled=busy||state.running||!['failed','stopped'].includes(el.dataset.status));
            const eligible=boxes.filter(el=>['failed','stopped'].includes(el.dataset.status)),checked=eligible.filter(el=>el.checked).length;
            $('#results-select-failed').checked=eligible.length>0&&checked===eligible.length;
            $('#results-select-failed').indeterminate=checked>0&&checked<eligible.length;
        }
        function renderResultRows() {
            hideHistoryDelete();
            const report=selectedReport();
            $('#results-empty').hidden=Boolean(report?.rows?.length);
            $('#results-empty').textContent=t($('#history-search').value||$('#history-status').value!=='all'||$('#history-mode').value!=='all'?'historyNoMatches':'resultsEmpty');
            $('#results-rows').innerHTML=(report?.rows||[]).map(row=>`<tr><td><input type="checkbox" data-result-row="${row.row}" data-status="${escapeHtml(row.status)}" aria-label="${escapeHtml(t('resultsRow'))} ${row.row}"></td><td>${row.row}</td><td>${escapeHtml(row.source)}</td><td>${escapeHtml(row.destination)}</td><td data-result-status="${escapeHtml(row.status)}">${escapeHtml(t('results'+row.status[0].toUpperCase()+row.status.slice(1)))}</td><td>${row.exitCode??'—'}</td></tr>`).join('');
            $('#results-select-failed').checked=false;updateResultActions();
        }
        function hideHistoryDelete() {state.historyDeleteId=null;$('#history-delete-confirm').hidden=true;state.historyMaintenance=null;$('#history-maintenance-confirm').hidden=true;}
        function cancelHistoryRequests() {clearTimeout(loadResults.timer);++state.historyRequest;state.historyLoading=false;hideHistoryDelete();updateResultActions();}
        async function loadResults({reset=false}={}) {
            clearTimeout(loadResults.timer);hideHistoryDelete();
            const request=++state.historyRequest,selected=$('#results-run').value;
            if(reset)state.historyOffset=0;
            state.historyLoading=true;updateResultActions();
            try {
                const page=await api('/api/results/search',{body:JSON.stringify({...historyFilters(),offset:state.historyOffset,limit:state.historyLimit})});
                if(request!==state.historyRequest)return;
                state.resultReports=page.reports||[];state.historyTotal=page.total||0;state.historyOffset=page.offset||0;state.historyLimit=page.limit||20;
                $('#results-run').innerHTML=state.resultReports.map(r=>`<option value="${escapeHtml(r.id)}">${escapeHtml(r.startedAt?new Date(r.startedAt).toLocaleString():r.id)} · ${t(r.accountMode==='single'?'resultsSingle':'resultsMultiple')} · ${r.dryRun?t('dryRun'):t('historySync')} · ${r.rows.filter(row=>row.status==='failed'||row.status==='stopped').length} ${escapeHtml(t('errors'))}</option>`).join('');
                if(state.resultReports.some(r=>r.id===selected))$('#results-run').value=selected;
                renderResultRows();
            } catch(error) {if(request===state.historyRequest){state.resultReports=[];state.historyTotal=0;state.historyOffset=0;$('#results-run').innerHTML='';renderResultRows();throw error;}} finally {if(request===state.historyRequest){state.historyLoading=false;updateResultActions();}}
        }
        function scheduleHistorySearch() {
            clearTimeout(loadResults.timer);hideHistoryDelete();
            // Immediately invalidate requests/actions for the previous search.
            ++state.historyRequest;state.historyLoading=true;updateResultActions();
            loadResults.timer=setTimeout(()=>loadResults({reset:true}).catch(error=>showToast(error.message||String(error))),200);
        }
        $('#history-search').addEventListener('input',scheduleHistorySearch);
        $('#history-search').addEventListener('search',scheduleHistorySearch);
        for(const id of ['history-status','history-mode']) $('#'+id).addEventListener('change',()=>loadResults({reset:true}).catch(error=>showToast(error.message||String(error))));
        $('#history-prev').addEventListener('click',()=>{state.historyOffset=Math.max(0,state.historyOffset-state.historyLimit);loadResults().catch(error=>showToast(error.message||String(error)));});
        $('#history-next').addEventListener('click',()=>{state.historyOffset+=state.historyLimit;loadResults().catch(error=>showToast(error.message||String(error)));});
        $('#history-refresh').addEventListener('click',()=>loadResults().catch(error=>showToast(error.message||String(error))));
        $('#history-delete').addEventListener('click',()=>{
            const report=selectedReport();if(state.resultsBusy||state.historyLoading||!report?.canDelete)return;
            state.historyDeleteId=report.id;$('#history-delete-confirm').hidden=false;
            $('#history-delete-text').textContent=`${t('historyDeleteQuestion')} ${report.startedAt?new Date(report.startedAt).toLocaleString():report.id} · ${report.id.slice(0,8)}`;
            $('#history-delete-no').focus();
        });
        $('#history-delete-no').addEventListener('click',()=>{hideHistoryDelete();$('#history-delete').focus();});
        $('#history-delete-yes').addEventListener('click',async()=>{
            const id=state.historyDeleteId;
            if(state.resultsBusy||state.historyLoading||!id||id!==selectedReport()?.id)return;
            state.resultsBusy=true;updateResultActions();
            try {await api('/api/results/delete',{body:JSON.stringify({id})});hideHistoryDelete();await loadResults();showToast(t('historyDeleted'));}
            catch(error){hideHistoryDelete();showToast(error.message||String(error));}
            finally{state.resultsBusy=false;updateResultActions();}
        });
        $('#results-btn').addEventListener('click',async()=>{
            state.historyPolicyDays=null;
            try {const policy=await api('/api/results/policy');state.historyPolicyDays=policy.days;$('#history-retention').value=String(policy.days);}
            catch(error){showToast(error.message||String(error));}
            try {await loadResults();if(!$('#results-dialog').open)$('#results-dialog').showModal();}
            catch(error){showToast(error.message||String(error));}
        });
        const historyMessage=(key,values)=>Object.entries(values).reduce((text,[key,value])=>text.replaceAll('{'+key+'}',String(value)),t(key));
        $('#history-retention').addEventListener('change',()=>{hideHistoryDelete();updateResultActions();});
        async function previewHistoryCleanup(savePolicy) {
            if(state.resultsBusy||state.historyLoading)return;
            hideHistoryDelete();state.resultsBusy=true;updateResultActions();
            const generation=state.historyRequest,days=savePolicy?Number($('#history-retention').value):0;
            try {
                const preview=await api('/api/results/cleanup/preview',{body:JSON.stringify({days,savePolicy})});
                if(generation!==state.historyRequest||!$('#results-dialog').open)return;
                if(!savePolicy&&!preview.count){showToast(t('historyNoCompleted'));return;}
                state.historyMaintenance={days,ids:preview.ids,savePolicy,sessionStartedAt:preview.sessionStartedAt};
                const period=$('#history-retention').selectedOptions[0].textContent;
                $('#history-maintenance-text').textContent=historyMessage(savePolicy?(days?'historyPolicyQuestion':'historyForeverQuestion'):'historyBulkQuestion',{count:preview.count,period});
                $('#history-maintenance-yes').textContent=t(savePolicy?'historyPolicyApply':'historyDeleteConfirm');
                $('#history-maintenance-confirm').hidden=false;$('#history-maintenance-no').focus();
            }catch(error){showToast(error.message||String(error));}
            finally{state.resultsBusy=false;updateResultActions();if(state.historyMaintenance)$('#history-maintenance-no').focus();}
        }
        $('#history-policy-save').addEventListener('click',()=>previewHistoryCleanup(true));
        $('#history-bulk').addEventListener('click',()=>previewHistoryCleanup(false));
        $('#history-maintenance-no').addEventListener('click',()=>{const save=state.historyMaintenance?.savePolicy;hideHistoryDelete();$('#'+(save?'history-policy-save':'history-bulk')).focus();});
        $('#history-maintenance-yes').addEventListener('click',async()=>{
            const request=state.historyMaintenance;if(!request||state.resultsBusy||state.historyLoading)return;
            state.resultsBusy=true;updateResultActions();
            try {
                const result=await api('/api/results/cleanup',{body:JSON.stringify(request)});
                if(request.savePolicy)state.historyPolicyDays=request.days;
                hideHistoryDelete();await loadResults();showToast(historyMessage('historyMaintenanceDone',result));
            }catch(error){hideHistoryDelete();showToast(error.message||String(error));}
            finally{state.resultsBusy=false;updateResultActions();}
        });
        $('#results-dialog').addEventListener('cancel',event=>{if(state.resultsBusy)event.preventDefault();});
        $('#results-close').addEventListener('click',()=>{cancelHistoryRequests();$('#results-dialog').close();});
        $('#results-dialog').addEventListener('close',()=>{if(!$('#results-dialog').open)cancelHistoryRequests();});
        $('#results-run').addEventListener('change',renderResultRows);
        $('#results-rows').addEventListener('change',updateResultActions);
        $('#results-select-failed').addEventListener('change',event=>{document.querySelectorAll('#results-rows input').forEach(el=>{el.checked=!el.disabled&&event.target.checked;});updateResultActions();});
        $('#results-retry').addEventListener('click',async()=>{
            if(state.running||state.submitting||state.oauthBusy||state.resultsBusy||state.historyLoading)return;
            state.resultsBusy=true;updateResultActions();
            try {
                const rows=[...document.querySelectorAll('#results-rows input:checked')].map(el=>Number(el.dataset.resultRow));
                const record=await api('/api/results/retry',{body:JSON.stringify({id:$('#results-run').value,rows})});
                applyDraft({...record.settings,dryRun:true,pass1:'',pass2:'',resumeBatch:record.id,resumeResults:[]});saveDraft();state.previewReady=false;updateReadiness();selectPanel(0);$('#results-dialog').close();showToast(t('resultsPrepared'));
            }catch(error){showToast(error.message||String(error));}
            finally{state.resultsBusy=false;updateStatusUi();}
        });
        for(const format of ['csv','json'])$(`#results-${format}`).addEventListener('click',async()=>{
            if(state.resultsBusy||state.historyLoading)return;state.resultsBusy=true;updateResultActions();
            try{const result=await api('/api/results/export',{body:JSON.stringify({id:$('#results-run').value,format,lang:state.lang})});if(result.saved)showToast(t('resultsSaved'));}
            catch(error){showToast(error.message||String(error));}
            finally{state.resultsBusy=false;updateResultActions();}
        });
        for(const format of ['csv','json']) $('#history-export-'+format).addEventListener('click',async()=>{
            if(state.resultsBusy||state.historyLoading||!state.historyTotal)return;
            const filters=historyFilters();hideHistoryDelete();state.resultsBusy=true;updateResultActions();
            try {
                const result=await api('/api/results/export-matching',{body:JSON.stringify({...filters,format,lang:state.lang})});
                if(result.saved)showToast(t('historyExportSaved').replace('{count}',String(result.runCount)));
            }catch(error){showToast(error.message||String(error));}
            finally{state.resultsBusy=false;updateResultActions();}
        });
        $('#recovery-btn').addEventListener('click',async()=>{
            if(state.running||state.submitting||state.oauthBusy) return;
            try {
                const records=await api('/api/recovery');
                const record=records.find(entry=>entry.settings);
                if(!record) return showToast(t('recoveryUnreadable'));
                const data={...record.settings,dryRun:true,pass1:'',pass2:'',resumeBatch:record.kind==='batch'?record.id:'',resumeResults:record.kind==='batch'?record.results:[]};
                if(record.kind!=='batch') {data.accounts1=[{user:'',password:''}];data.accounts2=[{user:'',password:''}];}
                for(const side of [1,2]) {data[`provider${side}`]=data[`provider${side}`]||'custom';data[`auth${side}`]=data[`auth${side}`]||'password';}
                applyDraft(data);saveDraft();state.previewReady=false;updateReadiness();selectPanel(0);showToast(t('recoveryLoaded'));
            }catch(error){showToast(error.message||String(error));}
        });
        $('#resume-clear-btn').addEventListener('click',()=>{
            if(state.running||state.submitting||state.oauthBusy)return;
            state.resumeBatch=null;state.resumeResults=[];state.resumeActive=false;
            fields.forEach(id=>document.getElementById(id).disabled=false);
            captureAccounts();renderAccounts();saveDraft();updateReadiness();
        });
        $('#sample-btn').addEventListener('click', loadSample);
        $('#preview-btn').addEventListener('click', async () => {
            selectPanel(panels.indexOf(document.querySelector('.preview-panel')));
            try { await updatePreview(); }
            catch (error) { showToast(error.message || String(error)); }
        });
        $('#copy-btn').addEventListener('click', async () => {
            await navigator.clipboard.writeText($('#command-preview').textContent);
            showToast(t('copied'));
        });

        document.querySelectorAll('[data-test]').forEach(button => {
            button.addEventListener('click', () => testConnection(button.dataset.test));
        });

        fields.forEach(id => {
            const el = document.getElementById(id);
            el.addEventListener('input', () => {
                if(id.startsWith('user')) renderAuthUi();
                state.previewReady = false;
                saveDraft();
                updateReadiness();
            });
            el.addEventListener('change', () => {
                if(id.startsWith('provider')) changePreset(Number(id.slice(-1)));
                if(id.startsWith('auth')) {captureAccounts();const side=Number(id.slice(-1)),provider=$('#provider'+side);if(el.value==='google'&&provider.value!=='gmail'){provider.value='gmail';changePreset(side);}else if(el.value==='microsoft'&&!['exchange','outlook'].includes(provider.value)){provider.value='exchange';changePreset(side);}renderAccounts();}
                state.previewReady = false;
                saveDraft();
                updateReadiness();
            });
        });

        renderAccounts();
        $('#sync-form').noValidate=true;
        $('#sync-form').addEventListener('submit', async event => {
            event.preventDefault();
            if (state.running || state.submitting || state.oauthBusy) return;
            state.submitting = true;
            updateStatusUi();
            try {
                await updatePreview(true);
                const payload = await api('/api/sync', {
                    method: 'POST',
                    body: JSON.stringify(getFormData())
                });
                state.resumeActive=true;
                applyStatus(payload.status);
                showToast(t('started'));
            } catch (error) {
                showToast(error.message || String(error));
            } finally {
                state.submitting = false;
                updateStatusUi();
            }
        });

        $('#stop-btn').addEventListener('click', async () => {
            if (!state.running || state.stopping || state.stopPending) return;
            state.stopPending = true;
            updateStatusUi();
            try {
                const payload = await api('/api/stop', { method: 'POST' });
                applyStatus(payload.status);
                showToast(t(payload.status.running ? 'stopRequested' : 'stopped'));
            } catch (error) {
                showToast(error.message || String(error));
            } finally {
                state.stopPending = false;
                updateStatusUi();
            }
        });

        $('#clear-log-btn').addEventListener('click', () => {
            state.logs = [];
            renderLogs();
            updateMetrics();
        });

        $('#autoscroll-btn').addEventListener('click', () => {
            state.autoscroll = !state.autoscroll;
            $('#autoscroll-btn').textContent = state.autoscroll ? t('autoscroll') : t('autoscrollOff');
        });

        document.querySelectorAll('.tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tab').forEach(item => item.classList.remove('active'));
                tab.classList.add('active');
                state.filter = tab.dataset.filter;
                renderLogs();
            });
        });

        async function pollLogs() {
            try { const logs = await invoke('logs'); state.logs = logs; renderLogs(); updateMetrics(); } catch(error) { showToast(String(error)); }
        }
        setInterval(pollLogs, 1000);
        pollLogs();
        const panels = [...document.querySelectorAll('section.panel')];
        let active = 0;
        const nav = document.querySelector('.panel-tabs');
        panels.forEach((panel, index) => {
            panel.id = `panel-${index}`; panel.setAttribute('role','tabpanel');
            const key = panel.querySelector('h2').dataset.i18n;
            const button = document.createElement('button');
            button.className='panel-tab'; button.dataset.i18n=key; button.setAttribute('role','tab');
            button.id=`tab-${index}`; button.setAttribute('aria-controls',panel.id);
            panel.setAttribute('aria-labelledby',button.id);
            button.addEventListener('click',()=>selectPanel(index));
            button.addEventListener('keydown',e=> { if(['ArrowLeft','ArrowRight','Home','End'].includes(e.key)){ e.preventDefault(); const n=e.key==='Home'?0:e.key==='End'?panels.length-1:(active+(e.key==='ArrowRight'?1:-1)+panels.length)%panels.length; selectPanel(n); nav.children[n].focus(); }});
            nav.append(button);
        });
        function selectPanel(index) { active=index; panels.forEach((p,i)=>p.classList.toggle('selected',i===index)); [...nav.children].forEach((b,i)=>{b.setAttribute('aria-selected',String(i===index)); b.tabIndex=i===index?0:-1;}); const tab=nav.children[index]; if(tab) nav.scrollTo({left:Math.max(0,tab.offsetLeft-nav.offsetLeft-(nav.clientWidth-tab.offsetWidth)/2),behavior:'smooth'}); }
        selectPanel(active);
        document.querySelectorAll('.panel-subtitle,.switch-row strong+span,.safety-title strong+span').forEach(help=> {
            const icon=document.createElement('button'); icon.type='button'; icon.className='tiny-button'; icon.textContent='ⓘ'; icon.title=help.textContent; icon.setAttribute('aria-label',help.textContent); icon.dataset.help=help.dataset.i18n; help.parentNode.append(icon);
        });
        restoreDraft();
        const sharedDraft=await api('/api/draft');
        if (sharedDraft && Object.keys(sharedDraft).length) applyDraft(sharedDraft);
        else {await api('/api/save-draft',{body:JSON.stringify(getFormData())});}
        draftSignature=JSON.stringify(getFormData());
        setInterval(async()=>{
            if (fields.includes(document.activeElement?.id) || document.activeElement?.matches('[data-account-side]') || state.submitting || state.oauthBusy || $('#google-oauth-dialog').open) return;
            try {const draft=await api('/api/draft');const signature=JSON.stringify(draft);
                if(signature!==draftSignature){ applyDraft(draft);draftSignature=signature;state.previewReady=false;updateReadiness();}
            }catch(error){showToast(String(error));}
        },1000);
        try {await Promise.all([refreshOAuth(),refreshOAuth('microsoft')]);}catch {}
        applyTranslations();
        updateReadiness();
        refreshStatus();
        setInterval(async()=>{if(state.oauthBusy||state.running||document.activeElement?.matches('input,select')||$('#google-oauth-dialog').open)return;try{const previous=JSON.stringify([state.oauth,state.microsoftOauth]);await Promise.all([refreshOAuth(),refreshOAuth('microsoft')]);if(previous!==JSON.stringify([state.oauth,state.microsoftOauth])){captureAccounts();renderAccounts();updateReadiness();}}catch{}},3000);
        setInterval(updateElapsed, 1000);
        setInterval(refreshStatus, 1000);
    
})().catch(error=>{document.getElementById("toast").textContent=String(error);document.getElementById("toast").classList.add("show");});
