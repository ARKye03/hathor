Here’s a **curated, “makes sense together” feature set** for a cross-platform Tauri FFmpeg wrapper—organized like a product: start simple, add power in logical layers, and avoid random one-off buttons.

## Core workflow (MVP)

These are the features that make it useful on day 1.

- **Import media**
  - Drag & drop files / folders
  - Auto-read metadata (via `ffprobe`): duration, codecs, bitrate, resolution, audio tracks, subtitles

- **Preset-based Convert**
  - Output container: MP4 / MKV / MOV / WebM / GIF
  - Quality modes: _Target quality (CRF/CQ)_ and _Target bitrate_
  - Resolution: keep / scale to common sizes (1080p/720p/480p) + “match source”
  - Frame rate: keep / set (optional)

- **Queue + Batch processing**
  - Multiple jobs, reorder, pause/resume, retry failed
  - Per-job logs + last command used

- **Progress + cancellation**
  - Progress bar, ETA, speed
  - Cancel job safely (and cleanup partial output option)

## Editing basics (small but high-value)

Keep it “wrapper-simple”: no timelines, just operations FFmpeg excels at.

- **Trim / Cut**
  - Set start/end times
  - “Fast cut” (keyframe) vs “Accurate cut” (re-encode)

- **Crop / Pad / Rotate**
  - Crop preset UI + freeform
  - Letterbox/pillarbox padding
  - Rotate 90/180/270, flip

- **Merge / Concatenate**
  - Combine multiple clips (same format) using concat demuxer
  - Smart warning if sources don’t match

## Audio features (commonly requested)

- **Extract audio**
  - MP3 / AAC / Opus / WAV

- **Replace audio track**
  - Keep video, swap audio file

- **Normalize loudness**
  - EBU R128 (loudnorm) “Podcast / YouTube” style presets

- **Basic audio controls**
  - Volume boost/reduce
  - Fade in/out

## Subtitle & track management (makes it feel “pro”)

- **Burn subtitles in**
  - SRT/ASS into video

- **Mux/Remux tracks**
  - Add/remove audio/subtitle tracks without re-encoding when possible

- **Select audio/subtitle streams**
  - Choose which tracks to keep
  - Language tags (optional)

## Compression / delivery presets (what most people want)

Think “targets” rather than “flags”.

- **Device presets**
  - iPhone/iPad compatible MP4
  - Android-friendly

- **Web presets**
  - YouTube / TikTok / Instagram (resolution + bitrate + audio)
  - WebM (VP9/AV1) optional

- **Size targeting**
  - “Fit to X MB” (two-pass bitrate mode) with quality warning

## Media utilities (fast, addictive tools)

- **Thumbnail / poster**
  - Capture frame at timestamp

- **Image sequence**
  - Export frames (PNG/JPG) every N seconds

- **GIF maker**
  - Trim + scale + fps + palette (good quality)

- **Remux (no re-encode)**
  - MKV → MP4, MOV → MP4, etc. when compatible
  - Clearly shows “fast/no quality loss” badge

## Power-user layer (still curated)

These keep flexibility without turning the app into a terminal.

- **Advanced per-job options drawer**
  - Encoder selection: x264/x265/AV1, hardware encoders (NVENC/QSV/AMF/VideoToolbox) if detected
  - Keyframe interval, preset (fast/medium/slow)
  - Pixel format (yuv420p for compatibility)

- **Custom arguments (guarded)**
  - “Extra args” textbox that appends _after_ validated base args
  - Show the final generated command (copy button)

- **Profile manager**
  - Save/load presets (JSON)
  - Import/export preset packs

## Quality-of-life that matters

- **Output rules**
  - Default output folder + naming templates
  - “Overwrite?” prompt + “auto-increment filename”

- **Smart warnings**
  - “Remux possible” suggestion
  - “Your cut will re-encode” warning
  - “This container doesn’t support that codec” warning

- **History**
  - Recent jobs, success/fail, reopen settings

---

# Recommended “product tiers” (so you don’t overbuild)

### MVP (ship fast)

Convert presets + queue + progress + logs + remux + trim.

### v1 (feels complete)

Crop/pad/rotate, extract/replace audio, GIF maker, subtitle burn-in, basic track selection.

### v1.5+ (power)

Hardware encoders, size targeting, profile manager, guarded custom args.

---

If you tell me your target user (“I just want smaller MP4s” vs “creators/power users”), I can narrow this into a **tight v1 scope** and propose **the exact operations API** (Rust structs → FFmpegPlan → args) to match it.
