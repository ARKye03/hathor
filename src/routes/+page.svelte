<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import {
    runFfmpeg, cancelFfmpeg, onLog, onDone, onProgress, onCommand, probeMedia, expandMediaInputs,
    type FfmpegOperation, type MediaInfo, type FfmpegProgress
  } from "$lib/ffmpeg";
  import { theme, type ThemePref } from "$lib/theme.svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import FileVideo2 from "@lucide/svelte/icons/file-video-2";
  import Scissors from "@lucide/svelte/icons/scissors";
  import Crop from "@lucide/svelte/icons/crop";
  import Combine from "@lucide/svelte/icons/combine";
  import Minimize2 from "@lucide/svelte/icons/minimize-2";
  import Boxes from "@lucide/svelte/icons/boxes";
  import Settings2 from "@lucide/svelte/icons/settings-2";

  type Tab = "convert" | "trim" | "transform" | "merge" | "compress" | "remux";
  type Container = "mp4" | "mkv" | "mov" | "webm" | "gif";
  type QualityMode = "crf" | "bitrate";
  type TrimMode = "fast" | "accurate";
  type Rotate = "keep" | "90" | "180" | "270";
  type Flip = "none" | "horizontal" | "vertical" | "both";
  type Resolution = "keep" | "1080p" | "720p" | "480p";
  type Fps = "keep" | "24" | "30" | "60";

  interface ModeItem {
    tab: Tab;
    label: string;
    icon: typeof FileVideo2;
  }

  const MODES: ModeItem[] = [
    { tab: "convert", label: "Encode", icon: FileVideo2 },
    { tab: "trim", label: "Trim", icon: Scissors },
    { tab: "transform", label: "Edit", icon: Crop },
    { tab: "merge", label: "Merge", icon: Combine },
    { tab: "compress", label: "Shrink", icon: Minimize2 },
    { tab: "remux", label: "Remux", icon: Boxes },
  ];

  // ── Queue ─────────────────────────────────────────────────────────────────────

  interface QueueJob {
    id: string;
    operation: FfmpegOperation;
    status: "pending" | "running" | "done" | "error" | "cancelled";
    logs: string[];
    progress: FfmpegProgress | null;
    durationSecs: number;
    command: string;
    cleanupPartial: boolean;
  }

  let queue = $state<QueueJob[]>([]);
  let selectedJobId = $state<string | null>(null);
  let queueRunning = $state(false);
  let queuePaused = $state(false);
  let runningJobId = $state(""); // reactive so template can read it
  let stopRequested = false;
  let dropActive = $state(false);
  let cancelCleanupEnabled = $state(true);
  let logCollapsed = $state(false);

  // ── Form state ────────────────────────────────────────────────────────────────

  let activeTab = $state<Tab>("convert");
  let logPanel = $state<HTMLDivElement | null>(null);

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
  let remuxInput = $state("");
  let remuxOutput = $state("");

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
    unlistenLog = await onLog(async (line) => {
      const job = queue.find(j => j.id === runningJobId);
      if (job) {
        job.logs.push(line);
        if (selectedJobId === runningJobId) {
          await tick();
          if (logPanel) logPanel.scrollTop = logPanel.scrollHeight;
        }
      }
    });
    unlistenDone = await onDone((code) => {
      const job = queue.find(j => j.id === runningJobId);
      if (job && job.status === "running") {
        job.status = code === 0 ? "done" : "error";
      }
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
        if (payload.paths && payload.paths.length > 0) {
          void importDroppedPaths(payload.paths);
        }
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

  // ── Auto-update output extension when container changes ───────────────────────

  const CONTAINERS: Container[] = ["mp4", "mkv", "mov", "webm", "gif"];
  $effect(() => {
    const ext = convertContainer;
    if (!convertOutput) return;
    const dot = convertOutput.lastIndexOf(".");
    if (dot === -1) return;
    const cur = convertOutput.slice(dot + 1).toLowerCase();
    if ((CONTAINERS as string[]).includes(cur)) {
      convertOutput = convertOutput.slice(0, dot + 1) + ext;
    }
  });

  // ── Build operation ───────────────────────────────────────────────────────────

  function splitPath(path: string): { dir: string; base: string; ext: string } {
    const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    const dir = slash >= 0 ? path.slice(0, slash + 1) : "";
    const name = slash >= 0 ? path.slice(slash + 1) : path;
    const dot = name.lastIndexOf(".");
    if (dot <= 0) return { dir, base: name, ext: "" };
    return { dir, base: name.slice(0, dot), ext: name.slice(dot + 1) };
  }

  function inferOutputPath(tab: Tab, inputPath: string): string {
    const { dir, base } = splitPath(inputPath);
    const ext = tab === "convert"
      ? convertContainer
      : tab === "transform"
      ? "mp4"
      : tab === "remux"
      ? "mp4"
      : "mp4";
    return `${dir}${base}_out.${ext}`;
  }

  function inferMergeOutputPath(paths: string[]): string {
    const first = paths[0] ?? "";
    if (!first) return "";
    const { dir, ext } = splitPath(first);
    const outExt = ext || "mp4";
    return `${dir}merged_out.${outExt}`;
  }

  function buildOperation(inputOverride?: string, forceAutoOutput = false): FfmpegOperation {
    if (activeTab === "convert") {
      const input = inputOverride ?? convertInput;
      const output = forceAutoOutput || !convertOutput ? inferOutputPath("convert", input) : convertOutput;
      return {
        type: "convert",
        input,
        output,
        container: convertContainer,
        quality_mode: convertQualityMode,
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
        type: "transform",
        input,
        output,
        crop: transformCropEnabled
          ? {
              x: transformCropX,
              y: transformCropY,
              width: transformCropWidth,
              height: transformCropHeight,
            }
          : null,
        pad: transformPadEnabled
          ? {
              width: transformPadWidth,
              height: transformPadHeight,
              color: transformPadColor,
            }
          : null,
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
      return { type: "compress", input, output, crf: compressCrf };
    } else {
      const input = inputOverride ?? remuxInput;
      const output = forceAutoOutput || !remuxOutput ? inferOutputPath("remux", input) : remuxOutput;
      return { type: "remux", input, output };
    }
  }

  // ── Queue operations ──────────────────────────────────────────────────────────

  function createQueueJob(operation: FfmpegOperation, durationSecs = 0): QueueJob {
    return {
      id: crypto.randomUUID(),
      operation,
      status: "pending",
      logs: [],
      progress: null,
      durationSecs,
      command: "",
      cleanupPartial: cancelCleanupEnabled,
    };
  }

  function addToQueue() {
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

  function jobLabel(job: QueueJob): string {
    if (job.operation.type === "merge") {
      const n = job.operation.inputs.length;
      return n > 0 ? `merge ${n} file${n === 1 ? "" : "s"}` : "merge";
    }
    const input = (job.operation as { input: string }).input;
    return input.split("/").pop() ?? input;
  }

  // ── Probe + file pickers ──────────────────────────────────────────────────────

  async function probeFile(path: string) {
    probing = true;
    mediaInfo = null;
    try { mediaInfo = await probeMedia(path); } catch {}
    probing = false;
  }

  const VIDEO_FILTERS = [{ name: "Media", extensions: ["mp4", "mkv", "avi", "mov", "webm", "m4v", "flv", "ts", "wmv", "gif"] }];

  async function pickInput(setter: (v: string) => void) {
    const path = await open({ multiple: false, filters: VIDEO_FILTERS });
    if (typeof path === "string") { setter(path); probeFile(path); }
  }

  async function pickOutput(setter: (v: string) => void) {
    const path = await save({ filters: VIDEO_FILTERS });
    if (path) setter(path);
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
      mergeInputs.map(async (p) => {
        try { return await probeMedia(p); } catch { return null; }
      })
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
      if (mediaPaths.length === 1) {
        applySingleImport(mediaPaths[0]);
        return;
      }
      if (activeTab === "merge") {
        const uniq = [...mergeInputs];
        for (const p of mediaPaths) if (!uniq.includes(p)) uniq.push(p);
        mergeInputs = uniq;
        if (!mergeOutput) mergeOutput = inferMergeOutputPath(mergeInputs);
        await refreshMergeInfos();
        return;
      }
      for (const path of mediaPaths) {
        queue.push(createQueueJob(buildOperation(path, true)));
      }
      if (!selectedJobId) selectedJobId = queue[0]?.id ?? null;
    } catch {}
  }

  async function copyCommand(command: string) {
    try {
      await navigator.clipboard.writeText(command);
    } catch {}
  }

  function toggleLogs() {
    logCollapsed = !logCollapsed;
  }

  // ── Derived ───────────────────────────────────────────────────────────────────

  const isRunning = $derived(queueRunning);
  const pendingCount = $derived(queue.filter(j => j.status === "pending").length);
  const selectedJob = $derived(queue.find(j => j.id === selectedJobId) ?? null);
  const activeMode = $derived(MODES.find((m) => m.tab === activeTab) ?? MODES[0]);

  type QueueStatus = "idle" | "running" | "paused" | "done" | "error";
  const queueStatus = $derived<QueueStatus>(
    queueRunning ? "running" :
    queuePaused ? "paused" :
    queue.length === 0 ? "idle" :
    queue.some(j => j.status === "error" || j.status === "cancelled") ? "error" :
    queue.every(j => j.status === "done") ? "done" :
    "idle"
  );

  const crfLabel = $derived(
    compressCrf <= 17 ? "Lossless" :
    compressCrf <= 23 ? "High Quality" :
    compressCrf <= 28 ? "Good" :
    compressCrf <= 35 ? "Compressed" : "Low Quality"
  );
  const crfFill = $derived(`${((compressCrf / 51) * 100).toFixed(1)}%`);

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
  const mergeMismatchWarning = $derived(() => {
    if (mergeInputs.length < 2 || mergeInfos.length !== mergeInputs.length) return null;
    const first = mergeInfos[0];
    if (!first) return "Could not read metadata for the first file.";
    const v0 = first.streams.find((s) => s.codec_type === "video");
    const a0 = first.streams.find((s) => s.codec_type === "audio");
    for (let i = 1; i < mergeInfos.length; i += 1) {
      const cur = mergeInfos[i];
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
  });

  // ── Formatting ────────────────────────────────────────────────────────────────

  function fmtDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  function fmtBytes(bytes: number): string {
    if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
    if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
    return `${Math.round(bytes / 1e3)} KB`;
  }

  function fmtFps(fr: string | null): string | null {
    if (!fr) return null;
    const [n, d] = fr.split("/").map(Number);
    if (!n || !d) return null;
    const fps = n / d;
    return Number.isInteger(fps) ? `${fps}` : fps.toFixed(3).replace(/\.?0+$/, "");
  }

  function fmtChannels(ch: number | null): string {
    if (ch === 1) return "Mono";
    if (ch === 2) return "Stereo";
    return ch ? `${ch}ch` : "";
  }
</script>

<div class="relative h-screen flex flex-col overflow-hidden bg-background text-foreground font-sans">

  <!-- Header -->
  <header class="h-12 flex items-center justify-between px-6 border-b border-border flex-shrink-0">
    <span class="text-[11px] font-bold tracking-[0.35em] uppercase select-none">HATHOR</span>

    <div class="flex items-center gap-4">
      <!-- Theme switcher -->
      <div class="flex border border-border text-[9px] font-semibold tracking-[0.15em] uppercase">
        {#each (["light", "dark", "system"] as ThemePref[]) as opt, i}
          <button
            onclick={() => theme.pref = opt}
            class="px-2.5 py-1 font-mono border-0 border-r border-border cursor-pointer transition-colors"
            class:bg-primary={theme.pref === opt}
            class:text-primary-foreground={theme.pref === opt}
            class:bg-transparent={theme.pref !== opt}
            class:text-muted-foreground={theme.pref !== opt}
            class:border-r-0={i === 2}
          >{opt}</button>
        {/each}
      </div>

      <!-- Status chip -->
      <div
        class="flex items-center gap-2 px-3 py-1 border text-[9px] font-semibold tracking-[0.2em] uppercase transition-colors"
        class:text-muted-foreground={queueStatus === "idle" || queueStatus === "paused"}
        class:border-border={queueStatus === "idle" || queueStatus === "paused"}
        class:text-foreground={queueStatus === "running" || queueStatus === "done"}
        class:border-foreground={queueStatus === "running" || queueStatus === "done"}
        class:text-destructive={queueStatus === "error"}
        class:border-destructive={queueStatus === "error"}
      >
        <span
          class="inline-block w-[5px] h-[5px] bg-current flex-shrink-0"
          class:dot-pulse={queueStatus === "running"}
        ></span>
        {queueStatus}
      </div>
    </div>
  </header>

  <!-- Workspace -->
  <div class="grid grid-cols-[56px_340px_1fr] flex-1 overflow-hidden">

    <!-- Activity Rail -->
    <nav class="activity-rail">
      <div class="activity-main">
        {#each MODES as mode}
          {@const Icon = mode.icon}
          <button
            type="button"
            title={mode.label}
            aria-label={mode.label}
            onclick={() => { activeTab = mode.tab; mediaInfo = null; }}
            class="activity-btn"
            class:activity-btn-active={activeTab === mode.tab}
          >
            <Icon size={17} strokeWidth={1.8} />
          </button>
        {/each}
      </div>
      <div class="activity-foot">
        <button
          type="button"
          title="Settings"
          aria-label="Settings"
          class="activity-btn"
        >
          <Settings2 size={17} strokeWidth={1.8} />
        </button>
      </div>
    </nav>

    <!-- Config Panel -->
    <aside class="flex flex-col border-r border-border overflow-hidden bg-card">

      <!-- Mode Header -->
      <div class="h-12 flex items-center justify-between px-5 border-b border-border flex-shrink-0">
        <span class="text-[9px] font-semibold tracking-[0.22em] uppercase text-muted-foreground">Mode</span>
        <span class="text-[10px] font-semibold tracking-[0.18em] uppercase text-foreground">{activeMode.label}</span>
      </div>

      <!-- Fields -->
      <div class="fields-panel flex-1 px-5 py-5 flex flex-col gap-4 overflow-y-auto">

        {#if activeTab === "convert"}
          <!-- Input -->
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={convertInput} placeholder="/path/to/input.mkv"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickInput((v) => convertInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>

          <!-- Container -->
          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Container</span>
            <div class="flex border border-border">
              {#each (["mp4", "mkv", "mov", "webm", "gif"] as Container[]) as c, i}
                <button type="button" onclick={() => convertContainer = c}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={convertContainer === c}
                  class:text-primary-foreground={convertContainer === c}
                  class:bg-transparent={convertContainer !== c}
                  class:text-muted-foreground={convertContainer !== c}
                  class:border-r-0={i === 4}
                >{c}</button>
              {/each}
            </div>
          </div>

          <!-- Quality mode -->
          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Quality</span>
            <div class="flex border border-border">
              {#each ([["crf", "CRF"], ["bitrate", "Bitrate"]] as [QualityMode, string][]) as [mode, label]}
                <button type="button" onclick={() => convertQualityMode = mode}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase font-mono border-0 border-r border-border last:border-r-0 cursor-pointer transition-colors"
                  class:bg-primary={convertQualityMode === mode}
                  class:text-primary-foreground={convertQualityMode === mode}
                  class:bg-transparent={convertQualityMode !== mode}
                  class:text-muted-foreground={convertQualityMode !== mode}
                >{label}</button>
              {/each}
            </div>
            {#if convertQualityMode === "crf"}
              <div class="flex items-baseline justify-between text-[9px] mt-1">
                <span class="text-muted-foreground">CRF</span>
                <span class="text-foreground tabular-nums">{convertCrf}</span>
              </div>
              <input type="range" min="0" max="51" bind:value={convertCrf} class="slider w-full"
                style="--fill: {((convertCrf / 51) * 100).toFixed(1)}%" />
              <div class="flex justify-between text-[9px] text-muted-foreground">
                <span>0 · Lossless</span><span>51 · Worst</span>
              </div>
            {:else}
              <input type="text" spellcheck="false" bind:value={convertBitrate} placeholder="2000k"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
            {/if}
          </div>

          <!-- Resolution -->
          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Resolution</span>
            <div class="flex border border-border">
              {#each (["keep", "1080p", "720p", "480p"] as Resolution[]) as r, i}
                <button type="button" onclick={() => convertResolution = r}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={convertResolution === r}
                  class:text-primary-foreground={convertResolution === r}
                  class:bg-transparent={convertResolution !== r}
                  class:text-muted-foreground={convertResolution !== r}
                  class:border-r-0={i === 3}
                >{r}</button>
              {/each}
            </div>
          </div>

          <!-- Frame rate -->
          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Frame Rate</span>
            <div class="flex border border-border">
              {#each (["keep", "24", "30", "60"] as Fps[]) as f, i}
                <button type="button" onclick={() => convertFps = f}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={convertFps === f}
                  class:text-primary-foreground={convertFps === f}
                  class:bg-transparent={convertFps !== f}
                  class:text-muted-foreground={convertFps !== f}
                  class:border-r-0={i === 3}
                >{f === "keep" ? f : `${f} fps`}</button>
              {/each}
            </div>
          </div>

          <!-- Output -->
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={convertOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickOutput((v) => convertOutput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>

        {:else if activeTab === "trim"}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={trimInput} placeholder="/path/to/input.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickInput((v) => trimInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={trimOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickOutput((v) => trimOutput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Cut Mode</span>
            <div class="flex border border-border">
              {#each ([["accurate", "Accurate"], ["fast", "Fast"]] as [TrimMode, string][]) as [mode, label], i}
                <button type="button" onclick={() => trimMode = mode}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={trimMode === mode}
                  class:text-primary-foreground={trimMode === mode}
                  class:bg-transparent={trimMode !== mode}
                  class:text-muted-foreground={trimMode !== mode}
                  class:border-r-0={i === 1}
                >{label}</button>
              {/each}
            </div>
            <p class="text-[9px] text-muted-foreground">
              {trimMode === "fast"
                ? "Keyframe cut, very fast, may be slightly less precise."
                : "Frame-accurate cut, slower due to re-encoding."}
            </p>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <label class="flex flex-col gap-2">
              <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Start</span>
              <input type="text" spellcheck="false" bind:value={trimStart} placeholder="00:00:10"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
            </label>
            <label class="flex flex-col gap-2">
              <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Duration</span>
              <input type="text" spellcheck="false" bind:value={trimDuration} placeholder="00:00:30"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
            </label>
          </div>

        {:else if activeTab === "transform"}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={transformInput} placeholder="/path/to/input.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickInput((v) => transformInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={transformOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickOutput((v) => transformOutput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>

          <div class="flex flex-col gap-2">
            <label class="flex items-center gap-2 text-[9px] font-semibold tracking-[0.18em] uppercase text-muted-foreground">
              <input type="checkbox" bind:checked={transformCropEnabled} class="accent-current w-3 h-3" />
              Crop
            </label>
            {#if transformCropEnabled}
              <div class="grid grid-cols-2 gap-2">
                <input type="number" min="0" bind:value={transformCropX} placeholder="x"
                  class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
                <input type="number" min="0" bind:value={transformCropY} placeholder="y"
                  class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
                <input type="number" min="1" bind:value={transformCropWidth} placeholder="width"
                  class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
                <input type="number" min="1" bind:value={transformCropHeight} placeholder="height"
                  class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
              </div>
            {/if}
          </div>

          <div class="flex flex-col gap-2">
            <label class="flex items-center gap-2 text-[9px] font-semibold tracking-[0.18em] uppercase text-muted-foreground">
              <input type="checkbox" bind:checked={transformPadEnabled} class="accent-current w-3 h-3" />
              Pad
            </label>
            {#if transformPadEnabled}
              <div class="grid grid-cols-2 gap-2">
                <input type="number" min="1" bind:value={transformPadWidth} placeholder="width"
                  class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
                <input type="number" min="1" bind:value={transformPadHeight} placeholder="height"
                  class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
              </div>
              <input type="text" spellcheck="false" bind:value={transformPadColor} placeholder="#000000"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
              <p class="text-[9px] text-muted-foreground">Pads to target size and centers source (letterbox/pillarbox).</p>
            {/if}
          </div>

          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Rotate</span>
            <div class="flex border border-border">
              {#each (["keep", "90", "180", "270"] as Rotate[]) as r, i}
                <button type="button" onclick={() => transformRotate = r}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={transformRotate === r}
                  class:text-primary-foreground={transformRotate === r}
                  class:bg-transparent={transformRotate !== r}
                  class:text-muted-foreground={transformRotate !== r}
                  class:border-r-0={i === 3}
                >{r}</button>
              {/each}
            </div>
          </div>

          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Flip</span>
            <div class="flex border border-border">
              {#each (["none", "horizontal", "vertical", "both"] as Flip[]) as f, i}
                <button type="button" onclick={() => transformFlip = f}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={transformFlip === f}
                  class:text-primary-foreground={transformFlip === f}
                  class:bg-transparent={transformFlip !== f}
                  class:text-muted-foreground={transformFlip !== f}
                  class:border-r-0={i === 3}
                >{f}</button>
              {/each}
            </div>
          </div>

        {:else if activeTab === "merge"}
          <div class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Inputs</span>
            <div class="flex gap-2">
              <button type="button" onclick={pickMergeInputs}
                class="bg-input border border-border text-foreground text-[10px] tracking-[0.14em] uppercase font-semibold px-3 py-2 cursor-pointer hover:bg-muted transition-colors"
              >+ Add Files</button>
              <button type="button" onclick={() => { mergeInputs = []; mergeInfos = []; }}
                class="bg-transparent border border-border text-muted-foreground text-[10px] tracking-[0.14em] uppercase font-semibold px-3 py-2 cursor-pointer hover:text-foreground transition-colors"
              >Clear</button>
            </div>
            <div class="border border-border max-h-36 overflow-y-auto">
              {#if mergeInputs.length === 0}
                <p class="px-3 py-3 text-[10px] text-muted-foreground">Select 2+ files in playback order.</p>
              {:else}
                {#each mergeInputs as p, i}
                  <div class="flex items-center gap-2 px-2 py-1.5 border-b border-border last:border-b-0">
                    <span class="w-5 text-[9px] text-muted-foreground text-right tabular-nums">{String(i + 1).padStart(2, "0")}</span>
                    <span class="flex-1 min-w-0 truncate text-[10px] font-mono text-foreground">{p.split("/").pop() ?? p}</span>
                    <button type="button" onclick={() => moveMergeInput(i, -1)} class="text-[9px] text-muted-foreground hover:text-foreground">↑</button>
                    <button type="button" onclick={() => moveMergeInput(i, 1)} class="text-[9px] text-muted-foreground hover:text-foreground">↓</button>
                    <button type="button" onclick={() => removeMergeInput(i)} class="text-[9px] text-muted-foreground hover:text-destructive">×</button>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={mergeOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickOutput((v) => mergeOutput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <p class="text-[9px] text-muted-foreground">Uses concat demuxer with stream copy for matching files.</p>
          {#if mergeMismatchWarning}
            <p class="text-[9px] text-destructive">{mergeMismatchWarning}</p>
          {/if}

        {:else if activeTab === "compress"}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={compressInput} placeholder="/path/to/input.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickInput((v) => compressInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={compressOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickOutput((v) => compressOutput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <div class="flex flex-col gap-3">
            <div class="flex items-baseline justify-between">
              <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">CRF</span>
              <span class="text-[10px] text-foreground">{compressCrf} — {crfLabel}</span>
            </div>
            <input type="range" min="0" max="51" bind:value={compressCrf} class="slider w-full" style="--fill: {crfFill}" />
            <div class="flex justify-between text-[9px] text-muted-foreground">
              <span>0 · Lossless</span>
              <span>51 · Worst</span>
            </div>
          </div>

        {:else}
          <!-- Remux -->
          <div class="flex flex-col gap-1.5 pb-1">
            <p class="text-[9px] text-muted-foreground tracking-wider">Copies all streams without re-encoding. Instant, lossless.</p>
            <div class="flex items-center gap-1.5 text-[9px] font-semibold text-foreground tracking-wider uppercase">
              <span class="inline-block w-1.5 h-1.5 bg-foreground"></span>Fast · No quality loss
            </div>
          </div>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={remuxInput} placeholder="/path/to/input.mkv"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickInput((v) => remuxInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={remuxOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" aria-label="Browse" onclick={() => pickOutput((v) => remuxOutput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
        {/if}

        <!-- Media info strip -->
        {#if probing}
          <div class="border-t border-border pt-4 mt-1">
            <p class="text-[9px] text-muted-foreground tracking-widest uppercase animate-pulse">Probing…</p>
          </div>
        {:else if mediaInfo}
          {@const video = mediaInfo.streams.find(s => s.codec_type === "video")}
          {@const audios = mediaInfo.streams.filter(s => s.codec_type === "audio")}
          <div class="border-t border-border pt-4 mt-1 flex flex-col gap-1.5">
            <div class="flex justify-between text-[9px]">
              <span class="text-muted-foreground tracking-wider uppercase">Duration</span>
              <span class="text-foreground tabular-nums">{fmtDuration(mediaInfo.duration_secs)}</span>
            </div>
            {#if mediaInfo.size_bytes}
              <div class="flex justify-between text-[9px]">
                <span class="text-muted-foreground tracking-wider uppercase">Size</span>
                <span class="text-foreground">{fmtBytes(mediaInfo.size_bytes)}</span>
              </div>
            {/if}
            {#if video}
              <div class="flex justify-between text-[9px]">
                <span class="text-muted-foreground tracking-wider uppercase">Video</span>
                <span class="text-foreground">
                  {video.codec_name.toUpperCase()}
                  {#if video.width && video.height} · {video.width}×{video.height}{/if}
                  {#if fmtFps(video.frame_rate)} · {fmtFps(video.frame_rate)} fps{/if}
                </span>
              </div>
            {/if}
            {#each audios as a}
              <div class="flex justify-between text-[9px]">
                <span class="text-muted-foreground tracking-wider uppercase">Audio{a.language ? ` (${a.language})` : ""}</span>
                <span class="text-foreground">{a.codec_name.toUpperCase()} · {fmtChannels(a.channels)}</span>
              </div>
            {/each}
          </div>
        {/if}

      </div>

      <!-- Add to Queue button -->
      <div class="p-4 pt-0 flex-shrink-0">
        <button
          onclick={addToQueue}
          class="primary-cta bg-foreground text-background text-[10px] tracking-[0.25em] uppercase font-semibold py-3 w-full border-0 cursor-pointer hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
        >
          + Add to Queue
        </button>
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
          {#if isRunning}
            <button
              onclick={stopQueue}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent px-0"
            >Pause after current</button>
            <button
              onclick={handleCancel}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-destructive hover:opacity-70 transition-opacity cursor-pointer border-0 bg-transparent px-0"
            >Cancel</button>
          {:else}
            <button
              onclick={runQueue}
              disabled={pendingCount === 0}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-foreground hover:opacity-70 transition-opacity cursor-pointer border-0 bg-transparent px-0 disabled:opacity-30 disabled:cursor-not-allowed"
            >▶ {queuePaused ? "Resume" : "Run"}{pendingCount > 0 ? ` (${pendingCount})` : ""}</button>
            <button
              onclick={clearQueue}
              disabled={queue.length === 0}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent px-0 disabled:opacity-30 disabled:cursor-not-allowed"
            >Clear</button>
          {/if}
        </div>
      </div>

      <!-- Job list -->
      <div class="flex-shrink-0 overflow-y-auto border-b border-border" style="max-height: 180px;">
        {#if queue.length === 0}
          <div class="px-5 py-5">
            <div class="queue-empty">
              <div class="queue-empty-grid"></div>
              <p class="text-[11px] text-muted-foreground text-center tracking-widest">— add jobs using the form —</p>
            </div>
          </div>
        {:else}
          {#each queue as job}
            <div
              role="button"
              tabindex="0"
              onclick={() => selectedJobId = job.id}
              onkeydown={(e) => e.key === "Enter" && (selectedJobId = job.id)}
              class="flex items-center gap-3 px-5 py-2.5 border-b border-border last:border-b-0 cursor-pointer hover:bg-muted transition-colors"
              class:bg-muted={selectedJobId === job.id}
            >
              <!-- Status dot -->
              <span
                class="w-[5px] h-[5px] flex-shrink-0 bg-current"
                class:text-muted-foreground={job.status === "pending"}
                class:text-foreground={job.status === "running" || job.status === "done"}
                class:text-destructive={job.status === "error" || job.status === "cancelled"}
                class:dot-pulse={job.status === "running"}
              ></span>
              <!-- Type badge -->
              <span class="text-[8px] uppercase tracking-wider text-muted-foreground w-14 flex-shrink-0">{job.operation.type}</span>
              <!-- Filename -->
              <span class="flex-1 text-[11px] text-foreground truncate">{jobLabel(job)}</span>
              {#if !queueRunning}
                <button
                  type="button"
                  aria-label="Move job up"
                  onclick={(e) => { e.stopPropagation(); moveJob(job.id, -1); }}
                  class="text-[9px] text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
                >↑</button>
                <button
                  type="button"
                  aria-label="Move job down"
                  onclick={(e) => { e.stopPropagation(); moveJob(job.id, 1); }}
                  class="text-[9px] text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
                >↓</button>
              {/if}
              <!-- Action -->
              {#if job.status === "running"}
                <span class="spinner flex-shrink-0"></span>
              {:else if job.status === "done"}
                <span class="text-[9px] text-foreground flex-shrink-0">✓</span>
              {:else if job.status === "error" || job.status === "cancelled"}
                <button
                  type="button"
                  aria-label="Retry job"
                  onclick={(e) => { e.stopPropagation(); retryJob(job.id); }}
                  class="text-[9px] text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
                >↺</button>
              {:else if job.status === "pending"}
                <button
                  type="button"
                  aria-label="Remove job"
                  onclick={(e) => { e.stopPropagation(); removeJob(job.id); }}
                  class="text-[9px] text-muted-foreground hover:text-destructive transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
                >×</button>
              {/if}
            </div>
          {/each}
        {/if}
      </div>

      <!-- Progress (shown when the selected job is the running one) -->
      {#if currentProgress && selectedJobId === runningJobId && runningJobId !== ""}
        <div class="border-b border-border px-5 py-4 flex flex-col gap-3 flex-shrink-0">
          <div class="flex items-center gap-3">
            <div class="flex-1 h-px bg-border relative overflow-visible">
              {#if progressPct !== null}
                <div
                  class="absolute inset-y-0 left-0 bg-foreground transition-[width] duration-300"
                  style="width: {progressPct.toFixed(1)}%; height: 1px;"
                ></div>
              {:else}
                <div class="progress-indeterminate" style="height: 1px; background: var(--foreground);"></div>
              {/if}
            </div>
            {#if progressPct !== null}
              <span class="text-[9px] text-foreground tabular-nums w-9 text-right shrink-0">{progressPct.toFixed(0)}%</span>
            {/if}
          </div>
          <div class="flex flex-wrap gap-x-5 gap-y-1 text-[9px]">
            {#if currentProgress.time}
              <span class="text-foreground tabular-nums">
                {currentProgress.time}{encodeDuration > 0 ? ` / ${fmtDuration(encodeDuration)}` : ""}
              </span>
            {/if}
            {#if currentProgress.speed != null}
              <span class="text-muted-foreground">{currentProgress.speed.toFixed(2)}x</span>
            {/if}
            {#if etaSecs != null}
              <span class="text-muted-foreground">ETA {fmtDuration(etaSecs)}</span>
            {/if}
            {#if currentProgress.fps != null && currentProgress.fps > 0}
              <span class="text-muted-foreground">{Math.round(currentProgress.fps)} fps</span>
            {/if}
            {#if currentProgress.bitrate}
              <span class="text-muted-foreground">{currentProgress.bitrate}</span>
            {/if}
            {#if currentProgress.frame != null}
              <span class="text-muted-foreground">frame {currentProgress.frame}</span>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Log header -->
      <div class="h-10 flex items-center justify-between px-5 border-b border-border flex-shrink-0">
        <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground truncate">
          {selectedJob ? jobLabel(selectedJob) : "Output Log"}
        </span>
        <div class="flex items-center gap-3 flex-shrink-0 ml-3">
          <span class="text-[9px] text-muted-foreground tabular-nums">{selectedJob?.logs.length ?? 0} lines</span>
          <button
            type="button"
            onclick={toggleLogs}
            class="log-toggle text-[8px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border border-border bg-transparent px-2 py-1"
          >
            {logCollapsed ? "Expand Logs" : "Collapse Logs"}
          </button>
        </div>
      </div>

      {#if !logCollapsed && selectedJob?.command}
        <div class="px-5 py-2 border-b border-border flex items-center gap-3">
          <code class="flex-1 text-[9px] text-muted-foreground truncate font-mono">{selectedJob.command}</code>
          <button
            type="button"
            onclick={() => copyCommand(selectedJob.command)}
            class="text-[8px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent"
          >Copy</button>
        </div>
      {/if}

      <!-- Log body -->
      <div
        class="log-shell border-b border-border"
        class:log-shell-collapsed={logCollapsed}
      >
      {#if logCollapsed}
        <div class="h-10 px-5 flex items-center justify-between text-[10px]">
          <span class="text-muted-foreground tracking-widest uppercase">Logs collapsed</span>
          <span class="text-foreground truncate max-w-[60%] text-right">
            {selectedJob?.logs.at(-1) ?? "— awaiting process —"}
          </span>
        </div>
      {:else}
      <div class="flex-1 overflow-y-auto py-3" bind:this={logPanel}>
        {#if !selectedJob || selectedJob.logs.length === 0}
          <p class="px-5 py-8 text-[11px] text-muted-foreground text-center tracking-widest">— awaiting process —</p>
        {:else}
          {#each selectedJob.logs as line, i}
            <div class="flex gap-4 px-5 py-px text-[11px] leading-relaxed font-mono hover:bg-muted/70">
              <span class="text-muted-foreground select-none shrink-0 w-9 text-right opacity-40 tabular-nums">{String(i + 1).padStart(4, "0")}</span>
              <span class="text-foreground break-all whitespace-pre-wrap">{line}</span>
            </div>
          {/each}
        {/if}
      </div>
      {/if}
      </div>
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
    gap: 8px;
    width: 100%;
    align-items: center;
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

  /* Browse button — flush height with adjacent input */
  .browse-btn {
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--secondary);
    color: var(--secondary-foreground);
    border: 1px solid var(--border);
    border-left: none;
    cursor: pointer;
    transition: background 0.12s;
    flex-shrink: 0;
    border-radius: 2px;
  }
  .browse-btn:hover { background: var(--muted); }

  .fields-panel > * + * {
    border-top: 1px solid color-mix(in oklab, var(--border) 70%, transparent);
    padding-top: 12px;
  }

  .primary-cta {
    border-radius: 2px;
    letter-spacing: 0.26em;
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--background) 45%, transparent);
  }

  .queue-empty {
    border: 1px solid var(--border);
    border-radius: 2px;
    min-height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    background: color-mix(in oklab, var(--card) 84%, var(--background) 16%);
  }

  .queue-empty-grid {
    position: absolute;
    inset: 0;
    opacity: 0.25;
    background-image:
      linear-gradient(to right, color-mix(in oklab, var(--border) 60%, transparent) 1px, transparent 1px),
      linear-gradient(to bottom, color-mix(in oklab, var(--border) 60%, transparent) 1px, transparent 1px);
    background-size: 18px 18px;
  }

  .log-toggle {
    border-radius: 2px;
  }

  /* Range slider */
  .slider {
    appearance: none;
    -webkit-appearance: none;
    height: 1px;
    background: linear-gradient(
      to right,
      var(--foreground) var(--fill, 45%),
      var(--border) var(--fill, 45%)
    );
    outline: none;
    cursor: pointer;
  }
  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 11px;
    height: 11px;
    background: var(--foreground);
    cursor: pointer;
  }

  /* Pulsing status dot */
  .dot-pulse { animation: pulse 1s ease-in-out infinite; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.15; }
  }

  /* Spinner */
  .spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* Indeterminate progress bar */
  .progress-indeterminate {
    position: absolute;
    top: 0;
    width: 30%;
    animation: indeterminate 1.4s ease-in-out infinite;
  }
  @keyframes indeterminate {
    0%   { left: -30%; }
    100% { left: 100%; }
  }

  .log-shell {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    background: color-mix(in oklab, var(--background) 88%, var(--card) 12%);
    transition: flex 180ms ease, max-height 180ms ease;
  }

  .log-shell-collapsed {
    flex: 0 0 auto;
    max-height: 40px;
    border-bottom: none;
  }

  /* Scrollbars */
  ::-webkit-scrollbar { width: 3px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border); }
  ::-webkit-scrollbar-thumb:hover { background: var(--foreground); }
</style>
