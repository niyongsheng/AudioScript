<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="AudioScript">

<h1>AudioScript 声稿</h1>

<p>监听系统声音输出，把外语播客、视频与会议实时转成「原文 + 译文」双语悬浮字幕。识别与翻译全部在本机完成，默认中文，macOS 26+ 可选更多目标语言。</p>

<p>
  <a href="https://audioscript.looplooptech.com/">官网</a> ·
  <a href="https://audioscript.looplooptech.com/#download">下载</a>
</p>

<p>
  <img src="https://img.shields.io/badge/macOS-14.2%2B-111111?style=flat-square&logo=apple&logoColor=white" alt="macOS 14.2+">
  <img src="https://img.shields.io/badge/Windows-10%2B-111111?style=flat-square&logo=windows&logoColor=white" alt="Windows 10+">
  <img src="https://img.shields.io/badge/Ubuntu-24-111111?style=flat-square&logo=ubuntu&logoColor=white" alt="Ubuntu 24">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white" alt="Tauri 2">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-555555?style=flat-square" alt="MIT"></a>
</p>

</div>

<br>

https://github.com/user-attachments/assets/b1f88a82-b3fe-4721-8736-af6a9e4d828e

## 功能

- **采集**：系统声音 / 麦克风 / 两者混合。
- **识别**：Zipformer 流式双语识别（内置小模型 / 可选高精度 / Qwen3-ASR）。
- **翻译**：内置 Opus-MT（en ↔ zh）；macOS 26+ 可选系统翻译，目标语言跟随系统语言包。
- **字幕悬窗**：双语对照，字号 / 不透明度 / 行数 / 彩色流光边框与速度可调。
- **界面**：简体中文 / 繁體中文 / English / 日本語 / 한국어 / Français / Deutsch / Español。
- **常驻**：托盘、关窗不退出、单实例、开机自启。

## 隐私

* 语音识别与转译全部在本机完成，离线可用。
* Aptabase匿名使用统计，可在「通用 → 隐私」中关闭。

## 构建

```bash
pnpm install          # 依赖 + ASR 模型与 Silero VAD（≈ 59 MB）
pnpm fetch:mt         # 模型拉取
pnpm tauri dev        # 开发模式
pnpm tauri build      # 打包
```


## 联系我

- X: [@niyongsheng](https://x.com/niyongsheng)
- E‑mail: niyongsheng@Outlook.com

## 许可证

- [MIT](LICENSE)  

- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) Apache-2.0、[Qwen3-ASR](https://huggingface.co/Qwen/Qwen3-ASR-0.6B) Apache-2.0、[Silero VAD](https://github.com/snakers4/silero-vad) MIT、[CTranslate2](https://github.com/OpenNMT/CTranslate2) MIT、[opus-mt](https://github.com/Helsinki-NLP)、[translation-rs](https://github.com/doom-fish/translation-rs) MIT OR Apache-2.0、[flexaudio](https://crates.io/crates/flexaudio) MIT、[tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) MIT OR Apache-2.0
