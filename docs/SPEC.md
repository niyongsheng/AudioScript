# AudioScript「声稿」— 技术规范

**版本** 2.3 · **日期** 2026-10-29 · **状态** V0.4.0 已交付

> 配色、几何与组件语言见 [design.md](./design.md)。

---

## 1. 文档概述

本文描述架构、数据模型、模块边界与风险降级。

### 1.1 一致性语言

关键词 **MUST / MUST NOT / SHOULD / SHOULD NOT / MAY / OPTIONAL** 遵循 [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)。

### 1.2 全局约定

- **真相源唯一**：任务状态、段落文本、配置 MUST 由 Rust 侧持有。前端（主窗口 / 悬浮窗）MUST NOT 持有业务真相，MUST 视为**可随时销毁重建的渲染器**。
- **时间戳** Unix 毫秒（`u64`）；**文本编码** UTF-8；**主键** UUID v4 字符串。
- **音频采样**：引擎输入 MUST 统一为 **16 kHz / 单声道 / f32**，重采样由采集层负责。

---

## 2. 项目概况

### 2.1 项目定位

跨平台桌面**离线优先同传字幕工具**：监听**系统声音输出**，在悬浮窗里**流式显示源语言文字 + 译文**。麦克风采集为次要音源（设置里可选）。

### 2.2 平台支持等级

| 平台 | 等级 | 承诺 |
|---|---|---|
| **macOS** | 一等 | 透明无边框悬浮窗、点击穿透、置顶（含**跨他人全屏**）、全局热键、托盘常驻 |
| **Windows** | 一等 | 全部特性，含跨全屏（独占全屏游戏除外） |
| **Linux** | 二等 | 核心链路（采集 → 流式 → 悬浮窗 → 托盘）。穿透 / 置顶 / 定位 MUST 运行时探测，能力缺失时 MUST 降级并在托盘提示 |

**硬限制**：GNOME Wayland 不支持 `wlr-layer-shell`，该环境下窗口置顶不可用。

**隐私承诺 MUST 按引擎逐个标注，MUST NOT 按「本地 / 远程」二分**：`mac-native` 走 macOS 26+ 系统语音，语言资产由系统下载到本机、识别在端侧完成，MUST NOT 改用 `SFSpeechRecognizer`（默认把音频上传 Apple）。

### 2.3 核心技术栈

| 层次 | 技术 |
|---|---|
| 外壳 | Tauri 2.12（features：`tray-icon` / `image-png` / `macos-private-api`） |
| 核心语言 | Rust, edition 2021 |
| 前端 | React 19.2 · TypeScript 6.0 · Vite 8.1（多入口）· Tailwind v4 |
| 前端状态 | 自建 Context store（`src/main/store.tsx`），仅 UI 瞬态 |
| Lint / Format | Biome 2.5 · pnpm 10 |
| 音频采集 | `flexaudio` 0.5.4 —— 回环直接调系统原生 API，零第三方驱动 |
| 实时 ASR | `sherpa-onnx` 1.13（默认 feature `static`，自包含） |
| 机器翻译 | `ct2rs` 0.10（features 只留 `tokenizers` / `accelerate` / `ruy`） |
| 跨全屏 | `tauri-nspanel` 2.1（代价见 §11.2 风险 1） |
| 全局热键 | `tauri-plugin-global-shortcut`（macOS 走 Carbon `RegisterEventHotKey`） |
| 日志 | `tracing` + `tracing-appender` —— 托盘应用没有控制台，日志文件是唯一的事后诊断手段 |

- **MUST NOT 引入第三方状态库**（Zustand 等）：store 只是 Rust 快照的订阅缓存（§1.2），库的模块级单例会让 IPC 订阅脱离组件、在 HMR 下重复订阅。

---

## 3. 架构

### 3.1 分层

```
┌─ Tauri v2 (Rust) ── 唯一真相源 ─────────────────────┐
│  audio       采集 + 重采样 + 有界队列 + 静音心跳      │
│  asr         trait AsrEngine → streaming / offline / mac-native │
│  translate   trait Translator → 本地 NMT / 系统翻译   │
│  transcript  TranscriptStore 三态段落状态机           │
│  overlay     创建 / 穿透 / NSPanel 换类 / 自检循环    │
│  tray        托盘常驻 + 菜单                          │
│  lifecycle   关窗拦截 + 单实例 + 开机自启             │
│  shortcut    全局热键 + 注册失败清单                  │
│  config      配置与偏好（MUST NOT `use tauri::`）     │
└──────────────────────────────────────────────────────┘
      ↕ Tauri IPC（`commands::route` 派发；事件节流 ≥100ms）
┌─ 前端 ── 纯渲染器，不持有真相 ──────────────────────┐
│  /index.html    主窗口   ← src/main/                 │
│  /overlay.html  悬浮窗   ← src/overlay/              │
│  src/shared/    类型（typeshare 生成）· IPC 封装      │
└──────────────────────────────────────────────────────┘
```

