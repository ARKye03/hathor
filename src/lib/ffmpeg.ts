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
  | { type: "trim"; input: string; output: string; start: string; duration: string }
  | { type: "compress"; input: string; output: string; crf: number }
  | { type: "remux"; input: string; output: string };

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

export function runFfmpeg(op: FfmpegOperation): Promise<void> {
  return invoke("run_ffmpeg", { operation: op });
}

export function probeMedia(path: string): Promise<MediaInfo> {
  return invoke("probe_media", { path });
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

export function cancelFfmpeg(): Promise<void> {
  return invoke("cancel_ffmpeg");
}
