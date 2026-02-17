import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type FfmpegOperation =
  | { type: "convert"; input: string; output: string }
  | { type: "trim"; input: string; output: string; start: string; duration: string }
  | { type: "compress"; input: string; output: string; crf: number };

export function runFfmpeg(op: FfmpegOperation): Promise<void> {
  return invoke("run_ffmpeg", { operation: op });
}

export function onLog(cb: (line: string) => void): Promise<UnlistenFn> {
  return listen<string>("ffmpeg://log", (e) => cb(e.payload));
}

export function onDone(cb: (exitCode: number) => void): Promise<UnlistenFn> {
  return listen<number>("ffmpeg://done", (e) => cb(e.payload));
}
