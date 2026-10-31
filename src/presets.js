// IMAP endpoint presets. Authentication remains a separate account setting.
// Official source links and verification date are documented in README.md.
export const PRESETS = Object.freeze({
    icloud: {label:'iCloud', host:'imap.mail.me.com', port:'993', security:'ssl', help:'presetIcloud'},
    exchange: {label:'Microsoft Exchange / Microsoft 365', host:'outlook.office365.com', port:'993', security:'ssl', help:'presetExchange'},
    gmail: {label:'Google / Gmail / Google Workspace', host:'imap.gmail.com', port:'993', security:'ssl', help:'presetGoogle'},
    yahoo: {label:'Yahoo Mail', host:'imap.mail.yahoo.com', port:'993', security:'ssl', help:'presetYahoo'},
    yahoo_jp: {label:'Yahoo! JAPAN', host:'imap.mail.yahoo.co.jp', port:'993', security:'ssl', help:'presetYahooJapan'},
    aol: {label:'AOL', host:'imap.aol.com', port:'993', security:'ssl', help:'presetAol'},
    outlook: {label:'Outlook.com / Hotmail / Live', host:'outlook.office365.com', port:'993', security:'ssl', help:'presetOutlook'}
});
