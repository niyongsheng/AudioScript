# AudioScript「声稿」— 技术规范

**版本** 2.1 · **日期** 2026-10-16 · **状态** V0.1 已交付

> 配色、几何与组件语言见 [design.md](./design.md)。

---

## 1. 文档概述

本文描述 AudioScript 的架构、数据模型、模块边界与风险降级。

### 1.1 一致性语言

关键词 **MUST / MUST NOT / SHOULD / SHOULD NOT / MAY / OPTIONAL** 遵循 [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)。

### 1.2 全局约定

- **真相源唯一**：任务状态、段落文本、配置 MUST 由 Rust 侧持有。前端（主窗口 / 悬浮窗）MUST NOT 持有任何业务真相，MUST 视为**可随时销毁重建的渲染器**。
- **时间戳**：Rust 侧一律 Unix 毫秒（`u64`），展示层负责本地化格式化。
- **文本编码**：UTF-8。
- **音频采样**：引擎输入 MUST 统一为 **16 kHz / 单声道 / f32**；重采样由采集层负责。
- **主键**：任务与段落 MUST 使用 UUID v4 字符串。

---

## 2. 项目概况

### 2.1 项目定位

AudioScript 是**跨平台桌面离线优先同传字幕工具**：监听**系统声音输出**，在悬浮窗里**流式显示源语言文字 + 中文翻译**。

```
┌─────────────────────────────────────────────┐
│  So the key insight here is that we need    │  ← 源文（较小、较淡）
│  to handle the latency budget…              │
│  所以这里的关键洞察是我们需要处理延迟预算…    │  ← 译文（醒目）
└─────────────────────────────────────────────┘
```

麦克风采集保留为次要音源（设置里可选）。

### 2.2 平台支持等级

| 平台 | 等级 | 承诺 |
|---|---|---|
| **macOS** | 一等 | 透明无边框悬浮窗、点击穿透、置顶（含**跨他人全屏**）、全局热键、托盘常驻 |
| **Windows** | 一等 | 全部特性，含跨全屏（独占全屏游戏除外） |
| **Linux** | 二等 | 核心链路（采集 → 流式 → 悬浮窗 → 托盘）。穿透 / 置顶 / 定位 MUST 运行时探测，能力缺失时 MUST 降级并在托盘给出明确提示（§11.4） |

**硬限制（写代码无法解决）**：GNOME Wayland 不支持 `wlr-layer-shell`，该环境下窗口置顶不可用。

**隐私承诺 MUST 按引擎逐个标注，MUST NOT 按「本地 / 远程」二分**——`mac-native` 表面上"本地"，实则默认把音频传至 Apple 服务器（只有显式设 `requiresOnDeviceRecognition = true` 才留在本地，而 `supportedLocales()` **不区分**端侧与仅服务端）。

### 2.3 核心技术栈

| 层次 | 技术 |
|---|---|
| 外壳 | Tauri 2.12（features: `tray-icon` / `image-png` / `macos-private-api`） |
| 核心语言 | Rust, edition 2021 |
| 前端 | React ^19.2 · TypeScript ~6.0 · Vite ^8.1（多入口）· Tailwind v4 |
| 前端状态 | 自建 Context store（`src/main/store.tsx`），仅 UI 瞬态 |
| 图标 | Heroicons ^2.2（24 outline） |
| Lint / Format | Biome ^2.5 · pnpm ^10 |
| 音频采集 | `flexaudio` 0.5.4 —— 回环直接调系统原生 API，零第三方驱动 |
| 实时 ASR | `sherpa-onnx` 1.13（官方 crate，默认 feature `static` 自包含） |
| 机器翻译 | `ct2rs` 0.10（features 只留 `tokenizers` / `accelerate` / `ruy`） |
| 窗口换类 | `tauri-nspanel`（macOS 跨全屏，代价见 §11.2 风险 1） |
| 全局热键 | `tauri-plugin-global-shortcut`（macOS 底层 Carbon `RegisterEventHotKey`） |
| 日志 | `tracing` + `tracing-appender`——托盘应用没有控制台，日志文件是唯一的事后诊断手段 |

约束：

- **MUST NOT 引入第三方状态库**（Zustand 等）：store 只是 Rust 快照的订阅缓存（§1.2），库的模块级单例会让 IPC 订阅脱离组件、在 HMR 下重复订阅。
- `src/ui/Icon.tsx` 是**唯一**的图标入口，注册表按**语义名**索引——换图标库时 `navigation.ts` 与各页面一行都不用改。

---

## 3. 架构

### 3.1 分层

