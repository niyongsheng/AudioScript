<p align="center">
  <strong>English</strong> &nbsp;|&nbsp; <a href="README.zh-CN.md">简体中文</a>
</p>

<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="AudioScript">

<h1>AudioScript</h1>

<p>Listen to your system audio and turn foreign podcasts, videos, and meetings into live bilingual subtitles — source text plus translation, recognized and translated entirely on your machine. Chinese and English are built in; optional LMT-60 (Apache-2.0) and NLLB-200 models add multilingual translation, and macOS 26+ can also use system translation.</p>

<p>
  <a href="https://audioscript.looplooptech.com/">Website</a> ·
  <a href="https://audioscript.looplooptech.com/#download">Download</a>
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

## Features

- **Capture**: system audio / microphone / both mixed.
- **Recognition**: Zipformer streaming bilingual recognition (bundled lightweight model / optional high-accuracy), X-ASR for Chinese–English code-switching, Nemotron 3.5 for multilingual streaming, plus Qwen3-ASR. On macOS 26+, system speech recognition (SpeechAnalyzer) is also available.
- **Translation**: bundled Opus-MT (en ↔ zh); optional LMT-60 fast (0.6B) and high-accuracy (1.7B) for 60-language translation (Apache-2.0); optional NLLB-200 for 30-language translation (non-commercial use only); macOS 26+ can optionally translate with the installed system language packs.
- **Overlay**: bilingual source and translation, with adjustable font size, opacity, line count, and a colorful flowing border with adjustable speed and width.
- **Interface**: 简体中文 / 繁體中文 / English / 日本語 / 한국어 / Français / Deutsch / Español.
- **Background**: tray-resident, closing the window does not quit, single instance, launch at login.

## Privacy

* Speech recognition and translation run entirely on-device and work offline.
* Anonymous usage analytics via Aptabase; disable it under General → Privacy.

## Build

```bash
pnpm install          # deps + ASR model and Silero VAD (≈ 59 MB)
pnpm fetch:mt         # fetch MT models
pnpm tauri dev        # dev mode
pnpm tauri build      # package
```


## Contact

- X: [@niyongsheng](https://x.com/niyongsheng)
- E-mail: niyongsheng@Outlook.com

## License

- [MIT](LICENSE)  

- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) Apache-2.0, [X-ASR](https://huggingface.co/GilgameshWind/X-ASR-zh-en) Apache-2.0, [Nemotron 3.5 ASR](https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b) OpenMDW-1.1 (optional), [Qwen3-ASR](https://huggingface.co/Qwen/Qwen3-ASR-0.6B) Apache-2.0, [Silero VAD](https://github.com/snakers4/silero-vad) MIT, [CTranslate2](https://github.com/OpenNMT/CTranslate2) MIT, [opus-mt](https://github.com/Helsinki-NLP), [LMT-60](https://huggingface.co/NiuTrans/LMT-60-0.6B) Apache-2.0 (optional), [NLLB-200](https://huggingface.co/facebook/nllb-200-distilled-600M) CC-BY-NC-4.0 (optional), [translation-rs](https://github.com/doom-fish/translation-rs) MIT OR Apache-2.0, [flexaudio](https://crates.io/crates/flexaudio) MIT, [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) MIT OR Apache-2.0

[![Star History Chart](https://api.star-history.com/svg?repos=niyongsheng/AudioScript&type=Date)](https://star-history.com/#niyongsheng/AudioScript&Date)
