# Hathor

A desktop GUI for FFmpeg — no command memorization required.

![Hathor main UI](.github/assets/main.avif)

## What it does

Hathor wraps FFmpeg in a clean interface for common video and audio workflows:

**Video** — encode, trim, transform, merge, compress, remux
**Audio** — extract, replace track, loudness normalize, volume/channel controls

Smart warnings flag codec/container mismatches and suggest remux when re-encoding isn't needed. Output filenames use customizable templates.

## Requirements

- [FFmpeg](https://ffmpeg.org/) available in `PATH`
- [Bun](https://bun.sh/) (frontend toolchain)
- [Rust](https://rustup.rs/) (Tauri backend)

## Running locally

```sh
bun tauri dev
```

![Queue in action](.github/assets/mainOneQueue.avif)

## Stack

- [Tauri v2](https://tauri.app/) — native shell, Rust backend
- [SvelteKit](https://kit.svelte.dev/) — SPA mode, Svelte 5 runes
- FFmpeg — spawned as a child process from Rust