```
┌─ Tauri v2 (Rust) ── 唯一真相源 ─────────────────────┐
│  audio       采集 + 重采样 + 有界队列 + 静音心跳      │
│  asr         trait AsrEngine → StreamingAdapter      │
│  translate   trait Translator → LocalNmt / Mock      │
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

音频采集、ASR 推理、托盘、全局热键、透明悬浮窗**全部是 Node 侧不可达的原生能力**，因此核心为纯 Rust，不采用 `Rust 外壳 + Node sidecar`。

### 3.3 命令派发（DebugBridge）

Tauri v2 **没有**「按名字从 Rust 侧调用 command」的公开 API——command 由宏生成，派发器在内部。因此：

- `commands` 模块 MUST 定义**唯一的** `route(name, args) -> Result<Value, CmdError>`（异步入口为 `dispatch`）。
- 每一条 `#[tauri::command]` MUST 是它的薄包装，MUST NOT 在包装里写业务逻辑。
- 任何批处理 / 测试路径 MUST 走同一个 `route`。
- **`route` 是同步的**，而麦克风授权申请最长等 30s、模型加载以秒计。这类会阻塞的命令 MUST 由 `route_async` 用 `spawn_blocking` 兜出去（名单在 `route_async` 的 `matches!` 里），MUST NOT 让它们占住 tokio worker。`dispatch` 走 `route_async`；冒烟路径 `AUDIOSCRIPT_SMOKE_ROUTE` 仍走同步 `route`，且 MUST NOT 把阻塞命令加进去。

> 这条约束 MUST 在写第一条 command 时就建立：晚建立意味着回头重写全部 command。

**HTTP debug 桥（未实现）**：debug 构建 SHOULD 暴露一个仅监听 `127.0.0.1` 的 HTTP 服务（`POST /debug/invoke` 走 `route`、`GET /debug/state` 返回状态快照）。当前替代手段是 `AUDIOSCRIPT_SMOKE_ROUTE` 冒烟路径与日志。落地时 MUST 以 `#[cfg(debug_assertions)]` 编译、端口冲突时静默跳过、MUST NOT 暴露任何密钥内容。

### 3.4 IPC 与前端分工

- **分层硬规则**：`asr` / `translate` / `transcript` / `audio` / `config` MUST NOT `use tauri::`——这些模块的测试不该需要 event loop。
- Rust → 前端 MUST 使用 Tauri event 广播，MUST NOT 依赖某个特定窗口存在。
- **事件节流**：Rust 侧 MUST 节流至 **≥100ms 一次**再 emit，MUST NOT 每帧直推。
- **前端再聚合**：前端 SHOULD 在收到事件后聚合若干帧再触发一次 React 重绘。
- **窗口隐藏**：主窗口被 ✕ 隐藏时 MUST 调用 `hide()`。稳定版 Tauri 不支持「销毁 WebView 而保留窗口」，本应用不追求省内存，只保证不中断后台任务。
- **快照重建**：窗口无论隐藏、重建还是重新显示，前端 MUST 从 Rust 拉取全量快照重建，MUST NOT 依赖 WebView 内的任何残留状态。悬浮窗同样处理。

### 3.5 前端产物

**HTML 入口置于构建根，TS 代码置于 `src/` 下**：

| 构建入口（Vite root） | 对应代码 | 用途 |
|---|---|---|
| `/index.html` | `src/main/` | 主窗口：设置 / 历史 / 预览 |
| `/overlay.html` | `src/overlay/` | 悬浮窗：流式文本渲染器。MUST NOT 包含主窗口页面代码 |

两者 MUST 通过 `src/shared/` 共享类型定义与 IPC 封装，MUST NOT 各自重复维护。

约束：

- `vite.config.ts` 的 `build.rollupOptions.input` MUST 注册**全部**入口。漏注册的后果是**生产构建只 emit `index.html`，悬浮窗静默回退显示主界面且不报错**——而 dev 模式完全正常。因此验证清单 MUST 包含一次 `cargo tauri build`。
- `tauri.conf.json` 中窗口的 `url` MUST NOT 带前导斜杠（写 `"overlay.html"`）。
- **HTML 不放 `src/` 里**：Vite dev server 按相对 `root` 的路径服务文件，放在 `src/` 下时 dev URL 与 `url: "overlay.html"` 对不上。
- 悬浮窗 MUST 是独立轻量产物——共用单一 bundle 会让它每次启动都解析整个应用。

---

## 4. 数据模型

Rust 类型是唯一真相源，TS 类型由 typeshare 生成（`pnpm gen:types` → `src/shared/types.ts`，禁止手改）。

### 4.1 引擎与配置

```ts
type EngineType = "local-streaming"   // sherpa-onnx 流式 —— V0.1 唯一引擎
                | "local-whisper"     // 批式，V0.2
                | "mac-native"        // macOS SFSpeech，V0.2（默认上传 Apple）
                | "remote-api"        // V0.2

type AudioSource = "mic" | "loopback" | "mix"

interface EndpointConfig {          // 不自研 VAD，直接映射到引擎端点检测
  trailingSilenceMs: number         // → rule2_min_trailing_silence
  maxUtteranceMs: number            // → rule3_min_utterance_length
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
  engine: "off" | "local-nmt" | "remote-api" | "mock"
  sourceLang: string                // "auto" 未实现，传了明确报错——猜错方向是静默失败
  targetLang: string
  zhScript: "simplified" | "traditional"  // 中文一侧的用字，默认 simplified
  modelDir?: string
}

interface SessionConfig { asr: AsrConfig; translate: TranslateConfig }
```

