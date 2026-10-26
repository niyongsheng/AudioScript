use std::sync::atomic::{AtomicU8, Ordering};

use crate::config::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Locale {
    ZhCn = 0,
    ZhTw = 1,
    En = 2,
    Ja = 3,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::En => "en",
            Self::Ja => "ja",
        }
    }

    pub fn from_language(language: Language) -> Self {
        match language {
            Language::ZhCn => Self::ZhCn,
            Language::ZhTw => Self::ZhTw,
            Language::En => Self::En,
            Language::Ja => Self::Ja,
            Language::System => Self::system(),
        }
    }

    fn system() -> Self {
        let tag = sys_locale::get_locale()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if tag.starts_with("zh") {
            if tag.contains("hant")
                || tag.contains("tw")
                || tag.contains("hk")
                || tag.contains("mo")
            {
                Self::ZhTw
            } else {
                Self::ZhCn
            }
        } else if tag.starts_with("ja") {
            Self::Ja
        } else {
            Self::En
        }
    }
}

static LOCALE: AtomicU8 = AtomicU8::new(Locale::ZhCn as u8);

pub fn set_locale(locale: Locale) -> bool {
    LOCALE.swap(locale as u8, Ordering::SeqCst) != locale as u8
}

pub fn locale() -> Locale {
    match LOCALE.load(Ordering::SeqCst) {
        1 => Locale::ZhTw,
        2 => Locale::En,
        3 => Locale::Ja,
        _ => Locale::ZhCn,
    }
}

pub fn tf(key: Key, args: &[&str]) -> String {
    let template = tr(key);
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let placeholder = &after_open[..close];
        match placeholder.parse::<usize>().ok().and_then(|i| args.get(i)) {
            Some(arg) => out.push_str(arg),
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after_open[close + 1..];
    }
    out.push_str(rest);
    out
}

macro_rules! messages {
    ($( $key:ident { zh_cn: $zh_cn:literal, zh_tw: $zh_tw:literal, en: $en:literal, ja: $ja:literal } ),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Key {
            $($key),*
        }

        pub const ALL_KEYS: &[Key] = &[$(Key::$key),*];

        pub fn text(key: Key, locale: Locale) -> &'static str {
            match key {
                $(Key::$key => match locale {
                    Locale::ZhCn => $zh_cn,
                    Locale::ZhTw => $zh_tw,
                    Locale::En => $en,
                    Locale::Ja => $ja,
                }),*
            }
        }
    };
}

pub fn tr(key: Key) -> &'static str {
    text(key, locale())
}

