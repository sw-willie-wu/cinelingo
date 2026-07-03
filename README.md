<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.png">
    <img src="docs/logo-light.png" width="300" alt="Cinelingo">
  </picture>
</p>

<p align="center">
  <b>Local AI real-time subtitle &amp; translation, on top of an mpv media player.</b><br>
  Tauri + Vue 3 + TypeScript desktop app. Everything runs on your machine — no cloud.
</p>

<p align="center">
  <a href="https://github.com/sw-willie-wu/cinelingo/releases/latest"><img src="https://img.shields.io/github/v/release/sw-willie-wu/cinelingo?include_prereleases&label=release" alt="latest release"></a>
  <img src="https://img.shields.io/badge/platform-Windows-blue" alt="platform">
  <img src="https://img.shields.io/badge/Tauri%202-Vue%203-42b883" alt="stack">
</p>

---

Cinelingo plays local files and YouTube, transcribes speech live with a local
Whisper engine, and translates the subtitles with a local LLM — across three
modes, with the media player as the foundation:

- **Play** — local files / YouTube (paste-to-play, DASH quality switching), playback queue, per-track subtitles.
- **Capture** — transcribe any system/app audio (WASAPI loopback) as live captions; record captured audio to WAV + SRT.
- **Floating** — draggable, click-through floating caption window over other apps.

> App id `com.cinelingo.player`; user data under `%LOCALAPPDATA%\Cinelingo-data`.
> Extracted from the former `light-mpv` repo.

## Features

- **Real-time subtitles** — whisper.cpp sidecar (ahead-of-playhead for files, loopback streaming for capture), in-process Silero VAD segmentation, hallucination filtering, per-video SRT cache.
- **Real-time translation** — local `llama-server` + TranslateGemma (4B/12B), per-track "translate to", bilingual display, playhead-driven lazy translation for file playback with a per-target subtitle cache, anti-echo prompt handling.
- **On-demand engines** — the speech and translation engines (and their models) download inside the app when you enable the feature; nothing ships bundled.
- **Media player** — libmpv playback, adjustment panels (EQ / normalize / speed / image), YouTube quality switching, playback queue + recents.

## Stack

- **Frontend:** Vue 3 (`<script setup>`), TypeScript, Vite
- **Shell:** Tauri 2 (Rust)
- **Playback:** libmpv via `tauri-plugin-libmpv` (pinned, no-curl build)
- **Speech-to-text:** whisper.cpp sidecar (`large-v3-turbo` and others), CUDA on RTX-class GPUs
- **Translation:** llama.cpp `llama-server` + TranslateGemma GGUF
- **Remote sources:** yt-dlp (YouTube paste-to-play, DASH quality switching)

## Prerequisites

- **Windows** (the libmpv provisioning and capture paths are Windows-only)
- **Node.js 20+** and **Rust** (stable) with the Tauri prerequisites
- [**7-Zip**](https://www.7-zip.org/) on PATH or in the default install dir (used to unpack the pinned libmpv archive)
- A CUDA-capable GPU is recommended for real-time transcription + translation (CPU builds work but are slower)

## Getting started

```bash
npm install
npm run setup-libmpv   # provisions the pinned, no-curl libmpv DLLs into src-tauri/lib/
npm run tauri dev      # run the app (predev re-verifies libmpv automatically)
```

`npm run setup-libmpv` is idempotent and self-healing: it downloads the exact
pinned shinchiro (no-curl) `libmpv-2.dll` + wrapper, verifies them by SHA256, and
restores them if clobbered. **Do not** run `tauri-plugin-libmpv-api setup-lib` —
it pulls a libcurl-linked build that stalls on 4K streams. See
`scripts/libmpv-manifest.mjs` for the pinned versions.

The Whisper and translation engines are **not** bundled: enable the feature in
**Settings → 即時字幕 / 字幕翻譯** and the app installs the runtime binaries, then
you download the model of your choice from the panel.

## Scripts

| Command | Purpose |
| --- | --- |
| `npm run tauri dev` | Run the app in development |
| `npm run tauri build -- --bundles nsis` | Build the Windows NSIS installer |
| `npm test` | Run the Vitest suite |
| `npm run setup-libmpv` | (Re)provision the pinned no-curl libmpv DLLs |

## Releases

Tagging `v*` triggers a GitHub Actions workflow that builds the NSIS installer and
publishes a release (pre-release for `-suffix` tags, full release for clean tags).
Grab the latest installer from the
[Releases page](https://github.com/sw-willie-wu/cinelingo/releases).

## Recommended IDE setup

[VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
+ [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
+ [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