**端点阈值（三条都必须显式赋值）**：sherpa-onnx Rust 侧的开关字段名是 **`enable_endpoint`**，且 `OnlineRecognizerConfig::default()` 给三条规则的默认值是 **`0.0`**（C++ / Python 侧是 2.4 / 1.2 / 20.0，Rust 侧不对齐）。

> **三个阈值 MUST 显式赋值，MUST NOT 依赖 `Default`。** 只开开关而不管阈值，失败表现是「能出字但永不切分」——**看起来像成功**。因此 ASR 验收 MUST 显式断言端点确实触发，而不能只断言「出字了」。

### 4.2 段落模型：三态 + 译文

```ts
type SegmentState = "partial" | "final" | "edited"

/** 译文的生命周期。与 SegmentState 是两条独立的轴。 */
type TranslationState =
  | "idle"      // 还没开始翻（partial 阶段恒为此态）
  | "pending"   // 已入队，翻译中
  | "done"      // 译文就绪
  | "failed"    // 翻译失败，UI MUST 显示原文并给出提示

interface TranscriptSegment {
  id: string
  startMs: number
  endMs: number
  text: string
  translation?: string
  translationState: TranslationState
  state: SegmentState
  revision: number   // 原文或译文任一变化都递增；前端据此判断是否需要重绘
}
```

**状态迁移**：

```
partial  ──引擎报端点 / 静音超阈──▶  final  ──用户手动编辑──▶  edited
   │                                   │                        │
 可被覆盖                            冻结                    永久锁定
 不落盘                              写盘                    写盘
```

不变量：

- `partial` MUST NOT 落盘（SQLite / Markdown）。
- `final` 一经生成即冻结，MUST NOT 再被任何引擎输出覆盖。
- `edited` 永久锁定，任何引擎输出 MUST NOT 覆盖之。
- 崩溃最多丢失**当前尚未定稿的那一句**。

### 4.3 翻译的触发时机

**只翻译 `final` 段落，不翻 `partial`**：`partial` 每 100ms 就被整句改写一次，翻译它等于把算力烧在会被丢弃的文本上，且译文会跟着闪烁。

代价是**用户要等到句末才能看到译文**——段落长度因此直接决定**译文滞后多久**，这是本产品最核心的体验指标。两套预设必须分开，且**默认取同传**：

| 参数 | `Interpretation`（**默认**） | `Dictation` | 理由 |
|---|---|---|---|
| `rule2_min_trailing_silence` | **0.8 s** | 1.2 s | 外语语音的从句停顿通常 300–800ms；0.8s 切在自然停顿处，又不会碎到影响翻译质量 |
| `rule3_min_utterance_length` | **8 s** | 20 s | 最长句是译文滞后的**上界**，必须控制在个位数秒 |

实现为 `EndpointConfig::for_profile()`，而非"存一个 profile 字段"——后者会让 `profile` 与两个数值成为三个真相源。

约束：

- `EndpointConfig::default()` MUST 等于 `for_profile(Interpretation)`；有测试钉住（`default_profile_is_interpretation`）。
- 用户传入的数值 MUST 经 `sanitized()` 收敛：`trailing_silence_ms` 与 `max_utterance_ms` 各自夹到合法区间，且 MUST 维持 `max_utterance_ms >= trailing_silence_ms`——否则规则 2 永远没机会触发。

### 4.4 任务

```ts
interface TranscribeTask {          // 设计；V0.1 只有内存中的当前会话
  id: string
  createdAt: number
  audioSource: AudioSource
  config: SessionConfig
  segments: TranscriptSegment[]
  mdFilePath: string
  audioFilePath?: string
  status: "ready" | "capturing" | "transcribing"
        | "paused" | "finished" | "error"
}
```

V0.1 实现 `ready | capturing | error`（见 §7.1），任务历史与落库属 V0.2。

### 4.5 悬浮窗配置

```ts
interface OverlayConfig {
  enable: boolean                // 默认 true：启动即显示，显示/隐藏会落盘
  mode: "locked" | "edit"        // 默认 "edit"，改动落盘
  alwaysOnTop: boolean           // macOS 上不被采纳，见 §8
  opacity: number                // 0 ~ 100，只作用于背景，文字始终不透明
  fontSize: number
  fontColor: string
  bgColor: string                // 支持 rgba 半透明
  maxLines: number
  glow: boolean                  // 默认 true：2px 彩色渐变描边，无外发光
}
```

---

## 5. 引擎层（ASR + 翻译）

### 5.1 抽象 Trait

```rust
pub trait AsrEngine: Send {
    fn init(&mut self, cfg: &AsrConfig) -> Result<(), AsrError>;

    /// 路由依据：是否支持流式、是否产出 partial、是否要求整段输入等。
    fn capabilities(&self) -> EngineCaps;

    /// `pcm` MUST 为 16 kHz / 单声道 / f32（全局不变量，见 §1.2）。
    fn feed_pcm(&mut self, pcm: &[f32]) -> Result<Vec<TranscriptSegment>, AsrError>;

    /// 冲刷尾部：等价于「结束输入 + 排空解码循环 + 返回末尾段落」。
    fn flush(&mut self) -> Result<Vec<TranscriptSegment>, AsrError>;

    /// 仅释放资源，MUST NOT 失败。调用方 MUST 先 `flush()` 再 `shutdown()`。
    fn shutdown(&mut self);
}
```

