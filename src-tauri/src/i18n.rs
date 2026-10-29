use std::sync::atomic::{AtomicU8, Ordering};

use crate::config::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Locale {
    ZhCn = 0,
    ZhTw = 1,
    En = 2,
    Ja = 3,
    Ko = 4,
    Fr = 5,
    De = 6,
    Es = 7,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::En => "en",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Es => "es",
        }
    }

    pub fn from_language(language: Language) -> Self {
        match language {
            Language::ZhCn => Self::ZhCn,
            Language::ZhTw => Self::ZhTw,
            Language::En => Self::En,
            Language::Ja => Self::Ja,
            Language::Ko => Self::Ko,
            Language::Fr => Self::Fr,
            Language::De => Self::De,
            Language::Es => Self::Es,
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
        } else if tag.starts_with("ko") {
            Self::Ko
        } else if tag.starts_with("fr") {
            Self::Fr
        } else if tag.starts_with("de") {
            Self::De
        } else if tag.starts_with("es") {
            Self::Es
        } else {
            Self::En
        }
    }
}

pub const ALL_LOCALES: [Locale; 8] = [
    Locale::ZhCn,
    Locale::ZhTw,
    Locale::En,
    Locale::Ja,
    Locale::Ko,
    Locale::Fr,
    Locale::De,
    Locale::Es,
];

static LOCALE: AtomicU8 = AtomicU8::new(Locale::ZhCn as u8);

pub fn set_locale(locale: Locale) -> bool {
    LOCALE.swap(locale as u8, Ordering::SeqCst) != locale as u8
}

