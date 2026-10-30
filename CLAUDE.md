# CLAUDE.md

## 项目要求

- 离线优先：转写与翻译全在本地完成，不依赖云端 API。
- 前端没有测试框架；Rust 单测写在各模块的 `#[cfg(test)]` 里。
- 用户可见文案走 i18n。开发 / 调试阶段只维护 `zh-CN` 与 `en`，其他 locale 允许暂时缺词条并回退英文；发布前必须对齐全部 locale（含 Rust `i18n.rs` 与 `resources/*.lproj/InfoPlist.strings`），由 `check-release.mjs` 校验 key 与占位符，详见 release skill。日志与测试断言保持中文。
- 注释只保留功能性警告和约束。
- Rust 只格式化本次改动的文件；全仓 `cargo fmt` 会带出无关改动。

## 运行时约束

- 阻塞命令走 `run_lane`（`cmd-quick` / `cmd-session`），不要用 `spawn_blocking`，也不要在 CT2/MKL 初始化后为控制路径新建线程——Windows 加载器锁会让新线程停在 `LdrInitializeThunk`。
- 所有常驻线程（事件、收尾、翻译、`asr-offline-worker`）在 `setup` 里预创建；`LocalNmt::init` 启动时做一次预热翻译——删掉会在 Windows 上出现「命令到达但线程不执行」。
- 离线 ASR 解码不得阻塞采集线程；Silero VAD 切句，只出 final。

## 平台

- macOS 26+ 系统语音 / 翻译：Swift 静态库由 `build.rs` 编译，SDK < 26 时跳过；裸二进制没有 Info.plist，麦克风与系统声音权限调用会被 TCC 终止——只能在打包后的 `.app` 里验证，dev 模式「不工作」是预期行为。
- macOS 系统翻译的目标语言必须从运行时 `LanguageAvailability.supportedLanguages` 读取，不能写死；源语言在 Zipformer / 本地 NMT 下仍限 en/zh；语言包由系统管理，不能静默下载。
- 新增界面语言要补 `resources/*.lproj/InfoPlist.strings` 并在 `tauri.conf.json` 的 `resources` 注册。

## 数据与文档

- `src-tauri/assets/model-catalog.json`：可选模型必须填 `sha256` / `sizeBytes` 并设 `downloadable: true`，否则 UI 显示「资源尚未发布」；NLLB 许可 CC-BY-NC-4.0，仅限非商业。
- `docs/SPEC.md` 章节号被代码和测试引用；`docs/index.html` 的版本号、模型清单、平台与语言说明是手写的，改模型 / 发版时同步。

## 发布

- 更新签名私钥在 `~/.tauri/audioscript.key`，不可再生；CI 需要 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，缺 Secret 时 `.sig` 不生成、latest.json 步骤失败。本地 `pnpm tauri build` 不产出更新包，发布走 CI。

## 环境

- 首次 checkout 后除 `pnpm install`（会拉内置 ASR 模型）外还要 `pnpm fetch:mt`；`fetch:model:high` / `fetch:mt:multi` / `verify:models` 是维护者脚本。