**语义契约（不写下来就会静默出错）**：

- `feed_pcm` 返回的是 **delta**——自上次调用以来**新建或变更**的段落，MUST NOT 是快照。
- **同一 `id` MUST 跨 `partial → final` 保持**。它是"同一段的定稿"，不是新段；违反此条会让前端出现「同一句话出现两次」。
- 一次调用**可以同时返回一个 `final` 和一段新 `partial`**（端点触发 + 新句开始），调用方 MUST 能处理混合状态。
- `final` 一经发出，MUST NOT 再以同一 `id` 出现（§4.2 不变量）。
- `feed_pcm` MUST 只在单一推理线程调用——trait 是 `Send` 而非 `Sync`，这就是它的含义（cpal 回调 MUST NOT 推理，见 §6）。

接口约束：

- `feed_pcm` **MUST NOT 带采样率参数**：采样率是全局不变量，再往下传只会制造「按错误速率解码、不报错、直接输出垃圾文本」这类静默错误。协商发生在 `init(&cfg)`——模型期望采样率与配置不一致时 MUST 当场失败。
- **批式引擎（whisper）MUST 通过 `capabilities()` 声明**，MUST NOT 扭曲 `feed_pcm` 的语义去兼容它；UI 据此提示「该引擎不提供实时字幕」。
- `AsrError` 是纯 Rust 内部错误类型，**MUST NOT 加 `#[typeshare]`**——需要跨 IPC 时套一个可序列化的包装。

**段落的 id / revision 归属**：`id`（UUID）、`revision`、`state` MUST NOT 由各 adapter 各自实现——由 `asr/segment.rs` 的共享 `SegmentBuilder` 统一分配：以「引擎自增的 utterance 序号」为 key，负责分配 UUID、维护 `revision`、在端点时把 `state` 翻成 `final`；adapter 只提供原生字段（`text` / `start` / `end` / `is_final` / `utterance_index`）。

### 5.2 模型分发

**分档，不统一**：

| 引擎 | 体积 | 分发方式 |
|---|---|---|
| `local-streaming` 小模型 | ~58 MB | **内置**。首次启动零网络、零配置 |
| `local-streaming` 高精度双语 | ~200 MB | 可选下载（带进度与校验） |
| `local-whisper` | GB 级 | 用户自行下载，应用内给指引（V0.2） |
| `mac-native` / `remote-api` | 无 | 依赖系统语言包 / 用户填配置（V0.2） |

| 用途 | 模型 | 文件 |
|---|---|---|
| **V0.1 内置** | `sherpa-onnx-streaming-zipformer-small-bilingual-zh-en-2023-02-16` | encoder int8 41 MB + decoder **fp32** 13 MB + joiner int8 3.1 MB + `tokens.txt` + `bpe.model` ≈ **58 MB** |
| 可选高精度 | `sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20` | encoder int8 174–182 MB + decoder 13 MB + joiner int8 3.1 MB + tokens ≈ 200 MB |

约束：

- **int8 只在 encoder / joiner 上用，decoder MUST 保持 fp32**——官方示例的 int8 用法就是混合的，解码器量化对准确率的伤害不成比例。
- **下载后 MUST 剪裁**（`scripts/fetch-model.mjs`）：官方包里的 fp32 权重、`64/` `96/` chunk-size 变体、`test_wavs/` 全用不到，而 `bundle.resources` 是整目录复制、不做过滤——不剪裁时 `Contents/Resources/models/` 达 474 MB，剪裁后 58 MB。
- 剪裁规则与 `asr/streaming.rs` 的 `Precision` **耦合**（按「encoder/joiner 取 int8、decoder 取 fp32」保留唯一一套权重，避免 `discover_model_files` 歧义）——改一处必须同步改另一处。
- **模型文件 MUST NOT 提交进 git**：由 `scripts/fetch-model.mjs` 从官方 Releases 下载并剪裁到 `src-tauri/assets/models/`，挂 `package.json` 的 `prepare` 钩子，该目录进 `.gitignore`。
- **`src-tauri/assets/models/` 目录本身 MUST 进仓库**（`.gitkeep`）：`bundle.resources` 指向的路径在 `cargo check` 阶段就必须存在，否则构建直接失败。
- 内置模型 MUST 覆盖中英双语实时转写；模型切换 MUST NOT 要求重启应用；自定义模型路径 MUST 支持。

> 构建从此依赖一个外部下载步骤：配 CI 时 MUST 加缓存，且在首次跑通前 MUST NOT 写进发布流程。

### 5.3 翻译引擎

