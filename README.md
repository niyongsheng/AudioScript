<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="AudioScript">

<h1>AudioScript 声稿</h1>

<p>监听系统声音输出，把外语播客与外文视频实时转成「原文 + 中文译文」双语悬浮字幕。识别与翻译全部在本机完成。</p>

<p>
  <a href="https://zyg0733.github.io/AudioScript/">官网</a> ·
  <a href="https://github.com/zyg0733/AudioScript/releases/latest">下载</a>
</p>

<p>
  <img src="https://img.shields.io/badge/macOS-14.2%2B-111111?style=flat-square&logo=apple&logoColor=white" alt="macOS 14.2+">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white" alt="Tauri 2">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-555555?style=flat-square" alt="MIT"></a>
</p>

</div>

本仓库是 [Nico / niyongsheng 的 AudioScript](https://github.com/niyongsheng/AudioScript) 的 fork，由 [zyg0733](https://github.com/zyg0733) 继续维护，保留原项目 MIT 许可和作者署名。当前下载与更新仅来自本仓库。

<br>

https://github.com/user-attachments/assets/10649363-0b68-45f5-aa0b-97a9135dab53

## 功能

- **采集**：系统声音 / 麦克风 / 两者混合。
- **实时离线**：sherpa-onnx 流式识别 + CTranslate2 opus-mt 翻译（en ↔ zh）
- **字幕悬窗**：双语对照，字号 / 不透明度 / 行数可调
- **常驻**：托盘、关窗不退出、单实例、开机自启。

## 隐私

* 语音识别与转译全部在本机完成，离线可用。
* 匿名使用统计默认关闭。仅在构建时配置维护者自己的 Aptabase App Key 后，用户才能在「通用 → 隐私」中主动开启；未配置的构建不加载统计插件。
* 自动更新仅连接本仓库 Releases，并验证本仓库公钥。普通本地构建未配置公钥时禁用更新，可从上方下载入口手动安装。

## 构建

```bash
pnpm install          # 依赖 + ASR 模型（≈ 58 MB）
pnpm fetch:mt         # 模型拉取
pnpm tauri dev        # 开发模式
pnpm tauri build      # 打包
```


## 维护与原作者

- 当前 fork 的问题反馈与发布：[zyg0733/AudioScript](https://github.com/zyg0733/AudioScript)。
- fork 构建、签名与上游同步：[维护说明](docs/FORK_MAINTENANCE.md)。
- 原项目作者 Nico：X [@niyongsheng](https://x.com/niyongsheng)，E‑mail niyongsheng@Outlook.com。
- [赞助原作者 Nico](https://ifdian.net/a/nico2026)：此入口属于原作者。

## 许可证

- [MIT](LICENSE)  

- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) Apache-2.0、[CTranslate2](https://github.com/OpenNMT/CTranslate2) MIT、[opus-mt](https://github.com/Helsinki-NLP)、[flexaudio](https://crates.io/crates/flexaudio) MIT、[tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) MIT OR Apache-2.0
