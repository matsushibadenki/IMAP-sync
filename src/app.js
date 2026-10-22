import './style.css';
import body from './body';
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
        const fields = ['host1','port1','security1','user1','pass1','host2','port2','security2','user2','pass2','dryRun','syncDeletes','subscribeFolders','automapFolders','useUid','fastMode','providerSafeMode','maxMessagesPerSecond','maxSleepSeconds','maxBytesAfterMb','errorsMax','timeoutSeconds','stopAfterMb','maxAgeDays','minAgeDays','bandwidthKb','excludePattern','advancedArgs'];
        const state = {
            lang: localStorage.getItem('imapsync-lang') || 'ja',
            logs: [],
            filter: 'all',
            autoscroll: true,
            previewReady: false,
            startedAt: null,
            running: false
        };

        const $ = selector => document.querySelector(selector);
        const t = key => i18n[state.lang][key] || key;

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
            updateStatusUi();
        }

        function getFormData() {
            const data = {};
            fields.forEach(id => {
                const el = document.getElementById(id);
                data[id] = el.type === 'checkbox' ? el.checked : (el.type === 'password' ? el.value : el.value.trim());
            });
            return data;
        }

        function requiredComplete(prefix) {
            return ['host', 'user', 'pass'].every(name => document.getElementById(`${name}${prefix}`).value.trim());
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
            return invoke('api', {path, body:JSON.parse(options.body || '{}')});
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
            const allText = state.logs.map(log => log.message).join('\n');
            const messageMatches = allText.match(/(?:messages?|msgs?)\D+(\d+)/ig) || [];
            const transferredMatches = allText.match(/transferred\D+(\d+)/ig) || [];
            const count = transferredMatches.length || messageMatches.length || state.logs.filter(log => /message|mail|copy|append/i.test(log.message)).length;
            const errors = state.logs.filter(log => log.level === 'error' || /error|failed|failure/i.test(log.message)).length;
            const progress = state.lastExitCode === 0 && !state.running ? 100 : 0;
            $('#message-count').textContent = count;
            $('#error-count').textContent = errors;
            $('#meter').style.setProperty('--progress', `${progress}%`);
            $('#meter-value').textContent = state.running ? '…' : `${progress}%`;
        }

        function updateElapsed() {
            if (!state.startedAt) {
                $('#elapsed').textContent = '00:00';
                return;
            }
            const seconds = Math.max(0, Math.floor((Date.now() - new Date(state.startedAt).getTime()) / 1000));
            const mm = String(Math.floor(seconds / 60)).padStart(2, '0');
            const ss = String(seconds % 60).padStart(2, '0');
            $('#elapsed').textContent = `${mm}:${ss}`;
        }

        function updateStatusUi(status = {}) {
            const dot = $('#status-dot');
            const label = $('#status-text');
            dot.className = 'dot';
            $('#start-btn').disabled=state.running; $('#stop-btn').disabled=!state.running;
            if (state.running || status.running) {
                dot.classList.add('running');
                label.textContent = t('statusRunning');
                return;
            }
            if (status.lastExitCode === 0) {
                dot.classList.add('success');
                label.textContent = t('statusSuccess');
            } else if (typeof status.lastExitCode === 'number' && status.lastExitCode !== 0) {
                dot.classList.add('error');
                label.textContent = t('statusError');
            } else {
                label.textContent = t('statusIdle');
            }
        }

        async function refreshStatus() {
            try {
                const status = await api('/api/status');
                state.running = status.running;
                state.startedAt = status.startedAt; state.lastExitCode = status.lastExitCode;
                if (status.commandPreview) {
                    $('#command-preview').textContent = status.commandPreview;
                    state.previewReady = true;
                }
                updateStatusUi(status);
                updateMetrics();
                updateReadiness();
            } catch (error) { showToast(String(error)); }
        }

        let draftSignature='';
        function saveDraft() {
            const data = getFormData();
            data.pass1 = '';
            data.pass2 = '';
            localStorage.setItem('imapsync-draft', JSON.stringify(data));
            api('/api/save-draft',{body:JSON.stringify(getFormData())}).catch(e=>showToast(String(e)));
            draftSignature=JSON.stringify(getFormData());
        }

        function restoreDraft() {
            let data; try { data = JSON.parse(localStorage.getItem('imapsync-draft') || '{}'); } catch { data = {}; }
            Object.entries(data).forEach(([key, value]) => {
                const el = document.getElementById(key);
                if (!el) return;
                if (el.type === 'checkbox') el.checked = Boolean(value);
                else el.value = value;
            });
        }

        function loadSample() {
            const sample = {
                host1: 'imap.old-mail.example', port1: '993', user1: 'archive@example.com',
                host2: 'imap.new-mail.example', port2: '993', user2: 'archive@example.com',
                maxAgeDays: '365', bandwidthKb: '512', maxMessagesPerSecond: '0.5', maxSleepSeconds: '8', maxBytesAfterMb: '50', errorsMax: '10', timeoutSeconds: '180', excludePattern: '^Spam$\\n^Trash/Old', advancedArgs: '--nofoldersizes'
            };
            Object.entries(sample).forEach(([key, value]) => {
                const el = document.getElementById(key);
                if (el) el.value = value;
            });
            $('#dryRun').checked = true;
            $('#providerSafeMode').checked = true;
            $('#automapFolders').checked = true;
            $('#subscribeFolders').checked = true;
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

        $('#sample-btn').addEventListener('click', loadSample);
        $('#preview-btn').addEventListener('click', () => updatePreview().catch(error => showToast(error.message)));
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
                state.previewReady = false;
                saveDraft();
                updateReadiness();
            });
            el.addEventListener('change', () => {
                state.previewReady = false;
                saveDraft();
                updateReadiness();
            });
        });

        $('#sync-form').noValidate=true;
        $('#sync-form').addEventListener('submit', async event => {
            event.preventDefault();
            try {
                await updatePreview(true);
                const payload = await api('/api/sync', {
                    method: 'POST',
                    body: JSON.stringify(getFormData())
                });
                state.running = true;
                state.startedAt = payload.status?.startedAt || new Date().toISOString();
                updateStatusUi(payload.status);
                showToast(t('started'));
            } catch (error) {
                showToast(error.message);
            }
        });

        $('#stop-btn').addEventListener('click', async () => {
            try {
                const payload = await api('/api/stop', { method: 'POST' });
                state.running = false;
                updateStatusUi(payload.status);
                showToast(t('stopped'));
            } catch (error) {
                showToast(error.message);
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
        let active = Number(new URLSearchParams(location.search).get('panel') || 0);
        if (!panels[active]) active = 0;
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
        function selectPanel(index) { active=index; panels.forEach((p,i)=>p.classList.toggle('selected',i===index)); [...nav.children].forEach((b,i)=>{b.setAttribute('aria-selected',String(i===index)); b.tabIndex=i===index?0:-1;}); }
        selectPanel(active);
        if (window.__TAURI__) await window.__TAURI__.event.listen('dock-panel',event=>selectPanel(event.payload));
        $('#float-btn').onclick=()=>invoke('float_panel',{panel:active}).catch(e=>showToast(String(e)));
        $('#dock-btn').onclick=()=>invoke('dock_panel').catch(e=>showToast(String(e)));
        document.querySelectorAll('.panel-subtitle,.switch-row strong+span,.safety-title strong+span').forEach(help=> {
            const icon=document.createElement('button'); icon.type='button'; icon.className='tiny-button'; icon.textContent='ⓘ'; icon.title=help.textContent; icon.setAttribute('aria-label',help.textContent); icon.dataset.help=help.dataset.i18n; help.parentNode.append(icon);
        });
        restoreDraft();
        const sharedDraft=await api('/api/draft');
        if (sharedDraft && Object.keys(sharedDraft).length) { fields.forEach(id=> { if(id in sharedDraft){ const el=document.getElementById(id); if(el.type==='checkbox') el.checked=sharedDraft[id]; else el.value=sharedDraft[id]; } }); }
        else {await api('/api/save-draft',{body:JSON.stringify(getFormData())});}
        draftSignature=JSON.stringify(getFormData());
        setInterval(async()=>{
            if (fields.includes(document.activeElement?.id)) return;
            try {const draft=await api('/api/draft');const signature=JSON.stringify(draft);
                if(signature!==draftSignature){ fields.forEach(id=>{if(id in draft){const el=document.getElementById(id);if(el.type==='checkbox')el.checked=draft[id];else el.value=draft[id];}});draftSignature=signature;state.previewReady=false;updateReadiness();}
            }catch(error){showToast(String(error));}
        },1000);
        applyTranslations();
        updateReadiness();
        refreshStatus();
        setInterval(updateElapsed, 1000);
        setInterval(refreshStatus, 5000);
    
})().catch(error=>{document.getElementById("toast").textContent=String(error);document.getElementById("toast").classList.add("show");});