```rust
pub trait Translator: Send {
    fn init(&mut self, cfg: &TranslateConfig) -> Result<(), TranslateError>;
    fn capabilities(&self) -> TranslateCaps;

    /// 翻译一句。**同步阻塞**——调用方负责放到工作线程上。
    fn translate(&mut self, text: &str) -> Result<String, TranslateError>;

    /// 仅释放资源，MUST NOT 失败。
    fn shutdown(&mut self);
}
```

接口是同步的、不是流式的：本地 NMT 基本不支持增量翻译，边翻边吐只会得到一份不断被推翻的译文。调用方（翻译 worker 线程）负责调度，将来接在线 API 时这个 trait 不用改。

**调度模型**：

```text
采集线程 → ASR → store.apply()  ──emit（原文）──▶ 悬浮窗
                    │
                    └─ final 段落入队（只入队 final）
                              ↓
                        翻译 worker 线程（独立，单线程）
                              ↓
                        store.set_translation()  ──emit（带译文）──▶ 悬浮窗
```

- 翻译 MUST NOT 跑在采集线程上——本地 NMT 是 CPU 密集的，会直接导致音频积压被丢弃。
- 队列 MUST 有界，溢出时丢最旧的——用户看到的是实时内容，补上的译文也已经"过期"。
- 翻译失败 MUST NOT 阻断后续段落，`translationState` 置 `failed` 供 UI 展示。
- 译文到达 MUST 走 `store` 并重新广播该段落（`revision` 递增），MUST NOT 让前端自己补齐。

**选型：CTranslate2**——`sherpa-onnx` 静态链接 onnxruntime，同一镜像里不能再进第二份 ORT（无上游解法）；CTranslate2 有自己的运行时，与 sherpa 零冲突。

| 项 | 值 |
|---|---|
| crate | `ct2rs` 0.10（features 只留 `tokenizers` / `accelerate` / `ruy`） |
| 模型 | `Helsinki-NLP/opus-mt-{en-zh,zh-en}` 转 CTranslate2 **int8**，每方向 76 MB |
| 许可 | en→zh Apache-2.0；zh→en CC-BY-4.0（**MUST 在关于页署名**） |
| 延迟 | 70–77 ms/句（release，M 系列 CPU，beam=1） |

**简繁方向**：本地模型只覆盖简体 ↔ 英语，繁体方向靠 OpenCC 规则转换补齐，MUST 统一走 `translate::script`：

- `繁体中文 → 英语`：ASR 原文先转繁体（展示、导出与翻译任务用同一份文本），送入模型前再转简体；
- `英语 → 繁体中文`：模型输出简体后转繁体。

原文与译文 MUST 使用同一种用字，否则悬浮窗会出现「简体原文 + 繁体译文」的割裂。

三条实现硬规则（全部会导致**静默的**无限重复输出，而非报错）：

1. **MUST NOT 给 `ct2rs` 开 `sentencepiece` feature**——它与 sherpa 静态包内的 darts-clone 是运行时冲突。走 `tokenizers`（HF 纯 Rust）。
2. **Marian MUST 用两个 tokenizer**（`source.spm` / `target.spm` 是两套独立词表），实现 `MarianTokenizer` 持有两份、`encode` 走源端 `decode` 走目标端；`auto::Tokenizer` 只吃一个词表，对 Marian 是错的。
3. **`encode` MUST 手动追加 `</s>`**——Marian 需要源端 EOS 标记才知道句子在哪结束。

---

## 6. 音频采集模块

| 音源 | 状态 | 后端 |
|---|---|---|
| **系统声音输出（回环）** | ✅ 已实测 | macOS: Core Audio Process Tap ／ Windows: WASAPI loopback |
| 麦克风 | ✅ | 同上 |
| 麦克风 + 系统声混流 | ✅ | 库内完成混音 |
| 音频文件 | V0.2 | 批量转写走离线引擎 |

后端 MUST 用 `flexaudio` 而非 `cpal`：主场景是抓**系统声音输出**，`cpal` 需要用户自装 BlackHole、建多输出设备、手动路由。flexaudio 直接调系统原生 API，零第三方驱动，实测零丢弃。

**采集管道**：

```
设备原生采样率（44.1/48 kHz，可能多声道）
   ↓ flexaudio 回调线程
有界队列
   ↓ 工作线程 poll_chunk
按 100 ms 切块（1600 采样点）
   ↓
推理线程（CPU 密集，与采集解耦）
   ↓ 段落（delta）
TranscriptStore ← 状态机在此
   ↓ emit（节流 ≥100ms）
前端渲染器
```

采样率转换由库完成（rubato 带抗混叠），我们直接要 16 kHz 单声道。

约束：

- 采集回调 MUST NOT 执行分配、锁、IO 或推理。
- 推理落后过多时 MUST 丢帧而非阻塞采集。
- 采集与推理 MUST 全程运行于主窗口 WebView 生命周期之外——**主窗口销毁不得中断采集**。
- `poll_chunk()` MUST 与终结性错误检查配对：静默空转会让"捕获挂了"与"暂时没数据"看起来一模一样。
- `open()` 只建流，**`start()` 才真正开始采集**——少调 `start()` 的表现是零错误、零日志、永远拿不到数据。

