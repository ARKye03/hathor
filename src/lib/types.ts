import type { FfmpegOperation, FfmpegProgress } from "$lib/ffmpeg";
import type { MediaInfo } from "$lib/ffmpeg";

export type Tab = "convert" | "trim" | "transform" | "merge" | "compress" | "remux" | "extract_audio";
export type Container = "mp4" | "mkv" | "mov" | "webm" | "gif";
export type QualityMode = "crf" | "bitrate";
export type TrimMode = "fast" | "accurate";
export type Rotate = "keep" | "90" | "180" | "270";
export type Flip = "none" | "horizontal" | "vertical" | "both";
export type Resolution = "keep" | "1080p" | "720p" | "480p";
export type Fps = "keep" | "24" | "30" | "60";
export type QueueStatus = "idle" | "running" | "paused" | "done" | "error";

export interface ModeItem {
  tab: Tab;
  label: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  icon: any;
}

export interface QueueJob {
  id: string;
  operation: FfmpegOperation;
  status: "pending" | "running" | "done" | "error" | "cancelled";
  logs: string[];
  progress: FfmpegProgress | null;
  durationSecs: number;
  command: string;
  cleanupPartial: boolean;
}

export type { MediaInfo, FfmpegOperation, FfmpegProgress };