### 3.2 核心语言：纯 Rust，非 sidecar

音频采集、ASR 推理、托盘、热键、透明悬浮窗都是 Node 侧不可达的原生能力，因此核心为纯 Rust，不采用 sidecar。

### 3.3 命令派发（DebugBridge）

Tauri v2 没有「按名字从 Rust 侧调用 command」的公开 API，因此：

- `commands` MUST 定义**唯一的** `route(name, args)`（异步入口 `dispatch` → `route_async`）。每条 `#[tauri::command]` MUST 是它的薄包装，MUST NOT 写业务逻辑。
- 会阻塞的命令 MUST 走 `run_lane`：启动时预创建的 `cmd-quick` / `cmd-session` 常驻线程，按命令配超时。MUST NOT 用 `tauri::async_runtime::spawn_blocking`，也 MUST NOT 在 CT2/MKL 初始化之后为控制路径新建线程——Windows 加载器锁会让新线程停在 `LdrInitializeThunk`。
- 冒烟路径 `AUDIOSCRIPT_SMOKE_ROUTE` 仍走同步 `route`，MUST NOT 把阻塞命令加进去。

### 3.4 IPC 与前端分工

- **分层硬规则**：`asr` / `translate` / `transcript` / `audio` / `config` MUST NOT `use tauri::`——这些模块的测试不该需要 event loop。
- 广播 MUST 走 Tauri event，MUST NOT 依赖某个特定窗口存在；Rust 侧 MUST 节流至 **≥100ms 一次**再 emit。
- 窗口隐藏 MUST 调 `hide()`；无论隐藏、重建还是重新显示，前端 MUST 从 Rust 拉全量快照重建，MUST NOT 依赖 WebView 内的残留状态。

### 3.5 前端产物

| 构建入口（Vite root） | 对应代码 | 用途 |
|---|---|---|
| `/index.html` | `src/main/` | 主窗口：设置 / 历史 / 预览 |
| `/overlay.html` | `src/overlay/` | 悬浮窗：流式文本渲染器 |

- `vite.config.ts` 的 `build.rollupOptions.input` MUST 注册**全部**入口。漏注册的后果是生产构建只 emit `index.html`、悬浮窗静默回退且不报错，而 dev 模式完全正常。
- `tauri.conf.json` 中窗口的 `url` MUST NOT 带前导斜杠（写 `"overlay.html"`）。
- HTML MUST 置于构建根，MUST NOT 放 `src/` 下——Vite dev server 按相对 `root` 的路径服务文件。
- 悬浮窗 MUST 是独立轻量产物，MUST NOT 与主窗口共用一个 bundle。
- 两个入口 MUST 通过 `src/shared/` 共享类型与 IPC 封装，MUST NOT 各自重复维护。

---

## 4. 数据模型

Rust 类型是唯一真相源，TS 类型由 typeshare 生成（`pnpm gen:types` → `src/shared/types.ts`，禁止手改）。

### 4.1 引擎与配置

```ts
type EngineType = "local-streaming"   // sherpa-onnx 流式（默认）
                | "local-whisper"     // 批式，V0.2
                | "mac-native"        // macOS 26+ 系统语音
                | "remote-api"        // V0.2

type AudioSource = "mic" | "loopback" | "mix"

interface EndpointConfig {          // 不自研 VAD，直接映射引擎端点检测
  trailingSilenceMs: number
  maxUtteranceMs: number
}

interface RemoteApiConfig {
  endpoint: string
  apiKeyRef: string                 // MUST NOT 明文写入配置文件（§10.3）
  timeoutMs: number
}

interface AsrConfig {
  engineType: EngineType
  sourceLang: string
  targetLang: string
  modelDir?: string
  remoteApi?: RemoteApiConfig
  endpoint: EndpointConfig
}

interface TranslateConfig {
  engine: "off" | "local-nmt" | "mac-translation" | "remote-api" | "mock"
  sourceLang: string                // "auto" 未实现，传了明确报错
  targetLang: string
  zhScript: "simplified" | "traditional"
  modelDir?: string
  mtModelTier: "base" | "multilingual"
}

interface SessionConfig { asr: AsrConfig; translate: TranslateConfig }
```

