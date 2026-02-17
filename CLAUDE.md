# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Hathor** is a Tauri v2 desktop app providing a GUI wrapper around the FFmpeg CLI. The frontend is SvelteKit (SPA mode, static adapter), and the backend is Rust via Tauri commands that shell out to `ffmpeg`.

## Commands

**Development (runs both frontend + Tauri):**
```sh
bun tauri dev
```

**Build for production:**
```sh
bun tauri build
```

**Frontend only (no Tauri shell):**
```sh
bun dev
```

**Type-check Svelte:**
```sh
bun check
```

**Rust lint/check:**
```sh
cd src-tauri && cargo clippy
```

## Architecture

### Frontend (`src/`)
- SvelteKit in **SPA mode** — no SSR, uses `adapter-static` with `fallback: "index.html"`
- Svelte 5 (runes-based reactivity)
- Communication with Rust via `@tauri-apps/api` — use `invoke()` to call Tauri commands

### Backend (`src-tauri/`)
- Entry: `src/main.rs` → `src/lib.rs::run()`
- Tauri commands are defined in `src/lib.rs` with `#[tauri::command]` and registered in `invoke_handler`
- FFmpeg integration: spawn `ffmpeg` as a child process from Rust (use `std::process::Command` or `tauri-plugin-shell` for streaming output)
- Capabilities are configured in `src-tauri/capabilities/`

### Key Config
- `tauri.conf.json`: app metadata, window size (800×600), dev URL (`localhost:1420`)
- `svelte.config.js`: static adapter, SPA fallback
- `vite.config.js`: Vite config for the frontend build

### Adding a Tauri Command
1. Define `#[tauri::command] fn my_command(...) -> ... { }` in `src-tauri/src/lib.rs`
2. Register it: `.invoke_handler(tauri::generate_handler![..., my_command])`
3. Call from frontend: `import { invoke } from "@tauri-apps/api/core"; invoke("my_command", { args })`
