# Cinelingo

AI real-time subtitle / translation tool built on mpv playback, packaged as a
desktop app with Tauri + Vue 3 + TypeScript. Transcribes and (soon) translates
audio live with a local Whisper engine, across three modes — **play**,
**capture**, and **floating** — with the media player as the foundation.

> Extracted from the former `light-mpv` repo. App id `com.cinelingo.player`;
> user data lives under `%LOCALAPPDATA%\Cinelingo-data`.

## Stack

- **Frontend:** Vue 3 (`<script setup>`), TypeScript, Vite
- **Shell:** Tauri 2 (Rust)
- **Playback:** libmpv via `tauri-plugin-libmpv`
- **Speech-to-text:** whisper.cpp (`large-v3-turbo`) sidecar, CUDA on RTX-class GPUs
- **Remote sources:** yt-dlp (YouTube paste-to-play, DASH quality switching)

## Prerequisites

- Windows (the libmpv provisioning and capture paths are Windows-only)
- Node.js 18+ and Rust (stable) with the Tauri prerequisites
- [7-Zip](https://www.7-zip.org/) on PATH or in the default install dir
  (used to unpack the pinned libmpv archive)

## Getting started

```bash
cd <repo root>
npm install
npm run setup-libmpv   # provisions the pinned, no-curl libmpv DLLs into src-tauri/lib/
npm run tauri dev      # run the app (predev re-verifies libmpv automatically)
```

`npm run setup-libmpv` is idempotent and self-healing: it downloads the exact
pinned shinchiro (no-curl) `libmpv-2.dll` and the nini22P wrapper, verifies them
by SHA256, and restores them if they ever get clobbered. **Do not** run
`tauri-plugin-libmpv-api setup-lib` — it pulls a libcurl-linked build that stalls
on 4K streams. See `scripts/libmpv-manifest.mjs` for the pinned versions.

## Scripts

| Command | Purpose |
| --- | --- |
| `npm run tauri dev` | Run the app in development |
| `npm run tauri build` | Build the NSIS installer |
| `npm test` | Run the Vitest suite |
| `npm run setup-libmpv` | (Re)provision the pinned no-curl libmpv DLLs |

## Recommended IDE setup

[VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar)
+ [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
+ [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