**三个端点阈值 MUST 显式赋值**：sherpa-onnx Rust 侧的开关字段是 `enable_endpoint`，且 `OnlineRecognizerConfig::default()` 给三条规则的默认值是 `0.0`（C++ / Python 侧是 2.4 / 1.2 / 20.0）。只开开关而不管阈值，失败表现是「能出字但永不切分」——**看起来像成功**。因此 ASR 验收 MUST 断言端点确实触发，不能只断言「出字了」。

### 4.2 段落模型：三态 + 译文

```ts
type SegmentState = "partial" | "final" | "edited"

/** 译文生命周期，与 SegmentState 是两条独立的轴。 */
type TranslationState =
  | "idle"      // 还没开始翻（partial 恒为此态）
  | "pending"   // 已入队
  | "done"
  | "failed"    // UI MUST 显示原文并给出提示

interface TranscriptSegment {
  id: string
  startMs: number
  endMs: number
  text: string
  translation?: string
  translationState: TranslationState
  state: SegmentState
  revision: number   // 原文或译文任一变化都递增
}
```

```
partial  ──引擎报端点 / 静音超阈──▶  final  ──用户编辑──▶  edited
   │                                   │                    │
 可被覆盖                            冻结                 永久锁定
 不落盘                              写盘                  写盘
```

- `partial` MUST NOT 落盘；`final` 一经生成即冻结；`edited` 永久锁定，任何引擎输出 MUST NOT 覆盖。
- 崩溃最多丢失**当前尚未定稿的那一句**。

### 4.3 翻译的触发时机

**只翻译 `final`，不翻 `partial`**：partial 每 100ms 被整句改写一次，翻译它等于把算力烧在会被丢弃的文本上。

代价是**用户要等到句末才看到译文**——段落长度因此直接决定译文滞后，这是本产品最核心的体验指标。两套预设必须分开，且**默认取同传**：

| 参数 | `Interpretation`（**默认**） | `Dictation` | 理由 |
|---|---|---|---|
| `rule2_min_trailing_silence` | **0.8 s** | 1.2 s | 外语从句停顿通常 300–800ms，0.8s 切在自然停顿处 |
| `rule3_min_utterance_length` | **8 s** | 20 s | 最长句是译文滞后的**上界**，必须控制在个位数秒 |

实现为 `EndpointConfig::for_profile()`，而非「存一个 profile 字段」——后者会让 profile 与两个数值成为三个真相源。

- `EndpointConfig::default()` MUST 等于 `for_profile(Interpretation)`，有测试钉住（`default_profile_is_interpretation`）。
- 用户传入的数值 MUST 经 `sanitized()` 收敛：各自夹到合法区间，且 MUST 维持 `max_utterance_ms >= trailing_silence_ms`，否则规则 2 永远没机会触发。

### 4.4 任务

V0.1 只有内存中的当前会话：`SessionInfo { status, device?, deviceSampleRate? }`，状态为 `ready | capturing | error`。任务历史与落库属 V0.2。

### 4.5 悬浮窗配置

```ts
interface OverlayConfig {
  enable: boolean                // 默认 true：启动即显示，改动落盘
  mode: "locked" | "edit"        // 默认 "edit"，改动落盘
  alwaysOnTop: boolean           // macOS 上不被采纳，见 §8
  opacity: number                // 0 ~ 100，只作用于背景
  fontSize: number
  fontColor: string
  bgColor: string
  maxLines: number
  glow: boolean                  // 默认 false：2px 彩色描边
  glowSpeed: number              // 0 ~ 100，默认 0（静止）
}
```

---

## 5. 引擎层（ASR + 翻译）

### 5.1 抽象 Trait

```rust
pub trait AsrEngine: Send {
    fn init(&mut self, cfg: &AsrConfig) -> Result<(), AsrError>;
    fn capabilities(&self) -> EngineCaps;
    fn feed_pcm(&mut self, pcm: &[f32]) -> Result<Vec<TranscriptSegment>, AsrError>;
    fn flush(&mut self) -> Result<Vec<TranscriptSegment>, AsrError>;
    fn shutdown(&mut self);
}
```

语义契约：

- `feed_pcm` 返回 **delta**——自上次调用以来新建或变更的段落，MUST NOT 是快照。
- **同一 `id` MUST 跨 `partial → final` 保持**，否则前端会出现「同一句话出现两次」。
- 一次调用**可以同时返回一个 `final` 和一段新 `partial`**。
- `final` 一经发出，MUST NOT 再以同一 `id` 出现。
- `feed_pcm` MUST 只在单一推理线程调用（trait 是 `Send` 而非 `Sync`）；cpal 回调 MUST NOT 推理。
- `feed_pcm` MUST NOT 带采样率参数：采样率是全局不变量，协商发生在 `init(&cfg)`，不一致时 MUST 当场失败。
- 批式引擎 MUST 通过 `capabilities()` 声明，MUST NOT 扭曲 `feed_pcm` 的语义去兼容它。
- `AsrError` MUST NOT 加 `#[typeshare]`，跨 IPC 时套可序列化包装。
- `id` / `revision` / `state` MUST NOT 由各 adapter 各自实现，统一由 `asr/segment.rs` 的 `SegmentBuilder` 分配。
- `shutdown` 仅释放资源，MUST NOT 失败；调用方 MUST 先 `flush()` 再 `shutdown()`。

