# Hathor

A desktop GUI for FFmpeg, no command memorization required.

![Hathor main UI](.github/assets/main.avif)

## Description

Hathor wraps FFmpeg in a focused desktop interface for media workflows. It supports video, audio, image conversion, batch queue processing, progress reporting, and safety checks.

## Features

### Video

- Convert, trim, transform, merge, compress, remux
- Compression and delivery presets, iPhone/iPad, Android, YouTube, TikTok, Instagram
- Size targeting, fit to X MB with two-pass encoding
- Subtitle burn-in, SRT and ASS
- Track management, keep or add audio and subtitle tracks

### Audio

- Extract audio, MP3, AAC, Opus, WAV
- Replace audio track
- Loudness normalization presets
- Basic audio controls, volume, fade in, fade out

### Image and Utilities

- Image to image conversion, PNG, JPG, WebP, AVIF, ICO
- Resize by percentage with resize method selection
- Thumbnail capture
- Image sequence export
- GIF maker, trim, scale, FPS, palette flow

### Queue and Safety

- Multi-job queue, reorder, retry, pause after current, resume
- Per-job logs and generated command display
- Progress, speed, ETA, cancellation support
- Cleanup partial output on cancel, configurable default
- Smart warnings, codec and container compatibility, remux suggestion, trim re-encode warning

### Quality of Life

- Default output directory
- Output naming templates
- Output collision policy, overwrite or auto-increment
- Job history, recent jobs and success or fail tracking
- Reopen settings on launch

## Requirements

- [FFmpeg](https://ffmpeg.org/) available in `PATH`
- [Bun](https://bun.sh/) (frontend toolchain)
- [Rust](https://rustup.rs/) (Tauri backend)

## Usage

Run the app in development mode:

```sh
bun tauri dev
```

![Queue in action](.github/assets/mainOneQueue.avif)

Frontend only:

```sh
bun dev
```

Frontend type checks:

```sh
bun check
```

Rust checks:

```sh
cd src-tauri && cargo check
```

Build production app:

```sh
bun tauri build
```

## Architecture

- [Tauri v2](https://tauri.app/), native shell and Rust backend
- [SvelteKit](https://kit.svelte.dev/), SPA mode with Svelte 5 runes
- FFmpeg, spawned as child process from Rust

Backend module split in `src-tauri/src/ffmpeg/`:

- `ops.rs`, operation types and enums
- `args.rs`, FFmpeg argument builders (The magic part)
- `progress.rs`, progress parsing
- `probe.rs`, ffprobe and media path expansion
- `ffmpeg.rs`, runtime orchestration and Tauri command entrypoints
