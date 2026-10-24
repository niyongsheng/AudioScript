# CLAUDE.md

离线优先的语音转写桌面应用：Tauri v2 + React 19 + TypeScript，Rust 后端，pnpm 管理。

## 常用命令

```bash
pnpm install          # 依赖；prepare 会拉内置 ASR 模型
pnpm fetch:mt         # 拉内置翻译模型
pnpm tauri dev        # 开发模式（前端固定 5173）
pnpm typecheck        # tsc --noEmit
pnpm lint             # 只读静态与格式检查
pnpm lint:fix         # 显式修复格式与可修复问题
pnpm test             # Vitest 回归测试
pnpm test:scripts     # 发布配置脚本回归测试
pnpm verify:project   # fork 身份与链接一致性
pnpm build            # typeshare + vite 生产构建
pnpm tauri build      # 打包
cd src-tauri && cargo test
```

前端使用 Vitest，Rust 单测写在各模块的 `#[cfg(test)]` 里。PR Checks 校验前端类型、静态检查、回归测试、双窗口构建、模型目录与 fork 配置，并运行 Rust 测试和生成类型差异检查。

## 结构

- `src/shared/ipc.ts`：所有前后端调用走单一 `dispatch(name, args)`；新增命令要同时改前后端。
- `src-tauri/src/commands/mod.rs`：命令路由。阻塞命令走 `run_lane`（启动时预创建的 `cmd-quick` / `cmd-session` 常驻线程）+ 超时，不要用 `tauri::async_runtime::spawn_blocking`，也不要在 CT2/MKL 初始化之后为控制路径新建线程（Windows 加载器锁卡住时新线程会停在 `LdrInitializeThunk`）。
- `src-tauri/src/session.rs`：转写会话生命周期。事件、收尾、翻译线程都在 `setup` 里预创建；`LocalNmt::init` 会在启动线程上做一次预热翻译，逼 CT2/MKL/OpenMP 完成惰性初始化——这两点都不要删，否则 Windows 上会出现「停止/开始命令到达但线程不执行」。
- `src-tauri/src/models/`：模型下载、校验、安装。
- `src-tauri/assets/model-catalog.json`：内嵌模型目录。可选模型必须填好 `sha256` / `sizeBytes` 并设 `downloadable: true`，否则 UI 显示「资源尚未发布」。
- `docs/SPEC.md`：行为规范（章节号被代码和测试引用）；`docs/design.md`：配色与视觉。

## 约定

- 用户可见文案、日志、错误信息用中文。
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

fork 身份：`zyg0733/AudioScript`，应用标识 `io.github.zyg0733.audioscript`。项目链接集中在根目录 `project.json`；保留原作者与 MIT，不得沿用上游更新公钥或统计 Key。新应用标识使用独立配置目录，旧安装的数据不自动迁移。

自动更新签名：仓库 Actions 变量 `TAURI_UPDATER_PUBLIC_KEY` 保存本项目公钥；Secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 保存本项目私钥及其密码。CI 的 `scripts/configure-release.mjs` 生成 `.cache/tauri-release.conf.json`；tag 发布缺配置立即失败，手动验证缺配置不产出更新工件。普通本地构建未设置公钥时更新明确禁用，禁止回退到上游。不要生成、读取或输出私钥。

统计默认关闭，仅构建环境 `AUDIOSCRIPT_APTABASE_APP_KEY` 非空才加载插件并允许用户主动开启。完整流程见 `docs/FORK_MAINTENANCE.md`。

`fetch:model:high` 仅供维护者启用可选模型，普通开发构建不需要。