### 5.2 模型分发

| 引擎 | 体积 | 分发方式 |
|---|---|---|
| `local-streaming` 轻量版 | 58 MB | **内置**，首次启动零网络、零配置 |
| `x-asr` | 162 MB | 可选下载（中英流式，带标点，480ms 分块） |
| `local-streaming` 高精度版 | 190 MB | 可选下载（带进度与校验） |
| `nemotron` | 651 MB | 可选下载（40 语言流式，带标点） |
| `qwen3` | 972 MB | 可选下载（离线，句末出字） |
| `mac-native` | 无 | macOS 26+ 系统语音资产 |
| `local-whisper` / `remote-api` | — | V0.2 |

约束：

- **int8 只在 encoder / joiner 上用，decoder MUST 保持 fp32**——解码器量化对准确率的伤害不成比例。例外：Nemotron 官方归档只提供 int8 decoder，MUST 按归档使用；不得把该例外推广到其他模型。
- **X-ASR 仍是 en/zh 双语流式**：zipformer2 transducer，480ms 分块，自带标点与大小写；MUST 保持 decoder fp32，MUST NOT 因标点输出而改变 `SegmentBuilder` 的 partial/final 语义。
- **Nemotron 必须按流设置语言**：由 catalog 的 `required.asr.streamLanguage` 声明，`asr/streaming.rs` 据此调用 `OnlineStream::set_option("language", <base>)` 传入源语言；中文与中英混说质量显著低于 X-ASR，MUST NOT 把它作为 zh-en 默认档。
- **Qwen3-ASR 是离线模型**：用 Silero VAD 切句，只输出 final，MUST NOT 声称提供逐字 partial；识别跑在启动时预创建的离线线程上，MUST NOT 阻塞音频采集线程。
- **下载后 MUST 剪裁**（运行时 `models/extract.rs`，维护者脚本 `scripts/fetch-model.mjs`）：官方包里的 fp32 权重、chunk-size 变体、`test_wavs/` 全用不到，而 `bundle.resources` 是整目录复制。剪裁方案只有 `sherpa-asr-mixed-int8`（encoder/joiner int8、decoder fp32）与 `sherpa-asr-all-int8`（三者均 int8，仅 Nemotron 使用）；剪裁规则在 `models/extract.rs`，`asr/streaming.rs` 按 catalog 的 `required.asr.encoder/decoder/joiner` 选权重，两处 MUST 与 catalog 声明一致。
- **模型文件 MUST NOT 提交进 git**；`src-tauri/assets/models/` 目录本身 MUST 进仓库（`.gitkeep`），否则 `bundle.resources` 在 `cargo check` 阶段就失败。
- 内置模型 MUST 覆盖中英双语实时转写；模型切换 MUST NOT 要求重启应用；自定义模型路径 MUST 支持。
- **X-ASR / Nemotron 正式启用前 MUST 做 10 分钟真实音频 A/B**：记录 WER/CER、首 partial 位置、峰值内存与 CPU；自报指标不能替代真实业务音频。
- 可选模型下载位置 MUST 可在「模型」页更改（`modelsDir`）；下载、磁盘检查、删除与清理 MUST 跟随该目录，查找 MUST 先自定义目录、后默认应用数据目录，切换后旧目录中的模型仍可用。`AUDIOSCRIPT_MODEL_DIR` 只覆盖查找顺序，不改变下载位置。
- 构建依赖外部下载步骤：CI MUST 加缓存，且首次跑通前 MUST NOT 写进发布流程。

#### macOS 原生 ASR（实验性）

`EngineType::MacNative` 使用 **SpeechAnalyzer + SpeechTranscriber**，仅 macOS 26+ Apple Silicon 可用；其他平台或系统 MUST NOT 展示该选项。