### 6.1 设备选择与静音诊断

**这不是可选的体验优化，是产品可用性的前提**：系统默认输入设备可能是**虚拟回环设备**，只在其他程序把声音路由进去时才有信号，否则返回**数字静音**——而设备打开成功、流播放成功、回调按时触发，一切"正常"。

1. **MUST 提供输入设备选择**。枚举时同时给出 `isDefault` 与 `isVirtual` 标志。
   - `isVirtual` 是**启发式**（按名称匹配 `BlackHole` / `Loopback` / `Soundflower` / `Virtual` / `Aggregate`）——CoreAudio 没有"这是虚拟设备"的标志位。
   - **MUST NOT 自动切换设备**，只做视觉提示——真的想录系统声音的用户会被自动切换误伤。
   - 设备选择器 MUST 用自定义浮层：每个选项同时给出设备名与实时电平；原生 `<select>` 的 `<option>` 渲染不了动态内容。
   - 「麦克风 + 系统声音」下 MUST 提供**两个**独立选择器（麦克风 / 系统声音），分别写入 `Preferences.mixMicDeviceId` 与 `mixSystemDeviceId`，并映射到 flexaudio 的 `mix_mic_device_id` / `mix_system_device_id`；MUST NOT 复用单设备 `device_id`——flexaudio 对 Mix 会忽略它。
2. **MUST 有音频心跳**：每 5 秒统计最近窗口的 RMS。
   - `rms < 1e-4`（约 -80 dBFS）判定为静音并记 **WARN**，日志文案 MUST 明确写出"电平接近零"而不是含糊的"正常"。
   - 阈值刻意留低：真实环境即使安静也有本底噪声，而全零缓冲、被 deny 的麦克风、选错的设备都会精确给出接近 0 的值——宁可漏报，也不要把"很安静的房间"误报成故障。
3. **静音状态变化时 MUST 广播给 UI**（事件 `transcript://audio`），UI 提示用户换设备。**只在状态变化时播报**——每 5 秒弹一次提示会变成骚扰。
4. **电平表**：会话采集中 MUST 广播输入电平（事件 `transcript://level`，负载是裸 RMS，间隔 200ms）。
   - 「音频」设置页打开期间 MUST 提供输入电平预览（事件 `transcript://device-level`，负载 `{ source, device, rms }`，间隔 200ms；`device` 为 `null` 表示系统默认）：进入页面自动开始；音源或设备变化时 MUST 重启；离开页面、开始转写或主窗口隐藏时 MUST 停止。预览 MUST NOT 在后台常驻。
   - 预览目标 MUST 以 `(source, device)` 唯一标识；Mix 下 MUST 同时覆盖麦克风侧与系统侧。任一下拉展开期间 MUST 额外预览该侧列表中的全部设备，让用户一眼看出哪一路有输出；收起后 MUST 只保留当前选中设备。单个设备启动失败 MUST 只让该路没有电平，MUST NOT 影响其余设备。
   - 预览与转写 MUST NOT 同时持有采集设备：转写开始前 MUST 先停止预览，转写失败或结束后按页面状态恢复。macOS 的采集指示灯随采集流常亮，预览必须严格跟随页面与主窗口可见性。
   - 它和静音判定 MUST NOT 共用累加窗口：判定要 5 秒平均值（抗瞬时停顿），表要 200ms 快照（跟手）。
   - **没有"归零"事件**——电平广播停止（会话结束、离开预览页或主窗口隐藏）时，UI MUST 自己把表清零。

---

## 7. 转写任务控制

### 7.1 状态机

```
ready ──▶ capturing ──▶ transcribing ──▶ finished
              │              │
              └──▶ paused ◀──┘
                     │
                     └──▶ error
```

V0.1 实现到 `capturing`（另有 `error`）。任务状态 MUST 由 Rust 侧持有；托盘图标 MUST 反映后台任务状态。

### 7.2 后台常驻

- 转写任务 MUST 在主窗口隐藏 / 销毁后继续运行。
- 托盘菜单 MUST 提供：开始 / 暂停 / 停止转写、显示主窗口、显示 / 隐藏悬浮窗、锁定 / 编辑悬浮窗、退出。

### 7.3 编辑

- 用户编辑仅对 `final` 段落生效。
- 编辑后段落状态 MUST 迁移为 `edited` 并立即写盘。
- 编辑操作 MUST NOT 影响正在进行的 `partial` 追加。

---

## 8. 悬浮字幕窗

独立的轻量 WebView 产物（`overlay.html` → `src/overlay/`），配置见 §4.5。

