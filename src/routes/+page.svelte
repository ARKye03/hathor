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
  import Settings2 from "@lucide/svelte/icons/settings-2";

  import type { Tab, Container, QualityMode, TrimMode, Rotate, Flip, Resolution, Fps, QueueJob, QueueStatus, ModeItem } from "$lib/types";
  import ConvertPanel from "$lib/components/panels/ConvertPanel.svelte";
  import TrimPanel from "$lib/components/panels/TrimPanel.svelte";
  import TransformPanel from "$lib/components/panels/TransformPanel.svelte";
  import MergePanel from "$lib/components/panels/MergePanel.svelte";
  import CompressPanel from "$lib/components/panels/CompressPanel.svelte";
  import RemuxPanel from "$lib/components/panels/RemuxPanel.svelte";
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
  ];
  const DEFAULT_OUTPUT_DIR_KEY = "hathor-default-output-dir";
  const DEFAULT_CLEANUP_KEY = "hathor-cleanup-default";

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
    try {
      defaultOutputDir = localStorage.getItem(DEFAULT_OUTPUT_DIR_KEY) ?? "";
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
    } catch {}
  });

  // ── Auto-update output extension when container changes ───────────────────────

  const CONTAINERS: Container[] = ["mp4", "mkv", "mov", "webm", "gif"];
  $effect(() => {
    const ext = convertContainer;
    if (!convertOutput) return;
    const dot = convertOutput.lastIndexOf(".");
    if (dot === -1) return;
    const cur = convertOutput.slice(dot + 1).toLowerCase();
    if ((CONTAINERS as string[]).includes(cur)) convertOutput = convertOutput.slice(0, dot + 1) + ext;
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

  function inferOutputPath(tab: Tab, inputPath: string): string {
    const { dir, base, ext } = splitPath(inputPath);
    const outDir = defaultOutputDir || dir;
    const outExt = tab === "convert"
      ? convertContainer
      : tab === "remux"
      ? (ext || "mp4")
      : "mp4";
    return joinPath(outDir, `${base}_out.${outExt}`);
  }

  function inferMergeOutputPath(paths: string[]): string {
    const first = paths[0] ?? "";
    if (!first) return "";
    const { dir, ext } = splitPath(first);
    const outDir = defaultOutputDir || dir;
    return joinPath(outDir, `merged_out.${ext || "mp4"}`);
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
      return { type: "compress", input, output, crf: compressCrf };
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

  const VIDEO_FILTERS = [{ name: "Media", extensions: ["mp4", "mkv", "avi", "mov", "webm", "m4v", "flv", "ts", "wmv", "gif"] }];

  async function pickInput(setter: (v: string) => void) {
    const path = await open({ multiple: false, filters: VIDEO_FILTERS });
    if (typeof path === "string") { setter(path); probeFile(path); }
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
        {#each MODES as mode}
          {@const Icon = mode.icon}
          <button
            type="button"
            title={mode.label}
            aria-label={mode.label}
            onclick={() => { activeTab = mode.tab; mediaInfo = null; settingsOpen = false; }}
            class="activity-btn"
            class:activity-btn-active={activeTab === mode.tab && !settingsOpen}
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
            onpickinput={() => pickInput((v) => transformInput = v)}
            onpickoutput={() => pickOutput((v) => transformOutput = v)}
          />
        {:else if activeTab === "merge"}
          <MergePanel
            bind:inputs={mergeInputs}
            bind:output={mergeOutput}
            bind:infos={mergeInfos}
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
            onpickinput={() => pickInput((v) => compressInput = v)}
            onpickoutput={() => pickOutput((v) => compressOutput = v)}
          />
        {:else}
          <RemuxPanel
            bind:input={remuxInput}
            bind:output={remuxOutput}
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
            class="primary-cta bg-foreground text-background text-[10px] tracking-[0.25em] uppercase font-semibold py-3 w-full border-0 cursor-pointer hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
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