- **桥接**：Swift-only async API 由 `src-tauri/macos/speech_bridge.swift` 暴露 C ABI（create / feed / poll / flush / destroy），`build.rs` 用 `swiftc -emit-library -static` 编成静态库；SDK < 26 时跳过编译，走不可用桩。
- **音频**：Rust 固定喂 16 kHz / 单声道 / f32，桥接层转换到 `SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith:)`；MUST NOT 阻塞采集线程。
- **结果**：声明 `streaming: true` / `produces_partial: true`，把 `volatileResults` 与 final 映射为段落 delta；句末切分由系统决定，`EndpointConfig` 不生效，UI MUST 禁用两个阈值滑杆与预设。
- **资产**：由系统 `AssetInventory` 管理，应用 MUST NOT 打包或静默下载；当前语言资产缺失时 MUST 明确报错并指出系统设置入口。
- **权限**：`Info.plist` MUST 含 `NSSpeechRecognitionUsageDescription`。裸二进制没有 Info.plist，调用会被 TCC 直接 SIGABRT——系统语音只在打包后的 `.app` 可用。
- **语言**：源语言取自翻译方向，按基础语言在 `installedLocales` 中解析（en → `en_US`、zh → `zh_CN` / `zh_TW`）。
- **质量门槛（未完成）**：正式启用前 MUST 用 10 分钟中英音频对比高精度模型的 WER / partial 延迟 / 内存。当前默认仍是 `local-streaming`。

### 5.3 翻译引擎

```rust
pub trait Translator: Send {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError>;
    fn capabilities(&self) -> TranslateCaps;
    fn translate(&mut self, text: &str) -> Result<String, TranslateError>;  // 同步阻塞
    fn shutdown(&mut self);
}
```

接口是同步的、不是流式的：本地 NMT 基本不支持增量翻译，边翻边吐只会得到一份不断被推翻的译文。`shutdown` 仅释放资源，MUST NOT 失败。

```
采集线程 → ASR → store.apply()  ──emit（原文）──▶ 悬浮窗
                    └─ final 段落入队
                              ↓
                     翻译 worker 线程（独立，单线程）
                              ↓
                   store.set_translation()  ──emit（带译文）──▶ 悬浮窗
```

- 翻译 MUST NOT 跑在采集线程上——本地 NMT 是 CPU 密集的。
- 队列 MUST 有界，溢出时丢最旧的。
- 翻译失败 MUST NOT 阻断后续段落，`translationState` 置 `failed`。
- 译文到达 MUST 走 `store` 并重新广播该段落（`revision` 递增）。

**选型：CTranslate2**——`sherpa-onnx` 静态链接 onnxruntime，同一镜像里不能再进第二份 ORT；CTranslate2 有自己的运行时。

| 模型 | 体积 | 许可 |
|---|---|---|
| `opus-mt-{en-zh,zh-en}` 转 CT2 **int8** | 每方向 82 MB | en→zh Apache-2.0；zh→en CC-BY-4.0（**MUST 署名**） |
| 可选 `nllb-200-distilled-600M` 转 CT2 **int8** | 下载 558 MB / 安装 616 MB | **CC-BY-NC-4.0**，仅限非商业用途，**MUST 在模型页展示** |
| 可选 `NiuTrans/LMT-60-0.6B` 转 CT2 **int8** | 下载 513 MB / 安装 585 MB | **Apache-2.0** |
| 可选 `NiuTrans/LMT-60-1.7B` 转 CT2 **int8** | 下载 1.43 GB / 安装 1.62 GB | **Apache-2.0** |

约束：

- **NLLB 的语言 token 是硬约束**：`encode` MUST 前置 NLLB 语言码（`zho_Hans` / `zho_Hant` / `yue_Hant` / `eng_Latn` …）、追加 `</s>`；解码 MUST 走 `translate_batch_with_target_prefix`。
- **LMT 的 chat 模板是硬约束**：prompt MUST 为 `Translate the following text from {source} into {target}:` + `{source}: {text}` + `{target}:`，并包在 `<|im_start|>user … <|im_end|>\n<|im_start|>assistant\n` 里；解码 MUST 用 `ct2rs::Generator` 并关闭 `include_prompt_in_result`。
- **MUST 用官方 `tokenizer.json`**（`HfTokenizer` 关闭后处理特殊 token），MUST NOT 自建 sentencepiece 词表。
- 只有 `Qwen3-ASR` / `Nemotron` 配合 `mt-multilingual` / `lmt-fast` / `lmt-high`（或系统翻译）才放开非 en/zh 的源语言；X-ASR / Zipformer / OPUS-MT 仍限 en/zh。可选源语言 MUST 取所选 ASR 与 MT 模型 `languages` 的交集，`Preferences::sanitized` 与启动校验 MUST 与 catalog 声明一致。
- LMT 官方只覆盖 En↔X 与 Zh↔X 的 234 个方向；非中英方向为 best effort，不保证质量；`mk` 不在训练语言内，MUST NOT 在 LMT 档展示该语言。
- 模型可选下载，MUST NOT 打进安装包；运行时按 `mtModelTier` 选择 `base` / `multilingual` / `lmt-fast` / `lmt-high`。

