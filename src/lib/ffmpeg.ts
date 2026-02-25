import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type FfmpegOperation =
  | {
      type: "convert";
      input: string;
      output: string;
      container: string;
      quality_mode: string;
      crf: number | null;
      bitrate: string | null;
      resolution: string | null;
      fps: number | null;
    }
  | { type: "trim"; input: string; output: string; start: string; duration: string; trim_mode: "fast" | "accurate" }
  | {
      type: "transform";
      input: string;
      output: string;
      crop: { x: number; y: number; width: number; height: number } | null;
      pad: { width: number; height: number; color: string } | null;
      rotate: 90 | 180 | 270 | null;
      flip: "horizontal" | "vertical" | "both" | null;
    }
  | { type: "merge"; inputs: string[]; output: string }
  | {
      type: "compress";
      input: string;
      output: string;
      crf: number;
      preset: "iphone_ipad" | "android" | "youtube" | "tiktok" | "instagram";
      target_size_mb: number | null;
    }
  | { type: "remux"; input: string; output: string }
  | { type: "thumbnail"; input: string; output: string; time: string }
  | {
      type: "image_sequence";
      input: string;
      output_pattern: string;
      start: string | null;
      duration: string | null;
      fps: number | null;
      scale_width: number | null;
      format: "png" | "jpg" | "webp";
    }
  | {
      type: "gif_maker";
      input: string;
      output: string;
      start: string | null;
      duration: string | null;
      width: number | null;
      fps: number;
      use_palette: boolean;
    }
  | { type: "extract_audio"; input: string; output: string; format: "mp3" | "aac" | "opus" | "wav" }
  | { type: "replace_audio"; input: string; audio_input: string; output: string }
  | { type: "loudness"; input: string; output: string; preset: "broadcast" | "streaming" | "podcast" }
  | { type: "audio_controls"; input: string; output: string; volume: number; fade_in_secs: number; fade_out_secs: number }
  | { type: "image_convert"; input: string; output: string; format: "png" | "jpg" | "webp" | "avif" | "ico"; quality: number }
  | { type: "burn_subtitles"; input: string; subtitle_input: string; output: string }
  | {
      type: "manage_tracks";
      input: string;
      output: string;
      keep_audio_indices: number[];
      keep_subtitle_indices: number[];
      add_audio_input: string | null;
      add_subtitle_input: string | null;
    };

export interface StreamInfo {
  index: number;
  codec_type: string;
  codec_name: string;
  width: number | null;
  height: number | null;
  frame_rate: string | null;
  sample_rate: number | null;
  channels: number | null;
  language: string | null;
}

export interface MediaInfo {
  duration_secs: number;
  format: string;
  bit_rate: number | null;
  size_bytes: number | null;
  streams: StreamInfo[];
}

export interface FfmpegProgress {
  frame: number | null;
  fps: number | null;
  time: string | null;
  time_secs: number | null;
  speed: number | null;
  bitrate: string | null;
  size_kb: number | null;
}

export function runFfmpeg(op: FfmpegOperation, opts?: { cleanupPartial?: boolean }): Promise<void> {
  return invoke("run_ffmpeg", { operation: op, cleanupPartial: opts?.cleanupPartial ?? true });
}

export function probeMedia(path: string): Promise<MediaInfo> {
  return invoke("probe_media", { path });
}

export function expandMediaInputs(paths: string[]): Promise<string[]> {
  return invoke("expand_media_inputs", { paths });
}

export function resolveOutputPath(path: string, policy: "overwrite" | "auto_increment"): Promise<string> {
  return invoke("resolve_output_path", { path, policy });
}

export function onLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("ffmpeg://log", (e) => cb(e.payload));
}

export function onProgress(cb: (p: FfmpegProgress) => void): Promise<UnlistenFn> {
  return listen<FfmpegProgress>("ffmpeg://progress", (e) => cb(e.payload));
}

export function onDone(cb: (exitCode: number) => void): Promise<UnlistenFn> {
  return listen<number>("ffmpeg://done", (e) => cb(e.payload));
}

export function onCommand(cb: (command: string) => void): Promise<UnlistenFn> {
  return listen<string>("ffmpeg://command", (e) => cb(e.payload));
}

export function cancelFfmpeg(): Promise<void> {
  return invoke("cancel_ffmpeg");
}
