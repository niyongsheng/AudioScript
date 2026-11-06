# CLAUDE.md

## 项目要求

- 离线优先：转写与翻译全在本地完成，不依赖云端 API。
- 用户可见文案走 i18n；开发期只维护 `zh-CN` / `en`，发布前用 `check-release.mjs` 对齐全部 locale（含 Rust `i18n.rs` 与 `*.lproj`）。日志与测试断言用中文。
- 前端无测试框架，Rust 单测写在各模块的 `#[cfg(test)]`；注释只保留功能性警告；只格式化改动文件（全仓 `cargo fmt` 会带出无关改动）。

## 运行时约束

- 阻塞命令走 `run_lane`，不要 `spawn_blocking`；CT2/MKL 初始化后不要为控制路径新建线程（Windows 加载器锁会卡在 `LdrInitializeThunk`）。
- 事件、收尾、翻译、`asr-offline-worker` 线程都在 `setup` 预创建；`LocalNmt::init` 的预热翻译不能删（Windows「命令到达但线程不执行」）。
- 翻译线程按会话队列工作：发出 `TranslateCommand::Start` 后失败必须关闭该队列（`QueueGuard`），否则后续只出原文。
- 离线 ASR 解码不得阻塞采集线程；Silero VAD 切句，只出 final。

## 平台

- macOS 26+ 系统语音 / 翻译：Swift 静态库由 `build.rs` 编译，SDK < 26 跳过；权限只能在打包后的 `.app` 里验证（裸二进制没有 Info.plist）。
- macOS 系统翻译的目标语言必须读运行时 `LanguageAvailability.supportedLanguages`，语言包不能静默下载；源语言在 Zipformer / 本地 NMT 下仍限 en/zh。
- Windows 麦克风原生超过两声道时走 `audio/windows_mic.rs` 的 cpal 回退（下混 + 16 kHz 重采样），mix 由应用侧混音。
- 新增界面语言要补 `resources/*.lproj/InfoPlist.strings` 并在 `tauri.conf.json` 注册。

## 数据与文档

- `model-catalog.json`：可选模型必须填 `sha256` / `sizeBytes` 并设 `downloadable: true`；NLLB 许可 CC-BY-NC-4.0，仅限非商业。
- `docs/SPEC.md` 章节号被代码和测试引用，只写长期规范条文；`docs/index.html` 的版本号、模型清单、平台与语言说明手写，改模型 / 发版时同步。
- 缺陷与环境坑记 GitHub Issues（提交用 `Fixes #N`），不写进 SPEC；启动日志固定输出环境信息，panic 必须落盘。