**macOS 26+ 系统翻译**（`mac-translation`）走 `Translation.framework`，默认 `.lowLatency`；仅 macOS 26+ 展示该选项，语言包由系统设置管理，应用 MUST NOT 尝试静默下载。目标语言 MUST 从运行时 `LanguageAvailability.supportedLanguages` 读取，MUST NOT 写死列表。

**简繁方向**：内置 OPUS-MT 只覆盖简体 ↔ 英语，繁体靠 OpenCC 规则补齐；NLLB 直接用 `zho_Hans` / `zho_Hant`；LMT 用 `Chinese` prompt 输出简体，再走规则转换。三条路径 MUST 统一走 `translate::script`，原文与译文 MUST 使用同一种用字。

三条实现硬规则（违反都会导致**静默的**无限重复输出，而非报错）：

1. MUST NOT 给 `ct2rs` 开 `sentencepiece` feature——与 sherpa 静态包内的 darts-clone 运行时冲突。
2. Marian MUST 用两个 tokenizer（`source.spm` / `target.spm` 是两套词表），`auto::Tokenizer` 只吃一个词表，对 Marian 是错的。
3. `encode` MUST 手动追加 `</s>`——Marian 需要源端 EOS 才能知道句子在哪结束。

---

## 6. 音频采集模块

| 音源 | 状态 | 后端 |
|---|---|---|
| **系统声音输出（回环）** | ✅ | macOS: Core Audio Process Tap ／ Windows: WASAPI loopback |
| 麦克风 | ✅ | 同上 |
| 麦克风 + 系统声混流 | ✅ | 库内完成混音 |
| 音频文件 | V0.2 | 批量转写走离线引擎 |

后端 MUST 用 `flexaudio` 而非 `cpal`：主场景是抓系统声音输出，`cpal` 需要用户自装 BlackHole、建多输出设备、手动路由。

```
设备原生采样率（44.1/48 kHz，可能多声道）
   ↓ flexaudio 回调线程 → 有界队列
   ↓ 工作线程 poll_chunk，按 100 ms 切块（1600 采样点）
   ↓ 推理线程（CPU 密集，与采集解耦）
TranscriptStore ← 状态机在此
   ↓ emit（节流 ≥100ms）
前端渲染器
```

约束：

- 采集回调 MUST NOT 执行分配、锁、IO 或推理。
- 推理落后过多时 MUST 丢帧而非阻塞采集。
- 采集与推理 MUST 全程运行于主窗口 WebView 生命周期之外。
- `poll_chunk()` MUST 与终结性错误检查配对：静默空转会让「捕获挂了」与「暂时没数据」看起来一模一样。
- `open()` 只建流，**`start()` 才真正开始采集**。

### 6.1 设备选择与静音诊断

系统默认输入设备可能是**虚拟回环设备**，只在其他程序把声音路由进去时才有信号，否则返回**数字静音**——而设备打开成功、回调按时触发，一切「正常」。

1. **MUST 提供输入设备选择**，枚举时给出 `isDefault` 与 `isLoopback`（后者由 flexaudio 提供）。
   - **MUST NOT 自动切换设备**，只做视觉提示。
   - 「麦克风 + 系统声音」下 MUST 提供两个独立选择器，分别写入 `mixMicDeviceId` / `mixSystemDeviceId`，MUST NOT 复用单设备 `deviceId`——flexaudio 对 Mix 会忽略它。
2. **MUST 有音频心跳**：每 5 秒统计最近窗口的 RMS，`rms < 1e-4`（约 -80 dBFS）判为静音并记 **WARN**，日志文案 MUST 明确写出「电平接近零」。阈值刻意留低：宁可漏报，也不要把很安静的房间误报成故障。
3. **静音状态变化时 MUST 广播**（`transcript://audio`）。只在状态变化时播报，每 5 秒弹一次会变成骚扰。
4. **电平表心跳**：会话采集中 MUST 广播 `transcript://level`（裸 RMS，200ms）；「音频」设置页打开期间广播 `transcript://device-level`（`{ source, device, rms }`，200ms，`device` 为 `null` 表示系统默认）。
   - 预览 MUST 在进入页面时开始，音源或设备变化时重启，离开页面 / 开始转写 / 主窗口隐藏时停止，MUST NOT 后台常驻。
   - 预览目标以 `(source, device)` 唯一标识；Mix 下 MUST 同时覆盖两侧。任一下拉展开期间 MUST 额外预览该侧全部设备。
   - 预览与转写 MUST NOT 同时持有采集设备：转写前 MUST 先停预览，失败或结束后按页面状态恢复。
   - 它与静音判定 MUST NOT 共用累加窗口：判定要 5 秒均值，表要 200ms 快照。
   - **没有「归零」事件**——广播停止时 UI MUST 自己把表清零。

