# CLAUDE.md

离线优先的语音转写桌面应用：Tauri v2 + React 19 + TypeScript，Rust 后端，pnpm 管理。

## 常用命令

```bash
pnpm install          # 依赖；prepare 会拉内置 ASR 模型与 Silero VAD
pnpm fetch:mt         # 拉内置翻译模型
pnpm tauri dev        # 开发模式（前端固定 5173）
pnpm typecheck        # tsc --noEmit
pnpm lint             # biome check --write src/
pnpm build            # typeshare + vite 生产构建
pnpm tauri build      # 打包
cd src-tauri && cargo test
```

前端没有测试框架；Rust 单测写在各模块的 `#[cfg(test)]` 里。

## 结构

- `src/shared/ipc.ts`：所有前后端调用走单一 `dispatch(name, args)`；新增命令要同时改前后端。
- `src-tauri/src/commands/mod.rs`：命令路由。阻塞命令走 `run_lane`（启动时预创建的 `cmd-quick` / `cmd-session` 常驻线程）+ 超时，不要用 `tauri::async_runtime::spawn_blocking`，也不要在 CT2/MKL 初始化之后为控制路径新建线程（Windows 加载器锁卡住时新线程会停在 `LdrInitializeThunk`）。
- `src-tauri/src/session.rs`：转写会话生命周期。事件、收尾、翻译线程都在 `setup` 里预创建；`LocalNmt::init` 会在启动线程上做一次预热翻译，逼 CT2/MKL/OpenMP 完成惰性初始化——这两点都不要删，否则 Windows 上会出现「停止/开始命令到达但线程不执行」。
- `src-tauri/src/asr/offline.rs`：Qwen3-ASR 离线适配器。`asr-offline-worker` 在 `lib.rs` 的 `setup` 里预创建；用 Silero VAD 切句，只出 final、不产生 partial；解码不得阻塞音频采集线程。
- `src-tauri/src/translate/mac_translation.rs`：macOS 26+ 系统翻译。源语言仍限 en/zh；目标语言必须从运行时 `LanguageAvailability.supportedLanguages`（`mac_translation_languages`）读取，不能写死；语言包由系统管理，不能静默下载。
- `src-tauri/src/models/`：模型下载、校验、安装。
- `src-tauri/assets/model-catalog.json`：内嵌模型目录。可选模型必须填好 `sha256` / `sizeBytes` 并设 `downloadable: true`，否则 UI 显示「资源尚未发布」。ASR 条目要填 `required.asr.format`（`transducer` / `qwen3-asr`）；Qwen3 还需要 `convFrontend` / `encoder` / `decoder` / `tokenizerDir`，并依赖内置的 `silero_vad.onnx`。
- `docs/SPEC.md`：行为规范（章节号被代码和测试引用）；`docs/design.md`：配色与视觉；`docs/index.html`：官网单页（GitHub Pages 发布），版本号、模型清单、平台支持与语言说明都是手写的，改模型/发版时同步。

## 约定

- 用户可见文案走 `src/shared/i18n`（`t()` / `useI18n()`）；Rust 用户可见文案走 `crate::i18n::{tr, tf, Key}`。新增词条必须同步全部 8 份：zh-CN / zh-TW / en / ja / ko / fr / de / es；日志与测试断言保持中文。
- macOS 权限弹窗文案在 `src-tauri/resources/*.lproj/InfoPlist.strings`，新增界面语言时同步补 lproj 并在 `tauri.conf.json` 的 `resources` 注册。
- 注释只保留功能性警告和约束，不写解释性内容。
- TS 格式由 Biome 决定：单引号、无分号、2 空格、120 列。`src/shared/types.ts` 由 typeshare 生成，不要手改。
- Rust 只格式化本次改动的文件；全仓 `cargo fmt` 会带出无关改动。
- macOS 麦克风 / 系统声音权限只能在打包后的 `.app` 里验证；dev 裸二进制缺 Info.plist，会表现为「假性不工作」。
- macOS 14.2+ / Windows 10+ / Ubuntu 24 均已实测可用。

## 可维护操作

```bash
pnpm fetch:model:high   # 下载高精度 ASR（511 MB）到 .cache/，打印 catalog 片段
pnpm verify:models      # 校验 model-catalog.json（--online 会重新下载核对）
```

自动更新签名：GitHub Actions 需要 `TAURI_SIGNING_PRIVATE_KEY`（`~/.tauri/audioscript.key` 的内容）和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。本地 `pnpm tauri build` 不产出更新包，CI 通过 `src-tauri/tauri.updater.conf.json` 打开 `createUpdaterArtifacts`；缺 Secret 时 `.sig` 不会生成，release job 的 latest.json 步骤会失败。

`fetch:model:high` 仅供维护者启用可选模型，普通开发构建不需要。Qwen3-ASR 由应用内下载（约 972 MB），不进 `prepare`。
