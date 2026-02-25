<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    runFfmpeg, cancelFfmpeg, onLog, onDone, onProgress, onCommand, probeMedia, expandMediaInputs,
    type FfmpegOperation, type MediaInfo, type FfmpegProgress
  } from "$lib/ffmpeg";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import FileVideo2 from "@lucide/svelte/icons/file-video-2";
  import Scissors from "@lucide/svelte/icons/scissors";
  import Crop from "@lucide/svelte/icons/crop";
  import Combine from "@lucide/svelte/icons/combine";
  import Minimize2 from "@lucide/svelte/icons/minimize-2";
  import Boxes from "@lucide/svelte/icons/boxes";
  import FileAudio2 from "@lucide/svelte/icons/file-audio-2";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import ImageIcon from "@lucide/svelte/icons/image";
  import Captions from "@lucide/svelte/icons/captions";
  import ListVideo from "@lucide/svelte/icons/list-video";
  import Settings2 from "@lucide/svelte/icons/settings-2";

  import type { Tab, Container, QualityMode, TrimMode, Rotate, Flip, Resolution, Fps, QueueJob, QueueStatus, ModeItem } from "$lib/types";
  import ConvertPanel from "$lib/components/panels/ConvertPanel.svelte";
  import TrimPanel from "$lib/components/panels/TrimPanel.svelte";
  import TransformPanel from "$lib/components/panels/TransformPanel.svelte";
  import MergePanel from "$lib/components/panels/MergePanel.svelte";
  import CompressPanel from "$lib/components/panels/CompressPanel.svelte";
  import RemuxPanel from "$lib/components/panels/RemuxPanel.svelte";
  import ThumbnailPanel from "$lib/components/panels/ThumbnailPanel.svelte";
  import ImageSequencePanel from "$lib/components/panels/ImageSequencePanel.svelte";
  import GifMakerPanel from "$lib/components/panels/GifMakerPanel.svelte";
  import ExtractAudioPanel from "$lib/components/panels/ExtractAudioPanel.svelte";
  import ReplaceAudioPanel from "$lib/components/panels/ReplaceAudioPanel.svelte";
  import LoudnessPanel from "$lib/components/panels/LoudnessPanel.svelte";
  import AudioControlsPanel from "$lib/components/panels/AudioControlsPanel.svelte";
  import ImagePanel from "$lib/components/panels/ImagePanel.svelte";
  import BurnSubtitlesPanel from "$lib/components/panels/BurnSubtitlesPanel.svelte";
  import TrackManagerPanel from "$lib/components/panels/TrackManagerPanel.svelte";
  import MediaInfoStrip from "$lib/components/MediaInfoStrip.svelte";
  import ProgressDisplay from "$lib/components/ProgressDisplay.svelte";
  import QueueList from "$lib/components/QueueList.svelte";
  import LogView from "$lib/components/LogView.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";

  const MODES: ModeItem[] = [
    { tab: "convert", label: "Encode", icon: FileVideo2 },
    { tab: "trim", label: "Trim", icon: Scissors },
    { tab: "transform", label: "Edit", icon: Crop },
    { tab: "merge", label: "Merge", icon: Combine },
    { tab: "compress", label: "Shrink", icon: Minimize2 },
    { tab: "remux", label: "Remux", icon: Boxes },
    { tab: "thumbnail", label: "Thumb", icon: ImageIcon },
    { tab: "image_sequence", label: "Frames", icon: ImageIcon },
    { tab: "gif_maker", label: "GIF", icon: ImageIcon },
    { tab: "burn_subtitles", label: "Sub Burn", icon: Captions },
    { tab: "track_manager", label: "Tracks", icon: ListVideo },
    { tab: "extract_audio", label: "Audio", icon: FileAudio2 },
    { tab: "replace_audio", label: "Replace", icon: RefreshCw },
    { tab: "loudness", label: "Loudness", icon: AudioLines },
    { tab: "audio_controls", label: "Audio FX", icon: SlidersHorizontal },
    { tab: "image", label: "Image", icon: ImageIcon },
  ];
  const VIDEO_MODE_TABS: Tab[] = ["convert", "trim", "transform", "merge", "compress", "remux", "thumbnail", "image_sequence", "gif_maker", "burn_subtitles", "track_manager"];
  const AUDIO_MODE_TABS: Tab[] = ["extract_audio", "replace_audio", "loudness", "audio_controls"];
  const IMAGE_MODE_TABS: Tab[] = ["image"];
  const RAIL_GROUPS: { label: string; icon: typeof FileVideo2; tabs: Tab[] }[] = [
    { label: "Video", icon: FileVideo2, tabs: VIDEO_MODE_TABS },
    { label: "Audio", icon: FileAudio2, tabs: AUDIO_MODE_TABS },
    { label: "Image", icon: ImageIcon, tabs: IMAGE_MODE_TABS },
  ];
  const DEFAULT_OUTPUT_DIR_KEY = "hathor-default-output-dir";
  const DEFAULT_CLEANUP_KEY = "hathor-cleanup-default";
  const OUTPUT_TEMPLATE_KEY = "hathor-output-name-template";

  // ── Queue ─────────────────────────────────────────────────────────────────────

  let queue = $state<QueueJob[]>([]);
  let selectedJobId = $state<string | null>(null);
  let queueRunning = $state(false);
  let queuePaused = $state(false);
  let runningJobId = $state("");
  let stopRequested = false;
  let dropActive = $state(false);
  let cancelCleanupEnabled = $state(true);
  let logCollapsed = $state(false);
  let settingsOpen = $state(false);
  let defaultOutputDir = $state("");
  let outputNameTemplate = $state("{name}_out");

  // ── Form state ────────────────────────────────────────────────────────────────

  let activeTab = $state<Tab>("convert");

  let convertInput = $state("");
  let convertOutput = $state("");
  let convertContainer = $state<Container>("mp4");
  let convertQualityMode = $state<QualityMode>("crf");
  let convertCrf = $state(23);
  let convertBitrate = $state("2000k");
  let convertResolution = $state<Resolution>("keep");
  let convertFps = $state<Fps>("keep");

  let trimInput = $state("");
  let trimOutput = $state("");
  let trimMode = $state<TrimMode>("accurate");
  let trimStart = $state("00:00:00");
  let trimDuration = $state("00:00:30");

  let transformInput = $state("");
  let transformOutput = $state("");
  let transformCropEnabled = $state(false);
  let transformCropX = $state(0);
  let transformCropY = $state(0);
  let transformCropWidth = $state(640);
  let transformCropHeight = $state(360);
  let transformPadEnabled = $state(false);
  let transformPadWidth = $state(1280);
  let transformPadHeight = $state(720);
  let transformPadColor = $state("#000000");
  let transformRotate = $state<Rotate>("keep");
  let transformFlip = $state<Flip>("none");

  let mergeInputs = $state<string[]>([]);
  let mergeOutput = $state("");
  let mergeInfos = $state<(MediaInfo | null)[]>([]);

  let compressInput = $state("");
  let compressOutput = $state("");
  let compressCrf = $state(23);
  let compressPreset = $state<"iphone_ipad" | "android" | "youtube" | "tiktok" | "instagram">("youtube");
  let compressSizeTargetEnabled = $state(false);
  let compressSizeTargetMb = $state(25);

  let remuxInput = $state("");
  let remuxOutput = $state("");

  let thumbnailInput = $state("");
  let thumbnailOutput = $state("");
  let thumbnailTime = $state("00:00:01");

  let imageSequenceInput = $state("");
  let imageSequenceOutputPattern = $state("");
  let imageSequenceStart = $state("");
  let imageSequenceDuration = $state("");
  let imageSequenceFps = $state(1);
  let imageSequenceScaleWidth = $state(0);
  let imageSequenceFormat = $state<"png" | "jpg" | "webp">("png");

  let gifMakerInput = $state("");
  let gifMakerOutput = $state("");
  let gifMakerStart = $state("00:00:00");
  let gifMakerDuration = $state("00:00:06");
  let gifMakerFps = $state(15);
  let gifMakerWidth = $state(480);
  let gifMakerUsePalette = $state(true);

  let extractAudioInput = $state("");
  let extractAudioOutput = $state("");
  let extractAudioFormat = $state<"mp3" | "aac" | "opus" | "wav">("mp3");

  let replaceAudioInput = $state("");
  let replaceAudioTrackInput = $state("");
  let replaceAudioOutput = $state("");

  let loudnessInput = $state("");
  let loudnessOutput = $state("");
  let loudnessPreset = $state<"broadcast" | "streaming" | "podcast">("broadcast");

  let audioControlsInput = $state("");
  let audioControlsOutput = $state("");
  let audioControlsVolume = $state(1);
  let audioControlsFadeIn = $state(0);
  let audioControlsFadeOut = $state(0);

  let imageInput = $state("");
  let imageOutput = $state("");
  let imageFormat = $state<"png" | "jpg" | "webp" | "avif" | "ico">("png");
  let imageQuality = $state(82);

  let burnSubtitlesInput = $state("");
  let burnSubtitlesFile = $state("");
  let burnSubtitlesOutput = $state("");

  let trackManagerInput = $state("");
  let trackManagerOutput = $state("");
  let trackManagerKeepAudio = $state<number[]>([]);
  let trackManagerKeepSubtitles = $state<number[]>([]);
  let trackManagerAddAudio = $state("");
  let trackManagerAddSubtitle = $state("");
  let trackManagerInitForInput = $state("");

  // ── Probe + progress ──────────────────────────────────────────────────────────

  let mediaInfo = $state<MediaInfo | null>(null);
  let probing = $state(false);
  let currentProgress = $state<FfmpegProgress | null>(null);

  // ── Listeners ─────────────────────────────────────────────────────────────────

  let unlistenLog: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenProgress: UnlistenFn | null = null;
  let unlistenCommand: UnlistenFn | null = null;
  let unlistenDragDrop: UnlistenFn | null = null;

  onMount(async () => {
    try {
      defaultOutputDir = localStorage.getItem(DEFAULT_OUTPUT_DIR_KEY) ?? "";
      outputNameTemplate = localStorage.getItem(OUTPUT_TEMPLATE_KEY) ?? "{name}_out";
      const cleanupPref = localStorage.getItem(DEFAULT_CLEANUP_KEY);
      if (cleanupPref != null) cancelCleanupEnabled = cleanupPref === "1";
    } catch {}

    unlistenLog = await onLog((line) => {
      const job = queue.find(j => j.id === runningJobId);
      if (job) job.logs.push(line);
    });
    unlistenDone = await onDone((code) => {
      const job = queue.find(j => j.id === runningJobId);
      if (job && job.status === "running") job.status = code === 0 ? "done" : "error";
    });
    unlistenProgress = await onProgress((p) => {
      currentProgress = p;
      const job = queue.find(j => j.id === runningJobId);
      if (job) job.progress = p;
    });
    unlistenCommand = await onCommand((command) => {
      const job = queue.find(j => j.id === runningJobId);
      if (job) job.command = command;
    });

    const appWindow = getCurrentWebviewWindow();
    unlistenDragDrop = await appWindow.onDragDropEvent((event) => {
      const payload = event.payload as { type: string; paths?: string[] };
      if (payload.type === "enter" || payload.type === "over") {
        dropActive = true;
      } else if (payload.type === "leave") {
        dropActive = false;
      } else if (payload.type === "drop") {
        dropActive = false;
        if (payload.paths && payload.paths.length > 0) void importDroppedPaths(payload.paths);
      }
    });
  });

  onDestroy(() => {
    unlistenLog?.();
    unlistenDone?.();
    unlistenProgress?.();
    unlistenCommand?.();
    unlistenDragDrop?.();
  });

  $effect(() => {
    try {
      localStorage.setItem(DEFAULT_OUTPUT_DIR_KEY, defaultOutputDir);
      localStorage.setItem(DEFAULT_CLEANUP_KEY, cancelCleanupEnabled ? "1" : "0");
      localStorage.setItem(OUTPUT_TEMPLATE_KEY, outputNameTemplate);
    } catch {}
  });

  // ── Auto-update output extension when container changes ───────────────────────

  const CONTAINERS: Container[] = ["mp4", "mkv", "mov", "webm", "gif"];
  type KnownOutputContainer = Container | "avi";
  const CONTAINER_HINTS: Record<KnownOutputContainer, { video: string[]; audio: string[] }> = {
    mp4: {
      video: ["h264", "hevc", "mpeg4", "av1"],
      audio: ["aac", "mp3", "ac3", "eac3", "alac"],
    },
    mov: {
      video: ["h264", "hevc", "mpeg4", "prores", "dnxhd", "av1"],
      audio: ["aac", "alac", "pcm_s16le", "pcm_s24le", "ac3"],
    },
    webm: {
      video: ["vp8", "vp9", "av1"],
      audio: ["opus", "vorbis"],
    },
    gif: {
      video: ["gif"],
      audio: [],
    },
    mkv: {
      video: [],
      audio: [],
    },
    avi: {
      video: ["mpeg4", "h264", "mpeg2video", "msmpeg4v3"],
      audio: ["mp3", "ac3", "pcm_s16le"],
    },
  };
  $effect(() => {
    const ext = convertContainer;
    if (!convertOutput) return;
    const dot = convertOutput.lastIndexOf(".");
    if (dot === -1) return;
    const cur = convertOutput.slice(dot + 1).toLowerCase();
    if ((CONTAINERS as string[]).includes(cur)) convertOutput = convertOutput.slice(0, dot + 1) + ext;
  });
  $effect(() => {
    if (!extractAudioOutput) return;
    const dot = extractAudioOutput.lastIndexOf(".");
    if (dot === -1) return;
    const cur = extractAudioOutput.slice(dot + 1).toLowerCase();
    if (["mp3", "aac", "opus", "wav"].includes(cur)) {
      extractAudioOutput = extractAudioOutput.slice(0, dot + 1) + extractAudioFormat;
    }
  });
  $effect(() => {
    if (!imageOutput) return;
    const dot = imageOutput.lastIndexOf(".");
    if (dot === -1) return;
    const cur = imageOutput.slice(dot + 1).toLowerCase();
    if (["png", "jpg", "jpeg", "webp", "avif", "ico"].includes(cur)) {
      imageOutput = imageOutput.slice(0, dot + 1) + imageFormat;
    }
  });
  $effect(() => {
    if (!imageSequenceOutputPattern) return;
    const dot = imageSequenceOutputPattern.lastIndexOf(".");
    if (dot === -1) return;
    const cur = imageSequenceOutputPattern.slice(dot + 1).toLowerCase();
    if (["png", "jpg", "jpeg", "webp"].includes(cur)) {
      imageSequenceOutputPattern = imageSequenceOutputPattern.slice(0, dot + 1) + imageSequenceFormat;
    }
  });
  $effect(() => {
    if (activeTab !== "track_manager") return;
    if (!mediaInfo || !trackManagerInput) return;
    if (trackManagerInitForInput === trackManagerInput) return;
    trackManagerKeepAudio = mediaInfo.streams.filter((s) => s.codec_type === "audio").map((s) => s.index);
    trackManagerKeepSubtitles = mediaInfo.streams.filter((s) => s.codec_type === "subtitle").map((s) => s.index);
    trackManagerInitForInput = trackManagerInput;
  });

  // ── Path helpers ──────────────────────────────────────────────────────────────

  function splitPath(path: string): { dir: string; base: string; ext: string } {
    const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    const dir = slash >= 0 ? path.slice(0, slash + 1) : "";
    const name = slash >= 0 ? path.slice(slash + 1) : path;
    const dot = name.lastIndexOf(".");
    if (dot <= 0) return { dir, base: name, ext: "" };
    return { dir, base: name.slice(0, dot), ext: name.slice(dot + 1) };
  }

  function joinPath(dir: string, file: string): string {
    if (!dir) return file;
    const sep = dir.includes("\\") ? "\\" : "/";
    return dir.endsWith("/") || dir.endsWith("\\") ? `${dir}${file}` : `${dir}${sep}${file}`;
  }

  function sanitizeFilenamePart(value: string): string {
    return value.replace(/[<>:"/\\|?*\x00-\x1F]/g, "_").trim();
  }

  function outputStem(tab: Tab, inputPath: string): string {
    const { base, ext } = splitPath(inputPath);
    const ts = new Date().toISOString().replace(/[-:TZ.]/g, "").slice(0, 14);
    const template = (outputNameTemplate || "{name}_out").trim() || "{name}_out";
    const raw = template
      .replaceAll("{name}", base || "output")
      .replaceAll("{mode}", tab)
      .replaceAll("{ext}", ext || "")
      .replaceAll("{ts}", ts);
    const cleaned = sanitizeFilenamePart(raw);
    return cleaned || `${base || "output"}_out`;
  }

  function parseOutputContainer(path: string): KnownOutputContainer | null {
    const dot = path.lastIndexOf(".");
    if (dot < 0 || dot === path.length - 1) return null;
    const ext = path.slice(dot + 1).toLowerCase();
    if (ext === "avi") return "avi";
    return (CONTAINERS as string[]).includes(ext) ? ext as Container : null;
  }

  function prettyCodec(codec: string): string {
    if (!codec) return "unknown";
    return codec.toUpperCase();
  }

  function remuxCompatibilityWarning(info: MediaInfo | null, outputPath: string): string | null {
    if (!info || !outputPath) return null;
    const container = parseOutputContainer(outputPath);
    if (!container) return null;

    const video = info.streams.find((s) => s.codec_type === "video");
    const audio = info.streams.find((s) => s.codec_type === "audio");

    if (container === "gif") {
      if (audio) return "GIF does not support audio streams. Remux output may fail.";
      if (video && video.codec_name !== "gif") return `GIF expects GIF video, but input video is ${prettyCodec(video.codec_name)}.`;
      return null;
    }

    if (container === "mkv") return null;
    const hints = CONTAINER_HINTS[container];

    if (video && hints.video.length > 0 && !hints.video.includes(video.codec_name)) {
      return `${container.toUpperCase()} may not support ${prettyCodec(video.codec_name)} video in many players. Use Encode instead of Remux.`;
    }
    if (audio && hints.audio.length > 0 && !hints.audio.includes(audio.codec_name)) {
      return `${container.toUpperCase()} may not support ${prettyCodec(audio.codec_name)} audio in many players. Use Encode instead of Remux.`;
    }
    return null;
  }

  function containerCompatIssue(info: MediaInfo | null, container: KnownOutputContainer): string | null {
    if (!info) return null;
    const video = info.streams.find((s) => s.codec_type === "video");
    const audio = info.streams.find((s) => s.codec_type === "audio");
    if (container === "gif") {
      if (audio) return "audio";
      if (video && video.codec_name !== "gif") return "video";
      return null;
    }
    if (container === "mkv") return null;
    const hints = CONTAINER_HINTS[container];
    if (video && hints.video.length > 0 && !hints.video.includes(video.codec_name)) return "video";
    if (audio && hints.audio.length > 0 && !hints.audio.includes(audio.codec_name)) return "audio";
    return null;
  }

  function mergeMismatchWarning(infos: (MediaInfo | null)[], inputs: string[]): string | null {
    if (inputs.length < 2 || infos.length !== inputs.length) return null;
    const first = infos[0];
    if (!first) return "Could not read metadata for the first file.";
    const v0 = first.streams.find((s) => s.codec_type === "video");
    const a0 = first.streams.find((s) => s.codec_type === "audio");
    for (let i = 1; i < infos.length; i += 1) {
      const cur = infos[i];
      if (!cur) return "Could not read metadata for one or more files.";
      const v = cur.streams.find((s) => s.codec_type === "video");
      const a = cur.streams.find((s) => s.codec_type === "audio");
      if (!!v0 !== !!v || !!a0 !== !!a) return "Stream layout mismatch detected (video/audio presence differs).";
      if (v0 && v && (v.codec_name !== v0.codec_name || v.width !== v0.width || v.height !== v0.height)) {
        return "Video codec or resolution mismatch detected; concat copy may fail.";
      }
      if (a0 && a && (a.codec_name !== a0.codec_name || a.channels !== a0.channels || a.sample_rate !== a0.sample_rate)) {
        return "Audio stream mismatch detected; concat copy may fail.";
      }
    }
    return null;
  }

  function cropVideoStream(info: MediaInfo | null) {
    return info?.streams.find((s) => s.codec_type === "video" && s.width != null && s.height != null) ?? null;
  }

  function applyCenteredAspectCrop(aspectW: number, aspectH: number) {
    const stream = cropVideoStream(mediaInfo);
    if (!stream || !stream.width || !stream.height) return;

    const srcW = stream.width;
    const srcH = stream.height;
    const target = aspectW / aspectH;
    const src = srcW / srcH;
    let width = srcW;
    let height = srcH;

    if (src > target) {
      width = Math.floor(srcH * target);
      width -= width % 2;
    } else {
      height = Math.floor(srcW / target);
      height -= height % 2;
    }

    if (width < 2 || height < 2) return;
    transformCropEnabled = true;
    transformCropWidth = width;
    transformCropHeight = height;
    transformCropX = Math.max(0, Math.floor((srcW - width) / 2));
    transformCropY = Math.max(0, Math.floor((srcH - height) / 2));
  }

  function centerCurrentCrop() {
    const stream = cropVideoStream(mediaInfo);
    if (!stream || !stream.width || !stream.height) return;
    const srcW = stream.width;
    const srcH = stream.height;
    const width = Math.min(Math.max(2, transformCropWidth), srcW);
    const height = Math.min(Math.max(2, transformCropHeight), srcH);
    transformCropWidth = width;
    transformCropHeight = height;
    transformCropX = Math.max(0, Math.floor((srcW - width) / 2));
    transformCropY = Math.max(0, Math.floor((srcH - height) / 2));
  }

  function inferOutputPath(tab: Tab, inputPath: string): string {
    const { dir, base, ext } = splitPath(inputPath);
    const outDir = defaultOutputDir || dir;
    const outExt = tab === "convert"
      ? convertContainer
      : tab === "extract_audio"
      ? extractAudioFormat
      : tab === "image"
      ? imageFormat
      : tab === "burn_subtitles" || tab === "track_manager"
      ? (ext || "mp4")
      : tab === "thumbnail"
      ? "png"
      : tab === "image_sequence"
      ? imageSequenceFormat
      : tab === "gif_maker"
      ? "gif"
      : tab === "replace_audio" || tab === "loudness" || tab === "audio_controls"
      ? (ext || "mp4")
      : tab === "remux"
      ? (ext || "mp4")
      : "mp4";
    return joinPath(outDir, `${outputStem(tab, inputPath || base)}.${outExt}`);
  }

  function inferMergeOutputPath(paths: string[]): string {
    const first = paths[0] ?? "";
    if (!first) return "";
    const { dir, ext, base } = splitPath(first);
    const outDir = defaultOutputDir || dir;
    return joinPath(outDir, `${outputStem("merge", first || base)}.${ext || "mp4"}`);
  }

  // ── Build operation ───────────────────────────────────────────────────────────

  function buildOperation(inputOverride?: string, forceAutoOutput = false): FfmpegOperation {
    if (activeTab === "convert") {
      const input = inputOverride ?? convertInput;
      const output = forceAutoOutput || !convertOutput ? inferOutputPath("convert", input) : convertOutput;
      return {
        type: "convert", input, output, container: convertContainer, quality_mode: convertQualityMode,
        crf: convertQualityMode === "crf" ? convertCrf : null,
        bitrate: convertQualityMode === "bitrate" ? convertBitrate : null,
        resolution: convertResolution === "keep" ? null : convertResolution,
        fps: convertFps === "keep" ? null : parseInt(convertFps),
      };
    } else if (activeTab === "trim") {
      const input = inputOverride ?? trimInput;
      const output = forceAutoOutput || !trimOutput ? inferOutputPath("trim", input) : trimOutput;
      return { type: "trim", input, output, start: trimStart, duration: trimDuration, trim_mode: trimMode };
    } else if (activeTab === "transform") {
      const input = inputOverride ?? transformInput;
      const output = forceAutoOutput || !transformOutput ? inferOutputPath("transform", input) : transformOutput;
      return {
        type: "transform", input, output,
        crop: transformCropEnabled ? { x: transformCropX, y: transformCropY, width: transformCropWidth, height: transformCropHeight } : null,
        pad: transformPadEnabled ? { width: transformPadWidth, height: transformPadHeight, color: transformPadColor } : null,
        rotate: transformRotate === "keep" ? null : parseInt(transformRotate) as 90 | 180 | 270,
        flip: transformFlip === "none" ? null : transformFlip,
      };
    } else if (activeTab === "merge") {
      const inputs = inputOverride ? [inputOverride] : mergeInputs;
      const output = forceAutoOutput || !mergeOutput ? inferMergeOutputPath(inputs) : mergeOutput;
      return { type: "merge", inputs, output };
    } else if (activeTab === "compress") {
      const input = inputOverride ?? compressInput;
      const output = forceAutoOutput || !compressOutput ? inferOutputPath("compress", input) : compressOutput;
      return {
        type: "compress",
        input,
        output,
        crf: compressCrf,
        preset: compressPreset,
        target_size_mb: compressSizeTargetEnabled ? Math.max(1, compressSizeTargetMb) : null,
      };
    } else if (activeTab === "extract_audio") {
      const input = inputOverride ?? extractAudioInput;
      const output = forceAutoOutput || !extractAudioOutput ? inferOutputPath("extract_audio", input) : extractAudioOutput;
      return { type: "extract_audio", input, output, format: extractAudioFormat };
    } else if (activeTab === "thumbnail") {
      const input = inputOverride ?? thumbnailInput;
      const output = forceAutoOutput || !thumbnailOutput ? inferOutputPath("thumbnail", input) : thumbnailOutput;
      return { type: "thumbnail", input, output, time: thumbnailTime };
    } else if (activeTab === "image_sequence") {
      const input = inputOverride ?? imageSequenceInput;
      const { dir, base } = splitPath(input);
      const outDir = defaultOutputDir || dir;
      const defaultPattern = joinPath(outDir, `${outputStem("image_sequence", input || base)}_%05d.${imageSequenceFormat}`);
      const output_pattern = forceAutoOutput || !imageSequenceOutputPattern ? defaultPattern : imageSequenceOutputPattern;
      return {
        type: "image_sequence",
        input,
        output_pattern,
        start: imageSequenceStart.trim() || null,
        duration: imageSequenceDuration.trim() || null,
        fps: Math.max(1, imageSequenceFps),
        scale_width: imageSequenceScaleWidth > 0 ? imageSequenceScaleWidth : null,
        format: imageSequenceFormat,
      };
    } else if (activeTab === "gif_maker") {
      const input = inputOverride ?? gifMakerInput;
      const output = forceAutoOutput || !gifMakerOutput ? inferOutputPath("gif_maker", input) : gifMakerOutput;
      return {
        type: "gif_maker",
        input,
        output,
        start: gifMakerStart.trim() || null,
        duration: gifMakerDuration.trim() || null,
        width: gifMakerWidth > 0 ? gifMakerWidth : null,
        fps: Math.max(1, gifMakerFps),
        use_palette: gifMakerUsePalette,
      };
    } else if (activeTab === "replace_audio") {
      const input = inputOverride ?? replaceAudioInput;
      const output = forceAutoOutput || !replaceAudioOutput ? inferOutputPath("replace_audio", input) : replaceAudioOutput;
      return { type: "replace_audio", input, audio_input: replaceAudioTrackInput, output };
    } else if (activeTab === "loudness") {
      const input = inputOverride ?? loudnessInput;
      const output = forceAutoOutput || !loudnessOutput ? inferOutputPath("loudness", input) : loudnessOutput;
      return { type: "loudness", input, output, preset: loudnessPreset };
    } else if (activeTab === "audio_controls") {
      const input = inputOverride ?? audioControlsInput;
      const output = forceAutoOutput || !audioControlsOutput ? inferOutputPath("audio_controls", input) : audioControlsOutput;
      return {
        type: "audio_controls",
        input,
        output,
        volume: Math.max(0, audioControlsVolume),
        fade_in_secs: Math.max(0, audioControlsFadeIn),
        fade_out_secs: Math.max(0, audioControlsFadeOut),
      };
    } else if (activeTab === "image") {
      const input = inputOverride ?? imageInput;
      const output = forceAutoOutput || !imageOutput ? inferOutputPath("image", input) : imageOutput;
      return {
        type: "image_convert",
        input,
        output,
        format: imageFormat,
        quality: Math.max(1, Math.min(100, imageQuality)),
      };
    } else if (activeTab === "burn_subtitles") {
      const input = inputOverride ?? burnSubtitlesInput;
      const output = forceAutoOutput || !burnSubtitlesOutput ? inferOutputPath("burn_subtitles", input) : burnSubtitlesOutput;
      return { type: "burn_subtitles", input, subtitle_input: burnSubtitlesFile, output };
    } else if (activeTab === "track_manager") {
      const input = inputOverride ?? trackManagerInput;
      const output = forceAutoOutput || !trackManagerOutput ? inferOutputPath("track_manager", input) : trackManagerOutput;
      return {
        type: "manage_tracks",
        input,
        output,
        keep_audio_indices: [...trackManagerKeepAudio].sort((a, b) => a - b),
        keep_subtitle_indices: [...trackManagerKeepSubtitles].sort((a, b) => a - b),
        add_audio_input: trackManagerAddAudio.trim() || null,
        add_subtitle_input: trackManagerAddSubtitle.trim() || null,
      };
    } else {
      const input = inputOverride ?? remuxInput;
      const output = forceAutoOutput || !remuxOutput ? inferOutputPath("remux", input) : remuxOutput;
      return { type: "remux", input, output };
    }
  }

  // ── Queue operations ──────────────────────────────────────────────────────────

  function createQueueJob(operation: FfmpegOperation, durationSecs = 0): QueueJob {
    return { id: crypto.randomUUID(), operation, status: "pending", logs: [], progress: null, durationSecs, command: "", cleanupPartial: cancelCleanupEnabled };
  }

  function addToQueue() {
    if (activeTab === "merge" && mergeInputs.length > 1 && mergeConcatMismatchWarning) return;
    if (activeTab === "replace_audio" && !replaceAudioTrackInput) return;
    if (activeTab === "burn_subtitles" && !burnSubtitlesFile) return;
    const job = createQueueJob(buildOperation(), mediaInfo?.duration_secs ?? 0);
    queue.push(job);
    if (!selectedJobId) selectedJobId = job.id;
  }

  async function runJob(job: QueueJob): Promise<void> {
    runningJobId = job.id;
    selectedJobId = job.id;
    job.status = "running";
    job.logs = [];
    job.progress = null;
    job.command = "";
    currentProgress = null;
    try {
      await runFfmpeg(job.operation, { cleanupPartial: job.cleanupPartial });
      if (job.status === "running") job.status = "done";
    } catch (e) {
      job.logs.push(`[error] ${e}`);
      if (job.status === "running") job.status = "error";
    }
    runningJobId = "";
    currentProgress = null;
  }

  async function runQueue() {
    if (queueRunning) return;
    queueRunning = true;
    queuePaused = false;
    stopRequested = false;
    for (const job of queue) {
      if (stopRequested) break;
      if (job.status !== "pending") continue;
      await runJob(job);
    }
    queueRunning = false;
    queuePaused = stopRequested && queue.some((j) => j.status === "pending");
  }

  function stopQueue() { stopRequested = true; }

  async function handleCancel() {
    const job = queue.find(j => j.id === runningJobId);
    if (job) job.status = "cancelled";
    try { await cancelFfmpeg(); } catch {}
  }

  function retryJob(id: string) {
    const job = queue.find(j => j.id === id);
    if (job) { job.status = "pending"; job.logs = []; job.progress = null; job.command = ""; }
  }

  function removeJob(id: string) {
    const idx = queue.findIndex(j => j.id === id);
    if (idx !== -1) queue.splice(idx, 1);
    if (selectedJobId === id) selectedJobId = queue[0]?.id ?? null;
  }

  function clearQueue() {
    if (queueRunning) return;
    queue.splice(0, queue.length);
    selectedJobId = null;
  }

  function moveJob(id: string, dir: -1 | 1) {
    if (queueRunning) return;
    const idx = queue.findIndex((j) => j.id === id);
    if (idx < 0) return;
    const to = idx + dir;
    if (to < 0 || to >= queue.length) return;
    const [job] = queue.splice(idx, 1);
    queue.splice(to, 0, job);
  }

  // ── Probe + file pickers ──────────────────────────────────────────────────────

  async function probeFile(path: string) {
    probing = true;
    mediaInfo = null;
    try { mediaInfo = await probeMedia(path); } catch {}
    probing = false;
  }

  const VIDEO_FILTERS = [{
    name: "Media",
    extensions: [
      "mp4", "mkv", "avi", "mov", "webm", "m4v", "flv", "ts", "wmv", "gif",
      "mp3", "aac", "opus", "wav", "m4a",
      "png", "jpg", "jpeg", "webp", "avif", "ico"
    ]
  }];

  async function pickInput(setter: (v: string) => void) {
    const path = await open({ multiple: false, filters: VIDEO_FILTERS });
    if (typeof path === "string") { setter(path); probeFile(path); }
  }

  async function pickPath(setter: (v: string) => void) {
    const path = await open({ multiple: false, filters: VIDEO_FILTERS });
    if (typeof path === "string") setter(path);
  }

  async function pickOutput(setter: (v: string) => void) {
    const path = await save({ filters: VIDEO_FILTERS });
    if (path) setter(path);
  }

  async function pickDefaultOutputDir() {
    const path = await open({ directory: true, multiple: false });
    if (typeof path === "string") defaultOutputDir = path;
  }

  async function pickMergeInputs() {
    const picked = await open({ multiple: true, filters: VIDEO_FILTERS });
    const paths = Array.isArray(picked) ? picked.filter((p): p is string => typeof p === "string") : [];
    if (paths.length === 0) return;
    const uniq = [...mergeInputs];
    for (const p of paths) if (!uniq.includes(p)) uniq.push(p);
    mergeInputs = uniq;
    if (!mergeOutput) mergeOutput = inferMergeOutputPath(mergeInputs);
    await refreshMergeInfos();
  }

  async function refreshMergeInfos() {
    mergeInfos = await Promise.all(
      mergeInputs.map(async (p) => { try { return await probeMedia(p); } catch { return null; } })
    );
  }

  function removeMergeInput(idx: number) {
    mergeInputs = mergeInputs.filter((_, i) => i !== idx);
    mergeInfos = mergeInfos.filter((_, i) => i !== idx);
  }

  function moveMergeInput(idx: number, dir: -1 | 1) {
    const to = idx + dir;
    if (to < 0 || to >= mergeInputs.length) return;
    const nextInputs = [...mergeInputs];
    const nextInfos = [...mergeInfos];
    [nextInputs[idx], nextInputs[to]] = [nextInputs[to], nextInputs[idx]];
    [nextInfos[idx], nextInfos[to]] = [nextInfos[to], nextInfos[idx]];
    mergeInputs = nextInputs;
    mergeInfos = nextInfos;
  }

  function applySingleImport(path: string) {
    if (activeTab === "convert") {
      convertInput = path;
      if (!convertOutput) convertOutput = inferOutputPath("convert", path);
    } else if (activeTab === "trim") {
      trimInput = path;
      if (!trimOutput) trimOutput = inferOutputPath("trim", path);
    } else if (activeTab === "transform") {
      transformInput = path;
      if (!transformOutput) transformOutput = inferOutputPath("transform", path);
    } else if (activeTab === "merge") {
      if (!mergeInputs.includes(path)) mergeInputs = [...mergeInputs, path];
      if (!mergeOutput) mergeOutput = inferMergeOutputPath(mergeInputs);
      void refreshMergeInfos();
    } else if (activeTab === "compress") {
      compressInput = path;
      if (!compressOutput) compressOutput = inferOutputPath("compress", path);
    } else if (activeTab === "extract_audio") {
      extractAudioInput = path;
      if (!extractAudioOutput) extractAudioOutput = inferOutputPath("extract_audio", path);
    } else if (activeTab === "thumbnail") {
      thumbnailInput = path;
      if (!thumbnailOutput) thumbnailOutput = inferOutputPath("thumbnail", path);
    } else if (activeTab === "image_sequence") {
      imageSequenceInput = path;
      if (!imageSequenceOutputPattern) {
        const { dir, base } = splitPath(path);
        const outDir = defaultOutputDir || dir;
        imageSequenceOutputPattern = joinPath(outDir, `${outputStem("image_sequence", path || base)}_%05d.${imageSequenceFormat}`);
      }
    } else if (activeTab === "gif_maker") {
      gifMakerInput = path;
      if (!gifMakerOutput) gifMakerOutput = inferOutputPath("gif_maker", path);
    } else if (activeTab === "replace_audio") {
      replaceAudioInput = path;
      if (!replaceAudioOutput) replaceAudioOutput = inferOutputPath("replace_audio", path);
    } else if (activeTab === "loudness") {
      loudnessInput = path;
      if (!loudnessOutput) loudnessOutput = inferOutputPath("loudness", path);
    } else if (activeTab === "audio_controls") {
      audioControlsInput = path;
      if (!audioControlsOutput) audioControlsOutput = inferOutputPath("audio_controls", path);
    } else if (activeTab === "image") {
      imageInput = path;
      if (!imageOutput) imageOutput = inferOutputPath("image", path);
    } else if (activeTab === "burn_subtitles") {
      burnSubtitlesInput = path;
      if (!burnSubtitlesOutput) burnSubtitlesOutput = inferOutputPath("burn_subtitles", path);
    } else if (activeTab === "track_manager") {
      trackManagerInput = path;
      trackManagerInitForInput = "";
      if (!trackManagerOutput) trackManagerOutput = inferOutputPath("track_manager", path);
    } else {
      remuxInput = path;
      if (!remuxOutput) remuxOutput = inferOutputPath("remux", path);
    }
    void probeFile(path);
  }

  async function importDroppedPaths(rawPaths: string[]) {
    try {
      const mediaPaths = await expandMediaInputs(rawPaths);
      if (mediaPaths.length === 0) return;
      if (mediaPaths.length === 1) { applySingleImport(mediaPaths[0]); return; }
      if (activeTab === "merge") {
        const uniq = [...mergeInputs];
        for (const p of mediaPaths) if (!uniq.includes(p)) uniq.push(p);
        mergeInputs = uniq;
        if (!mergeOutput) mergeOutput = inferMergeOutputPath(mergeInputs);
        await refreshMergeInfos();
        return;
      }
      for (const path of mediaPaths) queue.push(createQueueJob(buildOperation(path, true)));
      if (!selectedJobId) selectedJobId = queue[0]?.id ?? null;
    } catch {}
  }

  async function copyCommand(command: string) {
    try { await navigator.clipboard.writeText(command); } catch {}
  }

  // ── Derived ───────────────────────────────────────────────────────────────────

  const pendingCount = $derived(queue.filter(j => j.status === "pending").length);
  const selectedJob = $derived(queue.find(j => j.id === selectedJobId) ?? null);
  const activeMode = $derived(MODES.find((m) => m.tab === activeTab) ?? MODES[0]);
  const convertCompatibilityWarning = $derived(
    mediaInfo && activeTab === "convert" && convertContainer === "gif" && mediaInfo.streams.some((s) => s.codec_type === "audio")
      ? "GIF does not support audio. Input audio tracks will be dropped."
      : null
  );
  const convertRemuxSuggestion = $derived(
    activeTab === "convert" &&
    mediaInfo &&
    convertContainer !== "gif" &&
    convertResolution === "keep" &&
    convertFps === "keep" &&
    !containerCompatIssue(mediaInfo, convertContainer)
      ? "Input appears container-compatible with no resize/fps changes. Use Remux for faster, no-quality-loss output."
      : null
  );
  const remuxContainerWarning = $derived(
    activeTab === "remux" ? remuxCompatibilityWarning(mediaInfo, remuxOutput) : null
  );
  const burnSubtitlesWarning = $derived(
    activeTab === "burn_subtitles" && !burnSubtitlesFile ? "Select an SRT or ASS file." : null
  );
  const replaceAudioWarning = $derived(
    activeTab === "replace_audio" && !replaceAudioTrackInput ? "Select an audio track to replace with." : null
  );
  const mergeConcatMismatchWarning = $derived(
    activeTab === "merge" ? mergeMismatchWarning(mergeInfos, mergeInputs) : null
  );
  const mergeConcatReady = $derived(
    activeTab === "merge" && mergeInputs.length >= 2 && mergeInfos.length === mergeInputs.length && !mergeConcatMismatchWarning
  );
  const compressSizeTargetWarning = $derived(
    activeTab === "compress" && compressSizeTargetEnabled && (!Number.isFinite(compressSizeTargetMb) || compressSizeTargetMb < 1)
      ? "Target size must be at least 1 MB."
      : null
  );

  const queueStatus = $derived<QueueStatus>(
    queueRunning ? "running" :
    queuePaused ? "paused" :
    queue.length === 0 ? "idle" :
    queue.some(j => j.status === "error" || j.status === "cancelled") ? "error" :
    queue.every(j => j.status === "done") ? "done" :
    "idle"
  );

  const encodeDuration = $derived(queue.find(j => j.status === "running")?.durationSecs ?? 0);
  const progressPct = $derived(
    encodeDuration > 0 && currentProgress?.time_secs != null
      ? Math.min(100, (currentProgress.time_secs / encodeDuration) * 100)
      : null
  );
  const etaSecs = $derived(
    encodeDuration > 0 && currentProgress?.time_secs != null && currentProgress.speed != null && currentProgress.speed > 0
      ? Math.max(0, (encodeDuration - currentProgress.time_secs) / currentProgress.speed)
      : null
  );
</script>

<div class="relative h-screen flex flex-col overflow-hidden bg-background text-foreground font-sans">

  <!-- Header -->
  <header class="h-12 flex items-center justify-between px-6 border-b border-border flex-shrink-0">
    <span class="text-[11px] font-bold tracking-[0.35em] uppercase select-none">HATHOR</span>
    <div class="flex items-center">
      <div
        class="flex items-center gap-2 px-3 py-1 border text-[9px] font-semibold tracking-[0.2em] uppercase transition-colors"
        class:text-muted-foreground={queueStatus === "idle" || queueStatus === "paused"}
        class:border-border={queueStatus === "idle" || queueStatus === "paused"}
        class:text-foreground={queueStatus === "running" || queueStatus === "done"}
        class:border-foreground={queueStatus === "running" || queueStatus === "done"}
        class:text-destructive={queueStatus === "error"}
        class:border-destructive={queueStatus === "error"}
      >
        <span class="inline-block w-[5px] h-[5px] bg-current flex-shrink-0" class:dot-pulse={queueStatus === "running"}></span>
        {queueStatus}
      </div>
    </div>
  </header>

  <!-- Workspace -->
  <div class="grid grid-cols-[56px_340px_1fr] flex-1 overflow-hidden">

    <!-- Activity Rail -->
    <nav class="activity-rail">
      <div class="activity-main">
        {#each RAIL_GROUPS as group}
          {@const GroupIcon = group.icon}
          <div class="dropdown dropdown-right dropdown-hover activity-master">
            <button
              type="button"
              aria-label={group.label}
              class="activity-btn"
              class:activity-btn-active={!settingsOpen && group.tabs.includes(activeTab)}
            >
              <GroupIcon size={17} strokeWidth={1.8} />
            </button>
            <ul class="dropdown-content menu activity-dropdown">
              <li class="menu-title"><span>{group.label}</span></li>
              {#each MODES.filter((m) => group.tabs.includes(m.tab)) as mode}
                {@const Icon = mode.icon}
                <li>
                  <button
                    type="button"
                    class:activity-dropdown-btn-active={!settingsOpen && activeTab === mode.tab}
                    onclick={() => { activeTab = mode.tab; settingsOpen = false; mediaInfo = null; }}
                  >
                    <Icon size={15} strokeWidth={1.8} />
                    <span>{mode.label}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
      <div class="activity-foot">
        <button
          type="button"
          aria-label="Settings"
          class="activity-btn"
          class:activity-btn-active={settingsOpen}
          onclick={() => settingsOpen = !settingsOpen}
        >
          <Settings2 size={17} strokeWidth={1.8} />
        </button>
      </div>
    </nav>

    <!-- Config Panel -->
    <aside class="flex flex-col border-r border-border overflow-hidden bg-card">
      <div class="h-12 flex items-center justify-between px-5 border-b border-border flex-shrink-0">
        <span class="text-[9px] font-semibold tracking-[0.22em] uppercase text-muted-foreground">Mode</span>
        <span class="text-[10px] font-semibold tracking-[0.18em] uppercase text-foreground">{settingsOpen ? "Settings" : activeMode.label}</span>
      </div>

      <div class="fields-panel flex-1 px-5 py-5 flex flex-col gap-4 overflow-y-auto">
        {#if settingsOpen}
          <SettingsPanel
            bind:defaultOutputDir
            bind:outputNameTemplate
            bind:cancelCleanupEnabled
            onpickdir={pickDefaultOutputDir}
          />
        {:else if activeTab === "convert"}
          <ConvertPanel
            bind:input={convertInput}
            bind:output={convertOutput}
            bind:container={convertContainer}
            bind:qualityMode={convertQualityMode}
            bind:crf={convertCrf}
            bind:bitrate={convertBitrate}
            bind:resolution={convertResolution}
            bind:fps={convertFps}
            compatibilityWarning={convertCompatibilityWarning}
            remuxSuggestion={convertRemuxSuggestion}
            onpickinput={() => pickInput((v) => convertInput = v)}
            onpickoutput={() => pickOutput((v) => convertOutput = v)}
          />
        {:else if activeTab === "trim"}
          <TrimPanel
            bind:input={trimInput}
            bind:output={trimOutput}
            bind:trimMode
            bind:trimStart
            bind:trimDuration
            onpickinput={() => pickInput((v) => trimInput = v)}
            onpickoutput={() => pickOutput((v) => trimOutput = v)}
          />
        {:else if activeTab === "transform"}
          <TransformPanel
            bind:input={transformInput}
            bind:output={transformOutput}
            bind:cropEnabled={transformCropEnabled}
            bind:cropX={transformCropX}
            bind:cropY={transformCropY}
            bind:cropWidth={transformCropWidth}
            bind:cropHeight={transformCropHeight}
            bind:padEnabled={transformPadEnabled}
            bind:padWidth={transformPadWidth}
            bind:padHeight={transformPadHeight}
            bind:padColor={transformPadColor}
            bind:rotate={transformRotate}
            bind:flip={transformFlip}
            sourceWidth={cropVideoStream(mediaInfo)?.width ?? null}
            sourceHeight={cropVideoStream(mediaInfo)?.height ?? null}
            oncrop169={() => applyCenteredAspectCrop(16, 9)}
            oncrop11={() => applyCenteredAspectCrop(1, 1)}
            oncentercrop={centerCurrentCrop}
            onpickinput={() => pickInput((v) => transformInput = v)}
            onpickoutput={() => pickOutput((v) => transformOutput = v)}
          />
        {:else if activeTab === "merge"}
          <MergePanel
            bind:inputs={mergeInputs}
            bind:output={mergeOutput}
            bind:infos={mergeInfos}
            mergeMismatchWarning={mergeConcatMismatchWarning}
            mergeReady={mergeConcatReady}
            onaddfiles={pickMergeInputs}
            onpickoutput={() => pickOutput((v) => mergeOutput = v)}
            onremoveinput={removeMergeInput}
            onmoveinput={moveMergeInput}
            onclearinputs={() => { mergeInputs = []; mergeInfos = []; }}
          />
        {:else if activeTab === "compress"}
          <CompressPanel
            bind:input={compressInput}
            bind:output={compressOutput}
            bind:crf={compressCrf}
            bind:preset={compressPreset}
            bind:sizeTargetEnabled={compressSizeTargetEnabled}
            bind:sizeTargetMb={compressSizeTargetMb}
            onpickinput={() => pickInput((v) => compressInput = v)}
            onpickoutput={() => pickOutput((v) => compressOutput = v)}
          />
          {#if compressSizeTargetWarning}
            <p class="text-[9px] text-destructive">{compressSizeTargetWarning}</p>
          {/if}
        {:else if activeTab === "extract_audio"}
          <ExtractAudioPanel
            bind:input={extractAudioInput}
            bind:output={extractAudioOutput}
            bind:format={extractAudioFormat}
            onpickinput={() => pickInput((v) => extractAudioInput = v)}
            onpickoutput={() => pickOutput((v) => extractAudioOutput = v)}
          />
        {:else if activeTab === "thumbnail"}
          <ThumbnailPanel
            bind:input={thumbnailInput}
            bind:output={thumbnailOutput}
            bind:time={thumbnailTime}
            onpickinput={() => pickInput((v) => thumbnailInput = v)}
            onpickoutput={() => pickOutput((v) => thumbnailOutput = v)}
          />
        {:else if activeTab === "image_sequence"}
          <ImageSequencePanel
            bind:input={imageSequenceInput}
            bind:outputPattern={imageSequenceOutputPattern}
            bind:start={imageSequenceStart}
            bind:duration={imageSequenceDuration}
            bind:fps={imageSequenceFps}
            bind:scaleWidth={imageSequenceScaleWidth}
            bind:format={imageSequenceFormat}
            onpickinput={() => pickInput((v) => imageSequenceInput = v)}
            onpickoutput={() => pickOutput((v) => imageSequenceOutputPattern = v)}
          />
        {:else if activeTab === "gif_maker"}
          <GifMakerPanel
            bind:input={gifMakerInput}
            bind:output={gifMakerOutput}
            bind:start={gifMakerStart}
            bind:duration={gifMakerDuration}
            bind:fps={gifMakerFps}
            bind:width={gifMakerWidth}
            bind:usePalette={gifMakerUsePalette}
            onpickinput={() => pickInput((v) => gifMakerInput = v)}
            onpickoutput={() => pickOutput((v) => gifMakerOutput = v)}
          />
        {:else if activeTab === "replace_audio"}
          <ReplaceAudioPanel
            bind:input={replaceAudioInput}
            bind:audioInput={replaceAudioTrackInput}
            bind:output={replaceAudioOutput}
            warning={replaceAudioWarning}
            onpickinput={() => pickInput((v) => replaceAudioInput = v)}
            onpickaudio={() => pickPath((v) => replaceAudioTrackInput = v)}
            onpickoutput={() => pickOutput((v) => replaceAudioOutput = v)}
          />
        {:else if activeTab === "loudness"}
          <LoudnessPanel
            bind:input={loudnessInput}
            bind:output={loudnessOutput}
            bind:preset={loudnessPreset}
            onpickinput={() => pickInput((v) => loudnessInput = v)}
            onpickoutput={() => pickOutput((v) => loudnessOutput = v)}
          />
        {:else if activeTab === "audio_controls"}
          <AudioControlsPanel
            bind:input={audioControlsInput}
            bind:output={audioControlsOutput}
            bind:volume={audioControlsVolume}
            bind:fadeInSecs={audioControlsFadeIn}
            bind:fadeOutSecs={audioControlsFadeOut}
            onpickinput={() => pickInput((v) => audioControlsInput = v)}
            onpickoutput={() => pickOutput((v) => audioControlsOutput = v)}
          />
        {:else if activeTab === "image"}
          <ImagePanel
            bind:input={imageInput}
            bind:output={imageOutput}
            bind:format={imageFormat}
            bind:quality={imageQuality}
            onpickinput={() => pickInput((v) => imageInput = v)}
            onpickoutput={() => pickOutput((v) => imageOutput = v)}
          />
        {:else if activeTab === "burn_subtitles"}
          <BurnSubtitlesPanel
            bind:input={burnSubtitlesInput}
            bind:subtitleInput={burnSubtitlesFile}
            bind:output={burnSubtitlesOutput}
            onpickinput={() => pickInput((v) => burnSubtitlesInput = v)}
            onpicksubtitle={() => pickPath((v) => burnSubtitlesFile = v)}
            onpickoutput={() => pickOutput((v) => burnSubtitlesOutput = v)}
          />
          {#if burnSubtitlesWarning}
            <p class="text-[9px] text-destructive">{burnSubtitlesWarning}</p>
          {/if}
        {:else if activeTab === "track_manager"}
          <TrackManagerPanel
            bind:input={trackManagerInput}
            bind:output={trackManagerOutput}
            bind:addAudioInput={trackManagerAddAudio}
            bind:addSubtitleInput={trackManagerAddSubtitle}
            bind:keepAudioIndices={trackManagerKeepAudio}
            bind:keepSubtitleIndices={trackManagerKeepSubtitles}
            audioStreams={mediaInfo?.streams.filter((s) => s.codec_type === "audio") ?? []}
            subtitleStreams={mediaInfo?.streams.filter((s) => s.codec_type === "subtitle") ?? []}
            onpickinput={() => pickInput((v) => trackManagerInput = v)}
            onpickoutput={() => pickOutput((v) => trackManagerOutput = v)}
            onpickaddaudio={() => pickPath((v) => trackManagerAddAudio = v)}
            onpickaddsubtitle={() => pickPath((v) => trackManagerAddSubtitle = v)}
          />
        {:else}
          <RemuxPanel
            bind:input={remuxInput}
            bind:output={remuxOutput}
            compatibilityWarning={remuxContainerWarning}
            onpickinput={() => pickInput((v) => remuxInput = v)}
            onpickoutput={() => pickOutput((v) => remuxOutput = v)}
          />
        {/if}

        {#if !settingsOpen}
          <MediaInfoStrip {probing} {mediaInfo} />
        {/if}
      </div>

      <!-- Bottom CTA -->
      <div class="p-4 pt-0 flex-shrink-0">
        {#if settingsOpen}
          <button
            onclick={() => settingsOpen = false}
            class="bg-input border border-border text-foreground text-[10px] tracking-[0.2em] uppercase font-semibold py-3 w-full cursor-pointer hover:bg-muted transition-colors"
          >Back to {activeMode.label}</button>
        {:else}
          <button
            onclick={addToQueue}
            disabled={
              (activeTab === "merge" && mergeInputs.length > 1 && !!mergeConcatMismatchWarning) ||
              (activeTab === "replace_audio" && !replaceAudioTrackInput) ||
              (activeTab === "burn_subtitles" && !burnSubtitlesFile) ||
              !!compressSizeTargetWarning
            }
            class="primary-cta bg-foreground text-background text-[10px] tracking-[0.25em] uppercase font-semibold py-3 w-full border-0 cursor-pointer hover:opacity-90 transition-opacity flex items-center justify-center gap-2 disabled:opacity-40 disabled:cursor-not-allowed"
          >+ Add to Queue</button>
        {/if}
      </div>
    </aside>

    <!-- Queue + Log Panel -->
    <section class="flex flex-col overflow-hidden">

      <!-- Queue header -->
      <div class="h-10 flex items-center justify-between px-5 border-b border-border flex-shrink-0">
        <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">
          Queue{queue.length > 0 ? ` · ${queue.length}` : ""}
        </span>
        <div class="flex items-center gap-4">
          <label class="flex items-center gap-1.5 text-[8px] tracking-[0.12em] uppercase text-muted-foreground cursor-pointer select-none">
            <input type="checkbox" bind:checked={cancelCleanupEnabled} class="accent-current w-3 h-3" />
            Cleanup partial on cancel
          </label>
          {#if queueRunning}
            <button onclick={stopQueue} class="text-[9px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent px-0">Pause after current</button>
            <button onclick={handleCancel} class="text-[9px] font-semibold tracking-[0.15em] uppercase text-destructive hover:opacity-70 transition-opacity cursor-pointer border-0 bg-transparent px-0">Cancel</button>
          {:else}
            <button onclick={runQueue} disabled={pendingCount === 0} class="text-[9px] font-semibold tracking-[0.15em] uppercase text-foreground hover:opacity-70 transition-opacity cursor-pointer border-0 bg-transparent px-0 disabled:opacity-30 disabled:cursor-not-allowed">▶ {queuePaused ? "Resume" : "Run"}{pendingCount > 0 ? ` (${pendingCount})` : ""}</button>
            <button onclick={clearQueue} disabled={queue.length === 0} class="text-[9px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent px-0 disabled:opacity-30 disabled:cursor-not-allowed">Clear</button>
          {/if}
        </div>
      </div>

      <!-- Job list -->
      <div class="flex-shrink-0 overflow-y-auto border-b border-border" style="max-height: 180px;">
        <QueueList
          {queue}
          {selectedJobId}
          {queueRunning}
          onselect={(id) => selectedJobId = id}
          onmove={moveJob}
          onretry={retryJob}
          onremove={removeJob}
        />
      </div>

      <!-- Progress -->
      {#if currentProgress && selectedJobId === runningJobId && runningJobId !== ""}
        <ProgressDisplay
          {currentProgress}
          {progressPct}
          {etaSecs}
          {encodeDuration}
        />
      {/if}

      <!-- Log -->
      <LogView
        {selectedJob}
        bind:logCollapsed
        oncopycommand={copyCommand}
        ontoggle={() => logCollapsed = !logCollapsed}
      />
    </section>

  </div>

  {#if dropActive}
    <div class="absolute inset-0 z-20 pointer-events-none flex items-center justify-center bg-background/80 border-2 border-dashed border-foreground">
      <p class="text-[11px] tracking-[0.2em] uppercase text-foreground">Drop files or folders to import</p>
    </div>
  {/if}
</div>

<style>
  .activity-rail {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-right: 1px solid var(--border);
    background: color-mix(in oklab, var(--card) 90%, var(--background) 10%);
  }

  .activity-main,
  .activity-foot {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    align-items: center;
  }

  .activity-main {
    overflow: visible;
    padding: 4px 0 6px;
  }

  .activity-master {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .activity-dropdown {
    margin-left: 0;
    transform: translateX(-1px);
    min-width: 176px;
    width: 176px;
    border: 1px solid var(--border);
    background: color-mix(in oklab, var(--card) 92%, var(--background) 8%);
    box-shadow:
      0 12px 30px color-mix(in oklab, var(--background) 68%, transparent),
      inset 0 0 0 1px color-mix(in oklab, var(--foreground) 5%, transparent);
    padding: 6px;
    z-index: 40;
  }

  .activity-dropdown :global(.menu-title) {
    padding: 2px 8px 6px;
    font-size: 8px;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--muted-foreground);
  }

  .activity-dropdown :global(li > button) {
    border: 1px solid transparent;
    border-radius: 0;
    color: var(--muted-foreground);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 7px 8px;
    min-height: unset;
    transition: color 120ms ease, background 120ms ease, border-color 120ms ease;
  }

  .activity-dropdown :global(li > button:hover) {
    color: var(--foreground);
    background: var(--muted);
    border-color: color-mix(in oklab, var(--border) 85%, transparent);
  }

  .activity-dropdown-btn-active {
    color: var(--foreground);
    background: color-mix(in oklab, var(--muted) 70%, var(--card) 30%) !important;
    border-color: var(--border) !important;
  }

  .activity-btn {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    border-left: 2px solid transparent;
    border-radius: 2px;
    color: var(--muted-foreground);
    background: transparent;
    cursor: pointer;
    transition: color 120ms ease, background 120ms ease, border-color 120ms ease;
  }

  .activity-btn:hover {
    color: var(--foreground);
    background: var(--muted);
    border-color: color-mix(in oklab, var(--border) 80%, transparent);
  }

  .activity-btn-active {
    color: var(--foreground);
    background: color-mix(in oklab, var(--muted) 70%, var(--card) 30%);
    border-color: var(--border);
    border-left-color: var(--foreground);
  }

  :global(.fields-panel > * + *) {
    border-top: 1px solid color-mix(in oklab, var(--border) 70%, transparent);
    padding-top: 12px;
  }

  .primary-cta {
    border-radius: 2px;
    letter-spacing: 0.26em;
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--background) 45%, transparent);
  }
</style>