---

## 7. 转写任务控制

### 7.1 状态机

```
ready ──▶ capturing
   ╲         ╱
      error
```

V0.1 只实现 `ready | capturing | error`，任一步失败进入 `error`（`transcribing` / `paused` / `finished` 属 V0.2）。任务状态 MUST 由 Rust 侧持有；托盘图标 MUST 反映后台任务状态。

### 7.2 后台常驻

- 转写任务 MUST 在主窗口隐藏 / 销毁后继续运行。
- 托盘菜单 MUST 提供：开始 / 停止转写、显示主窗口、显示 / 隐藏悬浮窗、锁定 / 编辑悬浮窗、退出。

### 7.3 编辑

- 用户编辑仅对 `final` 段落生效，编辑后 MUST 迁移为 `edited` 并立即写盘。
- 编辑 MUST NOT 影响正在进行的 `partial` 追加。

---

## 8. 悬浮字幕窗

独立的轻量 WebView 产物（`overlay.html` → `src/overlay/`），配置见 §4.5。

- **编辑态**（默认）才可拖拽 / 缩放 / 选中文字；**锁定态** MUST 完全点击穿透。热键与托盘 MUST 都能切换两态。
- **可见性与模式 MUST 落盘**：托盘 / 热键 / 主窗口三条路径都会写回 `enable` 与 `mode`。启动时按 `enable` 决定是否显示（窗口 spec 里是 `visible: false`）。
- **逃生路径 MUST 完整**：穿透一旦开启，切换入口必须存在于窗口系统之外（托盘 / 全局热键）。热键注册失败 MUST 可见（日志 + 托盘提示）。
- **macOS MUST 把窗口换类成 `NSPanel`**（`tauri-nspanel`）：Tauri 的 `set_always_on_top` 只映射到 `level = 3`，盖不住其他应用的全屏窗口。必需属性：`NSScreenSaverWindowLevel`（1000）+ `NonactivatingPanel` + `CanJoinAllApplications` + `FullScreenAuxiliary` + `CanJoinAllSpaces` + `Stationary`。
  - 属性 MUST 在主线程应用，且 MUST 在每次 Space 切换、每次重新显示之后重新断言。
  - 显示 MUST 用 `orderFrontRegardless`，MUST NOT 用 `makeKeyAndOrderFront`（会激活本应用，把用户从全屏里拽出来）。
  - 自检循环 MUST 存在：窗口掉出当前 Space 时重新断言；完全不在任何屏幕上时搬回主屏。
- 非 macOS MUST 是 no-op：`tauri.conf.json` 的 `alwaysOnTop` 已足够。

---

## 9. 应用生命周期

### 9.1 关闭语义

| 动作 | 行为 |
|---|---|
| 主窗口 ✕ | 拦截 `CloseRequested` → `prevent_close()` → `hide()`。应用存活，转写继续 |
| 悬浮窗 ✕ / 托盘隐藏 | 隐藏悬浮窗，转写 MUST NOT 受影响 |
| 托盘 → 退出 | **唯一的**真正退出路径 |

不拦截会让同一个 ✕ 按钮的行为**随悬浮窗是否可见而变化**。

### 9.2 配套

- **单实例**：MUST 第一个注册，第二次启动 MUST 唤醒并显示已有实例——否则会有两个托盘图标、两条音频管线。
- **开机自启**：SHOULD 提供开关。
- **Dock 图标**：主窗口可见时 Regular，收进托盘后退回 Accessory。唤起 MUST 走 `lifecycle::show_main()`——只 `show()` 不激活，窗口会停在别的应用底下。

---

## 10. 存储与输出

### 10.1 Markdown 导出

由 `transcript::to_markdown()` 整体生成，用户经系统保存对话框选定路径：

```markdown
# AudioScript 转写记录

- 导出时间：{datetime}
- 翻译方向：{sourceLang → targetLang}      ← 仅翻译开启时
- 段落数：{n}

---

**[mm:ss]** {段落原文}

> {段落译文}                                ← 仅有译文时
```

正文 MUST 只包含 `final` 与 `edited` 段落；时间戳超过一小时才带小时位（`mm:ss` / `h:mm:ss`）。

### 10.2 落盘时机

