# Hathor Roadmap

This roadmap now tracks implementation status directly.

## Core Workflow (MVP)

- [x] Import media files
  - [x] Drag and drop files
  - [x] Drag and drop folders (recursive media discovery)
  - [x] Auto-read metadata via `ffprobe` (duration, codecs, bitrate, resolution, audio tracks, language tags)

- [ ] Preset-based Convert
  - [x] Output container: MP4 / MKV / MOV / WebM
  - [x] Output container: GIF
  - [x] Quality modes: target quality (CRF) + target bitrate
  - [x] Resolution: keep / 1080p / 720p / 480p
  - [x] Frame rate: keep / set

- [x] Queue + Batch processing
  - [x] Multiple jobs
  - [x] Reorder jobs
  - [x] Pause after current + resume pending queue
  - [x] Retry failed jobs
  - [x] Per-job logs
  - [x] Last command used (shown and copyable)

- [x] Progress + cancellation
  - [x] Progress bar
  - [x] Speed
  - [x] ETA
  - [x] Safe cancel
  - [x] Cleanup partial output option on cancel

## Editing Basics (v1)

- [x] Trim / Cut (start + duration)
  - [x] Fast cut (keyframe copy) vs accurate cut (re-encode) mode switch

- [x] Crop / Pad / Rotate
  - [x] Freeform crop (x/y/width/height)
  - [x] Crop presets (16:9, 1:1, center-crop shortcuts)
  - [x] Letterbox/pillarbox-style padding (target canvas + color)
  - [x] Rotate 90/180/270, flip

- [x] Merge / Concatenate
  - [x] Concatenate flow for matching clips (concat demuxer + stream copy)
  - [x] Source mismatch warnings

## Audio Features

- [x] Extract audio (MP3 / AAC / Opus / WAV)
- [ ] Replace audio track
- [ ] Loudness normalize (EBU R128 presets)
- [ ] Basic audio controls (volume, fade in/out)

## Subtitle & Track Management

- [ ] Burn subtitles (SRT/ASS)
- [ ] Add/remove tracks (mux/remux where possible)
- [ ] Track keep/select UI (audio/subtitle stream selection)

## Compression / Delivery Presets

- [ ] Device presets (iPhone/iPad, Android)
- [ ] Web presets (YouTube/TikTok/Instagram)
- [ ] Size targeting ("fit to X MB", two-pass)

## Media Utilities

- [ ] Thumbnail capture
- [ ] Image sequence export
- [ ] GIF maker (trim + scale + fps + palette)
- [x] Remux with no re-encode, explicit "fast/no quality loss" positioning

## Power Layer (v1.5+)

- [ ] Advanced options drawer
  - [ ] Encoder choice (x264/x265/AV1)
  - [ ] Hardware encoders (NVENC/QSV/AMF/VideoToolbox detection)
  - [ ] Keyframe interval / preset
  - [ ] Pixel format

- [ ] Guarded custom arguments
  - [ ] Extra args input
  - [ ] Final generated command review

- [ ] Profile manager
  - [ ] Save/load presets
  - [ ] Import/export preset packs

## Quality-of-Life

- [ ] Output rules
  - [ ] Default output folder
  - [x] Naming templates
  - [ ] Overwrite/auto-increment policies

- [ ] Smart warnings
  - [x] Remux possible suggestion
  - [x] "This cut will re-encode" warning
  - [x] Codec/container compatibility warning

- [ ] History
  - [ ] Recent jobs
  - [ ] Success/fail tracking
  - [ ] Reopen settings

## Next Implementation Targets

1. Add replace audio track.
2. Add loudness normalize (EBU R128 presets).
3. Add basic audio controls (volume, fade in/out).
