# AGENTS.md

This file is the authoritative implementation baseline for Hathor. Treat it as non-optional guidance.

## Project Overview

**Hathor** is a Tauri v2 desktop app that wraps FFmpeg in a focused GUI.

- Frontend: SvelteKit (SPA mode, adapter-static)
- Backend: Rust Tauri commands that execute `ffmpeg` / `ffprobe`
- Runtime model: queue-based job execution with logs, progress, cancellation, and cleanup control

## Commands

Development:

```sh
bun tauri dev
```

Frontend only:

```sh
bun dev
```

Frontend checks:

```sh
bun check
```

Rust checks:

```sh
cd src-tauri && cargo check
cd src-tauri && cargo clippy
```

Production build:

```sh
bun tauri build
```

## Architecture

### Frontend (`src/`)

- Svelte 5 runes (`$state`, `$derived`, `$bindable`)
- Single orchestration page: `src/routes/+page.svelte`
- Tauri bridge in `src/lib/ffmpeg.ts`
- Shared app types in `src/lib/types.ts`

### Backend (`src-tauri/`)

- Entry: `src/main.rs` -> `src/lib.rs::run()`
- FFmpeg command construction + process lifecycle: `src-tauri/src/ffmpeg.rs`
- Commands registered in `invoke_handler`

## Implemented Command Surface (Tauri)

The frontend already relies on these commands. Do not remove or rename without coordinated migration.

- `run_ffmpeg`
- `cancel_ffmpeg`
- `probe_media`
- `expand_media_inputs`
- `resolve_output_path`

## Component Baseline

Core components:

- `src/lib/components/BrowseInput.svelte`
- `src/lib/components/MediaInfoStrip.svelte`
- `src/lib/components/ProgressDisplay.svelte`
- `src/lib/components/SettingsPanel.svelte`
- `src/lib/components/QueueList.svelte`
- `src/lib/components/LogView.svelte`

Panels:

- `src/lib/components/panels/ConvertPanel.svelte`
- `src/lib/components/panels/TrimPanel.svelte`
- `src/lib/components/panels/TransformPanel.svelte`
- `src/lib/components/panels/MergePanel.svelte`
- `src/lib/components/panels/CompressPanel.svelte`
- `src/lib/components/panels/RemuxPanel.svelte`
- `src/lib/components/panels/ExtractAudioPanel.svelte`
- `src/lib/components/panels/ReplaceAudioPanel.svelte`
- `src/lib/components/panels/LoudnessPanel.svelte`
- `src/lib/components/panels/AudioControlsPanel.svelte`
- `src/lib/components/panels/ImagePanel.svelte`
- `src/lib/components/panels/ThumbnailPanel.svelte`
- `src/lib/components/panels/ImageSequencePanel.svelte`
- `src/lib/components/panels/GifMakerPanel.svelte`
- `src/lib/components/panels/BurnSubtitlesPanel.svelte`
- `src/lib/components/panels/TrackManagerPanel.svelte`

## Feature Baseline (Already Shipped)

Do not propose these as pending work.

### Video

- Convert (MP4/MKV/MOV/WebM/GIF)
- Trim (fast copy + accurate re-encode)
- Transform (crop/pad/rotate/flip)
- Merge/concatenate with mismatch warnings
- Compress with delivery presets:
  - Device presets: iPhone/iPad, Android
  - Web presets: YouTube, TikTok, Instagram
  - Size targeting (two-pass)
- Remux

### Audio

- Extract audio (MP3/AAC/Opus/WAV)
- Replace audio track
- Loudness normalize (EBU R128-style presets)
- Audio controls (volume, fade in/out)

### Image + Utilities

- Image convert (PNG/JPG/WebP/AVIF/ICO)
- Image resize by percent with selectable resampling method
- Thumbnail capture
- Image sequence export
- GIF maker (trim + scale + fps + palette flow)

### Subtitle / Tracks

- Burn subtitles (SRT/ASS)
- Add/remove tracks (mux/remux where possible)
- Track keep/select UI (audio/subtitle stream selection)

### Queue / Progress / Safety

- Multi-job queue, reorder, retry, pause-after-current, resume
- Per-job logs and command display/copy
- Progress + ETA + speed
- Cancellation with partial output cleanup toggle

### Quality of Life

- Default output directory
- Output naming templates
- Output collision policy (overwrite vs auto-increment)
- History (recent jobs + success/fail/cancel tracking)
- Reopen settings on launch
- Smart warnings (trim re-encode, remux possible, codec/container compatibility)

## UI Baseline

- Left rail is grouped by category (Video / Audio / Image) using flyout menus.
- Settings is a dedicated rail action.
- Keep UX compact and functional; avoid regressing to overflowing flat icon lists.

## localStorage Keys (Active)

- `hathor-theme`
- `hathor-default-output-dir`
- `hathor-cleanup-default`
- `hathor-output-name-template`
- `hathor-output-collision-policy`
- `hathor-reopen-settings`
- `hathor-settings-open`
- `hathor-job-history`

## Non-Negotiable Engineering Rules

- Do not re-implement shipped features as "new."
- Do not break the existing `FfmpegOperation` discriminated union contract across frontend/backend.
- When adding a new operation:
  1. Add variant in `src/lib/ffmpeg.ts`
  2. Add backend variant/args in `src-tauri/src/ffmpeg.rs`
  3. Register command updates in `src-tauri/src/lib.rs` if needed
  4. Wire `buildOperation()` + panel binding in `src/routes/+page.svelte`
- Validate before claiming completion:
  - `bun check`
  - `cd src-tauri && cargo check`

## Svelte 5 Rules

- Use `$bindable()` for child-to-parent panel state.
- Use `$derived(expression)`, not `$derived(() => expression)`.
- Keep comments sparse; only annotate genuinely complex logic.