pub fn locale() -> Locale {
    let raw = LOCALE.load(Ordering::SeqCst);
    ALL_LOCALES
        .into_iter()
        .find(|l| *l as u8 == raw)
        .unwrap_or(Locale::ZhCn)
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
    ($( $key:ident { zh_cn: $zh_cn:literal, zh_tw: $zh_tw:literal, en: $en:literal, ja: $ja:literal, ko: $ko:literal, fr: $fr:literal, de: $de:literal, es: $es:literal } ),* $(,)?) => {
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
                    Locale::Ko => $ko,
                    Locale::Fr => $fr,
                    Locale::De => $de,
                    Locale::Es => $es,
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
        ja: "AudioScript",
        ko: "AudioScript",
        fr: "AudioScript",
        de: "AudioScript",
        es: "AudioScript"
    },

    TrayShowMain {
        zh_cn: "显示主窗口",
        zh_tw: "顯示主視窗",
        en: "Show main window",
        ja: "メインウィンドウを表示",
        ko: "메인 창 표시",
        fr: "Afficher la fenêtre principale",
        de: "Hauptfenster anzeigen",
        es: "Mostrar ventana principal"
    },
    TraySessionStart {
        zh_cn: "开始转写",
        zh_tw: "開始轉寫",
        en: "Start transcribing",
        ja: "文字起こしを開始",
        ko: "전사 시작",
        fr: "Lancer la transcription",
        de: "Transkription starten",
        es: "Iniciar transcripción"
    },
    TraySessionStop {
        zh_cn: "停止转写",
        zh_tw: "停止轉寫",
        en: "Stop transcribing",
        ja: "文字起こしを停止",
        ko: "전사 중지",
        fr: "Arrêter la transcription",
        de: "Transkription beenden",
        es: "Detener transcripción"
    },
    TrayShowOverlay {
        zh_cn: "显示悬浮窗",
        zh_tw: "顯示懸浮窗",
        en: "Show overlay",
        ja: "オーバーレイを表示",
        ko: "오버레이 표시",
        fr: "Afficher la superposition",
        de: "Overlay einblenden",
        es: "Mostrar superposición"
    },
    TrayHideOverlay {
        zh_cn: "隐藏悬浮窗",
        zh_tw: "隱藏懸浮窗",
        en: "Hide overlay",
        ja: "オーバーレイを隠す",
        ko: "오버레이 숨기기",
        fr: "Masquer la superposition",
        de: "Overlay ausblenden",
        es: "Ocultar superposición"
    },
    TrayLocked {
        zh_cn: "点击穿透（锁定）",
        zh_tw: "點擊穿透（鎖定）",
        en: "Click-through (locked)",
        ja: "クリック透過（ロック）",
        ko: "클릭 통과 (잠금)",
        fr: "Clics traversants (verrouillé)",
        de: "Klickdurchlässig (gesperrt)",
        es: "Clics pasantes (bloqueado)"
    },
    TrayQuit {
        zh_cn: "退出 AudioScript",
        zh_tw: "結束 AudioScript",
        en: "Quit AudioScript",
        ja: "AudioScript を終了",
        ko: "AudioScript 종료",
        fr: "Quitter AudioScript",
        de: "AudioScript beenden",
        es: "Salir de AudioScript"
    },
    TrayTooltip {
        zh_cn: "AudioScript 声稿",
        zh_tw: "AudioScript 聲稿",
        en: "AudioScript",
        ja: "AudioScript",
        ko: "AudioScript",
        fr: "AudioScript",
        de: "AudioScript",
        es: "AudioScript"
    },

    ShortcutEmpty {
        zh_cn: "加速键不能为空",
        zh_tw: "快速鍵不能為空",
        en: "Shortcut cannot be empty",
        ja: "ショートカットを空にできません",
        ko: "단축키는 비워 둘 수 없습니다",
        fr: "Le raccourci ne peut pas être vide",
        de: "Tastenkürzel darf nicht leer sein",
        es: "El atajo no puede estar vacío"
    },
    ShortcutUnknownId {
        zh_cn: "未知的热键 id：{0}",
        zh_tw: "未知的快速鍵 id：{0}",
        en: "Unknown shortcut id: {0}",
        ja: "不明なショートカット id：{0}",
        ko: "알 수 없는 단축키 ID: {0}",
        fr: "ID de raccourci inconnu : {0}",
        de: "Unbekannte Tastenkürzel-ID: {0}",
        es: "ID de atajo desconocido: {0}"
    },
    ShortcutParseFailed {
        zh_cn: "加速键 {0} 无法解析：{1}",
        zh_tw: "快速鍵 {0} 無法解析：{1}",
        en: "Cannot parse shortcut {0}: {1}",
        ja: "ショートカット {0} を解析できません：{1}",
        ko: "단축키 {0}을(를) 해석할 수 없습니다: {1}",
        fr: "Impossible d’analyser le raccourci {0} : {1}",
        de: "Tastenkürzel {0} kann nicht verarbeitet werden: {1}",
        es: "No se puede analizar el atajo {0}: {1}"
    },
    ShortcutRegisterFailed {
        zh_cn: "键位 {0} 注册失败（可能已被其他应用占用）：{1}",
        zh_tw: "鍵位 {0} 註冊失敗（可能已被其他應用程式佔用）：{1}",
        en: "Failed to register {0} (it may be used by another app): {1}",
        ja: "{0} の登録に失敗しました（他のアプリが使用中の可能性があります）：{1}",
        ko: "{0} 등록 실패(다른 앱이 사용 중일 수 있음): {1}",
        fr: "Échec de l’enregistrement de {0} (peut-être utilisé par une autre application) : {1}",
        de: "{0} konnte nicht registriert werden (möglicherweise von einer anderen App verwendet): {1}",
        es: "No se pudo registrar {0} (puede que lo use otra aplicación): {1}"
    },

    CmdUnknown {
        zh_cn: "未知命令：{0}",
        zh_tw: "未知指令：{0}",
        en: "Unknown command: {0}",
        ja: "不明なコマンド：{0}",
        ko: "알 수 없는 명령: {0}",
        fr: "Commande inconnue : {0}",
        de: "Unbekannter Befehl: {0}",
        es: "Comando desconocido: {0}"
    },
    CmdBadArgs {
        zh_cn: "参数无效：{0}",
        zh_tw: "參數無效：{0}",
        en: "Invalid argument: {0}",
        ja: "引数が無効です：{0}",
        ko: "잘못된 인수: {0}",
        fr: "Argument invalide : {0}",
        de: "Ungültiges Argument: {0}",
        es: "Argumento no válido: {0}"
    },
    CmdThreadStartFailed {
        zh_cn: "启动后台线程失败：{0}",
        zh_tw: "啟動背景執行緒失敗：{0}",
        en: "Failed to start background thread: {0}",
        ja: "バックグラウンドスレッドの開始に失敗しました：{0}",
        ko: "백그라운드 스레드를 시작하지 못했습니다: {0}",
        fr: "Impossible de démarrer le thread d’arrière-plan : {0}",
        de: "Hintergrundthread konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar el hilo en segundo plano: {0}"
    },
    CmdTaskPanicked {
        zh_cn: "后台任务异常退出",
        zh_tw: "背景工作異常結束",
        en: "Background task exited unexpectedly",
        ja: "バックグラウンドタスクが異常終了しました",
        ko: "백그라운드 작업이 예기치 않게 종료되었습니다",
        fr: "La tâche d’arrière-plan s’est terminée de façon inattendue",
        de: "Hintergrundaufgabe unerwartet beendet",
        es: "La tarea en segundo plano terminó inesperadamente"
    },
    CmdTimeout {
        zh_cn: "后台任务超时，请重试",
        zh_tw: "背景工作逾時，請重試",
        en: "Background task timed out. Try again.",
        ja: "バックグラウンドタスクがタイムアウトしました。再試行してください",
        ko: "백그라운드 작업 시간이 초과되었습니다. 다시 시도하세요.",
        fr: "La tâche d’arrière-plan a expiré. Réessayez.",
        de: "Zeitüberschreitung der Hintergrundaufgabe. Versuchen Sie es erneut.",
        es: "La tarea en segundo plano agotó el tiempo. Inténtelo de nuevo."
    },
    CmdLaneExited {
        zh_cn: "命令执行线程已退出",
        zh_tw: "指令執行執行緒已結束",
        en: "Command worker has exited",
        ja: "コマンド実行スレッドが終了しました",
        ko: "명령 작업자가 종료되었습니다",
        fr: "Le worker de commandes s’est arrêté",
        de: "Befehls-Worker wurde beendet",
        es: "El trabajador de comandos se ha cerrado"
    },
    CmdMicDenied {
        zh_cn: "麦克风权限已被拒绝，系统不会再弹窗。请到「通用 → 权限」打开系统设置重新允许。",
        zh_tw: "麥克風權限已被拒絕，系統不會再彈出提示。請到「一般 → 權限」開啟系統設定重新允許。",
        en: "Microphone permission was denied and the system will not prompt again. Open System Settings from General → Permissions to allow it.",
        ja: "マイクの権限が拒否されており、システムから再度確認されません。「一般 → 権限」からシステム設定を開き、許可し直してください。",
        ko: "마이크 권한이 거부되어 시스템이 더 이상 묻지 않습니다. 시스템 설정의 일반 → 권한에서 직접 허용하세요.",
        fr: "L’autorisation du microphone a été refusée et le système ne redemandera plus. Ouvrez Réglages Système → Général → Autorisations pour l’autoriser.",
        de: "Die Mikrofonberechtigung wurde verweigert und das System fragt nicht mehr. Öffnen Sie Systemeinstellungen → Allgemein → Berechtigungen, um sie zu erlauben.",
        es: "Se denegó el permiso del micrófono y el sistema no volverá a preguntar. Abra Ajustes del Sistema → General → Permisos para permitirlo."
    },
    CmdConfigParseFailed {
        zh_cn: "config 解析失败：{0}",
        zh_tw: "config 解析失敗：{0}",
        en: "Failed to parse config: {0}",
        ja: "config の解析に失敗しました：{0}",
        ko: "설정을 해석하지 못했습니다: {0}",
        fr: "Impossible d’analyser la configuration : {0}",
        de: "Konfiguration konnte nicht verarbeitet werden: {0}",
        es: "No se pudo analizar la configuración: {0}"
    },
    CmdMissingArg {
        zh_cn: "缺少 {0}",
        zh_tw: "缺少 {0}",
        en: "Missing {0}",
        ja: "{0} が指定されていません",
        ko: "{0}이(가) 없습니다",
        fr: "{0} manquant",
        de: "{0} fehlt",
        es: "Falta {0}"
    },
    CmdArgParseFailed {
        zh_cn: "{0} 解析失败：{1}",
        zh_tw: "{0} 解析失敗：{1}",
        en: "Failed to parse {0}: {1}",
        ja: "{0} の解析に失敗しました：{1}",
        ko: "{0}을(를) 해석하지 못했습니다: {1}",
        fr: "Impossible d’analyser {0} : {1}",
        de: "{0} konnte nicht verarbeitet werden: {1}",
        es: "No se pudo analizar {0}: {1}"
    },
    CmdNoWritableDir {
        zh_cn: "找不到可写入的位置：{0}",
        zh_tw: "找不到可寫入的位置：{0}",
        en: "No writable location found: {0}",
        ja: "書き込み可能な場所が見つかりません：{0}",
        ko: "쓸 수 있는 위치를 찾지 못했습니다: {0}",
        fr: "Aucun emplacement inscriptible trouvé : {0}",
        de: "Kein beschreibbarer Ort gefunden: {0}",
        es: "No se encontró una ubicación escribible: {0}"
    },
    CmdWriteFailed {
        zh_cn: "写入 {0} 失败：{1}",
        zh_tw: "寫入 {0} 失敗：{1}",
        en: "Failed to write {0}: {1}",
        ja: "{0} への書き込みに失敗しました：{1}",
        ko: "{0} 쓰기 실패: {1}",
        fr: "Échec de l’écriture de {0} : {1}",
        de: "Schreiben von {0} fehlgeschlagen: {1}",
        es: "No se pudo escribir {0}: {1}"
    },
    CmdOpenFailed {
        zh_cn: "无法打开{0}：{1}",
        zh_tw: "無法開啟{0}：{1}",
        en: "Failed to open {0}: {1}",
        ja: "{0} を開けませんでした：{1}",
        ko: "{0} 열기 실패: {1}",
        fr: "Impossible d’ouvrir {0} : {1}",
        de: "Öffnen von {0} fehlgeschlagen: {1}",
        es: "No se pudo abrir {0}: {1}"
    },
    OpenWhatBrowser {
        zh_cn: "浏览器",
        zh_tw: "瀏覽器",
        en: "browser",
        ja: "ブラウザ",
        ko: "브라우저",
        fr: "navigateur",
        de: "Browser",
        es: "navegador"
    },
    OpenWhatLink {
        zh_cn: "链接",
        zh_tw: "連結",
        en: "link",
        ja: "リンク",
        ko: "링크",
        fr: "lien",
        de: "Link",
        es: "enlace"
    },
    OpenWhatModelDir {
        zh_cn: "模型目录",
        zh_tw: "模型目錄",
        en: "model folder",
        ja: "モデルフォルダー",
        ko: "모델 폴더",
        fr: "dossier de modèles",
        de: "Modellordner",
        es: "carpeta de modelos"
    },
    OpenWhatFileManager {
        zh_cn: "文件管理器",
        zh_tw: "檔案管理器",
        en: "file manager",
        ja: "ファイルマネージャー",
        ko: "파일 관리자",
        fr: "gestionnaire de fichiers",
        de: "Dateimanager",
        es: "gestor de archivos"
    },
    CmdPathMissing {
        zh_cn: "路径不存在：{0}",
        zh_tw: "路徑不存在：{0}",
        en: "Path does not exist: {0}",
        ja: "パスが存在しません：{0}",
        ko: "경로가 존재하지 않습니다: {0}",
        fr: "Le chemin n’existe pas : {0}",
        de: "Pfad existiert nicht: {0}",
        es: "La ruta no existe: {0}"
    },
    CmdCreateDirFailed {
        zh_cn: "创建 {0} 失败：{1}",
        zh_tw: "建立 {0} 失敗：{1}",
        en: "Failed to create {0}: {1}",
        ja: "{0} の作成に失敗しました：{1}",
        ko: "{0} 만들기 실패: {1}",
        fr: "Impossible de créer {0} : {1}",
        de: "{0} konnte nicht erstellt werden: {1}",
        es: "No se pudo crear {0}: {1}"
    },
    CmdReadLogDirFailed {
        zh_cn: "读取日志目录失败：{0}",
        zh_tw: "讀取日誌目錄失敗：{0}",
        en: "Failed to read log folder: {0}",
        ja: "ログフォルダーの読み取りに失敗しました：{0}",
        ko: "로그 폴더를 읽지 못했습니다: {0}",
        fr: "Impossible de lire le dossier des journaux : {0}",
        de: "Protokollordner konnte nicht gelesen werden: {0}",
        es: "No se pudo leer la carpeta de registros: {0}"
    },
    CmdCopyFailed {
        zh_cn: "复制 {0} 失败：{1}",
        zh_tw: "複製 {0} 失敗：{1}",
        en: "Failed to copy {0}: {1}",
        ja: "{0} のコピーに失敗しました：{1}",
        ko: "{0} 복사 실패: {1}",
        fr: "Impossible de copier {0} : {1}",
        de: "{0} konnte nicht kopiert werden: {1}",
        es: "No se pudo copiar {0}: {1}"
    },
    CmdNoLogs {
        zh_cn: "{0} 中没有日志文件，无可导出",
        zh_tw: "{0} 中沒有日誌檔，無可匯出",
        en: "No log files to export in {0}",
        ja: "{0} にエクスポートできるログファイルがありません",
        ko: "{0}에 내보낼 로그 파일이 없습니다",
        fr: "Aucun fichier journal à exporter dans {0}",
        de: "Keine Protokolldateien zum Exportieren in {0}",
        es: "No hay archivos de registro para exportar en {0}"
    },
    CmdLogsFolderName {
        zh_cn: "AudioScript-日志-{0}",
        zh_tw: "AudioScript-日誌-{0}",
        en: "AudioScript-logs-{0}",
        ja: "AudioScript-logs-{0}",
        ko: "AudioScript-logs-{0}",
        fr: "AudioScript-logs-{0}",
        de: "AudioScript-logs-{0}",
        es: "AudioScript-logs-{0}"
    },
    CmdUnknownModel {
        zh_cn: "未知模型：{0}",
        zh_tw: "未知模型：{0}",
        en: "Unknown model: {0}",
        ja: "不明なモデル：{0}",
        ko: "알 수 없는 모델: {0}",
        fr: "Modèle inconnu : {0}",
        de: "Unbekanntes Modell: {0}",
        es: "Modelo desconocido: {0}"
    },
    CmdModelNotReady {
        zh_cn: "模型「{0}」尚未安装或文件不完整，请先下载",
        zh_tw: "模型「{0}」尚未安裝或檔案不完整，請先下載",
        en: "Model \"{0}\" is not installed or incomplete. Download it first.",
        ja: "モデル「{0}」が未インストールか不完全です。先にダウンロードしてください",
        ko: "모델 \"{0}\"이(가) 설치되지 않았거나 불완전합니다. 먼저 다운로드하세요.",
        fr: "Le modèle « {0} » n’est pas installé ou est incomplet. Téléchargez-le d’abord.",
        de: "Modell „{0}“ ist nicht installiert oder unvollständig. Laden Sie es zuerst herunter.",
        es: "El modelo «{0}» no está instalado o está incompleto. Descárguelo primero."
    },
    ModelAsrBuiltinTitle {
        zh_cn: "Zipformer 小模型",
        zh_tw: "Zipformer 小模型",
        en: "Zipformer small model",
        ja: "Zipformer 小型モデル",
        ko: "Zipformer 소형 모델",
        fr: "Zipformer petit modèle",
        de: "Zipformer kleines Modell",
        es: "Zipformer modelo pequeño"
    },
    ModelAsrHighTitle {
        zh_cn: "Zipformer 高精度",
        zh_tw: "Zipformer 高精度",
        en: "Zipformer high accuracy",
        ja: "Zipformer 高精度",
        ko: "Zipformer 고정밀",
        fr: "Zipformer haute précision",
        de: "Zipformer hohe Genauigkeit",
        es: "Zipformer alta precisión"
    },
    ModelAsrQwen3Title {
        zh_cn: "Qwen3-ASR 高精度",
        zh_tw: "Qwen3-ASR 高精度",
        en: "Qwen3-ASR high accuracy",
        ja: "Qwen3-ASR 高精度",
        ko: "Qwen3-ASR 고정밀",
        fr: "Qwen3-ASR haute précision",
        de: "Qwen3-ASR hohe Genauigkeit",
        es: "Qwen3-ASR alta precisión"
    },
    ModelMtBaseTitle {
        zh_cn: "OPUS-MT 基础版",
        zh_tw: "OPUS-MT 基礎版",
        en: "OPUS-MT base",
        ja: "OPUS-MT ベース",
        ko: "OPUS-MT 기본",
        fr: "OPUS-MT de base",
        de: "OPUS-MT Basis",
        es: "OPUS-MT base"
    },
    ModelMtMultilingualTitle {
        zh_cn: "NLLB-200 多语言",
        zh_tw: "NLLB-200 多語言",
        en: "NLLB-200 multilingual",
        ja: "NLLB-200 多言語",
        ko: "NLLB-200 다국어",
        fr: "NLLB-200 multilingue",
        de: "NLLB-200 mehrsprachig",
        es: "NLLB-200 multilingüe"
    },
    CmdUnknownAsrTier {
        zh_cn: "未知 ASR 档位：{0}",
        zh_tw: "未知 ASR 等級：{0}",
        en: "Unknown ASR tier: {0}",
        ja: "不明な ASR ティア：{0}",
        ko: "알 수 없는 ASR 등급: {0}",
        fr: "Niveau ASR inconnu : {0}",
        de: "Unbekannte ASR-Stufe: {0}",
        es: "Nivel de ASR desconocido: {0}"
    },
    CmdUnknownMtTier {
        zh_cn: "未知翻译模型档位：{0}",
        zh_tw: "未知翻譯模型等級：{0}",
        en: "Unknown translation model tier: {0}",
        ja: "不明な翻訳モデルティア：{0}",
        ko: "알 수 없는 번역 모델 등급: {0}",
        fr: "Niveau de modèle de traduction inconnu : {0}",
        de: "Unbekannte Übersetzungsmodell-Stufe: {0}",
        es: "Nivel de modelo de traducción desconocido: {0}"
    },
    CmdUnknownMode {
        zh_cn: "未知模式 {0}",
        zh_tw: "未知模式 {0}",
        en: "Unknown mode {0}",
        ja: "不明なモード {0}",
        ko: "알 수 없는 모드 {0}",
        fr: "Mode inconnu {0}",
        de: "Unbekannter Modus {0}",
        es: "Modo desconocido {0}"
    },
    CmdUnknownPane {
        zh_cn: "未知设置页 {0}",
        zh_tw: "未知設定頁 {0}",
        en: "Unknown settings pane {0}",
        ja: "不明な設定ページ {0}",
        ko: "알 수 없는 설정 패널 {0}",
        fr: "Panneau de réglages inconnu {0}",
        de: "Unbekannter Einstellungsbereich {0}",
        es: "Panel de ajustes desconocido {0}"
    },
    CmdLinkRejected {
        zh_cn: "拒绝打开非项目链接：{0}",
        zh_tw: "拒絕開啟非專案連結：{0}",
        en: "Refusing to open a non-project link: {0}",
        ja: "プロジェクト外のリンクは開けません：{0}",
        ko: "프로젝트 링크가 아니므로 열지 않습니다: {0}",
        fr: "Refus d’ouvrir un lien hors projet : {0}",
        de: "Öffnen eines projektsfremden Links abgelehnt: {0}",
        es: "Se rechaza abrir un enlace ajeno al proyecto: {0}"
    },
    CmdMissingEnabled {
        zh_cn: "缺少 enabled",
        zh_tw: "缺少 enabled",
        en: "Missing enabled",
        ja: "enabled が指定されていません",
        ko: "enabled 값이 없습니다",
        fr: "enabled manquant",
        de: "enabled fehlt",
        es: "Falta enabled"
    },
    CmdAutostartFailed {
        zh_cn: "设置开机自启失败：{0}",
        zh_tw: "設定開機自動啟動失敗：{0}",
        en: "Failed to change launch-at-login: {0}",
        ja: "ログイン時の起動設定に失敗しました：{0}",
        ko: "로그인 시 실행 변경 실패: {0}",
        fr: "Échec de la modification du lancement à la connexion : {0}",
        de: "Änderung des Starts bei Anmeldung fehlgeschlagen: {0}",
        es: "No se pudo cambiar el inicio al iniciar sesión: {0}"
    },
    CmdProbeWhileCapturing {
        zh_cn: "正在转写中，请先停止再检测",
        zh_tw: "正在轉寫中，請先停止再偵測",
        en: "Transcription is running. Stop it before probing.",
        ja: "文字起こし中です。停止してから検出してください",
        ko: "전사가 실행 중입니다. 검사 전에 중지하세요.",
        fr: "La transcription est en cours. Arrêtez-la avant de tester.",
        de: "Transkription läuft. Beenden Sie sie vor der Prüfung.",
        es: "La transcripción está en curso. Deténgala antes de comprobar."
    },

    ModelUnknown {
        zh_cn: "未知模型：{0}",
        zh_tw: "未知模型：{0}",
        en: "Unknown model: {0}",
        ja: "不明なモデル：{0}",
        ko: "알 수 없는 모델: {0}",
        fr: "Modèle inconnu : {0}",
        de: "Unbekanntes Modell: {0}",
        es: "Modelo desconocido: {0}"
    },
    ModelNotDownloadable {
        zh_cn: "模型 {0} 暂不提供下载",
        zh_tw: "模型 {0} 暫不提供下載",
        en: "Model {0} is not available for download",
        ja: "モデル {0} はダウンロードできません",
        ko: "모델 {0}은(는) 다운로드할 수 없습니다",
        fr: "Le modèle {0} n’est pas disponible au téléchargement",
        de: "Modell {0} ist nicht zum Download verfügbar",
        es: "El modelo {0} no está disponible para descargar"
    },
    ModelAlreadyInstalled {
        zh_cn: "模型 {0} 已安装，如需修复请先删除",
        zh_tw: "模型 {0} 已安裝，如需修復請先刪除",
        en: "Model {0} is already installed. Delete it first to repair.",
        ja: "モデル {0} はインストール済みです。修復するには先に削除してください",
        ko: "모델 {0}이(가) 이미 설치되어 있습니다. 복구하려면 먼저 삭제하세요.",
        fr: "Le modèle {0} est déjà installé. Supprimez-le d’abord pour le réparer.",
        de: "Modell {0} ist bereits installiert. Löschen Sie es zuerst, um es zu reparieren.",
        es: "El modelo {0} ya está instalado. Elimínelo primero para repararlo."
    },
    ModelBundled {
        zh_cn: "内置模型随应用分发，不能删除",
        zh_tw: "內建模型隨應用程式發行，不能刪除",
        en: "Bundled models ship with the app and cannot be deleted",
        ja: "内蔵モデルはアプリに含まれているため削除できません",
        ko: "내장 모델은 앱과 함께 배포되며 삭제할 수 없습니다",
        fr: "Les modèles intégrés sont livrés avec l’application et ne peuvent pas être supprimés",
        de: "Integrierte Modelle werden mit der App ausgeliefert und können nicht gelöscht werden",
        es: "Los modelos incluidos se distribuyen con la aplicación y no se pueden eliminar"
    },
    ModelSelected {
        zh_cn: "该模型当前被选中，请先切换到其他模型再删除",
        zh_tw: "該模型目前被選中，請先切換到其他模型再刪除",
        en: "This model is selected. Switch to another model before deleting.",
        ja: "このモデルは選択中です。別のモデルに切り替えてから削除してください",
        ko: "이 모델이 선택되어 있습니다. 삭제하기 전에 다른 모델로 전환하세요.",
        fr: "Ce modèle est sélectionné. Passez à un autre modèle avant de le supprimer.",
        de: "Dieses Modell ist ausgewählt. Wechseln Sie vor dem Löschen zu einem anderen Modell.",
        es: "Este modelo está seleccionado. Cambie a otro modelo antes de eliminarlo."
    },
    ModelInUse {
        zh_cn: "正在转写，请先停止再删除模型",
        zh_tw: "正在轉寫，請先停止再刪除模型",
        en: "Transcription is running. Stop it before deleting a model.",
        ja: "文字起こし中です。停止してからモデルを削除してください",
        ko: "전사가 실행 중입니다. 모델을 삭제하기 전에 중지하세요.",
        fr: "La transcription est en cours. Arrêtez-la avant de supprimer un modèle.",
        de: "Transkription läuft. Beenden Sie sie, bevor Sie ein Modell löschen.",
        es: "La transcripción está en curso. Deténgala antes de eliminar un modelo."
    },
    ModelJobInProgress {
        zh_cn: "已有模型下载任务进行中",
        zh_tw: "已有模型下載工作進行中",
        en: "A model download is already in progress",
        ja: "モデルのダウンロードが進行中です",
        ko: "모델 다운로드가 이미 진행 중입니다",
        fr: "Un téléchargement de modèle est déjà en cours",
        de: "Ein Modell-Download läuft bereits",
        es: "Ya hay una descarga de modelo en curso"
    },
    ModelDownloadFailed {
        zh_cn: "下载失败：{0}",
        zh_tw: "下載失敗：{0}",
        en: "Download failed: {0}",
        ja: "ダウンロードに失敗しました：{0}",
        ko: "다운로드 실패: {0}",
        fr: "Échec du téléchargement : {0}",
        de: "Download fehlgeschlagen: {0}",
        es: "Error de descarga: {0}"
    },
    ModelHttpFailed {
        zh_cn: "下载失败：HTTP {0}",
        zh_tw: "下載失敗：HTTP {0}",
        en: "Download failed: HTTP {0}",
        ja: "ダウンロードに失敗しました：HTTP {0}",
        ko: "다운로드 실패: HTTP {0}",
        fr: "Échec du téléchargement : HTTP {0}",
        de: "Download fehlgeschlagen: HTTP {0}",
        es: "Error de descarga: HTTP {0}"
    },
    ModelChecksumFailed {
        zh_cn: "校验失败：期望 {0}，实际 {1}",
        zh_tw: "校驗失敗：預期 {0}，實際 {1}",
        en: "Checksum mismatch: expected {0}, got {1}",
        ja: "検証に失敗しました：期待値 {0}、実際 {1}",
        ko: "체크섬 불일치: 예상 {0}, 실제 {1}",
        fr: "Somme de contrôle incorrecte : attendu {0}, obtenu {1}",
        de: "Prüfsumme stimmt nicht: erwartet {0}, erhalten {1}",
        es: "Suma de comprobación incorrecta: se esperaba {0}, se obtuvo {1}"
    },
    ModelUnsafeArchive {
        zh_cn: "归档内容不安全：{0}",
        zh_tw: "封存內容不安全：{0}",
        en: "Unsafe archive contents: {0}",
        ja: "安全でないアーカイブ内容：{0}",
        ko: "안전하지 않은 압축 파일 내용: {0}",
        fr: "Contenu d’archive dangereux : {0}",
        de: "Unsicherer Archivinhalt: {0}",
        es: "Contenido de archivo no seguro: {0}"
    },
    ModelExtractFailed {
        zh_cn: "解压失败：{0}",
        zh_tw: "解壓縮失敗：{0}",
        en: "Extraction failed: {0}",
        ja: "展開に失敗しました：{0}",
        ko: "압축 해제 실패: {0}",
        fr: "Échec de l’extraction : {0}",
        de: "Entpacken fehlgeschlagen: {0}",
        es: "Error al extraer: {0}"
    },
    ModelDiskFull {
        zh_cn: "磁盘空间不足：需要约 {0} MB，可用约 {1} MB",
        zh_tw: "磁碟空間不足：需要約 {0} MB，可用約 {1} MB",
        en: "Not enough disk space: about {0} MB needed, {1} MB available",
        ja: "ディスク容量が不足しています：約 {0} MB 必要、空き約 {1} MB",
        ko: "디스크 공간이 부족합니다: 약 {0} MB 필요, {1} MB 사용 가능",
        fr: "Espace disque insuffisant : environ {0} Mo requis, {1} Mo disponibles",
        de: "Nicht genügend Speicherplatz: etwa {0} MB benötigt, {1} MB verfügbar",
        es: "No hay suficiente espacio en disco: se necesitan unos {0} MB, hay {1} MB disponibles"
    },
    ModelIncomplete {
        zh_cn: "模型文件不完整：{0}",
        zh_tw: "模型檔案不完整：{0}",
        en: "Model files are incomplete: {0}",
        ja: "モデルファイルが不完全です：{0}",
        ko: "모델 파일이 불완전합니다: {0}",
        fr: "Les fichiers du modèle sont incomplets : {0}",
        de: "Modelldateien sind unvollständig: {0}",
        es: "Los archivos del modelo están incompletos: {0}"
    },
    ModelFileOpFailed {
        zh_cn: "文件操作失败：{0}",
        zh_tw: "檔案操作失敗：{0}",
        en: "File operation failed: {0}",
        ja: "ファイル操作に失敗しました：{0}",
        ko: "파일 작업 실패: {0}",
        fr: "Échec d’une opération de fichier : {0}",
        de: "Dateivorgang fehlgeschlagen: {0}",
        es: "Error en una operación de archivo: {0}"
    },
    ModelCancelled {
        zh_cn: "下载已取消",
        zh_tw: "下載已取消",
        en: "Download cancelled",
        ja: "ダウンロードをキャンセルしました",
        ko: "다운로드 취소됨",
        fr: "Téléchargement annulé",
        de: "Download abgebrochen",
        es: "Descarga cancelada"
    },
    ModelNoDownloadInfo {
        zh_cn: "{0} 没有下载信息",
        zh_tw: "{0} 沒有下載資訊",
        en: "{0} has no download information",
        ja: "{0} にダウンロード情報がありません",
        ko: "{0}에 다운로드 정보가 없습니다",
        fr: "{0} n’a pas d’informations de téléchargement",
        de: "{0} hat keine Download-Informationen",
        es: "{0} no tiene información de descarga"
    },
    ModelNoChecksumInfo {
        zh_cn: "{0} 的下载校验信息尚未配置",
        zh_tw: "{0} 的下載校驗資訊尚未設定",
        en: "Download checksum for {0} is not configured",
        ja: "{0} のダウンロード検証情報が未設定です",
        ko: "{0}의 다운로드 체크섬이 설정되지 않았습니다",
        fr: "La somme de contrôle de {0} n’est pas configurée",
        de: "Die Download-Prüfsumme für {0} ist nicht konfiguriert",
        es: "La suma de comprobación de descarga de {0} no está configurada"
    },
    ModelThreadPanicked {
        zh_cn: "安装线程异常退出",
        zh_tw: "安裝執行緒異常結束",
        en: "Installer thread exited unexpectedly",
        ja: "インストールスレッドが異常終了しました",
        ko: "설치 스레드가 예기치 않게 종료되었습니다",
        fr: "Le thread d’installation s’est terminé de façon inattendue",
        de: "Installations-Thread unerwartet beendet",
        es: "El hilo del instalador terminó inesperadamente"
    },
    ModelThreadStartFailed {
        zh_cn: "启动安装线程失败：{0}",
        zh_tw: "啟動安裝執行緒失敗：{0}",
        en: "Failed to start installer thread: {0}",
        ja: "インストールスレッドの開始に失敗しました：{0}",
        ko: "설치 스레드를 시작하지 못했습니다: {0}",
        fr: "Impossible de démarrer le thread d’installation : {0}",
        de: "Installations-Thread konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar el hilo del instalador: {0}"
    },
    ModelNoDataDir {
        zh_cn: "无法解析应用数据目录",
        zh_tw: "無法解析應用程式資料目錄",
        en: "Cannot resolve the app data folder",
        ja: "アプリのデータフォルダーを解決できません",
        ko: "앱 데이터 폴더를 확인할 수 없습니다",
        fr: "Impossible de localiser le dossier de données de l’application",
        de: "App-Datenordner kann nicht ermittelt werden",
        es: "No se puede resolver la carpeta de datos de la aplicación"
    },
    ModelReplaceFailed {
        zh_cn: "无法替换已存在的模型目录（可能文件被占用）：{0}：{1}",
        zh_tw: "無法取代已存在的模型目錄（可能檔案被佔用）：{0}：{1}",
        en: "Cannot replace the existing model folder (files may be in use): {0}: {1}",
        ja: "既存のモデルフォルダーを置き換えられません（ファイルが使用中の可能性）：{0}：{1}",
        ko: "기존 모델 폴더를 교체할 수 없습니다(파일이 사용 중일 수 있음): {0}: {1}",
        fr: "Impossible de remplacer le dossier de modèle existant (fichiers peut-être utilisés) : {0} : {1}",
        de: "Vorhandener Modellordner kann nicht ersetzt werden (Dateien möglicherweise in Verwendung): {0}: {1}",
        es: "No se puede reemplazar la carpeta de modelo existente (puede que los archivos estén en uso): {0}: {1}"
    },
    ModelInstallFailed {
        zh_cn: "安装模型失败（可能文件被占用）：{0}：{1}",
        zh_tw: "安裝模型失敗（可能檔案被佔用）：{0}：{1}",
        en: "Model installation failed (files may be in use): {0}: {1}",
        ja: "モデルのインストールに失敗しました（ファイルが使用中の可能性）：{0}：{1}",
        ko: "모델 설치 실패(파일이 사용 중일 수 있음): {0}: {1}",
        fr: "Échec de l’installation du modèle (fichiers peut-être utilisés) : {0} : {1}",
        de: "Modellinstallation fehlgeschlagen (Dateien möglicherweise in Verwendung): {0}: {1}",
        es: "Error al instalar el modelo (puede que los archivos estén en uso): {0}: {1}"
    },
    ModelNoArchiveInfo {
        zh_cn: "{0} 没有归档信息",
        zh_tw: "{0} 沒有封存資訊",
        en: "{0} has no archive information",
        ja: "{0} にアーカイブ情報がありません",
        ko: "{0}에 압축 파일 정보가 없습니다",
        fr: "{0} n’a pas d’informations d’archive",
        de: "{0} hat keine Archivinformationen",
        es: "{0} no tiene información de archivo"
    },
    ModelUnsupportedEntry {
        zh_cn: "不支持的归档条目类型：{0}",
        zh_tw: "不支援的封存項目類型：{0}",
        en: "Unsupported archive entry type: {0}",
        ja: "未対応のアーカイブ項目タイプ：{0}",
        ko: "지원하지 않는 압축 항목 유형: {0}",
        fr: "Type d’entrée d’archive non pris en charge : {0}",
        de: "Nicht unterstützter Archiv-Eintragstyp: {0}",
        es: "Tipo de entrada de archivo no compatible: {0}"
    },
    ModelUnsafePath {
        zh_cn: "不安全的路径：{0}",
        zh_tw: "不安全的路徑：{0}",
        en: "Unsafe path: {0}",
        ja: "安全でないパス：{0}",
        ko: "안전하지 않은 경로: {0}",
        fr: "Chemin dangereux : {0}",
        de: "Unsicherer Pfad: {0}",
        es: "Ruta no segura: {0}"
    },
    ModelTooLarge {
        zh_cn: "解压后体积超过上限（{0} 字节）",
        zh_tw: "解壓縮後大小超過上限（{0} 位元組）",
        en: "Extracted size exceeds the limit ({0} bytes)",
        ja: "展開後のサイズが上限を超えています（{0} バイト）",
        ko: "압축 해제 크기가 한도를 초과했습니다({0} 바이트)",
        fr: "La taille extraite dépasse la limite ({0} octets)",
        de: "Entpackte Größe überschreitet das Limit ({0} Bytes)",
        es: "El tamaño extraído supera el límite ({0} bytes)"
    },
    ModelDownloadTooLarge {
        zh_cn: "服务器返回的体积超过预期（已下载 {0} 字节）",
        zh_tw: "伺服器傳回的大小超過預期（已下載 {0} 位元組）",
        en: "The server sent more data than expected ({0} bytes downloaded)",
        ja: "サーバーから返されたサイズが想定を超えています（{0} バイト受信）",
        ko: "서버가 예상보다 많은 데이터를 보냈습니다({0} 바이트 다운로드됨)",
        fr: "Le serveur a envoyé plus de données que prévu ({0} octets téléchargés)",
        de: "Der Server hat mehr Daten als erwartet gesendet ({0} Bytes heruntergeladen)",
        es: "El servidor envió más datos de los esperados ({0} bytes descargados)"
    },

    AsrNotInitialized {
        zh_cn: "引擎未初始化",
        zh_tw: "引擎未初始化",
        en: "Engine is not initialized",
        ja: "エンジンが初期化されていません",
        ko: "엔진이 초기화되지 않았습니다",
        fr: "Le moteur n’est pas initialisé",
        de: "Engine ist nicht initialisiert",
        es: "El motor no está inicializado"
    },
    AsrInvalidConfig {
        zh_cn: "配置无效：{0}",
        zh_tw: "設定無效：{0}",
        en: "Invalid configuration: {0}",
        ja: "設定が無効です：{0}",
        ko: "잘못된 구성: {0}",
        fr: "Configuration invalide : {0}",
        de: "Ungültige Konfiguration: {0}",
        es: "Configuración no válida: {0}"
    },
    AsrModelLoadFailed {
        zh_cn: "模型加载失败：{0}",
        zh_tw: "模型載入失敗：{0}",
        en: "Failed to load model: {0}",
        ja: "モデルの読み込みに失敗しました：{0}",
        ko: "모델을 불러오지 못했습니다: {0}",
        fr: "Impossible de charger le modèle : {0}",
        de: "Modell konnte nicht geladen werden: {0}",
        es: "No se pudo cargar el modelo: {0}"
    },
    AsrModelDirMissing {
        zh_cn: "模型目录缺少文件：{0}",
        zh_tw: "模型目錄缺少檔案：{0}",
        en: "Model folder is missing files: {0}",
        ja: "モデルフォルダーにファイルがありません：{0}",
        ko: "모델 폴더에 파일이 없습니다: {0}",
        fr: "Il manque des fichiers dans le dossier de modèle : {0}",
        de: "Im Modellordner fehlen Dateien: {0}",
        es: "Faltan archivos en la carpeta del modelo: {0}"
    },
    AsrReadModelDirFailed {
        zh_cn: "无法读取模型目录 {0}：{1}",
        zh_tw: "無法讀取模型目錄 {0}：{1}",
        en: "Cannot read model folder {0}: {1}",
        ja: "モデルフォルダー {0} を読み取れません：{1}",
        ko: "모델 폴더 {0}을(를) 읽을 수 없습니다: {1}",
        fr: "Impossible de lire le dossier de modèle {0} : {1}",
        de: "Modellordner {0} kann nicht gelesen werden: {1}",
        es: "No se puede leer la carpeta del modelo {0}: {1}"
    },
    AsrMissingFile {
        zh_cn: "缺少模型文件：{0}\n目录实际内容：{1}",
        zh_tw: "缺少模型檔案：{0}\n目錄實際內容：{1}",
        en: "Missing model file: {0}\nFolder contents: {1}",
        ja: "モデルファイルがありません：{0}\nフォルダーの内容：{1}",
        ko: "모델 파일 없음: {0}\n폴더 내용: {1}",
        fr: "Fichier de modèle manquant : {0}\nContenu du dossier : {1}",
        de: "Fehlende Modelldatei: {0}\nOrdnerinhalt: {1}",
        es: "Falta el archivo del modelo: {0}\nContenido de la carpeta: {1}"
    },
    AsrMissingPattern {
        zh_cn: "缺少模型文件：{0}/{1}*.onnx\n目录实际内容：{2}",
        zh_tw: "缺少模型檔案：{0}/{1}*.onnx\n目錄實際內容：{2}",
        en: "Missing model file: {0}/{1}*.onnx\nFolder contents: {2}",
        ja: "モデルファイルがありません：{0}/{1}*.onnx\nフォルダーの内容：{2}",
        ko: "모델 파일 없음: {0}/{1}*.onnx\n폴더 내용: {2}",
        fr: "Fichier de modèle manquant : {0}/{1}*.onnx\nContenu du dossier : {2}",
        de: "Fehlende Modelldatei: {0}/{1}*.onnx\nOrdnerinhalt: {2}",
        es: "Falta el archivo del modelo: {0}/{1}*.onnx\nContenido de la carpeta: {2}"
    },
    AsrEmptyFolder {
        zh_cn: "（空）",
        zh_tw: "（空）",
        en: "(empty)",
        ja: "（空）",
        ko: "(비어 있음)",
        fr: "(vide)",
        de: "(leer)",
        es: "(vacía)"
    },
    AsrModelDirNotSet {
        zh_cn: "model_dir 未设置",
        zh_tw: "model_dir 未設定",
        en: "model_dir is not set",
        ja: "model_dir が設定されていません",
        ko: "model_dir이 설정되지 않았습니다",
        fr: "model_dir n’est pas défini",
        de: "model_dir ist nicht gesetzt",
        es: "model_dir no está definido"
    },
    AsrRecognizerNone {
        zh_cn: "OnlineRecognizer::create 返回 None（模型目录 {0}）",
        zh_tw: "OnlineRecognizer::create 傳回 None（模型目錄 {0}）",
        en: "OnlineRecognizer::create returned None (model folder {0})",
        ja: "OnlineRecognizer::create が None を返しました（モデルフォルダー {0}）",
        ko: "OnlineRecognizer::create가 None을 반환했습니다(모델 폴더 {0})",
        fr: "OnlineRecognizer::create a renvoyé None (dossier de modèle {0})",
        de: "OnlineRecognizer::create gab None zurück (Modellordner {0})",
        es: "OnlineRecognizer::create devolvió None (carpeta del modelo {0})"
    },
    AsrOfflineRecognizerNone {
        zh_cn: "OfflineRecognizer::create 返回 None（模型目录 {0}）",
        zh_tw: "OfflineRecognizer::create 傳回 None（模型目錄 {0}）",
        en: "OfflineRecognizer::create returned None (model folder {0})",
        ja: "OfflineRecognizer::create が None を返しました（モデルフォルダー {0}）",
        ko: "OfflineRecognizer::create가 None을 반환했습니다(모델 폴더 {0})",
        fr: "OfflineRecognizer::create a renvoyé None (dossier de modèle {0})",
        de: "OfflineRecognizer::create gab None zurück (Modellordner {0})",
        es: "OfflineRecognizer::create devolvió None (carpeta del modelo {0})"
    },
    AsrVadModelMissing {
        zh_cn: "缺少 VAD 模型：{0}\n→ 运行 `pnpm install` 后重启；打包应用请重新安装",
        zh_tw: "缺少 VAD 模型：{0}\n→ 執行 `pnpm install` 後重新啟動；打包應用請重新安裝",
        en: "Missing VAD model: {0}\n→ Run `pnpm install` and restart; reinstall the packaged app",
        ja: "VAD モデルがありません：{0}\n→ `pnpm install` を実行して再起動するか、アプリを再インストールしてください",
        ko: "VAD 모델 없음: {0}\n→ `pnpm install`을 실행하고 재시작하거나, 패키징된 앱을 다시 설치하세요",
        fr: "Modèle VAD manquant : {0}\n→ Exécutez `pnpm install` et redémarrez ; réinstallez l’application empaquetée",
        de: "Fehlendes VAD-Modell: {0}\n→ Führen Sie `pnpm install` aus und starten Sie neu; installieren Sie die gepackte App neu",
        es: "Falta el modelo VAD: {0}\n→ Ejecute `pnpm install` y reinicie; reinstale la aplicación empaquetada"
    },
    AsrVadCreateNone {
        zh_cn: "VoiceActivityDetector::create 返回 None（模型 {0}）",
        zh_tw: "VoiceActivityDetector::create 傳回 None（模型 {0}）",
        en: "VoiceActivityDetector::create returned None (model {0})",
        ja: "VoiceActivityDetector::create が None を返しました（モデル {0}）",
        ko: "VoiceActivityDetector::create가 None을 반환했습니다(모델 {0})",
        fr: "VoiceActivityDetector::create a renvoyé None (modèle {0})",
        de: "VoiceActivityDetector::create gab None zurück (Modell {0})",
        es: "VoiceActivityDetector::create devolvió None (modelo {0})"
    },
    AsrMacSpeechUnsupported {
        zh_cn: "macOS 系统语音不可用：需要 macOS 26+ Apple Silicon",
        zh_tw: "macOS 系統語音不可用：需要 macOS 26+ Apple Silicon",
        en: "macOS system speech is unavailable: macOS 26+ on Apple Silicon is required",
        ja: "macOS システム音声は利用できません：macOS 26 以降の Apple Silicon が必要です",
        ko: "macOS 시스템 음성을 사용할 수 없습니다: macOS 26 이상 Apple Silicon이 필요합니다",
        fr: "La parole système macOS n’est pas disponible : macOS 26+ sur Apple Silicon requis",
        de: "macOS-Systemsprache ist nicht verfügbar: macOS 26+ auf Apple Silicon erforderlich",
        es: "El dictado del sistema macOS no está disponible: se requiere macOS 26+ en Apple Silicon"
    },
    AsrMacSpeechUsageDescription {
        zh_cn: "缺少 NSSpeechRecognitionUsageDescription：请在打包后的 .app 中运行（开发模式裸二进制没有权限描述，无法调用系统语音）",
        zh_tw: "缺少 NSSpeechRecognitionUsageDescription：請在打包後的 .app 中執行（開發模式裸二進位沒有權限描述，無法呼叫系統語音）",
        en: "NSSpeechRecognitionUsageDescription is missing: run the packaged .app (the bare development binary has no usage description and cannot access system speech)",
        ja: "NSSpeechRecognitionUsageDescription がありません：パッケージ済みの .app で実行してください（開発用の裸のバイナリには権限説明がなく、システム音声を利用できません）",
        ko: "NSSpeechRecognitionUsageDescription이 없습니다: 패키징된 .app에서 실행하세요(개발용 바이너리에는 권한 설명이 없어 시스템 음성을 사용할 수 없습니다)",
        fr: "NSSpeechRecognitionUsageDescription est absente : exécutez l’application .app empaquetée (le binaire de développement n’a pas de description d’autorisation et ne peut pas accéder à la parole système)",
        de: "NSSpeechRecognitionUsageDescription fehlt: Führen Sie die gepackte .app aus (das reine Entwicklungs-Binary hat keine Nutzungsbeschreibung und kann nicht auf die Systemsprache zugreifen)",
        es: "Falta NSSpeechRecognitionUsageDescription: ejecute la .app empaquetada (el binario de desarrollo no tiene descripción de uso y no puede acceder al dictado del sistema)"
    },
    AsrMacSpeechPermission {
        zh_cn: "未获得语音识别权限：请在「系统设置 → 隐私与安全性 → 语音识别」中允许 AudioScript",
        zh_tw: "未取得語音辨識權限：請在「系統設定 → 隱私權與安全性 → 語音辨識」中允許 AudioScript",
        en: "Speech recognition permission was denied: allow AudioScript in System Settings → Privacy & Security → Speech Recognition",
        ja: "音声認識の権限がありません：「システム設定 → プライバシーとセキュリティ → 音声認識」で AudioScript を許可してください",
        ko: "음성 인식 권한이 없습니다: 시스템 설정 → 개인정보 보호 및 보안 → 음성 인식에서 AudioScript를 허용하세요",
        fr: "Autorisation de reconnaissance vocale refusée : autorisez AudioScript dans Réglages Système → Confidentialité et sécurité → Reconnaissance vocale",
        de: "Spracherkennung nicht erlaubt: Erlauben Sie AudioScript in Systemeinstellungen → Datenschutz & Sicherheit → Spracherkennung",
        es: "Permiso de reconocimiento de voz denegado: permita AudioScript en Ajustes del Sistema → Privacidad y seguridad → Reconocimiento de voz"
    },
    AsrMacSpeechLocaleUnavailable {
        zh_cn: "macOS 系统语音不支持语言：{0}",
        zh_tw: "macOS 系統語音不支援語言：{0}",
        en: "macOS system speech does not support language: {0}",
        ja: "macOS システム音声は言語 {0} に対応していません",
        ko: "macOS 시스템 음성은 언어 {0}을(를) 지원하지 않습니다",
        fr: "La parole système macOS ne prend pas en charge la langue : {0}",
        de: "macOS-Systemsprache unterstützt folgende Sprache nicht: {0}",
        es: "El dictado del sistema macOS no admite el idioma: {0}"
    },
    AsrMacSpeechAssetsMissing {
        zh_cn: "macOS 尚未安装语言 {0} 的语音资产：请在「系统设置 → 键盘 → 听写语言」中添加该语言后重试",
        zh_tw: "macOS 尚未安裝語言 {0} 的語音資產：請在「系統設定 → 鍵盤 → 聽寫語言」中加入該語言後重試",
        en: "macOS has not installed speech assets for {0}: add the language in System Settings → Keyboard → Dictation Languages, then retry",
        ja: "macOS に言語 {0} の音声アセットがありません：「システム設定 → キーボード → 音声入力言語」で追加してから再試行してください",
        ko: "macOS에 언어 {0}의 음성 자산이 설치되어 있지 않습니다: 시스템 설정 → 키보드 → 받아쓰기 언어에서 추가한 뒤 다시 시도하세요",
        fr: "macOS n’a pas installé les ressources vocales pour {0} : ajoutez la langue dans Réglages Système → Clavier → Langues de dictée, puis réessayez",
        de: "macOS hat keine Sprachressourcen für {0} installiert: Fügen Sie die Sprache unter Systemeinstellungen → Tastatur → Diktiersprachen hinzu und versuchen Sie es erneut",
        es: "macOS no ha instalado los recursos de voz para {0}: añada el idioma en Ajustes del Sistema → Teclado → Idiomas de dictado y vuelva a intentarlo"
    },
    AsrMacSpeechFailed {
        zh_cn: "macOS 系统语音失败：{0}",
        zh_tw: "macOS 系統語音失敗：{0}",
        en: "macOS system speech failed: {0}",
        ja: "macOS システム音声に失敗しました：{0}",
        ko: "macOS 시스템 음성 실패: {0}",
        fr: "Échec de la parole système macOS : {0}",
        de: "macOS-Systemsprache fehlgeschlagen: {0}",
        es: "Error del dictado del sistema macOS: {0}"
    },
    AsrOfflineWorkerStartFailed {
        zh_cn: "无法启动离线 ASR 线程：{0}",
        zh_tw: "無法啟動離線 ASR 執行緒：{0}",
        en: "Failed to start offline ASR worker: {0}",
        ja: "オフライン ASR ワーカーを開始できません：{0}",
        ko: "오프라인 ASR 워커를 시작하지 못했습니다: {0}",
        fr: "Impossible de démarrer le worker ASR hors ligne : {0}",
        de: "Offline-ASR-Worker konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar el trabajador de ASR sin conexión: {0}"
    },
    AsrOfflineWorkerMissing {
        zh_cn: "离线 ASR 线程未启动",
        zh_tw: "離線 ASR 執行緒未啟動",
        en: "Offline ASR worker is not running",
        ja: "オフライン ASR ワーカーが起動していません",
        ko: "오프라인 ASR 워커가 실행 중이 아닙니다",
        fr: "Le worker ASR hors ligne n’est pas en cours d’exécution",
        de: "Offline-ASR-Worker läuft nicht",
        es: "El trabajador de ASR sin conexión no está en ejecución"
    },
    AsrOfflineWaitTimeout {
        zh_cn: "等待离线识别结果超时",
        zh_tw: "等待離線辨識結果逾時",
        en: "Timed out waiting for offline recognition",
        ja: "オフライン認識結果の待機がタイムアウトしました",
        ko: "오프라인 인식 결과를 기다리다 시간이 초과되었습니다",
        fr: "Délai dépassé en attendant la reconnaissance hors ligne",
        de: "Zeitüberschreitung beim Warten auf die Offline-Erkennung",
        es: "Se agotó el tiempo de espera del reconocimiento sin conexión"
    },

    TranslateNotInitialized {
        zh_cn: "翻译引擎未初始化",
        zh_tw: "翻譯引擎未初始化",
        en: "Translation engine is not initialized",
        ja: "翻訳エンジンが初期化されていません",
        ko: "번역 엔진이 초기화되지 않았습니다",
        fr: "Le moteur de traduction n’est pas initialisé",
        de: "Übersetzungs-Engine ist nicht initialisiert",
        es: "El motor de traducción no está inicializado"
    },
    TranslateModelLoadFailed {
        zh_cn: "模型加载失败：{0}",
        zh_tw: "模型載入失敗：{0}",
        en: "Failed to load model: {0}",
        ja: "モデルの読み込みに失敗しました：{0}",
        ko: "모델을 불러오지 못했습니다: {0}",
        fr: "Impossible de charger le modèle : {0}",
        de: "Modell konnte nicht geladen werden: {0}",
        es: "No se pudo cargar el modelo: {0}"
    },
    TranslateModelDirMissing {
        zh_cn: "模型目录缺少文件：{0}",
        zh_tw: "模型目錄缺少檔案：{0}",
        en: "Model folder is missing files: {0}",
        ja: "モデルフォルダーにファイルがありません：{0}",
        ko: "모델 폴더에 파일이 없습니다: {0}",
        fr: "Il manque des fichiers dans le dossier de modèle : {0}",
        de: "Im Modellordner fehlen Dateien: {0}",
        es: "Faltan archivos en la carpeta del modelo: {0}"
    },
    TranslatePlatformUnavailable {
        zh_cn: "macOS 本地翻译不可用：{0}\n→ 确认系统为 macOS 26+，并在「系统设置 → 通用 → 语言与地区 → 翻译语言」安装所需语言包",
        zh_tw: "macOS 本機翻譯不可用：{0}\n→ 確認系統為 macOS 26+，並在「系統設定 → 一般 → 語言與地區 → 翻譯語言」安裝所需語言包",
        en: "macOS on-device translation is unavailable: {0}\n→ Use macOS 26+, and install the language pair in System Settings → General → Language & Region → Translation Languages",
        ja: "macOS のオンデバイス翻訳を利用できません：{0}\n→ macOS 26 以降で、「システム設定 → 一般 → 言語と地域 → 翻訳言語」から必要な言語ペアをインストールしてください",
        ko: "macOS 온디바이스 번역을 사용할 수 없습니다: {0}\n→ macOS 26 이상을 사용하고, 시스템 설정 → 일반 → 언어 및 지역 → 번역 언어에서 언어 쌍을 설치하세요",
        fr: "La traduction sur l’appareil macOS est indisponible : {0}\n→ Utilisez macOS 26+ et installez la paire de langues dans Réglages Système → Général → Langue et région → Langues de traduction",
        de: "macOS-Übersetzung auf dem Gerät ist nicht verfügbar: {0}\n→ Verwenden Sie macOS 26+ und installieren Sie das Sprachpaar unter Systemeinstellungen → Allgemein → Sprache & Region → Übersetzungssprachen",
        es: "La traducción en el dispositivo macOS no está disponible: {0}\n→ Use macOS 26+ e instale el par de idiomas en Ajustes del Sistema → General → Idioma y región → Idiomas de traducción"
    },
    TranslateUnsupportedPair {
        zh_cn: "语言对不支持：{0} → {1}",
        zh_tw: "語言對不支援：{0} → {1}",
        en: "Unsupported language pair: {0} → {1}",
        ja: "未対応の言語ペア：{0} → {1}",
        ko: "지원하지 않는 언어 쌍: {0} → {1}",
        fr: "Paire de langues non prise en charge : {0} → {1}",
        de: "Nicht unterstütztes Sprachpaar: {0} → {1}",
        es: "Par de idiomas no compatible: {0} → {1}"
    },
    TranslateInvalidOutput {
        zh_cn: "译文无效（{0}）：翻译方向可能设置错误",
        zh_tw: "譯文無效（{0}）：翻譯方向可能設定錯誤",
        en: "Invalid translation ({0}): the direction may be set incorrectly",
        ja: "翻訳結果が無効です（{0}）：翻訳方向の設定が誤っている可能性があります",
        ko: "잘못된 번역({0}): 방향이 잘못 설정되었을 수 있습니다",
        fr: "Traduction invalide ({0}) : la direction est peut-être mal définie",
        de: "Ungültige Übersetzung ({0}): Die Richtung ist möglicherweise falsch eingestellt",
        es: "Traducción no válida ({0}): puede que la dirección esté mal configurada"
    },
    TranslateInternal {
        zh_cn: "引擎内部错误：{0}",
        zh_tw: "引擎內部錯誤：{0}",
        en: "Internal engine error: {0}",
        ja: "エンジン内部エラー：{0}",
        ko: "내부 엔진 오류: {0}",
        fr: "Erreur interne du moteur : {0}",
        de: "Interner Engine-Fehler: {0}",
        es: "Error interno del motor: {0}"
    },
    TranslateAutoUnsupported {
        zh_cn: "auto（尚未实现，请显式指定源语言）",
        zh_tw: "auto（尚未實作，請明確指定來源語言）",
        en: "auto (not supported yet; specify the source language explicitly)",
        ja: "auto（未対応です。元の言語を明示してください）",
        ko: "auto(아직 지원되지 않음; 원본 언어를 명시하세요)",
        fr: "auto (pas encore pris en charge ; précisez la langue source)",
        de: "auto (noch nicht unterstützt; geben Sie die Ausgangssprache explizit an)",
        es: "auto (aún no compatible; especifique el idioma de origen)"
    },
    TranslateRemedyFetchMt {
        zh_cn: "→ 运行 `pnpm fetch:mt` 补齐内置翻译模型",
        zh_tw: "→ 執行 `pnpm fetch:mt` 補齊內建翻譯模型",
        en: "→ Run `pnpm fetch:mt` to install the bundled translation model",
        ja: "→ `pnpm fetch:mt` を実行して内蔵翻訳モデルを追加してください",
        ko: "→ `pnpm fetch:mt`를 실행하여 내장 번역 모델을 설치하세요",
        fr: "→ Exécutez `pnpm fetch:mt` pour installer le modèle de traduction intégré",
        de: "→ Führen Sie `pnpm fetch:mt` aus, um das integrierte Übersetzungsmodell zu installieren",
        es: "→ Ejecute `pnpm fetch:mt` para instalar el modelo de traducción incluido"
    },
    TranslatePathMissing {
        zh_cn: "{0} 不存在。\n{1}",
        zh_tw: "{0} 不存在。\n{1}",
        en: "{0} does not exist.\n{1}",
        ja: "{0} が存在しません。\n{1}",
        ko: "{0}이(가) 존재하지 않습니다.\n{1}",
        fr: "{0} n’existe pas.\n{1}",
        de: "{0} existiert nicht.\n{1}",
        es: "{0} no existe.\n{1}"
    },
    TranslateFileMissing {
        zh_cn: "{0}/{1} 不存在。\n{2}",
        zh_tw: "{0}/{1} 不存在。\n{2}",
        en: "{0}/{1} does not exist.\n{2}",
        ja: "{0}/{1} が存在しません。\n{2}",
        ko: "{0}/{1}이(가) 존재하지 않습니다.\n{2}",
        fr: "{0}/{1} n’existe pas.\n{2}",
        de: "{0}/{1} existiert nicht.\n{2}",
        es: "{0}/{1} no existe.\n{2}"
    },
    TranslateEmptyResult {
        zh_cn: "引擎返回了空结果",
        zh_tw: "引擎傳回了空結果",
        en: "The engine returned an empty result",
        ja: "エンジンが空の結果を返しました",
        ko: "엔진이 빈 결과를 반환했습니다",
        fr: "Le moteur a renvoyé un résultat vide",
        de: "Die Engine gab ein leeres Ergebnis zurück",
        es: "El motor devolvió un resultado vacío"
    },
    TranslateSessionModelMissing {
        zh_cn: "所选翻译模型「{0}」未安装或文件不完整：{1}\n缺少：{2}\n→ 打开「模型」页下载，或切换回基础模型",
        zh_tw: "所選翻譯模型「{0}」未安裝或檔案不完整：{1}\n缺少：{2}\n→ 開啟「模型」頁下載，或切換回基礎模型",
        en: "Selected translation model \"{0}\" is not installed or incomplete: {1}\nMissing: {2}\n→ Download it on the Models page, or switch back to the base model",
        ja: "選択した翻訳モデル「{0}」が未インストールか不完全です：{1}\n不足：{2}\n→ 「モデル」ページでダウンロードするか、ベースモデルに戻してください",
        ko: "선택한 번역 모델 \"{0}\"이(가) 설치되지 않았거나 불완전합니다: {1}\n누락: {2}\n→ 모델 페이지에서 다운로드하거나 기본 모델로 다시 전환하세요",
        fr: "Le modèle de traduction sélectionné « {0} » n’est pas installé ou est incomplet : {1}\nManquant : {2}\n→ Téléchargez-le sur la page Modèles ou revenez au modèle de base",
        de: "Das ausgewählte Übersetzungsmodell „{0}“ ist nicht installiert oder unvollständig: {1}\nFehlend: {2}\n→ Laden Sie es auf der Modellseite herunter oder wechseln Sie zurück zum Basismodell",
        es: "El modelo de traducción seleccionado «{0}» no está instalado o está incompleto: {1}\nFalta: {2}\n→ Descárguelo en la página Modelos o vuelva al modelo base"
    },
    TranslateEngineNotWired {
        zh_cn: "翻译引擎 {0} 尚未接入。可选值：local-nmt（本地离线）、mock（开发用）、off（仅显示原文）",
        zh_tw: "翻譯引擎 {0} 尚未接入。可選值：local-nmt（本機離線）、mock（開發用）、off（僅顯示原文）",
        en: "Translation engine {0} is not available. Options: local-nmt (offline), mock (development), off (source text only)",
        ja: "翻訳エンジン {0} は未対応です。選択肢：local-nmt（ローカル）、mock（開発用）、off（原文のみ）",
        ko: "번역 엔진 {0}을(를) 사용할 수 없습니다. 옵션: local-nmt(오프라인), mock(개발용), off(원문만)",
        fr: "Le moteur de traduction {0} n’est pas disponible. Options : local-nmt (hors ligne), mock (développement), off (texte source uniquement)",
        de: "Übersetzungs-Engine {0} ist nicht verfügbar. Optionen: local-nmt (offline), mock (Entwicklung), off (nur Quelltext)",
        es: "El motor de traducción {0} no está disponible. Opciones: local-nmt (sin conexión), mock (desarrollo), off (solo texto de origen)"
    },

    PathNoAppData {
        zh_cn: "无法解析应用数据目录，不能保存下载的模型",
        zh_tw: "無法解析應用程式資料目錄，不能儲存下載的模型",
        en: "Cannot resolve the app data folder; downloaded models cannot be saved",
        ja: "アプリのデータフォルダーを解決できないため、モデルを保存できません",
        ko: "앱 데이터 폴더를 확인할 수 없어 다운로드한 모델을 저장할 수 없습니다",
        fr: "Impossible de localiser le dossier de données de l’application ; les modèles téléchargés ne peuvent pas être enregistrés",
        de: "App-Datenordner kann nicht ermittelt werden; heruntergeladene Modelle können nicht gespeichert werden",
        es: "No se puede resolver la carpeta de datos de la aplicación; los modelos descargados no se pueden guardar"
    },
    PathCreateFailed {
        zh_cn: "创建 {0} 失败：{1}",
        zh_tw: "建立 {0} 失敗：{1}",
        en: "Failed to create {0}: {1}",
        ja: "{0} の作成に失敗しました：{1}",
        ko: "{0} 만들기 실패: {1}",
        fr: "Impossible de créer {0} : {1}",
        de: "{0} konnte nicht erstellt werden: {1}",
        es: "No se pudo crear {0}: {1}"
    },
    PathCustomDirEmpty {
        zh_cn: "请选择一个文件夹作为模型存储位置",
        zh_tw: "請選擇一個資料夾作為模型儲存位置",
        en: "Choose a folder to store downloaded models",
        ja: "ダウンロードしたモデルの保存先フォルダーを選択してください",
        ko: "다운로드한 모델을 저장할 폴더를 선택하세요",
        fr: "Choisissez un dossier pour stocker les modèles téléchargés",
        de: "Wählen Sie einen Ordner zum Speichern heruntergeladener Modelle",
        es: "Elija una carpeta para guardar los modelos descargados"
    },
    PathCustomDirRelative {
        zh_cn: "模型存储位置必须是绝对路径：{0}",
        zh_tw: "模型儲存位置必須是絕對路徑：{0}",
        en: "The model storage folder must be an absolute path: {0}",
        ja: "モデルの保存先は絶対パスである必要があります：{0}",
        ko: "모델 저장 위치는 절대 경로여야 합니다: {0}",
        fr: "Le dossier de stockage des modèles doit être un chemin absolu : {0}",
        de: "Der Modellspeicherordner muss ein absoluter Pfad sein: {0}",
        es: "La carpeta de almacenamiento de modelos debe ser una ruta absoluta: {0}"
    },
    PathCustomDirInBundle {
        zh_cn: "不能把模型存放在应用安装目录内：{0}",
        zh_tw: "不能把模型存放在應用程式安裝目錄內：{0}",
        en: "Models cannot be stored inside the app installation folder: {0}",
        ja: "アプリのインストール先フォルダー内にモデルを保存できません：{0}",
        ko: "앱 설치 폴더 안에 모델을 저장할 수 없습니다: {0}",
        fr: "Les modèles ne peuvent pas être stockés dans le dossier d’installation de l’application : {0}",
        de: "Modelle dürfen nicht im Installationsordner der App gespeichert werden: {0}",
        es: "Los modelos no se pueden guardar dentro de la carpeta de instalación de la aplicación: {0}"
    },
    PathCustomDirNotWritable {
        zh_cn: "无法写入模型目录 {0}：{1}",
        zh_tw: "無法寫入模型目錄 {0}：{1}",
        en: "Cannot write to the model folder {0}: {1}",
        ja: "モデルフォルダー {0} に書き込めません：{1}",
        ko: "모델 폴더 {0}에 쓸 수 없습니다: {1}",
        fr: "Impossible d’écrire dans le dossier de modèles {0} : {1}",
        de: "Der Modellordner {0} ist nicht beschreibbar: {1}",
        es: "No se puede escribir en la carpeta de modelos {0}: {1}"
    },
    PathCustomDirBusy {
        zh_cn: "模型下载进行中，请结束后再更改存储位置",
        zh_tw: "模型下載進行中，請結束後再變更儲存位置",
        en: "A model download is in progress; change the storage folder after it finishes",
        ja: "モデルのダウンロード中です。完了後に保存先を変更してください",
        ko: "모델 다운로드가 진행 중입니다. 완료된 후 저장 위치를 변경하세요",
        fr: "Un téléchargement de modèle est en cours ; modifiez le dossier de stockage une fois terminé",
        de: "Ein Modelldownload läuft; ändern Sie den Speicherordner nach dessen Abschluss",
        es: "Hay una descarga de modelo en curso; cambie la carpeta de almacenamiento cuando termine"
    },
    PathNotFound {
        zh_cn: "找不到{0}。已尝试：\n  - {1}\n\n{2}",
        zh_tw: "找不到{0}。已嘗試：\n  - {1}\n\n{2}",
        en: "Cannot find {0}. Tried:\n  - {1}\n\n{2}",
        ja: "{0} が見つかりません。試行した場所：\n  - {1}\n\n{2}",
        ko: "{0}을(를) 찾을 수 없습니다. 시도한 위치:\n  - {1}\n\n{2}",
        fr: "Impossible de trouver {0}. Emplacements essayés :\n  - {1}\n\n{2}",
        de: "{0} kann nicht gefunden werden. Versucht:\n  - {1}\n\n{2}",
        es: "No se encuentra {0}. Se intentó:\n  - {1}\n\n{2}"
    },
    PathNoCandidate {
        zh_cn: "没有可用的候选目录",
        zh_tw: "沒有可用的候選目錄",
        en: "No candidate folder is available",
        ja: "利用可能な候補フォルダーがありません",
        ko: "사용 가능한 후보 폴더가 없습니다",
        fr: "Aucun dossier candidat disponible",
        de: "Kein Kandidatenordner verfügbar",
        es: "No hay ninguna carpeta candidata disponible"
    },
    PathNotExist {
        zh_cn: "{0} （不存在）",
        zh_tw: "{0} （不存在）",
        en: "{0} (does not exist)",
        ja: "{0}（存在しません）",
        ko: "{0} (존재하지 않음)",
        fr: "{0} (n’existe pas)",
        de: "{0} (existiert nicht)",
        es: "{0} (no existe)"
    },
    PathAsrLabel {
        zh_cn: "语音识别模型「{0}」",
        zh_tw: "語音辨識模型「{0}」",
        en: "speech recognition model \"{0}\"",
        ja: "音声認識モデル「{0}」",
        ko: "음성 인식 모델 \"{0}\"",
        fr: "modèle de reconnaissance vocale « {0} »",
        de: "Spracherkennungsmodell „{0}“",
        es: "modelo de reconocimiento de voz «{0}»"
    },
    PathMtLabel {
        zh_cn: "翻译模型「{0}」",
        zh_tw: "翻譯模型「{0}」",
        en: "translation model \"{0}\"",
        ja: "翻訳モデル「{0}」",
        ko: "번역 모델 \"{0}\"",
        fr: "modèle de traduction « {0} »",
        de: "Übersetzungsmodell „{0}“",
        es: "modelo de traducción «{0}»"
    },
    PathReadFailed {
        zh_cn: "无法读取 {0}：{1}",
        zh_tw: "無法讀取 {0}：{1}",
        en: "Cannot read {0}: {1}",
        ja: "{0} を読み取れません：{1}",
        ko: "{0}을(를) 읽을 수 없습니다: {1}",
        fr: "Impossible de lire {0} : {1}",
        de: "{0} kann nicht gelesen werden: {1}",
        es: "No se puede leer {0}: {1}"
    },
    PathCatalogAsr {
        zh_cn: "catalog 缺少 {0} 的 ASR 文件要求",
        zh_tw: "catalog 缺少 {0} 的 ASR 檔案要求",
        en: "catalog is missing ASR file requirements for {0}",
        ja: "catalog に {0} の ASR ファイル要件がありません",
        ko: "카탈로그에 {0}의 ASR 파일 요구 사항이 없습니다",
        fr: "le catalogue n’a pas d’exigences de fichiers ASR pour {0}",
        de: "Im Katalog fehlen ASR-Dateianforderungen für {0}",
        es: "al catálogo le faltan los requisitos de archivos ASR para {0}"
    },
    PathCatalogMt {
        zh_cn: "catalog 缺少 {0} 的 MT 文件要求",
        zh_tw: "catalog 缺少 {0} 的 MT 檔案要求",
        en: "catalog is missing MT file requirements for {0}",
        ja: "catalog に {0} の MT ファイル要件がありません",
        ko: "카탈로그에 {0}의 MT 파일 요구 사항이 없습니다",
        fr: "le catalogue n’a pas d’exigences de fichiers MT pour {0}",
        de: "Im Katalog fehlen MT-Dateianforderungen für {0}",
        es: "al catálogo le faltan los requisitos de archivos MT para {0}"
    },
    PathCatalogDirections {
        zh_cn: "catalog 未给 {0} 配置方向目录",
        zh_tw: "catalog 未為 {0} 設定方向目錄",
        en: "catalog has no direction folders for {0}",
        ja: "catalog に {0} の方向フォルダー設定がありません",
        ko: "카탈로그에 {0}의 방향 폴더가 없습니다",
        fr: "le catalogue n’a pas de dossiers de direction pour {0}",
        de: "Der Katalog hat keine Richtungsordner für {0}",
        es: "el catálogo no tiene carpetas de dirección para {0}"
    },
    PathDirMissing {
        zh_cn: "模型目录 {0} 缺少：{1}\n实际内容：{2}",
        zh_tw: "模型目錄 {0} 缺少：{1}\n實際內容：{2}",
        en: "Model folder {0} is missing: {1}\nContents: {2}",
        ja: "モデルフォルダー {0} に不足：{1}\n実際の内容：{2}",
        ko: "모델 폴더 {0}이(가) 없습니다: {1}\n내용: {2}",
        fr: "Le dossier de modèle {0} est manquant : {1}\nContenu : {2}",
        de: "Modellordner {0} fehlt: {1}\nInhalt: {2}",
        es: "Falta la carpeta del modelo {0}: {1}\nContenido: {2}"
    },
    PathBundledRemedyAsr {
        zh_cn: "→ 内置模型文件缺失：请重新安装应用；开发环境运行 `pnpm install`（或 `pnpm fetch:model`）后重启。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 內建模型檔案缺失：請重新安裝應用程式；開發環境請執行 `pnpm install`（或 `pnpm fetch:model`）後重新啟動。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Bundled model files are missing: reinstall the app; in development run `pnpm install` (or `pnpm fetch:model`) and restart.\n({0} is also searched first when set.)",
        ja: "→ 内蔵モデルファイルがありません：アプリを再インストールしてください。開発環境では `pnpm install`（または `pnpm fetch:model`）を実行して再起動してください。\n（{0} が設定されている場合も優先的に検索されます。）",
        ko: "→ 내장 모델 파일이 없습니다: 앱을 다시 설치하세요. 개발 중에는 `pnpm install`(또는 `pnpm fetch:model`)을 실행한 뒤 재시작하세요.\n({0}이(가) 설정된 경우 먼저 검색됩니다.)",
        fr: "→ Les fichiers du modèle intégré sont manquants : réinstallez l’application ; en développement, exécutez `pnpm install` (ou `pnpm fetch:model`) et redémarrez.\n({0} est aussi recherché en premier s’il est défini.)",
        de: "→ Integrierte Modelldateien fehlen: Installieren Sie die App neu; führen Sie in der Entwicklung `pnpm install` (oder `pnpm fetch:model`) aus und starten Sie neu.\n({0} wird ebenfalls zuerst durchsucht, wenn gesetzt.)",
        es: "→ Faltan archivos del modelo incluido: reinstale la aplicación; en desarrollo, ejecute `pnpm install` (o `pnpm fetch:model`) y reinicie.\n(También se busca primero {0} si está definido.)"
    },
    PathBundledRemedyMt {
        zh_cn: "→ 内置模型文件缺失：请重新安装应用；开发环境运行 `pnpm fetch:mt` 后重启。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 內建模型檔案缺失：請重新安裝應用程式；開發環境請執行 `pnpm fetch:mt` 後重新啟動。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Bundled model files are missing: reinstall the app; in development run `pnpm fetch:mt` and restart.\n({0} is also searched first when set.)",
        ja: "→ 内蔵モデルファイルがありません：アプリを再インストールしてください。開発環境では `pnpm fetch:mt` を実行して再起動してください。\n（{0} が設定されている場合も優先的に検索されます。）",
        ko: "→ 내장 모델 파일이 없습니다: 앱을 다시 설치하세요. 개발 중에는 `pnpm fetch:mt`를 실행한 뒤 재시작하세요.\n({0}이(가) 설정된 경우 먼저 검색됩니다.)",
        fr: "→ Les fichiers du modèle intégré sont manquants : réinstallez l’application ; en développement, exécutez `pnpm fetch:mt` et redémarrez.\n({0} est aussi recherché en premier s’il est défini.)",
        de: "→ Integrierte Modelldateien fehlen: Installieren Sie die App neu; führen Sie in der Entwicklung `pnpm fetch:mt` aus und starten Sie neu.\n({0} wird ebenfalls zuerst durchsucht, wenn gesetzt.)",
        es: "→ Faltan archivos del modelo incluido: reinstale la aplicación; en desarrollo, ejecute `pnpm fetch:mt` y reinicie.\n(También se busca primero {0} si está definido.)"
    },
    PathOptionalRemedyAsr {
        zh_cn: "→ 打开「模型」页下载该模型，或切换回内置模型。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 開啟「模型」頁下載該模型，或切換回內建模型。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Download this model on the Models page, or switch back to the bundled model.\n({0} is also searched first when set.)",
        ja: "→ 「モデル」ページでこのモデルをダウンロードするか、内蔵モデルに戻してください。\n（{0} が設定されている場合も優先的に検索されます。）",
        ko: "→ 모델 페이지에서 이 모델을 다운로드하거나 내장 모델로 다시 전환하세요.\n({0}이(가) 설정된 경우 먼저 검색됩니다.)",
        fr: "→ Téléchargez ce modèle sur la page Modèles ou revenez au modèle intégré.\n({0} est aussi recherché en premier s’il est défini.)",
        de: "→ Laden Sie dieses Modell auf der Modellseite herunter oder wechseln Sie zurück zum integrierten Modell.\n({0} wird ebenfalls zuerst durchsucht, wenn gesetzt.)",
        es: "→ Descargue este modelo en la página Modelos o vuelva al modelo incluido.\n(También se busca primero {0} si está definido.)"
    },
    PathOptionalRemedyMt {
        zh_cn: "→ 打开「模型」页下载该翻译模型，或切换回 OPUS-MT 基础版。\n（若设置了 {0}，它也会被优先查找。）",
        zh_tw: "→ 開啟「模型」頁下載該翻譯模型，或切換回 OPUS-MT 基礎版。\n（若設定了 {0}，它也會被優先查找。）",
        en: "→ Download this translation model on the Models page, or switch back to OPUS-MT base.\n({0} is also searched first when set.)",
        ja: "→ 「モデル」ページでこの翻訳モデルをダウンロードするか、OPUS-MT ベースに戻してください。\n（{0} が設定されている場合も優先的に検索されます。）",
        ko: "→ 모델 페이지에서 이 번역 모델을 다운로드하거나 OPUS-MT 기본으로 다시 전환하세요.\n({0}이(가) 설정된 경우 먼저 검색됩니다.)",
        fr: "→ Téléchargez ce modèle de traduction sur la page Modèles ou revenez à OPUS-MT de base.\n({0} est aussi recherché en premier s’il est défini.)",
        de: "→ Laden Sie dieses Übersetzungsmodell auf der Modellseite herunter oder wechseln Sie zurück zu OPUS-MT Basis.\n({0} wird ebenfalls zuerst durchsucht, wenn gesetzt.)",
        es: "→ Descargue este modelo de traducción en la página Modelos o vuelva a OPUS-MT base.\n(También se busca primero {0} si está definido.)"
    },
    SessionAlreadyCapturing {
        zh_cn: "已在采集中",
        zh_tw: "已在擷取中",
        en: "Already capturing",
        ja: "すでにキャプチャ中です",
        ko: "이미 캡처 중입니다",
        fr: "Capture déjà en cours",
        de: "Aufnahme läuft bereits",
        es: "Ya se está capturando"
    },
    SessionTeardownBusy {
        zh_cn: "上一次会话仍在收尾（音频设备尚未释放），请稍后重试",
        zh_tw: "上一次工作階段仍在收尾（音訊裝置尚未釋放），請稍後重試",
        en: "The previous session is still shutting down (audio device not released yet). Try again shortly.",
        ja: "前回のセッションがまだ終了処理中です（音声デバイスが未解放）。しばらくしてから再試行してください",
        ko: "이전 세션이 아직 종료 중입니다(오디오 장치가 아직 해제되지 않음). 잠시 후 다시 시도하세요.",
        fr: "La session précédente est encore en cours d’arrêt (périphérique audio non libéré). Réessayez dans un instant.",
        de: "Die vorherige Sitzung wird noch beendet (Audiogerät noch nicht freigegeben). Versuchen Sie es gleich erneut.",
        es: "La sesión anterior aún se está cerrando (el dispositivo de audio no se ha liberado). Inténtelo de nuevo en unos instantes."
    },
    SessionEngineNotWired {
        zh_cn: "引擎 {0} 尚未接入（当前支持 local-streaming / mac-native）",
        zh_tw: "引擎 {0} 尚未接入（目前支援 local-streaming / mac-native）",
        en: "Engine {0} is not available (local-streaming / mac-native are supported)",
        ja: "エンジン {0} は未対応です（現在は local-streaming / mac-native に対応）",
        ko: "엔진 {0}을(를) 사용할 수 없습니다(local-streaming / mac-native 지원)",
        fr: "Le moteur {0} n’est pas disponible (local-streaming / mac-native sont pris en charge)",
        de: "Engine {0} ist nicht verfügbar (local-streaming / mac-native werden unterstützt)",
        es: "El motor {0} no está disponible (se admiten local-streaming / mac-native)"
    },
    SessionCustomAsrMissing {
        zh_cn: "自定义 ASR 模型目录不存在：{0}",
        zh_tw: "自訂 ASR 模型目錄不存在：{0}",
        en: "Custom ASR model folder does not exist: {0}",
        ja: "カスタム ASR モデルフォルダーが存在しません：{0}",
        ko: "사용자 지정 ASR 모델 폴더가 없습니다: {0}",
        fr: "Le dossier de modèle ASR personnalisé n’existe pas : {0}",
        de: "Benutzerdefinierter ASR-Modellordner existiert nicht: {0}",
        es: "La carpeta del modelo ASR personalizado no existe: {0}"
    },
    SessionCustomAsrIncomplete {
        zh_cn: "自定义 ASR 模型目录不完整：{0}\n缺少：{1}",
        zh_tw: "自訂 ASR 模型目錄不完整：{0}\n缺少：{1}",
        en: "Custom ASR model folder is incomplete: {0}\nMissing: {1}",
        ja: "カスタム ASR モデルフォルダーが不完全です：{0}\n不足：{1}",
        ko: "사용자 지정 ASR 모델 폴더가 불완전합니다: {0}\n누락: {1}",
        fr: "Le dossier de modèle ASR personnalisé est incomplet : {0}\nManquant : {1}",
        de: "Benutzerdefinierter ASR-Modellordner ist unvollständig: {0}\nFehlend: {1}",
        es: "La carpeta del modelo ASR personalizado está incompleta: {0}\nFalta: {1}"
    },
    SessionAsrModelIncomplete {
        zh_cn: "所选语音识别模型「{0}」未安装或文件不完整：{1}\n缺少：{2}\n→ 打开「模型」页下载，或切换回内置模型",
        zh_tw: "所選語音辨識模型「{0}」未安裝或檔案不完整：{1}\n缺少：{2}\n→ 開啟「模型」頁下載，或切換回內建模型",
        en: "Selected speech recognition model \"{0}\" is not installed or incomplete: {1}\nMissing: {2}\n→ Download it on the Models page, or switch back to the bundled model",
        ja: "選択した音声認識モデル「{0}」が未インストールか不完全です：{1}\n不足：{2}\n→ 「モデル」ページでダウンロードするか、内蔵モデルに戻してください",
        ko: "선택한 음성 인식 모델 \"{0}\"이(가) 설치되지 않았거나 불완전합니다: {1}\n누락: {2}\n→ 모델 페이지에서 다운로드하거나 내장 모델로 다시 전환하세요",
        fr: "Le modèle de reconnaissance vocale sélectionné « {0} » n’est pas installé ou est incomplet : {1}\nManquant : {2}\n→ Téléchargez-le sur la page Modèles ou revenez au modèle intégré",
        de: "Das ausgewählte Spracherkennungsmodell „{0}“ ist nicht installiert oder unvollständig: {1}\nFehlend: {2}\n→ Laden Sie es auf der Modellseite herunter oder wechseln Sie zurück zum integrierten Modell",
        es: "El modelo de reconocimiento de voz seleccionado «{0}» no está instalado o está incompleto: {1}\nFalta: {2}\n→ Descárguelo en la página Modelos o vuelva al modelo incluido"
    },
    SessionAsrInitFailed {
        zh_cn: "引擎初始化失败：{0}",
        zh_tw: "引擎初始化失敗：{0}",
        en: "Failed to initialize engine: {0}",
        ja: "エンジンの初期化に失敗しました：{0}",
        ko: "엔진 초기화 실패: {0}",
        fr: "Impossible d’initialiser le moteur : {0}",
        de: "Engine konnte nicht initialisiert werden: {0}",
        es: "No se pudo inicializar el motor: {0}"
    },
    SessionTranslateThreadMissing {
        zh_cn: "翻译线程未初始化",
        zh_tw: "翻譯執行緒未初始化",
        en: "Translation worker is not initialized",
        ja: "翻訳スレッドが初期化されていません",
        ko: "번역 워커가 초기화되지 않았습니다",
        fr: "Le worker de traduction n’est pas initialisé",
        de: "Übersetzungs-Worker ist nicht initialisiert",
        es: "El trabajador de traducción no está inicializado"
    },
    SessionTranslateThreadExited {
        zh_cn: "翻译线程已退出",
        zh_tw: "翻譯執行緒已結束",
        en: "Translation worker has exited",
        ja: "翻訳スレッドが終了しました",
        ko: "번역 워커가 종료되었습니다",
        fr: "Le worker de traduction s’est arrêté",
        de: "Übersetzungs-Worker wurde beendet",
        es: "El trabajador de traducción se ha cerrado"
    },
    SessionTranslateInitFailed {
        zh_cn: "翻译引擎初始化失败：{0}",
        zh_tw: "翻譯引擎初始化失敗：{0}",
        en: "Failed to initialize the translation engine: {0}",
        ja: "翻訳エンジンの初期化に失敗しました：{0}",
        ko: "번역 엔진 초기화 실패: {0}",
        fr: "Impossible d’initialiser le moteur de traduction : {0}",
        de: "Übersetzungs-Engine konnte nicht initialisiert werden: {0}",
        es: "No se pudo inicializar el motor de traducción: {0}"
    },
    SessionStartCaptureFailed {
        zh_cn: "启动采集失败：{0}",
        zh_tw: "啟動擷取失敗：{0}",
        en: "Failed to start capture: {0}",
        ja: "キャプチャの開始に失敗しました：{0}",
        ko: "캡처 시작 실패: {0}",
        fr: "Impossible de démarrer la capture : {0}",
        de: "Aufnahme konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar la captura: {0}"
    },
    SessionEventThreadMissing {
        zh_cn: "事件线程未初始化",
        zh_tw: "事件執行緒未初始化",
        en: "Event worker is not initialized",
        ja: "イベントスレッドが初期化されていません",
        ko: "이벤트 워커가 초기화되지 않았습니다",
        fr: "Le worker d’événements n’est pas initialisé",
        de: "Ereignis-Worker ist nicht initialisiert",
        es: "El trabajador de eventos no está inicializado"
    },
    SessionEventThreadStartFailed {
        zh_cn: "启动事件线程失败：{0}",
        zh_tw: "啟動事件執行緒失敗：{0}",
        en: "Failed to start event worker: {0}",
        ja: "イベントスレッドの開始に失敗しました：{0}",
        ko: "이벤트 워커를 시작하지 못했습니다: {0}",
        fr: "Impossible de démarrer le worker d’événements : {0}",
        de: "Ereignis-Worker konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar el trabajador de eventos: {0}"
    },
    SessionTranslateThreadStartFailed {
        zh_cn: "启动翻译线程失败：{0}",
        zh_tw: "啟動翻譯執行緒失敗：{0}",
        en: "Failed to start translation worker: {0}",
        ja: "翻訳スレッドの開始に失敗しました：{0}",
        ko: "번역 워커를 시작하지 못했습니다: {0}",
        fr: "Impossible de démarrer le worker de traduction : {0}",
        de: "Übersetzungs-Worker konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar el trabajador de traducción: {0}"
    },
    SessionTeardownThreadStartFailed {
        zh_cn: "启动收尾线程失败：{0}",
        zh_tw: "啟動收尾執行緒失敗：{0}",
        en: "Failed to start teardown worker: {0}",
        ja: "終了処理スレッドの開始に失敗しました：{0}",
        ko: "종료 워커를 시작하지 못했습니다: {0}",
        fr: "Impossible de démarrer le worker d’arrêt : {0}",
        de: "Beendigungs-Worker konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar el trabajador de cierre: {0}"
    },

    PermissionProbing {
        zh_cn: "检测已在进行中",
        zh_tw: "偵測已在進行中",
        en: "A probe is already running",
        ja: "検出はすでに実行中です",
        ko: "이미 검사가 실행 중입니다",
        fr: "Un test est déjà en cours",
        de: "Es läuft bereits eine Prüfung",
        es: "Ya hay una comprobación en curso"
    },
    PermissionNoIo {
        zh_cn: "收到了音频帧，但没有任何进程在输出 I/O。请先播放一段音频再检测。",
        zh_tw: "收到了音訊影格，但沒有任何處理程序在輸出 I/O。請先播放一段音訊再偵測。",
        en: "Audio frames were received, but no process is producing output. Play some audio, then probe again.",
        ja: "音声フレームは受信しましたが、出力中のプロセスがありません。音声を再生してから再検出してください",
        ko: "오디오 프레임은 수신되었지만 출력을 생성하는 프로세스가 없습니다. 오디오를 재생한 뒤 다시 검사하세요.",
        fr: "Des trames audio ont été reçues, mais aucun processus ne produit de sortie. Lisez de l’audio, puis testez à nouveau.",
        de: "Audioframes wurden empfangen, aber kein Prozess erzeugt Ausgabe. Spielen Sie Audio ab und prüfen Sie erneut.",
        es: "Se recibieron tramas de audio, pero ningún proceso produce salida. Reproduzca audio y vuelva a comprobar."
    },
    PermissionPromptUnhandled {
        zh_cn: "整个检测窗口没有收到任何音频帧，通常意味着系统授权弹窗还停在屏幕上没被处理。请允许后再点一次「检测」。",
        zh_tw: "整個偵測視窗沒有收到任何音訊影格，通常表示系統授權提示還停在畫面上未被處理。請允許後再點一次「偵測」。",
        en: "No audio frames arrived during the probe window. The system permission prompt is probably still on screen. Allow it, then probe again.",
        ja: "検出中に音声フレームが届きませんでした。システムの許可ダイアログが画面に残っている可能性があります。許可してからもう一度「検出」を押してください",
        ko: "검사 시간 동안 오디오 프레임이 도착하지 않았습니다. 시스템 권한 프롬프트가 아직 화면에 있을 수 있습니다. 허용한 뒤 다시 검사하세요.",
        fr: "Aucune trame audio n’est arrivée pendant la fenêtre de test. La demande d’autorisation système est probablement encore à l’écran. Autorisez-la, puis testez à nouveau.",
        de: "Während des Prüffensters kamen keine Audioframes an. Die Systemberechtigungsabfrage ist möglicherweise noch sichtbar. Erlauben Sie sie und prüfen Sie erneut.",
        es: "No llegaron tramas de audio durante la ventana de comprobación. Puede que el aviso de permiso del sistema siga en pantalla. Permítalo y vuelva a comprobar."
    },
    PermissionUnbundled {
        zh_cn: "整个检测窗口没有收到任何音频帧。当前运行的不是打包后的 .app（Info.plist 里没有 NSAudioCaptureUsageDescription），系统既不会弹窗也不授权——请用 `pnpm build:app` 构建后从 .app 启动。",
        zh_tw: "整個偵測視窗沒有收到任何音訊影格。目前執行的不是封裝後的 .app（Info.plist 裡沒有 NSAudioCaptureUsageDescription），系統既不會彈出提示也不會授權——請用 `pnpm build:app` 建置後從 .app 啟動。",
        en: "No audio frames arrived during the probe window. This is not a packaged .app (Info.plist has no NSAudioCaptureUsageDescription), so the system neither prompts nor grants access. Build with `pnpm build:app` and launch from the .app.",
        ja: "検出中に音声フレームが届きませんでした。現在実行中のものはパッケージ済みの .app ではないため（Info.plist に NSAudioCaptureUsageDescription がありません）、システムは確認も許可も行いません。`pnpm build:app` でビルドし、.app から起動してください",
        ko: "검사 시간 동안 오디오 프레임이 도착하지 않았습니다. 패키징된 .app이 아니므로(Info.plist에 NSAudioCaptureUsageDescription 없음) 시스템이 묻거나 권한을 부여하지 않습니다. `pnpm build:app`으로 빌드한 뒤 .app에서 실행하세요.",
        fr: "Aucune trame audio n’est arrivée pendant la fenêtre de test. Ce n’est pas une .app empaquetée (Info.plist n’a pas NSAudioCaptureUsageDescription), donc le système ne demande ni n’accorde l’accès. Compilez avec `pnpm build:app` et lancez depuis la .app.",
        de: "Während des Prüffensters kamen keine Audioframes an. Dies ist keine gepackte .app (Info.plist hat kein NSAudioCaptureUsageDescription), daher fragt das System nicht und erteilt keinen Zugriff. Bauen Sie mit `pnpm build:app` und starten Sie aus der .app.",
        es: "No llegaron tramas de audio durante la ventana de comprobación. Esto no es una .app empaquetada (Info.plist no tiene NSAudioCaptureUsageDescription), así que el sistema no pregunta ni concede acceso. Compile con `pnpm build:app` e inicie desde la .app."
    },
    PermissionMicUnbundled {
        zh_cn: "当前运行的不是打包后的 .app，Info.plist 里没有 NSMicrophoneUsageDescription，系统不会弹窗（此时请求授权会直接终止进程）。请用 `pnpm build:app` 构建后从 .app 启动。",
        zh_tw: "目前執行的不是封裝後的 .app，Info.plist 裡沒有 NSMicrophoneUsageDescription，系統不會彈出提示（此時要求授權會直接終止處理程序）。請用 `pnpm build:app` 建置後從 .app 啟動。",
        en: "This is not a packaged .app and Info.plist has no NSMicrophoneUsageDescription, so the system will not prompt (requesting permission would terminate the process). Build with `pnpm build:app` and launch from the .app.",
        ja: "現在実行中のものはパッケージ済みの .app ではなく、Info.plist に NSMicrophoneUsageDescription がないため、システムは確認しません（この状態で権限を要求するとプロセスが終了します）。`pnpm build:app` でビルドし、.app から起動してください",
        ko: "패키징된 .app이 아니고 Info.plist에 NSMicrophoneUsageDescription이 없으므로 시스템이 묻지 않습니다(권한을 요청하면 프로세스가 종료됩니다). `pnpm build:app`으로 빌드한 뒤 .app에서 실행하세요.",
        fr: "Ce n’est pas une .app empaquetée et Info.plist n’a pas NSMicrophoneUsageDescription, donc le système ne demandera pas (demander l’autorisation terminerait le processus). Compilez avec `pnpm build:app` et lancez depuis la .app.",
        de: "Dies ist keine gepackte .app und Info.plist hat kein NSMicrophoneUsageDescription, daher fragt das System nicht (eine Berechtigungsanfrage würde den Prozess beenden). Bauen Sie mit `pnpm build:app` und starten Sie aus der .app.",
        es: "Esto no es una .app empaquetada e Info.plist no tiene NSMicrophoneUsageDescription, así que el sistema no preguntará (solicitar permiso terminaría el proceso). Compile con `pnpm build:app` e inicie desde la .app."
    },
    PermissionRequestInFlight {
        zh_cn: "授权请求已在进行中，请等待系统弹窗",
        zh_tw: "授權要求已在進行中，請等待系統提示",
        en: "A permission request is already in progress. Wait for the system prompt.",
        ja: "権限要求が進行中です。システムの確認を待ってください",
        ko: "이미 권한 요청이 진행 중입니다. 시스템 프롬프트를 기다리세요.",
        fr: "Une demande d’autorisation est déjà en cours. Attendez la demande système.",
        de: "Eine Berechtigungsanfrage läuft bereits. Warten Sie auf die Systemabfrage.",
        es: "Ya hay una solicitud de permiso en curso. Espere al aviso del sistema."
    },
    PermissionCallbackDisconnected {
        zh_cn: "授权回调通道意外断开",
        zh_tw: "授權回呼通道意外中斷",
        en: "The permission callback channel disconnected unexpectedly",
        ja: "権限コールバックのチャネルが予期せず切断されました",
        ko: "권한 콜백 채널이 예기치 않게 끊어졌습니다",
        fr: "Le canal de rappel d’autorisation s’est déconnecté de façon inattendue",
        de: "Der Berechtigungs-Callback-Kanal wurde unerwartet getrennt",
        es: "El canal de devolución de permisos se desconectó inesperadamente"
    },

    AudioDeviceNotFound {
        zh_cn: "未找到名为 {0} 的音频设备",
        zh_tw: "找不到名為 {0} 的音訊裝置",
        en: "Audio device \"{0}\" was not found",
        ja: "「{0}」という名前の音声デバイスが見つかりません",
        ko: "오디오 장치 \"{0}\"을(를) 찾을 수 없습니다",
        fr: "Le périphérique audio « {0} » est introuvable",
        de: "Audiogerät „{0}“ wurde nicht gefunden",
        es: "No se encontró el dispositivo de audio «{0}»"
    },
    AudioOpenFailed {
        zh_cn: "打开采集失败：{0}",
        zh_tw: "開啟擷取失敗：{0}",
        en: "Failed to open capture: {0}",
        ja: "キャプチャを開けませんでした：{0}",
        ko: "캡처 열기 실패: {0}",
        fr: "Impossible d’ouvrir la capture : {0}",
        de: "Aufnahme konnte nicht geöffnet werden: {0}",
        es: "No se pudo abrir la captura: {0}"
    },
    AudioStartFailed {
        zh_cn: "启动采集失败：{0}",
        zh_tw: "啟動擷取失敗：{0}",
        en: "Failed to start capture: {0}",
        ja: "キャプチャの開始に失敗しました：{0}",
        ko: "캡처 시작 실패: {0}",
        fr: "Impossible de démarrer la capture : {0}",
        de: "Aufnahme konnte nicht gestartet werden: {0}",
        es: "No se pudo iniciar la captura: {0}"
    },
    AudioEnumerateFailed {
        zh_cn: "设备枚举失败：{0}",
        zh_tw: "裝置列舉失敗：{0}",
        en: "Failed to enumerate devices: {0}",
        ja: "デバイスの列挙に失敗しました：{0}",
        ko: "장치 열거 실패: {0}",
        fr: "Impossible d’énumérer les périphériques : {0}",
        de: "Geräte konnten nicht aufgelistet werden: {0}",
        es: "No se pudieron enumerar los dispositivos: {0}"
    },
    AudioDefaultDevice {
        zh_cn: "(系统默认)",
        zh_tw: "(系統預設)",
        en: "(system default)",
        ja: "(システム既定)",
        ko: "(시스템 기본값)",
        fr: "(par défaut du système)",
        de: "(Systemstandard)",
        es: "(predeterminado del sistema)"
    },
    AudioAccumulatorBusy {
        zh_cn: "检测累加器仍被占用",
        zh_tw: "偵測累加器仍被佔用",
        en: "The probe accumulator is still in use",
        ja: "検出アキュムレータがまだ使用中です",
        ko: "검사 누산기가 아직 사용 중입니다",
        fr: "L’accumulateur de test est encore utilisé",
        de: "Der Prüf-Akkumulator wird noch verwendet",
        es: "El acumulador de comprobación aún está en uso"
    },

    UpdateNotReady {
        zh_cn: "更新组件未就绪：{0}",
        zh_tw: "更新元件未就緒：{0}",
        en: "Updater is not ready: {0}",
        ja: "アップデーターの準備ができていません：{0}",
        ko: "업데이터가 준비되지 않았습니다: {0}",
        fr: "Le programme de mise à jour n’est pas prêt : {0}",
        de: "Updater ist nicht bereit: {0}",
        es: "El actualizador no está listo: {0}"
    },
    UpdateCheckFailed {
        zh_cn: "检查更新失败：{0}",
        zh_tw: "檢查更新失敗：{0}",
        en: "Failed to check for updates: {0}",
        ja: "アップデートの確認に失敗しました：{0}",
        ko: "업데이트 확인 실패: {0}",
        fr: "Échec de la recherche de mises à jour : {0}",
        de: "Suche nach Updates fehlgeschlagen: {0}",
        es: "Error al buscar actualizaciones: {0}"
    },
    UpdateInProgress {
        zh_cn: "正在更新，请稍候",
        zh_tw: "正在更新，請稍候",
        en: "An update is already in progress. Please wait.",
        ja: "更新中です。しばらくお待ちください",
        ko: "업데이트가 이미 진행 중입니다. 잠시 기다리세요.",
        fr: "Une mise à jour est déjà en cours. Veuillez patienter.",
        de: "Ein Update läuft bereits. Bitte warten.",
        es: "Ya hay una actualización en curso. Espere."
    },
    UpdateLatest {
        zh_cn: "已是最新版本，无需更新",
        zh_tw: "已是最新版本，無需更新",
        en: "You're on the latest version",
        ja: "最新バージョンです。更新は不要です",
        ko: "최신 버전입니다",
        fr: "Vous êtes sur la dernière version",
        de: "Sie sind auf dem neuesten Stand",
        es: "Está en la última versión"
    },
    UpdateInstallFailed {
        zh_cn: "安装更新失败：{0}",
        zh_tw: "安裝更新失敗：{0}",
        en: "Failed to install the update: {0}",
        ja: "アップデートのインストールに失敗しました：{0}",
        ko: "업데이트 설치 실패: {0}",
        fr: "Échec de l’installation de la mise à jour : {0}",
        de: "Update-Installation fehlgeschlagen: {0}",
        es: "Error al instalar la actualización: {0}"
    },

    OverlayNotFound {
        zh_cn: "悬浮窗（label={0}）不存在",
        zh_tw: "懸浮窗（label={0}）不存在",
        en: "Overlay window (label={0}) does not exist",
        ja: "オーバーレイウィンドウ（label={0}）が存在しません",
        ko: "오버레이 창(label={0})이 없습니다",
        fr: "La fenêtre de superposition (label={0}) n’existe pas",
        de: "Overlay-Fenster (label={0}) existiert nicht",
        es: "La ventana de superposición (label={0}) no existe"
    },
    OverlayShowFailed {
        zh_cn: "显示悬浮窗失败：{0}",
        zh_tw: "顯示懸浮窗失敗：{0}",
        en: "Failed to show the overlay: {0}",
        ja: "オーバーレイの表示に失敗しました：{0}",
        ko: "오버레이 표시 실패: {0}",
        fr: "Impossible d’afficher la superposition : {0}",
        de: "Overlay konnte nicht eingeblendet werden: {0}",
        es: "No se pudo mostrar la superposición: {0}"
    },
    OverlayHideFailed {
        zh_cn: "隐藏悬浮窗失败：{0}",
        zh_tw: "隱藏懸浮窗失敗：{0}",
        en: "Failed to hide the overlay: {0}",
        ja: "オーバーレイの非表示に失敗しました：{0}",
        ko: "오버레이 숨기기 실패: {0}",
        fr: "Impossible de masquer la superposition : {0}",
        de: "Overlay konnte nicht ausgeblendet werden: {0}",
        es: "No se pudo ocultar la superposición: {0}"
    },
    OverlayClickThroughFailed {
        zh_cn: "设置鼠标穿透失败：{0}",
        zh_tw: "設定滑鼠穿透失敗：{0}",
        en: "Failed to set click-through: {0}",
        ja: "クリック透過の設定に失敗しました：{0}",
        ko: "클릭 통과 설정 실패: {0}",
        fr: "Impossible de définir les clics traversants : {0}",
        de: "Klickdurchlässigkeit konnte nicht gesetzt werden: {0}",
        es: "No se pudieron establecer los clics pasantes: {0}"
    },
    OverlayWindowUnavailable {
        zh_cn: "悬浮窗底层窗口不可用",
        zh_tw: "懸浮窗底層視窗不可用",
        en: "The overlay's native window is unavailable",
        ja: "オーバーレイのネイティブウィンドウが利用できません",
        ko: "오버레이의 네이티브 창을 사용할 수 없습니다",
        fr: "La fenêtre native de la superposition est indisponible",
        de: "Das native Fenster des Overlays ist nicht verfügbar",
        es: "La ventana nativa de la superposición no está disponible"
    },
    OverlaySizeFailed {
        zh_cn: "设置悬浮窗尺寸失败：{0}",
        zh_tw: "設定懸浮窗尺寸失敗：{0}",
        en: "Failed to resize the overlay: {0}",
        ja: "オーバーレイのサイズ設定に失敗しました：{0}",
        ko: "오버레이 크기 조정 실패: {0}",
        fr: "Impossible de redimensionner la superposition : {0}",
        de: "Overlay-Größe konnte nicht geändert werden: {0}",
        es: "No se pudo cambiar el tamaño de la superposición: {0}"
    },
    OverlayPositionFailed {
        zh_cn: "设置悬浮窗位置失败：{0}",
        zh_tw: "設定懸浮窗位置失敗：{0}",
        en: "Failed to move the overlay: {0}",
        ja: "オーバーレイの位置設定に失敗しました：{0}",
        ko: "오버레이 이동 실패: {0}",
        fr: "Impossible de déplacer la superposition : {0}",
        de: "Overlay konnte nicht verschoben werden: {0}",
        es: "No se pudo mover la superposición: {0}"
    },
    OverlayNoTrayIcon {
        zh_cn: "没有可用的托盘图标",
        zh_tw: "沒有可用的托盤圖示",
        en: "No tray icon is available",
        ja: "利用可能なトレイアイコンがありません",
        ko: "사용 가능한 트레이 아이콘이 없습니다",
        fr: "Aucune icône de barre des menus disponible",
        de: "Kein Menüleistensymbol verfügbar",
        es: "No hay ningún icono de barra de menús disponible"
    },

    PrefsNoConfigDir {
        zh_cn: "无法解析配置目录：{0}",
        zh_tw: "無法解析設定目錄：{0}",
        en: "Cannot resolve the config folder: {0}",
        ja: "設定フォルダーを解決できません：{0}",
        ko: "구성 폴더를 확인할 수 없습니다: {0}",
        fr: "Impossible de localiser le dossier de configuration : {0}",
        de: "Konfigurationsordner kann nicht ermittelt werden: {0}",
        es: "No se puede resolver la carpeta de configuración: {0}"
    },
    PrefsNoParent {
        zh_cn: "{0} 没有父目录，无法创建",
        zh_tw: "{0} 沒有父目錄，無法建立",
        en: "{0} has no parent folder and cannot be created",
        ja: "{0} に親フォルダーがなく作成できません",
        ko: "{0}에 상위 폴더가 없어 만들 수 없습니다",
        fr: "{0} n’a pas de dossier parent et ne peut pas être créé",
        de: "{0} hat keinen übergeordneten Ordner und kann nicht erstellt werden",
        es: "{0} no tiene carpeta principal y no se puede crear"
    },
    PrefsCreateDirFailed {
        zh_cn: "创建配置目录失败：{0}",
        zh_tw: "建立設定目錄失敗：{0}",
        en: "Failed to create the config folder: {0}",
        ja: "設定フォルダーの作成に失敗しました：{0}",
        ko: "구성 폴더 만들기 실패: {0}",
        fr: "Impossible de créer le dossier de configuration : {0}",
        de: "Konfigurationsordner konnte nicht erstellt werden: {0}",
        es: "No se pudo crear la carpeta de configuración: {0}"
    },
    PrefsSerializeFailed {
        zh_cn: "偏好序列化失败：{0}",
        zh_tw: "偏好序列化失敗：{0}",
        en: "Failed to serialize preferences: {0}",
        ja: "設定のシリアル化に失敗しました：{0}",
        ko: "기본 설정 직렬화 실패: {0}",
        fr: "Impossible de sérialiser les préférences : {0}",
        de: "Einstellungen konnten nicht serialisiert werden: {0}",
        es: "No se pudieron serializar las preferencias: {0}"
    },
    PrefsWriteTempFailed {
        zh_cn: "写入临时偏好文件失败：{0}",
        zh_tw: "寫入暫存偏好檔案失敗：{0}",
        en: "Failed to write the temporary preferences file: {0}",
        ja: "一時設定ファイルの書き込みに失敗しました：{0}",
        ko: "임시 기본 설정 파일 쓰기 실패: {0}",
        fr: "Impossible d’écrire le fichier de préférences temporaire : {0}",
        de: "Temporäre Einstellungsdatei konnte nicht geschrieben werden: {0}",
        es: "No se pudo escribir el archivo temporal de preferencias: {0}"
    },
    PrefsReplaceFailed {
        zh_cn: "替换偏好文件失败：{0}",
        zh_tw: "取代偏好檔案失敗：{0}",
        en: "Failed to replace the preferences file: {0}",
        ja: "設定ファイルの置き換えに失敗しました：{0}",
        ko: "기본 설정 파일 교체 실패: {0}",
        fr: "Impossible de remplacer le fichier de préférences : {0}",
        de: "Einstellungsdatei konnte nicht ersetzt werden: {0}",
        es: "No se pudo reemplazar el archivo de preferencias: {0}"
    },

    ExportTitle {
        zh_cn: "# AudioScript 转写记录",
        zh_tw: "# AudioScript 轉寫紀錄",
        en: "# AudioScript Transcript",
        ja: "# AudioScript 文字起こし",
        ko: "# AudioScript 전사",
        fr: "# Transcription AudioScript",
        de: "# AudioScript-Transkript",
        es: "# Transcripción de AudioScript"
    },
    ExportTime {
        zh_cn: "- 导出时间：{0}",
        zh_tw: "- 匯出時間：{0}",
        en: "- Exported at: {0}",
        ja: "- エクスポート日時：{0}",
        ko: "- 내보낸 시각: {0}",
        fr: "- Exporté le : {0}",
        de: "- Exportiert am: {0}",
        es: "- Exportado el: {0}"
    },
    ExportDirection {
        zh_cn: "- 翻译方向：{0}",
        zh_tw: "- 翻譯方向：{0}",
        en: "- Translation direction: {0}",
        ja: "- 翻訳方向：{0}",
        ko: "- 번역 방향: {0}",
        fr: "- Direction de traduction : {0}",
        de: "- Übersetzungsrichtung: {0}",
        es: "- Dirección de traducción: {0}"
    },
    ExportSegments {
        zh_cn: "- 段落数：{0}",
        zh_tw: "- 段落數：{0}",
        en: "- Segments: {0}",
        ja: "- セグメント数：{0}",
        ko: "- 세그먼트: {0}",
        fr: "- Segments : {0}",
        de: "- Segmente: {0}",
        es: "- Segmentos: {0}"
    },

    LangEnglish {
        zh_cn: "英语",
        zh_tw: "英語",
        en: "English",
        ja: "英語",
        ko: "영어",
        fr: "anglais",
        de: "Englisch",
        es: "inglés"
    },
    LangChineseSimplified {
        zh_cn: "简体中文",
        zh_tw: "簡體中文",
        en: "Simplified Chinese",
        ja: "簡体字中国語",
        ko: "중국어 간체",
        fr: "chinois simplifié",
        de: "Chinesisch (vereinfacht)",
        es: "chino simplificado"
    },
    LangChineseTraditional {
        zh_cn: "繁体中文",
        zh_tw: "繁體中文",
        en: "Traditional Chinese",
        ja: "繁体字中国語",
        ko: "중국어 번체",
        fr: "chinois traditionnel",
        de: "Chinesisch (traditionell)",
        es: "chino tradicional"
    },
    LangJapanese {
        zh_cn: "日语",
        zh_tw: "日語",
        en: "Japanese",
        ja: "日本語",
        ko: "일본어",
        fr: "japonais",
        de: "Japanisch",
        es: "japonés"
    },
    LangKorean {
        zh_cn: "韩语",
        zh_tw: "韓語",
        en: "Korean",
        ja: "韓国語",
        ko: "한국어",
        fr: "coréen",
        de: "Koreanisch",
        es: "coreano"
    },
    LangFrench {
        zh_cn: "法语",
        zh_tw: "法語",
        en: "French",
        ja: "フランス語",
        ko: "프랑스어",
        fr: "français",
        de: "Französisch",
        es: "francés"
    },
    DirectionPair {
        zh_cn: "{0} → {1}",
        zh_tw: "{0} → {1}",
        en: "{0} → {1}",
        ja: "{0} → {1}",
        ko: "{0} → {1}",
        fr: "{0} → {1}",
        de: "{0} → {1}",
        es: "{0} → {1}"
    },
    LangGerman {
        zh_cn: "德语",
        zh_tw: "德語",
        en: "German",
        ja: "ドイツ語",
        ko: "독일어",
        fr: "allemand",
        de: "Deutsch",
        es: "alemán"
    },
    LangSpanish {
        zh_cn: "西班牙语",
        zh_tw: "西班牙語",
        en: "Spanish",
        ja: "スペイン語",
        ko: "스페인어",
        fr: "espagnol",
        de: "Spanisch",
        es: "español"
    },

    PrefsUnknownAudioSource {
        zh_cn: "未知音源 {0}",
        zh_tw: "未知音源 {0}",
        en: "Unknown audio source {0}",
        ja: "不明な音声ソース {0}",
        ko: "알 수 없는 오디오 소스 {0}",
        fr: "Source audio inconnue {0}",
        de: "Unbekannte Audioquelle {0}",
        es: "Fuente de audio desconocida {0}"
    },
    PrefsUnknownSourceLang {
        zh_cn: "未知源语言 {0}（auto 尚未实现）",
        zh_tw: "未知來源語言 {0}（auto 尚未實作）",
        en: "Unknown source language {0} (auto is not supported yet)",
        ja: "不明な元言語 {0}（auto は未対応）",
        ko: "알 수 없는 원본 언어 {0} (auto는 아직 지원되지 않음)",
        fr: "Langue source inconnue {0} (auto n’est pas encore pris en charge)",
        de: "Unbekannte Ausgangssprache {0} (auto wird noch nicht unterstützt)",
        es: "Idioma de origen desconocido {0} (auto aún no es compatible)"
    },
    PrefsUnknownAsrEngine {
        zh_cn: "未知的 ASR 引擎：{0}",
        zh_tw: "未知的 ASR 引擎：{0}",
        en: "Unknown ASR engine: {0}",
        ja: "不明な ASR エンジン：{0}",
        ko: "알 수 없는 ASR 엔진: {0}",
        fr: "Moteur ASR inconnu : {0}",
        de: "Unbekannte ASR-Engine: {0}",
        es: "Motor ASR desconocido: {0}"
    },
    PrefsUnknownTargetLang {
        zh_cn: "未知目标语言 {0}",
        zh_tw: "未知目標語言 {0}",
        en: "Unknown target language {0}",
        ja: "不明な翻訳先言語 {0}",
        ko: "알 수 없는 대상 언어 {0}",
        fr: "Langue cible inconnue {0}",
        de: "Unbekannte Zielsprache {0}",
        es: "Idioma de destino desconocido {0}"
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
        assert_eq!(Locale::from_language(Language::Ko).as_str(), "ko");
        assert_eq!(Locale::from_language(Language::Fr).as_str(), "fr");
        assert_eq!(Locale::from_language(Language::De).as_str(), "de");
        assert_eq!(Locale::from_language(Language::Es).as_str(), "es");
    }

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
    fn every_key_has_non_empty_translations_in_every_locale() {
        for &key in ALL_KEYS {
            for locale in ALL_LOCALES {
                let value = text(key, locale);
                assert!(!value.trim().is_empty(), "{key:?} {locale:?}");
            }
        }
    }

    #[test]
    fn placeholders_match_across_locales() {
        for &key in ALL_KEYS {
            let expected = placeholders(text(key, Locale::ZhCn));
            for locale in ALL_LOCALES {
                assert_eq!(
                    expected,
                    placeholders(text(key, locale)),
                    "{key:?} {locale:?}"
                );
            }
        }
    }
}
