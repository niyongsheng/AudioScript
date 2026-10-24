# AudioScript fork 维护

当前维护仓库为 [zyg0733/AudioScript](https://github.com/zyg0733/AudioScript)，基于 [Nico 的原项目](https://github.com/niyongsheng/AudioScript)。保留 AudioScript 品牌、原作者署名和原 MIT LICENSE。应用中的赞助入口明确属于原作者 Nico。

## 应用身份与链接

应用标识为 `io.github.zyg0733.audioscript`。本 fork 与上游使用独立配置、模型和日志目录；旧安装的数据不自动迁移，也不会被删除。需要迁移时先备份原目录，再核对偏好格式与模型完整性。不要直接更改回上游标识来共用数据。

根目录 `project.json` 是仓库、网站、更新和原作者链接的来源，后端 `app_info` 将其提供给界面，系统浏览器只允许打开已知项目与模型许可链接。修改维护仓库时，同步核对 `tauri.conf.json`、模型 catalog 和静态网站；项目单测检查更新配置一致性。

官网与下载入口只指向本 fork。GitHub Pages 是否可用取决于仓库 Settings → Pages 中是否已选择 GitHub Actions；首次 fork 发布前下载页可能尚无安装包。

## 自动更新与发布

本 fork 不信任原项目公钥，也不向原项目请求更新。普通本地构建默认不生成更新工件；未设置 `TAURI_UPDATER_PUBLIC_KEY` 时不注册更新插件，设置页显示原因，后端拒绝安装更新。公钥必须在构建时提供，只改运行时环境不会启用已构建的应用。

发布前在本仓库 Settings → Secrets and variables → Actions 配置：

| 类型 | 名称 | 内容 |
| --- | --- | --- |
| Variable | `TAURI_UPDATER_PUBLIC_KEY` | 维护者为本 fork 管理的 Tauri 公钥文件完整内容；该文件内容本身是 base64 字符串 |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | 与公钥配对的本 fork 私钥，按 Tauri 官方方式保存 |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码；无密码的密钥可为空 |
| Variable，可选 | `AUDIOSCRIPT_APTABASE_APP_KEY` | 维护者自己的 Aptabase App Key |

不要把私钥写入源码、日志、生成的配置、issue 或聊天。密钥由维护者保管；本工作流不生成密钥，也不打印密钥。更新签名与 macOS/Windows 安装包的平台代码签名是两件事，发布前仍需对应平台验收。

`scripts/configure-release.mjs` 只把公钥和本项目 endpoint 写入被忽略的 `.cache/tauri-release.conf.json`。`v*` tag 发布缺公钥或签名 Secret 会在构建前失败；手动 workflow 缺配置可验证普通安装包，但不生成或上传更新工件。配置公钥但未配置私钥的手动构建可检查既有签名版本，仍不生成更新包。配置错误的公钥格式会直接失败。签名是否配对由构建签名和实际更新验收确认。

建议先手动触发 Build & Release，验证三平台安装包，再发布版本 tag。正式 release 汇总各平台签名到 `latest.json`，仅上传到本仓库 Releases。发布后用上一版本安装包验证：检查更新、签名校验、安装、重启和配置保留；没有通过这条流程前不要宣布自动更新可用。

本地需要验证签名构建时，在受控环境中设置上述环境变量，运行 `node scripts/configure-release.mjs --require-updater`，然后运行 `pnpm tauri build --config .cache/tauri-release.conf.json`。不要通过命令行参数传私钥。

官方参考：[Tauri 更新与签名说明](https://v2.tauri.app/plugin/updater/)、[更新插件公钥配置](https://docs.rs/tauri-plugin-updater/2.14.0/tauri_plugin_updater/struct.Builder.html#method.pubkey)。

## 匿名统计

默认偏好为关闭。只有构建时提供维护者自己的 `AUDIOSCRIPT_APTABASE_APP_KEY` 才注册插件，用户仍需主动开启。无 App Key 时主窗口显示不可开启，后端也拒绝启用请求；已有偏好中的开启状态不会绕过该限制。禁止沿用原项目 App Key。

统计只包含事件名、枚举与时长分档，不含音频、转写内容、设备名称或路径。关闭后停止入队新事件。无统计配置的构建不加载插件，识别和翻译可离线工作。

## 模型与上游同步

通用 ASR 模型仍从 k2-fsa/sherpa-onnx 获取，翻译模型仍来自 Helsinki-NLP。模型来源与软件更新来源分别维护。`mt-big` 的未来资源地址属于本 fork，但仍标记 `downloadable: false`；构建模型、填好 SHA256/体积、确认双方向目录安装与加载后才能开放，不得只修改开关。

上游同步使用独立分支和 pull request：先审查上游差异，再选取修复，运行类型检查、静态检查、Rust 单测与相关平台验证。合并时重点检查本 fork 的应用标识、发布 endpoint、公钥、统计默认值、作者说明和模型目录，避免重新引入上游运行配置。

## 回归检查

首次仅安装前端检查依赖可运行 `pnpm install --frozen-lockfile --ignore-scripts`，无需下载识别和翻译模型。常规检查：

```bash
pnpm typecheck
pnpm lint
pnpm test
pnpm test:scripts
pnpm verify:project
pnpm verify:models
pnpm build:frontend
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
```

Rust 测试仍需下载锁定的 crates 和原生推理库，但回归测试不依赖真实麦克风、系统权限或模型权重。生成类型必须使用 typeshare-cli 1.13.4：运行 `pnpm gen:types` 后检查 `src/shared/types.ts` 的差异，禁止手工维护该文件。

`.github/workflows/check.yml` 在 PR 和 main 分支更新时运行上述检查，并验证两个 HTML 入口及生成类型。提交前完成相关回归；真实音频设备断开、权限撤销、完整模型加载和签名更新安装仍需在打包应用中验收。GitHub 上的检查只有提交工作流后才会运行。
