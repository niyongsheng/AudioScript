---
name: audioscript-release
description: 发布 AudioScript 版本时使用：定版本号 → 新特性弹窗 What's New → 官网 docs/index.html → 模型资源与许可 → pnpm release → CI 产物校验，逐项检查不漏
---

# audioscript-release

把 AudioScript 从「功能已提交」推进到「Release 已发布」的固定流程。任何一步跳过，用户就会看到旧版本号、缺失的新功能介绍，或下载到 404 的模型。

## 1. 定版本号

```bash
git log --oneline $(git describe --tags --abbrev=0)..HEAD
git status --short
git rev-parse --abbrev-ref HEAD
```

- 修复 → patch；新增功能 / 模型档位 / 界面能力 → minor；破坏性改动 → major。
- 工作区必须干净、当前在 `main`、与 `origin/main` 同步（`pnpm release` 会硬性检查前两项）。
- 记下本次版本 `X.Y.Z`，后续所有步骤都用这个值。

## 2. 新特性页面（最容易漏）

`src/main/WhatsNew.tsx` 的 `WHATS_NEW` 以**本次发布版本号**为 key。`App.tsx` 用 `whatsNewFor(info.version)` 精确匹配：key 写成旧版本，弹窗永远不会出现；升级后还会把 `audioscript.seen-version` 写成当前版本，等于永久跳过。

- 每个用户可见新功能加一条 `{ icon, titleKey, bodyKey }`。`icon` 必须是 `src/ui/Icon.tsx` 里 `IconName` 的取值（文件第 39 行起）。
- 词条 key 统一 `whatsNew.<slug>Title` / `whatsNew.<slug>Body`，**8 个 locale 全部同步**：`en / zh-CN / zh-TW / ja / ko / fr / de / es`（`src/shared/i18n/`）。
- 文案口径与「模型」页、官网保持一致；中文以 `zh-CN` 为准。
- 纯内部修复、依赖升级、无用户可见变化：可以不配，但必须显式写明原因，并在自检时传 `--no-whats-new`。

```ts
const FEATURES_V051: readonly WhatsNewFeature[] = [
  { icon: 'waveform', titleKey: 'whatsNew.xAsrTitle', bodyKey: 'whatsNew.xAsrBody' },
  { icon: 'translate', titleKey: 'whatsNew.lmtTitle', bodyKey: 'whatsNew.lmtBody' },
]

const WHATS_NEW: Record<string, readonly WhatsNewFeature[]> = {
  '0.3.0': FEATURES_V030,
  '0.5.1': FEATURES_V051,
}
```

## 3. 官网 docs/index.html

静态值不会自动改（页面运行时才从 GitHub API 覆盖），必须手写同步：

- JSON-LD `"softwareVersion"`（约 2066 行）。
- 下载区静态兜底：`<span data-version>vX.Y.Z</span>` 与 `<span data-date data-iso="YYYY-MM-DD">`（约 2667 行）。
- 模型表 `#models`：模型名、流式方式、语言数、下载/安装体积、许可、适用场景。与 `src-tauri/assets/model-catalog.json` 对齐（`title`、`languages.length`、`archive.sizeBytes` + `sizes.installedBytes`、`licenses`）。
- 语言数、FAQ、能力描述里的数字（如「可选 29 种语言」「Nemotron 覆盖 28 种」）。
- 中英两套 `<span class="zh">` / `<span class="en">` 必须同时改，漏一个会出现中英混排。
- 平台与最低系统版本（macOS 14.2+、Windows 10+、Linux）。

改完在浏览器打开确认版本行、模型表、下载按钮没有破版。注意页面会请求 GitHub API：离线或 API 失败时看到的是静态兜底值。

## 4. 文档与许可

- `docs/SPEC.md`：行为 / 模型 / 配置变了才改；章节号被代码和测试引用，**不要改号**。
- `README.md` / `README.zh-CN.md`：功能列表、模型与许可链接。
- `CLAUDE.md`：只有新增运行时约束才改。
- 新增第三方模型：catalog `licenses`、README 许可段、官网模型表三处都要有；NLLB 必须标注 CC-BY-NC-4.0 仅限非商业。