- **编辑态**（默认）才可拖拽 / 缩放 / 选中文字；**锁定态** MUST 完全点击穿透：不拦鼠标、不抢焦点、不进窗口循环。热键与托盘 MUST 都能切换两态。
- **可见性与模式 MUST 落盘**：托盘 / 热键 / 主窗口按钮三条路径都会写回 `enable` 与 `mode`——漏掉一条的表现是「用户藏起来的悬浮窗下次启动自己冒出来」。启动时按 `enable` 决定显不显示（窗口 spec 里是 `visible: false`）。
- **逃生路径 MUST 完整**：穿透一旦开启，用户无法点击悬浮窗把它关掉，切换入口必须存在于窗口系统之外（托盘 / 全局热键）。因此热键注册失败 MUST 可见（日志 + 托盘提示）——静默忽略等于把用户锁死。
- **macOS MUST 把窗口换类成 `NSPanel`**（`tauri-nspanel`）：Tauri 的 `set_always_on_top` 只映射到 `level = 3`，**盖不住其他应用的全屏窗口**。必需属性：`NSScreenSaverWindowLevel`（1000）+ `NonactivatingPanel` + `CanJoinAllApplications` + `FullScreenAuxiliary` + `CanJoinAllSpaces` + `Stationary`。`always_on_top` 因此在 macOS 上不被采纳（关掉它也仍跨全屏置顶）。
  - 属性 MUST 在主线程应用，且 MUST 在每次 Space 切换、每次重新显示之后重新断言——窗口操作会把其中一些属性重置回默认值。
  - 显示 MUST 用 `orderFrontRegardless`，MUST NOT 用 `makeKeyAndOrderFront`（后者会激活本应用，把用户从全屏里拽出来）。
  - 自检循环 MUST 存在：发现窗口掉出当前 Space 时重新断言属性；完全不在任何屏幕上时搬回主屏（部分露出屏幕边缘是用户自己的摆放，MUST NOT 搬）。
- 非 macOS **MUST 是 no-op**：`tauri.conf.json` 的 `alwaysOnTop` 已足够；未在真机验证的平台代码 MUST NOT 提前写（隐藏负债）。

---

## 9. 应用生命周期

### 9.1 关闭语义（三平台统一）

| 动作 | 行为 |
|---|---|
| 主窗口 ✕ | 拦截 `CloseRequested` → `prevent_close()` → `hide()`。应用存活，转写继续。`hide()` 失败 MUST NOT 被静默吞掉——那会表现成「点了 ✕ 什么都没发生」 |
| 悬浮窗 ✕ / 托盘隐藏 | 隐藏悬浮窗。转写 MUST NOT 受影响 |
| 托盘 → 退出 | **唯一的**真正退出路径 |

> 依据：Tauri 默认「最后一个窗口关闭 → 应用退出」。不拦截会导致同一个 ✕ 按钮的行为**随悬浮窗是否可见而变化**——这是不可接受的。

### 9.2 配套

- **单实例**：MUST 加单实例锁且**第一个注册**。第二次启动 MUST 唤醒并显示已有实例——否则会有两个托盘图标、两条音频管线。
- **开机自启**：SHOULD 提供开关，并支持「启动即最小化到托盘」。
- **主窗口唤起与 Dock 图标**：主窗口可见时是 Regular（有 Dock 图标与 ⌘-Tab），收进托盘后退回 Accessory。唤起 MUST 走 `lifecycle::show_main()`——只 `show()` 不激活，窗口会停在别的应用底下，而用户除托盘外没有入口。

---

## 10. 存储与输出

### 10.1 Markdown 导出

由 `transcript::to_markdown()` 整体生成，落盘到导出目录（默认 `Documents/AudioScript/`，可自定义）：

```markdown
# AudioScript 转写记录

- 导出时间：{datetime}
- 翻译方向：{sourceLang → targetLang}      ← 仅翻译开启时
- 段落数：{n}

---

**[mm:ss]** {段落原文}

> {段落译文}                                ← 仅有译文时
```

导出的正文 MUST 只包含 `final` 与 `edited` 段落；段落时间戳超过一小时才带小时位（`mm:ss` / `h:mm:ss`）。

### 10.2 落盘时机

| 数据 | 时机 |
|---|---|
| `final` / `edited` 段落 | 存于内存真相源；SQLite 持久化属 V0.2 |
| `partial` 段落 | **不落盘** |
| Markdown | 导出时整体生成 |
| 任务元数据 / 偏好 | 状态变更时写入配置文件 |

**引导页状态（`Preferences.onboardingDone`）MUST 存在偏好里，MUST NOT 只存前端**——「重置所有设置」得能把它清掉，下次启动重新看到欢迎页（与 `sidebarVisible` 同一条规矩）。**权限检测结论相反：MUST NOT 落盘**，它是一次性的现场观测。

### 10.3 密钥

Remote API 密钥 MUST 存入系统密钥链（macOS Keychain / Windows Credential Manager / Linux Secret Service），MUST NOT 明文写入配置文件。配置文件中只存引用标识（`apiKeyRef`）。

---

## 11. 风险与降级

### 11.1 失败 MUST 可见

**静默失败是本项目的主要故障形态**——设备打开成功但没有音频、引擎出字但永不切分、窗口"已显示"却不可见。因此：

