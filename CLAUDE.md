# CLAUDE.md

离线优先的语音转写桌面应用：Tauri v2 + React 19 + TypeScript，Rust 后端，pnpm 管理。

## 常用命令

```bash
pnpm install          # 依赖；prepare 会拉内置 ASR 模型
pnpm fetch:mt         # 拉内置翻译模型（首次 checkout 后需要）
pnpm tauri dev        # 开发模式（前端固定 5173）
pnpm typecheck        # tsc --noEmit
pnpm lint             # biome check --write src/
pnpm build            # typeshare + vite 生产构建
pnpm tauri build      # 打包
cd src-tauri && cargo test
```

前端没有测试框架；Rust 单测写在各模块的 `#[cfg(test)]` 里。`fetch:model:high` / `fetch:mt:multi` / `verify:models` 是维护者脚本，普通开发不需要。

## 结构

- `src/shared/ipc.ts`：所有前后端调用走单一 `dispatch(name, args)`，新增命令要同时改前后端。
- `src-tauri/src/commands/mod.rs`：阻塞命令走 `run_lane`（`cmd-quick` / `cmd-session` 常驻线程）+ 超时。不要用 `spawn_blocking`，也不要在 CT2/MKL 初始化后为控制路径新建线程——Windows 加载器锁会让新线程停在 `LdrInitializeThunk`。
- `src-tauri/src/session.rs`：事件、收尾、翻译线程都在 `setup` 里预创建；`LocalNmt::init` 启动时做一次预热翻译，逼 CT2/MKL/OpenMP 完成惰性初始化。这两点删掉会在 Windows 上出现「命令到达但线程不执行」。
- `src-tauri/src/asr/offline.rs`：Qwen3-ASR 离线适配器。`asr-offline-worker` 在 `setup` 预创建；Silero VAD 切句，只出 final；解码不得阻塞采集线程。
- `src-tauri/macos/speech_bridge.swift` + `src-tauri/src/asr/mac_native.rs`：macOS 26+ 系统语音。Swift 静态库由 `build.rs` 编译，SDK < 26 时跳过；裸二进制没有 Info.plist，调用会被 TCC 终止——只在打包后的 `.app` 里可用，dev 模式「不工作」是预期行为。
- `src-tauri/src/translate/mac_translation.rs`：macOS 26+ 系统翻译。源语言在 Zipformer / 本地 NMT 下仍限 en/zh（`source_lang_allowed` 校验）；目标语言必须从运行时 `LanguageAvailability.supportedLanguages` 读取，不能写死；语言包由系统管理，不能静默下载。
- `src-tauri/assets/model-catalog.json`：可选模型必须填 `sha256` / `sizeBytes` 并设 `downloadable: true`，否则 UI 显示「资源尚未发布」。`required.asr.format` 默认 `transducer`，Qwen3 条目填 `qwen3-asr` 并补 `convFrontend` / `encoder` / `decoder` / `tokenizerDir`（依赖内置 `silero_vad.onnx`）。`required.mt.format` 默认 `marian`（`install.directions` 写两个方向目录），NLLB 填 `nllb`、单目录、`tier: multilingual`，许可 CC-BY-NC-4.0 仅限非商业用途。`languages` 决定语言标签，`multilingual: true` 显示「多语言」。
- `docs/SPEC.md`：行为规范（章节号被代码和测试引用）；`docs/design.md`：配色与视觉；`docs/index.html`：官网单页（GitHub Pages），版本号、模型清单、平台与语言说明都是手写的，改模型 / 发版时同步。

## 约定

- 用户可见文案：前端 `src/shared/i18n`（`t()` / `useI18n()`），Rust `crate::i18n::{tr, tf, Key}`。新增词条必须同步 8 份：zh-CN / zh-TW / en / ja / ko / fr / de / es；日志与测试断言保持中文。
- macOS 权限弹窗文案在 `src-tauri/resources/*.lproj/InfoPlist.strings`，新增界面语言时补 lproj 并在 `tauri.conf.json` 的 `resources` 注册。
- 注释只保留功能性警告和约束。
- TS 格式由 Biome 决定：单引号、无分号、2 空格、120 列。`src/shared/types.ts` 由 typeshare 生成，不要手改。
- Rust 只格式化本次改动的文件；全仓 `cargo fmt` 会带出无关改动。
- macOS 麦克风 / 系统声音权限只能在打包后的 `.app` 里验证；dev 裸二进制缺 Info.plist，会表现为「假性不工作」。

## 发布

自动更新签名：CI 需要 `TAURI_SIGNING_PRIVATE_KEY`（`~/.tauri/audioscript.key` 的内容）和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。本地 `pnpm tauri build` 不产出更新包，CI 通过 `src-tauri/tauri.updater.conf.json` 打开 `createUpdaterArtifacts`；缺 Secret 时 `.sig` 不会生成，release job 的 latest.json 步骤会失败。
