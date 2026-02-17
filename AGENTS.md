# CLAUDE.md

This file provides shared agent guidance for this repository.

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

### Component Tree

```
src/lib/
  types.ts                        # Shared TS types (Tab, Container, QueueJob, etc.)
  format.ts                       # fmtDuration, fmtBytes, fmtFps, fmtChannels
  ffmpeg.ts                       # FfmpegOperation, MediaInfo, FfmpegProgress + Tauri wrappers
  theme.svelte.ts                 # theme singleton ($state-based)
  components/
    BrowseInput.svelte            # input + browse button (bindable value)
    MediaInfoStrip.svelte         # probing indicator + metadata display
    ProgressDisplay.svelte        # progress bar + stats (indeterminate or pct)
    SettingsPanel.svelte          # theme, default output dir, cleanup toggle
    QueueList.svelte              # job rows + empty state
    LogView.svelte                # log header + command strip + scrolling log body
    panels/
      ConvertPanel.svelte
      TrimPanel.svelte
      TransformPanel.svelte
      MergePanel.svelte           # includes mismatch warning logic
      CompressPanel.svelte
      RemuxPanel.svelte
src/routes/+page.svelte           # orchestration only (~400 lines)
src/app.css                       # global CSS including .browse-btn, .slider, .dot-pulse, .spinner
```

### Svelte 5 Patterns

- Use `$bindable()` for child-to-parent state: `let { value = $bindable("") }: Props = $props()`
- Use `$derived(expression)` — **not** `$derived(() => expression)` (the latter stores a function, not a value)
- Cross-component CSS selectors need `:global()`: `:global(.fields-panel > * + *)`

### Tauri Event Channels

- `ffmpeg://log` — log lines from ffmpeg (string)
- `ffmpeg://progress` — FfmpegProgress object
- `ffmpeg://done` — exit code (number)
- `ffmpeg://command` — full ffmpeg command string

### localStorage Keys

- `hathor-theme` — ThemePref ("light" | "dark" | "system")
- `hathor-default-output-dir` — default output directory path
- `hathor-cleanup-default` — "1" or "0"

## Skills

Skills are reusable instruction bundles in `SKILL.md` files.

### Available skills

- `frontend-design`: Create distinctive, production-grade frontend interfaces with strong visual direction and polished implementation. Use for UI design/build requests (pages, components, dashboards, styling/beautification, layout work). (file: `~/.agents/skills/frontend-design/SKILL.md`)