messages! {
    AppWindowTitle {
        zh_cn: "AudioScript 声稿",
        zh_tw: "AudioScript 聲稿",
        en: "AudioScript",
        ja: "AudioScript"
    },

    TrayShowMain {
        zh_cn: "显示主窗口",
        zh_tw: "顯示主視窗",
        en: "Show main window",
        ja: "メインウィンドウを表示"
    },
    TraySessionStart {
        zh_cn: "开始转写",
        zh_tw: "開始轉寫",
        en: "Start transcribing",
        ja: "文字起こしを開始"
    },
    TraySessionStop {
        zh_cn: "停止转写",
        zh_tw: "停止轉寫",
        en: "Stop transcribing",
        ja: "文字起こしを停止"
    },
    TrayShowOverlay {
        zh_cn: "显示悬浮窗",
        zh_tw: "顯示懸浮窗",
        en: "Show overlay",
        ja: "オーバーレイを表示"
    },
    TrayHideOverlay {
        zh_cn: "隐藏悬浮窗",
        zh_tw: "隱藏懸浮窗",
        en: "Hide overlay",
        ja: "オーバーレイを隠す"
    },
    TrayLocked {
        zh_cn: "点击穿透（锁定）",
        zh_tw: "點擊穿透（鎖定）",
        en: "Click-through (locked)",
        ja: "クリック透過（ロック）"
    },
    TrayQuit {
        zh_cn: "退出 AudioScript",
        zh_tw: "結束 AudioScript",
        en: "Quit AudioScript",
        ja: "AudioScript を終了"
    },
    TrayTooltip {
        zh_cn: "AudioScript 声稿",
        zh_tw: "AudioScript 聲稿",
        en: "AudioScript",
        ja: "AudioScript"
    },

    ShortcutEmpty {
        zh_cn: "加速键不能为空",
        zh_tw: "快速鍵不能為空",
        en: "Shortcut cannot be empty",
        ja: "ショートカットを空にできません"
    },
    ShortcutUnknownId {
        zh_cn: "未知的热键 id：{0}",
        zh_tw: "未知的快速鍵 id：{0}",
        en: "Unknown shortcut id: {0}",
        ja: "不明なショートカット id：{0}"
    },
    ShortcutParseFailed {
        zh_cn: "加速键 {0} 无法解析：{1}",
        zh_tw: "快速鍵 {0} 無法解析：{1}",
        en: "Cannot parse shortcut {0}: {1}",
        ja: "ショートカット {0} を解析できません：{1}"
    },
    ShortcutRegisterFailed {
        zh_cn: "键位 {0} 注册失败（可能已被其他应用占用）：{1}",
        zh_tw: "鍵位 {0} 註冊失敗（可能已被其他應用程式佔用）：{1}",
        en: "Failed to register {0} (it may be used by another app): {1}",
        ja: "{0} の登録に失敗しました（他のアプリが使用中の可能性があります）：{1}"
    },

    CmdUnknown {
        zh_cn: "未知命令：{0}",
        zh_tw: "未知指令：{0}",
        en: "Unknown command: {0}",
        ja: "不明なコマンド：{0}"
    },
    CmdBadArgs {
        zh_cn: "参数无效：{0}",
        zh_tw: "參數無效：{0}",
        en: "Invalid argument: {0}",
        ja: "引数が無効です：{0}"
    },
    CmdThreadStartFailed {
        zh_cn: "启动后台线程失败：{0}",
        zh_tw: "啟動背景執行緒失敗：{0}",
        en: "Failed to start background thread: {0}",
        ja: "バックグラウンドスレッドの開始に失敗しました：{0}"
    },
    CmdTaskPanicked {
        zh_cn: "后台任务异常退出",
        zh_tw: "背景工作異常結束",
        en: "Background task exited unexpectedly",
        ja: "バックグラウンドタスクが異常終了しました"
    },
    CmdTimeout {
        zh_cn: "后台任务超时，请重试",
        zh_tw: "背景工作逾時，請重試",
        en: "Background task timed out. Try again.",
        ja: "バックグラウンドタスクがタイムアウトしました。再試行してください"
    },
    CmdLaneExited {
        zh_cn: "命令执行线程已退出",
        zh_tw: "指令執行執行緒已結束",
        en: "Command worker has exited",
        ja: "コマンド実行スレッドが終了しました"
    },
    CmdMicDenied {
        zh_cn: "麦克风权限已被拒绝，系统不会再弹窗。请到「通用 → 权限」打开系统设置重新允许。",
        zh_tw: "麥克風權限已被拒絕，系統不會再彈出提示。請到「一般 → 權限」開啟系統設定重新允許。",
        en: "Microphone permission was denied and the system will not prompt again. Open System Settings from General → Permissions to allow it.",
        ja: "マイクの権限が拒否されており、システムから再度確認されません。「一般 → 権限」からシステム設定を開き、許可し直してください。"
    },
    CmdConfigParseFailed {
        zh_cn: "config 解析失败：{0}",
        zh_tw: "config 解析失敗：{0}",
        en: "Failed to parse config: {0}",
        ja: "config の解析に失敗しました：{0}"
    },
    CmdMissingArg {
        zh_cn: "缺少 {0}",
        zh_tw: "缺少 {0}",
        en: "Missing {0}",
        ja: "{0} が指定されていません"
    },
    CmdArgParseFailed {
        zh_cn: "{0} 解析失败：{1}",
        zh_tw: "{0} 解析失敗：{1}",
        en: "Failed to parse {0}: {1}",
        ja: "{0} の解析に失敗しました：{1}"
    },
    CmdNoWritableDir {
        zh_cn: "找不到可写入的位置：{0}",
        zh_tw: "找不到可寫入的位置：{0}",
        en: "No writable location found: {0}",
        ja: "書き込み可能な場所が見つかりません：{0}"
    },
    CmdWriteFailed {
        zh_cn: "写入 {0} 失败：{1}",
        zh_tw: "寫入 {0} 失敗：{1}",
        en: "Failed to write {0}: {1}",
        ja: "{0} への書き込みに失敗しました：{1}"
    },
    CmdOpenFailed {
        zh_cn: "无法打开{0}：{1}",
        zh_tw: "無法開啟{0}：{1}",
        en: "Failed to open {0}: {1}",
        ja: "{0} を開けませんでした：{1}"
    },
    OpenWhatBrowser {
        zh_cn: "浏览器",
        zh_tw: "瀏覽器",
        en: "browser",
        ja: "ブラウザ"
    },
    OpenWhatLink {
        zh_cn: "链接",
        zh_tw: "連結",
        en: "link",
        ja: "リンク"
    },
    OpenWhatModelDir {
        zh_cn: "模型目录",
        zh_tw: "模型目錄",
        en: "model folder",
        ja: "モデルフォルダー"
    },
    OpenWhatFileManager {
        zh_cn: "文件管理器",
        zh_tw: "檔案管理器",
        en: "file manager",
        ja: "ファイルマネージャー"
    },
    CmdPathMissing {
        zh_cn: "路径不存在：{0}",
        zh_tw: "路徑不存在：{0}",
        en: "Path does not exist: {0}",
        ja: "パスが存在しません：{0}"
    },
    CmdCreateDirFailed {
        zh_cn: "创建 {0} 失败：{1}",
        zh_tw: "建立 {0} 失敗：{1}",
        en: "Failed to create {0}: {1}",
        ja: "{0} の作成に失敗しました：{1}"
    },
    CmdReadLogDirFailed {
        zh_cn: "读取日志目录失败：{0}",
        zh_tw: "讀取日誌目錄失敗：{0}",
        en: "Failed to read log folder: {0}",
        ja: "ログフォルダーの読み取りに失敗しました：{0}"
    },
    CmdCopyFailed {
        zh_cn: "复制 {0} 失败：{1}",
        zh_tw: "複製 {0} 失敗：{1}",
        en: "Failed to copy {0}: {1}",
        ja: "{0} のコピーに失敗しました：{1}"
    },
    CmdNoLogs {
        zh_cn: "{0} 中没有日志文件，无可导出",
        zh_tw: "{0} 中沒有日誌檔，無可匯出",
        en: "No log files to export in {0}",
        ja: "{0} にエクスポートできるログファイルがありません"
    },
    CmdLogsFolderName {
        zh_cn: "AudioScript-日志-{0}",
        zh_tw: "AudioScript-日誌-{0}",
        en: "AudioScript-logs-{0}",
        ja: "AudioScript-logs-{0}"
    },
    CmdUnknownModel {
        zh_cn: "未知模型：{0}",
        zh_tw: "未知模型：{0}",
        en: "Unknown model: {0}",
        ja: "不明なモデル：{0}"
    },
    CmdModelNotReady {
        zh_cn: "模型「{0}」尚未安装或文件不完整，请先下载",
        zh_tw: "模型「{0}」尚未安裝或檔案不完整，請先下載",
        en: "Model \"{0}\" is not installed or incomplete. Download it first.",
        ja: "モデル「{0}」が未インストールか不完全です。先にダウンロードしてください"
    },
    ModelAsrBuiltinTitle {
        zh_cn: "内置语音识别（小模型）",
        zh_tw: "內建語音辨識（小模型）",
        en: "Bundled speech recognition (small model)",
        ja: "内蔵音声認識（小型モデル）"
    },
    ModelAsrHighTitle {
        zh_cn: "高精度语音识别",
        zh_tw: "高精度語音辨識",
        en: "High-accuracy speech recognition",
        ja: "高精度音声認識"
    },
    ModelMtBaseTitle {
        zh_cn: "内置机器翻译（基础版）",
        zh_tw: "內建機器翻譯（基礎版）",
        en: "Bundled machine translation (base)",
        ja: "内蔵機械翻訳（ベース）"
    },
    ModelMtBigTitle {
        zh_cn: "大模型机器翻译（高精度）",
        zh_tw: "大型機器翻譯模型（高精度）",
        en: "Large machine translation model (high accuracy)",
        ja: "大型翻訳モデル（高精度）"
    },
    CmdUnknownAsrTier {
        zh_cn: "未知 ASR 档位：{0}",
        zh_tw: "未知 ASR 等級：{0}",
        en: "Unknown ASR tier: {0}",
        ja: "不明な ASR ティア：{0}"
    },
    CmdUnknownMtTier {
        zh_cn: "未知 MT 档位：{0}",
        zh_tw: "未知 MT 等級：{0}",
        en: "Unknown MT tier: {0}",
        ja: "不明な MT ティア：{0}"
    },
    CmdUnknownMode {
        zh_cn: "未知模式 {0}",
        zh_tw: "未知模式 {0}",
        en: "Unknown mode {0}",
        ja: "不明なモード {0}"
    },
    CmdUnknownPane {
        zh_cn: "未知设置页 {0}",
        zh_tw: "未知設定頁 {0}",
        en: "Unknown settings pane {0}",
        ja: "不明な設定ページ {0}"
    },
    CmdLinkRejected {
        zh_cn: "拒绝打开非项目链接：{0}",
        zh_tw: "拒絕開啟非專案連結：{0}",
        en: "Refusing to open a non-project link: {0}",
        ja: "プロジェクト外のリンクは開けません：{0}"
    },
    CmdMissingEnabled {
        zh_cn: "缺少 enabled",
        zh_tw: "缺少 enabled",
        en: "Missing enabled",
        ja: "enabled が指定されていません"
    },
    CmdAutostartFailed {
        zh_cn: "设置开机自启失败：{0}",
        zh_tw: "設定開機自動啟動失敗：{0}",
        en: "Failed to change launch-at-login: {0}",
        ja: "ログイン時の起動設定に失敗しました：{0}"
    },
    CmdProbeWhileCapturing {
        zh_cn: "正在转写中，请先停止再检测",
        zh_tw: "正在轉寫中，請先停止再偵測",
        en: "Transcription is running. Stop it before probing.",
        ja: "文字起こし中です。停止してから検出してください"
    },

    ModelUnknown {
        zh_cn: "未知模型：{0}",
        zh_tw: "未知模型：{0}",
        en: "Unknown model: {0}",
        ja: "不明なモデル：{0}"
    },
    ModelNotDownloadable {
        zh_cn: "模型 {0} 暂不提供下载",
        zh_tw: "模型 {0} 暫不提供下載",
        en: "Model {0} is not available for download",
        ja: "モデル {0} はダウンロードできません"
    },
    ModelAlreadyInstalled {
        zh_cn: "模型 {0} 已安装，如需修复请先删除",
        zh_tw: "模型 {0} 已安裝，如需修復請先刪除",
        en: "Model {0} is already installed. Delete it first to repair.",
        ja: "モデル {0} はインストール済みです。修復するには先に削除してください"
    },
    ModelBundled {
        zh_cn: "内置模型随应用分发，不能删除",
        zh_tw: "內建模型隨應用程式發行，不能刪除",
        en: "Bundled models ship with the app and cannot be deleted",
        ja: "内蔵モデルはアプリに含まれているため削除できません"
    },
    ModelSelected {
        zh_cn: "该模型当前被选中，请先切换到其他模型再删除",
        zh_tw: "該模型目前被選中，請先切換到其他模型再刪除",
        en: "This model is selected. Switch to another model before deleting.",
        ja: "このモデルは選択中です。別のモデルに切り替えてから削除してください"
    },
    ModelInUse {
        zh_cn: "正在转写，请先停止再删除模型",
        zh_tw: "正在轉寫，請先停止再刪除模型",
        en: "Transcription is running. Stop it before deleting a model.",
        ja: "文字起こし中です。停止してからモデルを削除してください"
    },
    ModelJobInProgress {
        zh_cn: "已有模型下载任务进行中",
        zh_tw: "已有模型下載工作進行中",
        en: "A model download is already in progress",
        ja: "モデルのダウンロードが進行中です"
    },
    ModelDownloadFailed {
        zh_cn: "下载失败：{0}",
        zh_tw: "下載失敗：{0}",
        en: "Download failed: {0}",
        ja: "ダウンロードに失敗しました：{0}"
    },
    ModelHttpFailed {
        zh_cn: "下载失败：HTTP {0}",
        zh_tw: "下載失敗：HTTP {0}",
        en: "Download failed: HTTP {0}",
        ja: "ダウンロードに失敗しました：HTTP {0}"
    },
    ModelChecksumFailed {
        zh_cn: "校验失败：期望 {0}，实际 {1}",
        zh_tw: "校驗失敗：預期 {0}，實際 {1}",
        en: "Checksum mismatch: expected {0}, got {1}",
        ja: "検証に失敗しました：期待値 {0}、実際 {1}"
    },
    ModelUnsafeArchive {
        zh_cn: "归档内容不安全：{0}",
        zh_tw: "封存內容不安全：{0}",
        en: "Unsafe archive contents: {0}",
        ja: "安全でないアーカイブ内容：{0}"
    },
    ModelExtractFailed {
        zh_cn: "解压失败：{0}",
        zh_tw: "解壓縮失敗：{0}",
        en: "Extraction failed: {0}",
        ja: "展開に失敗しました：{0}"
    },
    ModelDiskFull {
        zh_cn: "磁盘空间不足：需要约 {0} MB，可用约 {1} MB",
        zh_tw: "磁碟空間不足：需要約 {0} MB，可用約 {1} MB",
        en: "Not enough disk space: about {0} MB needed, {1} MB available",
        ja: "ディスク容量が不足しています：約 {0} MB 必要、空き約 {1} MB"
    },
    ModelIncomplete {
        zh_cn: "模型文件不完整：{0}",
        zh_tw: "模型檔案不完整：{0}",
        en: "Model files are incomplete: {0}",
        ja: "モデルファイルが不完全です：{0}"
    },
    ModelFileOpFailed {
        zh_cn: "文件操作失败：{0}",
        zh_tw: "檔案操作失敗：{0}",
        en: "File operation failed: {0}",
        ja: "ファイル操作に失敗しました：{0}"
    },
    ModelCancelled {
        zh_cn: "下载已取消",
        zh_tw: "下載已取消",
        en: "Download cancelled",
        ja: "ダウンロードをキャンセルしました"
    },
    ModelNoDownloadInfo {
        zh_cn: "{0} 没有下载信息",
        zh_tw: "{0} 沒有下載資訊",
        en: "{0} has no download information",
        ja: "{0} にダウンロード情報がありません"
    },
    ModelNoChecksumInfo {
        zh_cn: "{0} 的下载校验信息尚未配置",
        zh_tw: "{0} 的下載校驗資訊尚未設定",
        en: "Download checksum for {0} is not configured",
        ja: "{0} のダウンロード検証情報が未設定です"
    },
    ModelThreadPanicked {
        zh_cn: "安装线程异常退出",
        zh_tw: "安裝執行緒異常結束",
        en: "Installer thread exited unexpectedly",
        ja: "インストールスレッドが異常終了しました"
    },
    ModelThreadStartFailed {
        zh_cn: "启动安装线程失败：{0}",
        zh_tw: "啟動安裝執行緒失敗：{0}",
        en: "Failed to start installer thread: {0}",
        ja: "インストールスレッドの開始に失敗しました：{0}"
    },
    ModelNoDataDir {
        zh_cn: "无法解析应用数据目录",
        zh_tw: "無法解析應用程式資料目錄",
        en: "Cannot resolve the app data folder",
        ja: "アプリのデータフォルダーを解決できません"
    },
    ModelReplaceFailed {
        zh_cn: "无法替换已存在的模型目录（可能文件被占用）：{0}：{1}",
        zh_tw: "無法取代已存在的模型目錄（可能檔案被佔用）：{0}：{1}",
        en: "Cannot replace the existing model folder (files may be in use): {0}: {1}",
        ja: "既存のモデルフォルダーを置き換えられません（ファイルが使用中の可能性）：{0}：{1}"
    },
    ModelInstallFailed {
        zh_cn: "安装模型失败（可能文件被占用）：{0}：{1}",
        zh_tw: "安裝模型失敗（可能檔案被佔用）：{0}：{1}",
        en: "Model installation failed (files may be in use): {0}: {1}",
        ja: "モデルのインストールに失敗しました（ファイルが使用中の可能性）：{0}：{1}"
    },
    ModelNoArchiveInfo {
        zh_cn: "{0} 没有归档信息",
        zh_tw: "{0} 沒有封存資訊",
        en: "{0} has no archive information",
        ja: "{0} にアーカイブ情報がありません"
    },
    ModelUnsupportedEntry {
        zh_cn: "不支持的归档条目类型：{0}",
        zh_tw: "不支援的封存項目類型：{0}",
        en: "Unsupported archive entry type: {0}",
        ja: "未対応のアーカイブ項目タイプ：{0}"
    },
    ModelUnsafePath {
        zh_cn: "不安全的路径：{0}",
        zh_tw: "不安全的路徑：{0}",
        en: "Unsafe path: {0}",
        ja: "安全でないパス：{0}"
    },
    ModelTooLarge {
        zh_cn: "解压后体积超过上限（{0} 字节）",
        zh_tw: "解壓縮後大小超過上限（{0} 位元組）",
        en: "Extracted size exceeds the limit ({0} bytes)",
        ja: "展開後のサイズが上限を超えています（{0} バイト）"
    },
    ModelDownloadTooLarge {
        zh_cn: "服务器返回的体积超过预期（已下载 {0} 字节）",
        zh_tw: "伺服器傳回的大小超過預期（已下載 {0} 位元組）",
        en: "The server sent more data than expected ({0} bytes downloaded)",
        ja: "サーバーから返されたサイズが想定を超えています（{0} バイト受信）"
    },

    AsrNotInitialized {
        zh_cn: "引擎未初始化",
        zh_tw: "引擎未初始化",
        en: "Engine is not initialized",
        ja: "エンジンが初期化されていません"
    },
    AsrInvalidConfig {
        zh_cn: "配置无效：{0}",
        zh_tw: "設定無效：{0}",
        en: "Invalid configuration: {0}",
        ja: "設定が無効です：{0}"
    },
    AsrModelLoadFailed {
        zh_cn: "模型加载失败：{0}",
        zh_tw: "模型載入失敗：{0}",
        en: "Failed to load model: {0}",
        ja: "モデルの読み込みに失敗しました：{0}"
    },
    AsrModelDirMissing {
        zh_cn: "模型目录缺少文件：{0}",
        zh_tw: "模型目錄缺少檔案：{0}",
        en: "Model folder is missing files: {0}",
        ja: "モデルフォルダーにファイルがありません：{0}"
    },
    AsrReadModelDirFailed {
        zh_cn: "无法读取模型目录 {0}：{1}",
        zh_tw: "無法讀取模型目錄 {0}：{1}",
        en: "Cannot read model folder {0}: {1}",
        ja: "モデルフォルダー {0} を読み取れません：{1}"
    },
    AsrMissingFile {
        zh_cn: "缺少模型文件：{0}\n目录实际内容：{1}",
        zh_tw: "缺少模型檔案：{0}\n目錄實際內容：{1}",
        en: "Missing model file: {0}\nFolder contents: {1}",
        ja: "モデルファイルがありません：{0}\nフォルダーの内容：{1}"
    },
    AsrMissingPattern {
        zh_cn: "缺少模型文件：{0}/{1}*.onnx\n目录实际内容：{2}",
        zh_tw: "缺少模型檔案：{0}/{1}*.onnx\n目錄實際內容：{2}",
        en: "Missing model file: {0}/{1}*.onnx\nFolder contents: {2}",
        ja: "モデルファイルがありません：{0}/{1}*.onnx\nフォルダーの内容：{2}"
    },
    AsrEmptyFolder {
        zh_cn: "（空）",
        zh_tw: "（空）",
        en: "(empty)",
        ja: "（空）"
    },
    AsrModelDirNotSet {
        zh_cn: "model_dir 未设置",
        zh_tw: "model_dir 未設定",
        en: "model_dir is not set",
        ja: "model_dir が設定されていません"
    },
    AsrRecognizerNone {
        zh_cn: "OnlineRecognizer::create 返回 None（模型目录 {0}）",
        zh_tw: "OnlineRecognizer::create 傳回 None（模型目錄 {0}）",
        en: "OnlineRecognizer::create returned None (model folder {0})",
        ja: "OnlineRecognizer::create が None を返しました（モデルフォルダー {0}）"
    },

    TranslateNotInitialized {
        zh_cn: "翻译引擎未初始化",
        zh_tw: "翻譯引擎未初始化",
        en: "Translation engine is not initialized",
        ja: "翻訳エンジンが初期化されていません"
    },
    TranslateModelLoadFailed {
        zh_cn: "模型加载失败：{0}",
        zh_tw: "模型載入失敗：{0}",
        en: "Failed to load model: {0}",
        ja: "モデルの読み込みに失敗しました：{0}"
    },
    TranslateModelDirMissing {
        zh_cn: "模型目录缺少文件：{0}",
        zh_tw: "模型目錄缺少檔案：{0}",
        en: "Model folder is missing files: {0}",
        ja: "モデルフォルダーにファイルがありません：{0}"
    },
    TranslateUnsupportedPair {
        zh_cn: "语言对不支持：{0} → {1}",
        zh_tw: "語言對不支援：{0} → {1}",
        en: "Unsupported language pair: {0} → {1}",
        ja: "未対応の言語ペア：{0} → {1}"
    },
    TranslateInvalidOutput {
        zh_cn: "译文无效（{0}）：翻译方向可能设置错误",
        zh_tw: "譯文無效（{0}）：翻譯方向可能設定錯誤",
        en: "Invalid translation ({0}): the direction may be set incorrectly",
        ja: "翻訳結果が無効です（{0}）：翻訳方向の設定が誤っている可能性があります"
    },
    TranslateInternal {
        zh_cn: "引擎内部错误：{0}",
        zh_tw: "引擎內部錯誤：{0}",
        en: "Internal engine error: {0}",
        ja: "エンジン内部エラー：{0}"
    },
    TranslateAutoUnsupported {
        zh_cn: "auto（尚未实现，请显式指定源语言）",
        zh_tw: "auto（尚未實作，請明確指定來源語言）",
        en: "auto (not supported yet; specify the source language explicitly)",
        ja: "auto（未対応です。元の言語を明示してください）"
    },
    TranslateRemedyFetchMt {
        zh_cn: "→ 运行 `pnpm fetch:mt` 补齐内置翻译模型",
        zh_tw: "→ 執行 `pnpm fetch:mt` 補齊內建翻譯模型",
        en: "→ Run `pnpm fetch:mt` to install the bundled translation model",
        ja: "→ `pnpm fetch:mt` を実行して内蔵翻訳モデルを追加してください"
    },
    TranslateRemedyBig {
        zh_cn: "→ 打开「模型」页下载大模型，或切换回基础模型",
        zh_tw: "→ 開啟「模型」頁下載大型模型，或切換回基礎模型",
        en: "→ Download the large model on the Models page, or switch back to the base model",
        ja: "→ 「モデル」ページで大型モデルをダウンロードするか、ベースモデルに戻してください"
    },
    TranslatePathMissing {
        zh_cn: "{0} 不存在。\n{1}",
        zh_tw: "{0} 不存在。\n{1}",
        en: "{0} does not exist.\n{1}",
        ja: "{0} が存在しません。\n{1}"
    },
    TranslateFileMissing {
        zh_cn: "{0}/{1} 不存在。\n{2}",
        zh_tw: "{0}/{1} 不存在。\n{2}",
        en: "{0}/{1} does not exist.\n{2}",
        ja: "{0}/{1} が存在しません。\n{2}"
    },
    TranslateEmptyResult {
        zh_cn: "引擎返回了空结果",
        zh_tw: "引擎傳回了空結果",
        en: "The engine returned an empty result",
        ja: "エンジンが空の結果を返しました"
    },
    TranslateSessionModelMissing {
        zh_cn: "所选翻译模型「{0}」未安装或文件不完整：{1}\n缺少：{2}\n→ 打开「模型」页下载，或切换回基础模型",
        zh_tw: "所選翻譯模型「{0}」未安裝或檔案不完整：{1}\n缺少：{2}\n→ 開啟「模型」頁下載，或切換回基礎模型",
        en: "Selected translation model \"{0}\" is not installed or incomplete: {1}\nMissing: {2}\n→ Download it on the Models page, or switch back to the base model",
        ja: "選択した翻訳モデル「{0}」が未インストールか不完全です：{1}\n不足：{2}\n→ 「モデル」ページでダウンロードするか、ベースモデルに戻してください"
    },
    TranslateEngineNotWired {
        zh_cn: "翻译引擎 {0} 尚未接入。可选值：local-nmt（本地离线）、mock（开发用）、off（仅显示原文）",
        zh_tw: "翻譯引擎 {0} 尚未接入。可選值：local-nmt（本機離線）、mock（開發用）、off（僅顯示原文）",
        en: "Translation engine {0} is not available. Options: local-nmt (offline), mock (development), off (source text only)",
        ja: "翻訳エンジン {0} は未対応です。選択肢：local-nmt（ローカル）、mock（開発用）、off（原文のみ）"
    },

    PathNoAppData {
        zh_cn: "无法解析应用数据目录，不能保存下载的模型",
        zh_tw: "無法解析應用程式資料目錄，不能儲存下載的模型",
        en: "Cannot resolve the app data folder; downloaded models cannot be saved",
        ja: "アプリのデータフォルダーを解決できないため、モデルを保存できません"
    },
    PathCreateFailed {
        zh_cn: "创建 {0} 失败：{1}",
        zh_tw: "建立 {0} 失敗：{1}",
        en: "Failed to create {0}: {1}",
        ja: "{0} の作成に失敗しました：{1}"
    },
    PathNotFound {
        zh_cn: "找不到{0}。已尝试：\n  - {1}\n\n{2}",
        zh_tw: "找不到{0}。已嘗試：\n  - {1}\n\n{2}",
        en: "Cannot find {0}. Tried:\n  - {1}\n\n{2}",
        ja: "{0} が見つかりません。試行した場所：\n  - {1}\n\n{2}"
    },
    PathNoCandidate {
        zh_cn: "没有可用的候选目录",
        zh_tw: "沒有可用的候選目錄",
        en: "No candidate folder is available",
        ja: "利用可能な候補フォルダーがありません"
    },
    PathNotExist {
        zh_cn: "{0} （不存在）",
        zh_tw: "{0} （不存在）",
        en: "{0} (does not exist)",
        ja: "{0}（存在しません）"
    },
    PathAsrLabel {
        zh_cn: "语音识别模型「{0}」",
        zh_tw: "語音辨識模型「{0}」",
        en: "speech recognition model \"{0}\"",
        ja: "音声認識モデル「{0}」"
    },
    PathMtLabel {
        zh_cn: "翻译模型「{0}」",
        zh_tw: "翻譯模型「{0}」",
        en: "translation model \"{0}\"",
        ja: "翻訳モデル「{0}」"
    },
    PathReadFailed {
        zh_cn: "无法读取 {0}：{1}",
        zh_tw: "無法讀取 {0}：{1}",
        en: "Cannot read {0}: {1}",
        ja: "{0} を読み取れません：{1}"
    },
    PathCatalogAsr {
        zh_cn: "catalog 缺少 {0} 的 ASR 文件要求",
        zh_tw: "catalog 缺少 {0} 的 ASR 檔案要求",
        en: "catalog is missing ASR file requirements for {0}",
        ja: "catalog に {0} の ASR ファイル要件がありません"
    },
    PathCatalogMt {
        zh_cn: "catalog 缺少 {0} 的 MT 文件要求",
        zh_tw: "catalog 缺少 {0} 的 MT 檔案要求",
        en: "catalog is missing MT file requirements for {0}",
        ja: "catalog に {0} の MT ファイル要件がありません"
    },
    PathCatalogDirections {
        zh_cn: "catalog 未给 {0} 配置方向目录",
        zh_tw: "catalog 未為 {0} 設定方向目錄",
        en: "catalog has no direction folders for {0}",
        ja: "catalog に {0} の方向フォルダー設定がありません"
    },
    PathDirMissing {
        zh_cn: "模型目录 {0} 缺少：{1}\n实际内容：{2}",
        zh_tw: "模型目錄 {0} 缺少：{1}\n實際內容：{2}",
        en: "Model folder {0} is missing: {1}\nContents: {2}",
        ja: "モデルフォルダー {0} に不足：{1}\n実際の内容：{2}"
    },
    PathBundledRemedyAsr {
        zh_cn: "→ 内置模型文件缺失：请重新安装应用；开发环境运行 `pnpm install`（或 `pnpm fetch:model`）后重启。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 內建模型檔案缺失：請重新安裝應用程式；開發環境請執行 `pnpm install`（或 `pnpm fetch:model`）後重新啟動。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Bundled model files are missing: reinstall the app; in development run `pnpm install` (or `pnpm fetch:model`) and restart.\n({0} is also searched first when set.)",
        ja: "→ 内蔵モデルファイルがありません：アプリを再インストールしてください。開発環境では `pnpm install`（または `pnpm fetch:model`）を実行して再起動してください。\n（{0} が設定されている場合も優先的に検索されます。）"
    },
    PathBundledRemedyMt {
        zh_cn: "→ 内置模型文件缺失：请重新安装应用；开发环境运行 `pnpm fetch:mt` 后重启。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 內建模型檔案缺失：請重新安裝應用程式；開發環境請執行 `pnpm fetch:mt` 後重新啟動。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Bundled model files are missing: reinstall the app; in development run `pnpm fetch:mt` and restart.\n({0} is also searched first when set.)",
        ja: "→ 内蔵モデルファイルがありません：アプリを再インストールしてください。開発環境では `pnpm fetch:mt` を実行して再起動してください。\n（{0} が設定されている場合も優先的に検索されます。）"
    },
    PathOptionalRemedyAsr {
        zh_cn: "→ 打开「模型」页下载该模型，或切换回内置模型。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 開啟「模型」頁下載該模型，或切換回內建模型。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Download this model on the Models page, or switch back to the bundled model.\n({0} is also searched first when set.)",
        ja: "→ 「モデル」ページでこのモデルをダウンロードするか、内蔵モデルに戻してください。\n（{0} が設定されている場合も優先的に検索されます。）"
    },
    PathOptionalRemedyMt {
        zh_cn: "→ 打开「模型」页下载该模型，或切换回基础模型。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 開啟「模型」頁下載該模型，或切換回基礎模型。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Download this model on the Models page, or switch back to the base model.\n({0} is also searched first when set.)",
        ja: "→ 「モデル」ページでこのモデルをダウンロードするか、ベースモデルに戻してください。\n（{0} が設定されている場合も優先的に検索されます。）"
    },

    SessionAlreadyCapturing {
        zh_cn: "已在采集中",
        zh_tw: "已在擷取中",
        en: "Already capturing",
        ja: "すでにキャプチャ中です"
    },
    SessionTeardownBusy {
        zh_cn: "上一次会话仍在收尾（音频设备尚未释放），请稍后重试",
        zh_tw: "上一次工作階段仍在收尾（音訊裝置尚未釋放），請稍後重試",
        en: "The previous session is still shutting down (audio device not released yet). Try again shortly.",
        ja: "前回のセッションがまだ終了処理中です（音声デバイスが未解放）。しばらくしてから再試行してください"
    },
    SessionEngineNotWired {
        zh_cn: "引擎 {0} 尚未接入（当前仅支持 local-streaming）",
        zh_tw: "引擎 {0} 尚未接入（目前僅支援 local-streaming）",
        en: "Engine {0} is not available (only local-streaming is supported)",
        ja: "エンジン {0} は未対応です（現在は local-streaming のみ対応）"
    },
    SessionCustomAsrMissing {
        zh_cn: "自定义 ASR 模型目录不存在：{0}",
        zh_tw: "自訂 ASR 模型目錄不存在：{0}",
        en: "Custom ASR model folder does not exist: {0}",
        ja: "カスタム ASR モデルフォルダーが存在しません：{0}"
    },
    SessionCustomAsrIncomplete {
        zh_cn: "自定义 ASR 模型目录不完整：{0}\n缺少：{1}",
        zh_tw: "自訂 ASR 模型目錄不完整：{0}\n缺少：{1}",
        en: "Custom ASR model folder is incomplete: {0}\nMissing: {1}",
        ja: "カスタム ASR モデルフォルダーが不完全です：{0}\n不足：{1}"
    },
    SessionAsrModelIncomplete {
        zh_cn: "所选语音识别模型「{0}」未安装或文件不完整：{1}\n缺少：{2}\n→ 打开「模型」页下载，或切换回内置模型",
        zh_tw: "所選語音辨識模型「{0}」未安裝或檔案不完整：{1}\n缺少：{2}\n→ 開啟「模型」頁下載，或切換回內建模型",
        en: "Selected speech recognition model \"{0}\" is not installed or incomplete: {1}\nMissing: {2}\n→ Download it on the Models page, or switch back to the bundled model",
        ja: "選択した音声認識モデル「{0}」が未インストールか不完全です：{1}\n不足：{2}\n→ 「モデル」ページでダウンロードするか、内蔵モデルに戻してください"
    },
    SessionAsrInitFailed {
        zh_cn: "引擎初始化失败：{0}",
        zh_tw: "引擎初始化失敗：{0}",
        en: "Failed to initialize engine: {0}",
        ja: "エンジンの初期化に失敗しました：{0}"
    },
    SessionTranslateThreadMissing {
        zh_cn: "翻译线程未初始化",
        zh_tw: "翻譯執行緒未初始化",
        en: "Translation worker is not initialized",
        ja: "翻訳スレッドが初期化されていません"
    },
    SessionTranslateThreadExited {
        zh_cn: "翻译线程已退出",
        zh_tw: "翻譯執行緒已結束",
        en: "Translation worker has exited",
        ja: "翻訳スレッドが終了しました"
    },
    SessionTranslateInitFailed {
        zh_cn: "翻译引擎初始化失败：{0}",
        zh_tw: "翻譯引擎初始化失敗：{0}",
        en: "Failed to initialize the translation engine: {0}",
        ja: "翻訳エンジンの初期化に失敗しました：{0}"
    },
    SessionStartCaptureFailed {
        zh_cn: "启动采集失败：{0}",
        zh_tw: "啟動擷取失敗：{0}",
        en: "Failed to start capture: {0}",
        ja: "キャプチャの開始に失敗しました：{0}"
    },
    SessionEventThreadMissing {
        zh_cn: "事件线程未初始化",
        zh_tw: "事件執行緒未初始化",
        en: "Event worker is not initialized",
        ja: "イベントスレッドが初期化されていません"
    },
    SessionEventThreadStartFailed {
        zh_cn: "启动事件线程失败：{0}",
        zh_tw: "啟動事件執行緒失敗：{0}",
        en: "Failed to start event worker: {0}",
        ja: "イベントスレッドの開始に失敗しました：{0}"
    },
    SessionTranslateThreadStartFailed {
        zh_cn: "启动翻译线程失败：{0}",
        zh_tw: "啟動翻譯執行緒失敗：{0}",
        en: "Failed to start translation worker: {0}",
        ja: "翻訳スレッドの開始に失敗しました：{0}"
    },
    SessionTeardownThreadStartFailed {
        zh_cn: "启动收尾线程失败：{0}",
        zh_tw: "啟動收尾執行緒失敗：{0}",
        en: "Failed to start teardown worker: {0}",
        ja: "終了処理スレッドの開始に失敗しました：{0}"
    },

    PermissionProbing {
        zh_cn: "检测已在进行中",
        zh_tw: "偵測已在進行中",
        en: "A probe is already running",
        ja: "検出はすでに実行中です"
    },
    PermissionNoIo {
        zh_cn: "收到了音频帧，但没有任何进程在输出 I/O。请先播放一段音频再检测。",
        zh_tw: "收到了音訊影格，但沒有任何處理程序在輸出 I/O。請先播放一段音訊再偵測。",
        en: "Audio frames were received, but no process is producing output. Play some audio, then probe again.",
        ja: "音声フレームは受信しましたが、出力中のプロセスがありません。音声を再生してから再検出してください"
    },
    PermissionPromptUnhandled {
        zh_cn: "整个检测窗口没有收到任何音频帧，通常意味着系统授权弹窗还停在屏幕上没被处理。请允许后再点一次「检测」。",
        zh_tw: "整個偵測視窗沒有收到任何音訊影格，通常表示系統授權提示還停在畫面上未被處理。請允許後再點一次「偵測」。",
        en: "No audio frames arrived during the probe window. The system permission prompt is probably still on screen. Allow it, then probe again.",
        ja: "検出中に音声フレームが届きませんでした。システムの許可ダイアログが画面に残っている可能性があります。許可してからもう一度「検出」を押してください"
    },
    PermissionUnbundled {
        zh_cn: "整个检测窗口没有收到任何音频帧。当前运行的不是打包后的 .app（Info.plist 里没有 NSAudioCaptureUsageDescription），系统既不会弹窗也不授权——请用 `pnpm build:app` 构建后从 .app 启动。",
        zh_tw: "整個偵測視窗沒有收到任何音訊影格。目前執行的不是封裝後的 .app（Info.plist 裡沒有 NSAudioCaptureUsageDescription），系統既不會彈出提示也不會授權——請用 `pnpm build:app` 建置後從 .app 啟動。",
        en: "No audio frames arrived during the probe window. This is not a packaged .app (Info.plist has no NSAudioCaptureUsageDescription), so the system neither prompts nor grants access. Build with `pnpm build:app` and launch from the .app.",
        ja: "検出中に音声フレームが届きませんでした。現在実行中のものはパッケージ済みの .app ではないため（Info.plist に NSAudioCaptureUsageDescription がありません）、システムは確認も許可も行いません。`pnpm build:app` でビルドし、.app から起動してください"
    },
    PermissionMicUnbundled {
        zh_cn: "当前运行的不是打包后的 .app，Info.plist 里没有 NSMicrophoneUsageDescription，系统不会弹窗（此时请求授权会直接终止进程）。请用 `pnpm build:app` 构建后从 .app 启动。",
        zh_tw: "目前執行的不是封裝後的 .app，Info.plist 裡沒有 NSMicrophoneUsageDescription，系統不會彈出提示（此時要求授權會直接終止處理程序）。請用 `pnpm build:app` 建置後從 .app 啟動。",
        en: "This is not a packaged .app and Info.plist has no NSMicrophoneUsageDescription, so the system will not prompt (requesting permission would terminate the process). Build with `pnpm build:app` and launch from the .app.",
        ja: "現在実行中のものはパッケージ済みの .app ではなく、Info.plist に NSMicrophoneUsageDescription がないため、システムは確認しません（この状態で権限を要求するとプロセスが終了します）。`pnpm build:app` でビルドし、.app から起動してください"
    },
    PermissionRequestInFlight {
        zh_cn: "授权请求已在进行中，请等待系统弹窗",
        zh_tw: "授權要求已在進行中，請等待系統提示",
        en: "A permission request is already in progress. Wait for the system prompt.",
        ja: "権限要求が進行中です。システムの確認を待ってください"
    },
    PermissionCallbackDisconnected {
        zh_cn: "授权回调通道意外断开",
        zh_tw: "授權回呼通道意外中斷",
        en: "The permission callback channel disconnected unexpectedly",
        ja: "権限コールバックのチャネルが予期せず切断されました"
    },

    AudioDeviceNotFound {
        zh_cn: "未找到名为 {0} 的音频设备",
        zh_tw: "找不到名為 {0} 的音訊裝置",
        en: "Audio device \"{0}\" was not found",
        ja: "「{0}」という名前の音声デバイスが見つかりません"
    },
    AudioOpenFailed {
        zh_cn: "打开采集失败：{0}",
        zh_tw: "開啟擷取失敗：{0}",
        en: "Failed to open capture: {0}",
        ja: "キャプチャを開けませんでした：{0}"
    },
    AudioStartFailed {
        zh_cn: "启动采集失败：{0}",
        zh_tw: "啟動擷取失敗：{0}",
        en: "Failed to start capture: {0}",
        ja: "キャプチャの開始に失敗しました：{0}"
    },
    AudioEnumerateFailed {
        zh_cn: "设备枚举失败：{0}",
        zh_tw: "裝置列舉失敗：{0}",
        en: "Failed to enumerate devices: {0}",
        ja: "デバイスの列挙に失敗しました：{0}"
    },
    AudioDefaultDevice {
        zh_cn: "(系统默认)",
        zh_tw: "(系統預設)",
        en: "(system default)",
        ja: "(システム既定)"
    },
    AudioAccumulatorBusy {
        zh_cn: "检测累加器仍被占用",
        zh_tw: "偵測累加器仍被佔用",
        en: "The probe accumulator is still in use",
        ja: "検出アキュムレータがまだ使用中です"
    },

    UpdateNotReady {
        zh_cn: "更新组件未就绪：{0}",
        zh_tw: "更新元件未就緒：{0}",
        en: "Updater is not ready: {0}",
        ja: "アップデーターの準備ができていません：{0}"
    },
    UpdateCheckFailed {
        zh_cn: "检查更新失败：{0}",
        zh_tw: "檢查更新失敗：{0}",
        en: "Failed to check for updates: {0}",
        ja: "アップデートの確認に失敗しました：{0}"
    },
    UpdateInProgress {
        zh_cn: "正在更新，请稍候",
        zh_tw: "正在更新，請稍候",
        en: "An update is already in progress. Please wait.",
        ja: "更新中です。しばらくお待ちください"
    },
    UpdateLatest {
        zh_cn: "已是最新版本，无需更新",
        zh_tw: "已是最新版本，無需更新",
        en: "You're on the latest version",
        ja: "最新バージョンです。更新は不要です"
    },
    UpdateInstallFailed {
        zh_cn: "安装更新失败：{0}",
        zh_tw: "安裝更新失敗：{0}",
        en: "Failed to install the update: {0}",
        ja: "アップデートのインストールに失敗しました：{0}"
    },

    OverlayNotFound {
        zh_cn: "悬浮窗（label={0}）不存在",
        zh_tw: "懸浮窗（label={0}）不存在",
        en: "Overlay window (label={0}) does not exist",
        ja: "オーバーレイウィンドウ（label={0}）が存在しません"
    },
    OverlayShowFailed {
        zh_cn: "显示悬浮窗失败：{0}",
        zh_tw: "顯示懸浮窗失敗：{0}",
        en: "Failed to show the overlay: {0}",
        ja: "オーバーレイの表示に失敗しました：{0}"
    },
    OverlayHideFailed {
        zh_cn: "隐藏悬浮窗失败：{0}",
        zh_tw: "隱藏懸浮窗失敗：{0}",
        en: "Failed to hide the overlay: {0}",
        ja: "オーバーレイの非表示に失敗しました：{0}"
    },
    OverlayClickThroughFailed {
        zh_cn: "设置鼠标穿透失败：{0}",
        zh_tw: "設定滑鼠穿透失敗：{0}",
        en: "Failed to set click-through: {0}",
        ja: "クリック透過の設定に失敗しました：{0}"
    },
    OverlayWindowUnavailable {
        zh_cn: "悬浮窗底层窗口不可用",
        zh_tw: "懸浮窗底層視窗不可用",
        en: "The overlay's native window is unavailable",
        ja: "オーバーレイのネイティブウィンドウが利用できません"
    },
    OverlaySizeFailed {
        zh_cn: "设置悬浮窗尺寸失败：{0}",
        zh_tw: "設定懸浮窗尺寸失敗：{0}",
        en: "Failed to resize the overlay: {0}",
        ja: "オーバーレイのサイズ設定に失敗しました：{0}"
    },
    OverlayPositionFailed {
        zh_cn: "设置悬浮窗位置失败：{0}",
        zh_tw: "設定懸浮窗位置失敗：{0}",
        en: "Failed to move the overlay: {0}",
        ja: "オーバーレイの位置設定に失敗しました：{0}"
    },
    OverlayNoTrayIcon {
        zh_cn: "没有可用的托盘图标",
        zh_tw: "沒有可用的托盤圖示",
        en: "No tray icon is available",
        ja: "利用可能なトレイアイコンがありません"
    },

    PrefsNoConfigDir {
        zh_cn: "无法解析配置目录：{0}",
        zh_tw: "無法解析設定目錄：{0}",
        en: "Cannot resolve the config folder: {0}",
        ja: "設定フォルダーを解決できません：{0}"
    },
    PrefsNoParent {
        zh_cn: "{0} 没有父目录，无法创建",
        zh_tw: "{0} 沒有父目錄，無法建立",
        en: "{0} has no parent folder and cannot be created",
        ja: "{0} に親フォルダーがなく作成できません"
    },
    PrefsCreateDirFailed {
        zh_cn: "创建配置目录失败：{0}",
        zh_tw: "建立設定目錄失敗：{0}",
        en: "Failed to create the config folder: {0}",
        ja: "設定フォルダーの作成に失敗しました：{0}"
    },
    PrefsSerializeFailed {
        zh_cn: "偏好序列化失败：{0}",
        zh_tw: "偏好序列化失敗：{0}",
        en: "Failed to serialize preferences: {0}",
        ja: "設定のシリアル化に失敗しました：{0}"
    },
    PrefsWriteTempFailed {
        zh_cn: "写入临时偏好文件失败：{0}",
        zh_tw: "寫入暫存偏好檔案失敗：{0}",
        en: "Failed to write the temporary preferences file: {0}",
        ja: "一時設定ファイルの書き込みに失敗しました：{0}"
    },
    PrefsReplaceFailed {
        zh_cn: "替换偏好文件失败：{0}",
        zh_tw: "取代偏好檔案失敗：{0}",
        en: "Failed to replace the preferences file: {0}",
        ja: "設定ファイルの置き換えに失敗しました：{0}"
    },

    ExportTitle {
        zh_cn: "# AudioScript 转写记录",
        zh_tw: "# AudioScript 轉寫紀錄",
        en: "# AudioScript Transcript",
        ja: "# AudioScript 文字起こし"
    },
    ExportTime {
        zh_cn: "- 导出时间：{0}",
        zh_tw: "- 匯出時間：{0}",
        en: "- Exported at: {0}",
        ja: "- エクスポート日時：{0}"
    },
    ExportDirection {
        zh_cn: "- 翻译方向：{0}",
        zh_tw: "- 翻譯方向：{0}",
        en: "- Translation direction: {0}",
        ja: "- 翻訳方向：{0}"
    },
    ExportSegments {
        zh_cn: "- 段落数：{0}",
        zh_tw: "- 段落數：{0}",
        en: "- Segments: {0}",
        ja: "- セグメント数：{0}"
    },

    DirectionEnZhCn {
        zh_cn: "英语 → 简体中文",
        zh_tw: "英語 → 簡體中文",
        en: "English → Simplified Chinese",
        ja: "英語 → 簡体字中国語"
    },
    DirectionZhCnEn {
        zh_cn: "简体中文 → 英语",
        zh_tw: "簡體中文 → 英語",
        en: "Simplified Chinese → English",
        ja: "簡体字中国語 → 英語"
    },
    DirectionEnZhTw {
        zh_cn: "英语 → 繁体中文",
        zh_tw: "英語 → 繁體中文",
        en: "English → Traditional Chinese",
        ja: "英語 → 繁体字中国語"
    },
    DirectionZhTwEn {
        zh_cn: "繁体中文 → 英语",
        zh_tw: "繁體中文 → 英語",
        en: "Traditional Chinese → English",
        ja: "繁体字中国語 → 英語"
    },

    PrefsUnknownAudioSource {
        zh_cn: "未知音源 {0}",
        zh_tw: "未知音源 {0}",
        en: "Unknown audio source {0}",
        ja: "不明な音声ソース {0}"
    },
    PrefsUnknownSourceLang {
        zh_cn: "未知源语言 {0}（auto 尚未实现）",
        zh_tw: "未知來源語言 {0}（auto 尚未實作）",
        en: "Unknown source language {0} (auto is not supported yet)",
        ja: "不明な元言語 {0}（auto は未対応）"
    },
    PrefsUnknownTargetLang {
        zh_cn: "未知目标语言 {0}",
        zh_tw: "未知目標語言 {0}",
        en: "Unknown target language {0}",
        ja: "不明な翻訳先言語 {0}"
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_placeholders_are_replaced() {
        let text = tf(Key::CmdUnknown, &["demo"]);
        assert!(text.contains("demo"), "{text}");
        assert!(!text.contains("{0}"), "{text}");
    }

    #[test]
    fn multiple_placeholders_are_replaced_in_one_pass() {
        let text = tf(Key::ShortcutParseFailed, &["Cmd+X", "boom"]);
        assert!(text.contains("Cmd+X"), "{text}");
        assert!(text.contains("boom"), "{text}");
        assert!(!text.contains("{0}"), "{text}");
        assert!(!text.contains("{1}"), "{text}");
    }

    #[test]
    fn missing_arguments_keep_their_placeholders() {
        let text = tf(Key::ShortcutParseFailed, &["Cmd+X"]);
        assert!(text.contains("Cmd+X"), "{text}");
        assert!(text.contains("{1}"), "{text}");
    }

    #[test]
    fn explicit_preferences_map_to_locales() {
        assert_eq!(Locale::from_language(Language::ZhCn).as_str(), "zh-CN");
        assert_eq!(Locale::from_language(Language::ZhTw).as_str(), "zh-TW");
        assert_eq!(Locale::from_language(Language::En).as_str(), "en");
        assert_eq!(Locale::from_language(Language::Ja).as_str(), "ja");
    }

    const LOCALES: [Locale; 4] = [Locale::ZhCn, Locale::ZhTw, Locale::En, Locale::Ja];

    fn placeholders(value: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut rest = value;
        while let Some(start) = rest.find('{') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('}') else {
                break;
            };
            let inner = &after[..end];
            if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
                out.push(inner);
            }
            rest = &after[end + 1..];
        }
        out.sort_unstable();
        out
    }

    #[test]
    fn every_key_has_four_non_empty_translations() {
        for &key in ALL_KEYS {
            for locale in LOCALES {
                let value = text(key, locale);
                assert!(!value.trim().is_empty(), "{key:?} {locale:?}");
            }
        }
    }

    #[test]
    fn placeholders_match_across_locales() {
        for &key in ALL_KEYS {
            let expected = placeholders(text(key, Locale::ZhCn));
            for locale in LOCALES {
                assert_eq!(
                    expected,
                    placeholders(text(key, locale)),
                    "{key:?} {locale:?}"
                );
            }
        }
    }
}
