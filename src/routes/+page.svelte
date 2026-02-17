<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import {
    runFfmpeg, cancelFfmpeg, onLog, onDone, onProgress, probeMedia,
    type FfmpegOperation, type MediaInfo, type FfmpegProgress
  } from "$lib/ffmpeg";
  import { theme, type ThemePref } from "$lib/theme.svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";

  type Tab = "convert" | "trim" | "compress" | "remux";
  type Container = "mp4" | "mkv" | "mov" | "webm";
  type QualityMode = "crf" | "bitrate";
  type Resolution = "keep" | "1080p" | "720p" | "480p";
  type Fps = "keep" | "24" | "30" | "60";

  // ── Queue ─────────────────────────────────────────────────────────────────────

  interface QueueJob {
    id: string;
    operation: FfmpegOperation;
    status: "pending" | "running" | "done" | "error" | "cancelled";
    logs: string[];
    progress: FfmpegProgress | null;
    durationSecs: number;
  }

  let queue = $state<QueueJob[]>([]);
  let selectedJobId = $state<string | null>(null);
  let queueRunning = $state(false);
  let runningJobId = $state(""); // reactive so template can read it
  let stopRequested = false;

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
  let trimStart = $state("00:00:00");
  let trimDuration = $state("00:00:30");
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
  });

  onDestroy(() => {
    unlistenLog?.();
    unlistenDone?.();
    unlistenProgress?.();
  });

  // ── Auto-update output extension when container changes ───────────────────────

  const CONTAINERS: Container[] = ["mp4", "mkv", "mov", "webm"];
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

  function buildOperation(): FfmpegOperation {
    if (activeTab === "convert") {
      return {
        type: "convert",
        input: convertInput,
        output: convertOutput,
        container: convertContainer,
        quality_mode: convertQualityMode,
        crf: convertQualityMode === "crf" ? convertCrf : null,
        bitrate: convertQualityMode === "bitrate" ? convertBitrate : null,
        resolution: convertResolution === "keep" ? null : convertResolution,
        fps: convertFps === "keep" ? null : parseInt(convertFps),
      };
    } else if (activeTab === "trim") {
      return { type: "trim", input: trimInput, output: trimOutput, start: trimStart, duration: trimDuration };
    } else if (activeTab === "compress") {
      return { type: "compress", input: compressInput, output: compressOutput, crf: compressCrf };
    } else {
      return { type: "remux", input: remuxInput, output: remuxOutput };
    }
  }

  // ── Queue operations ──────────────────────────────────────────────────────────

  function addToQueue() {
    const job: QueueJob = {
      id: crypto.randomUUID(),
      operation: buildOperation(),
      status: "pending",
      logs: [],
      progress: null,
      durationSecs: mediaInfo?.duration_secs ?? 0,
    };
    queue.push(job);
    if (!selectedJobId) selectedJobId = job.id;
  }

  async function runJob(job: QueueJob): Promise<void> {
    runningJobId = job.id;
    selectedJobId = job.id;
    job.status = "running";
    job.logs = [];
    job.progress = null;
    currentProgress = null;
    try {
      await runFfmpeg(job.operation);
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
    stopRequested = false;
    for (const job of queue) {
      if (stopRequested) break;
      if (job.status !== "pending") continue;
      await runJob(job);
    }
    queueRunning = false;
  }

  function stopQueue() { stopRequested = true; }

  async function handleCancel() {
    const job = queue.find(j => j.id === runningJobId);
    if (job) job.status = "cancelled";
    try { await cancelFfmpeg(); } catch {}
  }

  function retryJob(id: string) {
    const job = queue.find(j => j.id === id);
    if (job) { job.status = "pending"; job.logs = []; job.progress = null; }
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

  function jobLabel(job: QueueJob): string {
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

  const VIDEO_FILTERS = [{ name: "Video", extensions: ["mp4", "mkv", "avi", "mov", "webm", "m4v", "flv", "ts", "wmv"] }];

  async function pickInput(setter: (v: string) => void) {
    const path = await open({ multiple: false, filters: VIDEO_FILTERS });
    if (typeof path === "string") { setter(path); probeFile(path); }
  }

  async function pickOutput(setter: (v: string) => void) {
    const path = await save({ filters: VIDEO_FILTERS });
    if (path) setter(path);
  }

  // ── Derived ───────────────────────────────────────────────────────────────────

  const isRunning = $derived(queueRunning);
  const pendingCount = $derived(queue.filter(j => j.status === "pending").length);
  const selectedJob = $derived(queue.find(j => j.id === selectedJobId) ?? null);

  type QueueStatus = "idle" | "running" | "done" | "error";
  const queueStatus = $derived<QueueStatus>(
    queueRunning ? "running" :
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

<div class="h-screen flex flex-col overflow-hidden bg-background text-foreground font-mono">

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
        class:text-muted-foreground={queueStatus === "idle"}
        class:border-border={queueStatus === "idle"}
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
  <div class="grid grid-cols-[340px_1fr] flex-1 overflow-hidden">

    <!-- Config Panel -->
    <aside class="flex flex-col border-r border-border overflow-hidden bg-card">

      <!-- Tabs -->
      <div class="flex border-b border-border flex-shrink-0">
        {#each (["convert", "trim", "compress", "remux"] as Tab[]) as tab, i}
          <button
            onclick={() => { activeTab = tab; mediaInfo = null; }}
            class="flex-1 py-3 text-[9px] font-semibold tracking-[0.18em] uppercase cursor-pointer bg-transparent border-0 border-r border-border transition-colors"
            class:text-foreground={activeTab === tab}
            class:tab-active={activeTab === tab}
            class:text-muted-foreground={activeTab !== tab}
            class:border-r-0={i === 3}
          >
            {tab}
          </button>
        {/each}
      </div>

      <!-- Fields -->
      <div class="flex-1 px-5 py-5 flex flex-col gap-4 overflow-y-auto">

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
              {#each (["mp4", "mkv", "mov", "webm"] as Container[]) as c, i}
                <button type="button" onclick={() => convertContainer = c}
                  class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
                  class:bg-primary={convertContainer === c}
                  class:text-primary-foreground={convertContainer === c}
                  class:bg-transparent={convertContainer !== c}
                  class:text-muted-foreground={convertContainer !== c}
                  class:border-r-0={i === 3}
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
          class="bg-primary text-primary-foreground font-mono text-[10px] tracking-[0.25em] uppercase font-semibold py-3 w-full border-0 cursor-pointer hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
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
        <div class="flex items-center gap-2">
          {#if isRunning}
            <button
              onclick={stopQueue}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent px-0"
            >Stop after current</button>
            <button
              onclick={handleCancel}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-destructive hover:opacity-70 transition-opacity cursor-pointer border-0 bg-transparent px-0"
            >Cancel</button>
          {:else}
            <button
              onclick={runQueue}
              disabled={pendingCount === 0}
              class="text-[9px] font-semibold tracking-[0.15em] uppercase text-foreground hover:opacity-70 transition-opacity cursor-pointer border-0 bg-transparent px-0 disabled:opacity-30 disabled:cursor-not-allowed"
            >▶ Run{pendingCount > 0 ? ` (${pendingCount})` : ""}</button>
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
          <p class="px-5 py-5 text-[11px] text-muted-foreground text-center tracking-widest">— add jobs using the form —</p>
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
        <span class="text-[9px] text-muted-foreground tabular-nums flex-shrink-0 ml-3">{selectedJob?.logs.length ?? 0} lines</span>
      </div>

      <!-- Log body -->
      <div class="flex-1 overflow-y-auto py-3" bind:this={logPanel}>
        {#if !selectedJob || selectedJob.logs.length === 0}
          <p class="px-5 py-8 text-[11px] text-muted-foreground text-center tracking-widest">— awaiting process —</p>
        {:else}
          {#each selectedJob.logs as line, i}
            <div class="flex gap-4 px-5 py-px text-[11px] leading-relaxed hover:bg-muted">
              <span class="text-muted-foreground select-none shrink-0 w-9 text-right opacity-40 tabular-nums">{String(i + 1).padStart(4, "0")}</span>
              <span class="text-foreground break-all whitespace-pre-wrap">{line}</span>
            </div>
          {/each}
        {/if}
      </div>
    </section>

  </div>
</div>

<style>
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
  }
  .browse-btn:hover { background: var(--muted); }

  /* Active tab: 2px bottom indicator */
  .tab-active {
    box-shadow: inset 0 -2px 0 var(--foreground);
  }

  /* Range slider */
  .slider {
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

  /* Scrollbars */
  ::-webkit-scrollbar { width: 3px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border); }
  ::-webkit-scrollbar-thumb:hover { background: var(--foreground); }
</style>