## 5. 模型资源校验

```bash
pnpm verify:models
gh release view models-v1 --repo niyongsheng/AudioScript --json assets -q '.assets[] | "\(.name) \(.size)"'
curl -sIL -o /dev/null -w '%{http_code}\n' <上游 url>
```

- `downloadable: true` 的条目必须有 HTTPS `url`、`sha256`、`sizeBytes`，否则 UI 显示「资源尚未发布」。
- 自托管归档放在 `models-v1`，仓库资产大小必须与 `archive.sizeBytes` 完全一致（下载后按字节校验）。
- 上游地址（如 k2-fsa/sherpa-onnx）必须返回 200；改动过 URL 的要重新验证。

## 6. 一键自检

```bash
node .claude/skills/audioscript-release/scripts/check-release.mjs X.Y.Z
```

检查项：

- `WhatsNew.tsx` 是否有 `X.Y.Z` 条目（没配且确认跳过时加 `--no-whats-new`）；
- 8 个 locale 的 `whatsNew.*` 词条是否一一对应；
- `docs/index.html` 的 `softwareVersion`、`data-version`、`data-date`；
- catalog 中可下载模型的 `url` / `sha256` / `sizeBytes`。

全绿再进入下一步。

## 7. 执行发布

```bash
pnpm release X.Y.Z        # 或 patch / minor / major
```

脚本会：

1. 校验干净工作区、在 `main`、`package.json` 与 `tauri.conf.json` 版本一致、tag 不存在；
2. 改写 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.lock`；
3. 跑 `pnpm typecheck`、`pnpm lint`、`cargo test`（失败自动回滚版本号）；
4. 提交 `chore: release vX.Y.Z`、打 tag、推送 `main` 与 tag。

约束：

- **不要手动改版本号或手动打 tag** 绕过脚本。
- 发布必须走 CI：本地 `pnpm tauri build` 不产出更新包。
- 更新签名私钥 `~/.tauri/audioscript.key` 不可再生；CI 缺 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 时 `.sig` 不生成、latest.json 步骤失败。
- `pnpm lint` 是 `biome check --write src/`，如果它自动改了文件，发布后工作区会残留改动，需要单独提交。
- 远端若为 HTTPS 且被 keychain 解析成别的账号，push 会 403；本项目 origin 用 SSH，直接推送即可。

## 8. 发布后校验

```bash
gh run list --repo niyongsheng/AudioScript --workflow build.yml --limit 1
gh run watch <run-id> --repo niyongsheng/AudioScript --exit-status
gh release view vX.Y.Z --repo niyongsheng/AudioScript --json isDraft,publishedAt,assets \
  -q '{isDraft, publishedAt, assets: [.assets[].name]}'
node .claude/skills/audioscript-release/scripts/check-release.mjs X.Y.Z --post
```

必须确认：

- 三平台产物齐全：macOS dmg + `AudioScript.app.tar.gz`（含 `.sig`）、Windows exe / msi、Linux AppImage / deb；
- `latest.json` 存在且指向新版本（自动更新依赖它）；
- Release 不是 draft；
- 官网版本行已更新（静态值仍是旧版说明漏了第 3 步）；
- 新增模型能在「模型」页下载并通过 sha256 校验；
- What's New 弹窗：本地升级验证，或 dev 下删掉 localStorage 的 `audioscript.seen-version` 再启动。

## 检查清单

- [ ] 版本号已定，工作区干净且在 main
- [ ] `WHATS_NEW` 以**本次版本号**为 key，8 个 locale 词条齐全
- [ ] `docs/index.html` 的 `softwareVersion` / `data-version` / `data-date`
- [ ] `docs/index.html` 模型表与 catalog 对齐，中英双语都改
- [ ] README 功能与许可、SPEC 对应章节
- [ ] 模型 URL 200、sha256 / sizeBytes 与归档一致
- [ ] `check-release.mjs` 全绿
- [ ] `pnpm release` 成功，tag 已推送
- [ ] CI 三平台 + latest.json + .sig 全部成功
- [ ] 官网与 What's New 线上验证