| 数据 | 时机 |
|---|---|
| `final` / `edited` 段落 | 存于内存真相源；SQLite 持久化属 V0.2 |
| `partial` 段落 | **不落盘** |
| Markdown | 导出时整体生成 |
| 任务元数据 / 偏好 | 状态变更时写入配置文件 |

**引导页状态（`onboardingDone`）MUST 存在偏好里，MUST NOT 只存前端**——「重置所有设置」得能把它清掉。**权限检测结论相反：MUST NOT 落盘**，它是一次性的现场观测。

### 10.3 密钥

Remote API 密钥 MUST 存入系统密钥链（Keychain / Credential Manager / Secret Service），MUST NOT 明文写入配置文件；配置只存引用（`apiKeyRef`）。

---

## 11. 风险与降级

### 11.1 失败 MUST 可见

**静默失败是本项目的主要故障形态**——设备打开成功但没有音频、引擎出字但永不切分、窗口「已显示」却不可见。因此：

- 初始化 / 注册 / 路径解析类失败 MUST 写日志，MUST NOT 被静默吞掉。
- 用户能感知的失败 MUST 同时出现在界面上并给出下一步动作。
- 资源路径解析（dev 与打包后不同）MUST 能从日志一眼看出走的是哪条分支。

### 11.2 已知风险

| # | 风险 | 处置 |
|---|---|---|
| 1 | **macOS 跨全屏置顶**——主场景的前提，Tauri 官方无此 API | 已用 `tauri-nspanel` 实现，见 §8。纯 `NSWindow` 的各条路（collectionBehavior、抬 level、`CGSSetWindowTags`、不加 `NonactivatingPanel` 的 `object_setClass`）均已实测失效，MUST NOT 重试。代价：依赖 `macos-private-api` → 无法上架 Mac App Store |
| 2 | **热键注册失败 = 逃逸路径消失** | 注册失败 MUST 可见（日志 + 托盘提示，`hotkey_failures`） |
| 3 | **Core Audio Process Tap 权限被拒时静音失败**：tap 创建成功但永远返回静音，Apple 无 API 查询 | 运行期靠 §6.1 心跳兜底；另有主动自测：先由 `is_output_active` 判断系统是否在出声，再试采 5.5s 读 `terminal_error()`。`TapRejected` = 已证明被拒；`SilentWhilePlaying` = 强指向但未证明。**前提**：`Info.plist` MUST 有 `NSAudioCaptureUsageDescription`。首次创建 tap 会弹授权窗，弹窗期间收不到任何帧——拿到 0 帧时 MUST 自动重测一次 |
| 4 | 全局热键若用 `CGEventTap` 会**连带要求辅助功能权限** | MUST 避开；只有 5 个媒体键走 `CGEventTap` |
| 5 | **冷启动 ~1.3 s**（首次出字 1.25–2.2 s，预算 200 ms，未达标），那段时间是两个模型的加载；稳态处理延迟 ≈ 0 | 在 `setup()` 里后台预建引擎、由首次 `session::start` 接管（代价：常驻内存 +约 150MB）。**尚未实施** |

### 11.3 技术前提（版本锚点）

| 前提 | 值 | 影响 |
|---|---|---|
| `tauri-runtime-wry` | **≥ 2.12.1** | 透明 / fullscreen 在 vanilla 路径下不再需要 `macOSPrivateApi` |
| `tauri-nspanel` | 2.1.1+ | 它依赖 `macos-private-api` feature，因此上一条结论**不适用于本项目**（代价见 §11.2 风险 1） |
| `sherpa-onnx`（官方 crate） | 1.13.8+ | 默认 feature `static`，自包含，无 dylib 打包 |
| `sherpa-rs`（第三方） | **已废弃** | MUST NOT 使用 |
| `Cargo.lock` | 已提交 | MUST NOT 抄用任何既有项目的 lock——`tauri 2.11.3` 会把 `tauri-runtime-wry` 钉死在 `< 2.12.1` |
| macOS | 14.2+ | Core Audio Process Tap 可用 |

---

## 12. 匿名使用统计

统计默认开启，可在「通用 → 隐私 → 匿名使用统计」关闭；关闭后 Rust 与前端都 MUST NOT 再入队新事件。

- 事件：`app_started` / `app_exited`、`session_started` / `session_stopped`、`model_download`、`overlay_toggled`、`setting_changed`、`onboarding_completed`。
- 属性只允许枚举值与分档（音源、翻译引擎、源语言、模型档位、时长分档、结果状态）。
- MUST NOT 包含转写文本、设备名称或 ID、文件路径、错误原文。
- 传输由 `tauri-plugin-aptabase` 完成；上报在后台线程，MUST NOT 阻塞启动、转写或退出。