- 初始化 / 注册 / 路径解析类失败 MUST 写日志，MUST NOT 被静默吞掉。
- 用户能感知的失败 MUST 同时出现在界面上并给出下一步动作，MUST NOT 只写进日志、MUST NOT 被折叠。
- 资源路径解析（dev 与打包后不同）MUST 能从日志一眼看出走的是哪条分支。

### 11.2 已知风险

| # | 风险 | 处置 |
|---|---|---|
| 1 | **macOS 跨全屏置顶**——「边看视频边读字幕」主场景的前提。Tauri 官方**无此 API** | 已实现，见 §8。**纯 `NSWindow` 的方案已实测全失效，MUST NOT 重试**：只设 `NSWindowCollectionBehavior`、只抬 level + `Stationary`、`CGSSetWindowTags` 私有 API（Sonoma 起被挡）、`object_setClass` 但不加 `NonactivatingPanel`、新建 NSPanel 再 reparent webview（Tao delegate panic）。代价：依赖 `macos-private-api` → **无法上架 Mac App Store**，走 GitHub Releases 分发 |
| 2 | **热键注册失败 = 逃逸路径消失**：穿透一旦开启就无法点击悬浮窗关闭它 | 注册失败 MUST 可见（日志 + 托盘提示，`hotkey_failures` 命令） |
| 3 | **Core Audio Process Tap 权限被拒时静音失败**：tap 创建成功但永远返回静音，Apple 无 API 查询该权限状态 | 分两层：运行期靠 §6.1 的音频心跳兜底；「权限」页另有**主动自测**——先由 `flexaudio::processes()` 的 `is_output_active` 判断系统是否真的在出声，再试采 5.5s 读 `terminal_error()`（库内建的判定器要连续 5s **逐位为零**且源在输出才会出证）。`TapRejected` = 库已证明被拒；`SilentWhilePlaying` = 强指向但未证明（**不**证明：房里的真实数字静音与它无法区分）。**前提**：`Info.plist` MUST 有 `NSAudioCaptureUsageDescription`——缺了它后端不弹窗、应用也不出现在隐私面板列表里，「去设置」会成为死路。另注：**首次创建 tap 会弹出授权窗，弹窗期间整个检测窗口一个音频帧都收不到**（表现为 `samples == 0`，与「被拒」的「有帧但逐位为零」是两种签名）；因此拿到 0 帧且 usage description 存在时 MUST 自动重测一次。 |
| 4 | 全局热键若用 `CGEventTap` 实现会**连带要求辅助功能权限** | MUST 避开；macOS 走 Carbon `RegisterEventHotKey`，只用「修饰键 + 单键」（仅 5 个媒体键走 `CGEventTap`） |
| 5 | **冷启动 ~1.3 s**：点「开始」→ 首次出字实测 1.25–2.2 s（预算 200 ms，**未达标**）——那段时间是**两个模型的加载**。稳态处理延迟实测 ≈ 0（80 秒内偏差 −165 ± 6 ms，无累积漂移） | 在 `setup()` 里后台预建引擎、由首次 `session::start` 接管（代价：常驻内存 +约 150MB，且引擎需要「会话间重置」）。**尚未实施** |

### 11.3 技术前提（版本锚点）

以下事实是本规范若干结论的**前提**，前提变化时相关结论必须重新评估：

| 前提 | 值 | 影响 |
|---|---|---|
| `tauri-runtime-wry` | **≥ 2.12.1** | 透明 / fullscreen 在 vanilla 路径下不再需要 `macOSPrivateApi` |
| `tauri-nspanel` | 2.1.1+ | 它依赖 tauri 的 `macos-private-api` feature——因此「透明 / fullscreen 不再需要 macOSPrivateApi」这个结论**不适用于本项目**（跨全屏主动开启了该 feature，代价见 §11.2 风险 1） |
| `sherpa-onnx`（官方 crate） | 1.13.8+ | 默认 feature `static`，自包含，无 dylib 打包 |
| `sherpa-rs`（第三方） | **已废弃** | MUST NOT 使用——搜到旧资料会走弯路 |
| `Cargo.lock` | 已提交 | **MUST NOT 抄用任何既有项目的 lock**：`tauri 2.11.3` 会把 `tauri-runtime-wry` 钉死在 `< 2.12.1`，使上表结论全部失效 |
| 分发渠道 | GitHub Releases 自建 | 使 `macos-private-api` 的代价可接受 |
| macOS | 14.2+ | Core Audio Process Tap 可用 |

---

## 12. 匿名使用统计

统计默认开启，可在「通用 → 隐私 → 匿名使用统计」关闭；关闭后 Rust 与前端都 MUST NOT 再入队新事件。

- 事件：`app_started` / `app_exited`、`session_started` / `session_stopped`、`model_download`、`overlay_toggled`、`setting_changed`、`onboarding_completed`。
- 属性只允许枚举值与分档（音源、翻译引擎、源语言、模型档位、时长分档、结果状态）。
- MUST NOT 包含转写文本、设备名称或 ID、文件路径、错误原文。
- 传输由 `tauri-plugin-aptabase` 完成，插件自身不自动采集任何事件；上报在后台线程，MUST NOT 阻塞启动、转写或退出。
