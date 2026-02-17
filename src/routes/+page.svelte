<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import {
    runFfmpeg, onLog, onDone, onProgress, probeMedia,
    type FfmpegOperation, type MediaInfo, type FfmpegProgress
  } from "$lib/ffmpeg";
  import { theme, type ThemePref } from "$lib/theme.svelte";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";

  type Tab = "convert" | "trim" | "compress";
  type Status = "idle" | "running" | "done" | "error";

  let activeTab = $state<Tab>("convert");
  let status = $state<Status>("idle");
  let logs = $state<string[]>([]);
  let logPanel = $state<HTMLDivElement | null>(null);

  let convertInput = $state("");
  let convertOutput = $state("");
  let trimInput = $state("");
  let trimOutput = $state("");
  let trimStart = $state("00:00:00");
  let trimDuration = $state("00:00:30");
  let compressInput = $state("");
  let compressOutput = $state("");
  let compressCrf = $state(23);

  // Probing + progress state
  let mediaInfo = $state<MediaInfo | null>(null);
  let probing = $state(false);
  let currentProgress = $state<FfmpegProgress | null>(null);
  let encodeDuration = $state(0);

  let unlistenLog: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;
  let unlistenProgress: UnlistenFn | null = null;

  onMount(async () => {
    unlistenLog = await onLog(async (line) => {
      logs = [...logs, line];
      await tick();
      if (logPanel) logPanel.scrollTop = logPanel.scrollHeight;
    });
    unlistenDone = await onDone((code) => {
      status = code === 0 ? "done" : "error";
    });
    unlistenProgress = await onProgress((p) => {
      currentProgress = p;
    });
  });

  onDestroy(() => {
    unlistenLog?.();
    unlistenDone?.();
    unlistenProgress?.();
  });

  function buildOperation(): FfmpegOperation {
    if (activeTab === "convert") {
      return { type: "convert", input: convertInput, output: convertOutput };
    } else if (activeTab === "trim") {
      return { type: "trim", input: trimInput, output: trimOutput, start: trimStart, duration: trimDuration };
    } else {
      return { type: "compress", input: compressInput, output: compressOutput, crf: compressCrf };
    }
  }

  async function handleRun() {
    status = "running";
    logs = [];
    currentProgress = null;
    encodeDuration = mediaInfo?.duration_secs ?? 0;
    try {
      await runFfmpeg(buildOperation());
    } catch (e) {
      logs = [...logs, `[error] ${e}`];
      status = "error";
    }
  }

  async function probeFile(path: string) {
    probing = true;
    mediaInfo = null;
    try {
      mediaInfo = await probeMedia(path);
    } catch {}
    probing = false;
  }

  const VIDEO_FILTERS = [{ name: "Video", extensions: ["mp4", "mkv", "avi", "mov", "webm", "m4v", "flv", "ts", "wmv"] }];

  async function pickInput(setter: (v: string) => void) {
    const path = await open({ multiple: false, filters: VIDEO_FILTERS });
    if (typeof path === "string") {
      setter(path);
      probeFile(path);
    }
  }

  async function pickOutput(setter: (v: string) => void) {
    const path = await save({ filters: VIDEO_FILTERS });
    if (path) setter(path);
  }

  const isRunning = $derived(status === "running");

  const crfLabel = $derived(
    compressCrf <= 17 ? "Lossless" :
    compressCrf <= 23 ? "High Quality" :
    compressCrf <= 28 ? "Good" :
    compressCrf <= 35 ? "Compressed" : "Low Quality"
  );
  const crfFill = $derived(`${((compressCrf / 51) * 100).toFixed(1)}%`);

  const progressPct = $derived(
    encodeDuration > 0 && currentProgress?.time_secs != null
      ? Math.min(100, (currentProgress.time_secs / encodeDuration) * 100)
      : null
  );

  // ── Formatting helpers ────────────────────────────────────────────────────

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
        class:text-muted-foreground={status === "idle"}
        class:border-border={status === "idle"}
        class:text-foreground={status === "running" || status === "done"}
        class:border-foreground={status === "running" || status === "done"}
        class:text-destructive={status === "error"}
        class:border-destructive={status === "error"}
      >
        <span
          class="inline-block w-[5px] h-[5px] bg-current flex-shrink-0"
          class:dot-pulse={status === "running"}
        ></span>
        {status}
      </div>
    </div>
  </header>

  <!-- Workspace -->
  <div class="grid grid-cols-[340px_1fr] flex-1 overflow-hidden">

    <!-- Config Panel -->
    <aside class="flex flex-col border-r border-border overflow-hidden bg-card">

      <!-- Tabs -->
      <div class="flex border-b border-border flex-shrink-0">
        {#each (["convert", "trim", "compress"] as Tab[]) as tab, i}
          <button
            onclick={() => { activeTab = tab; mediaInfo = null; }}
            class="flex-1 py-3 text-[9px] font-semibold tracking-[0.18em] uppercase cursor-pointer bg-transparent border-0 border-r border-border transition-colors"
            class:text-foreground={activeTab === tab}
            class:tab-active={activeTab === tab}
            class:text-muted-foreground={activeTab !== tab}
            class:border-r-0={i === 2}
          >
            {tab}
          </button>
        {/each}
      </div>

      <!-- Fields -->
      <div class="flex-1 px-5 py-5 flex flex-col gap-4 overflow-y-auto">

        {#if activeTab === "convert"}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={convertInput} placeholder="/path/to/input.mkv"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" onclick={() => pickInput((v) => convertInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={convertOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" onclick={() => pickOutput((v) => convertOutput = v)} class="browse-btn">
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
              <button type="button" onclick={() => pickInput((v) => trimInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={trimOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" onclick={() => pickOutput((v) => trimOutput = v)} class="browse-btn">
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

        {:else}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={compressInput} placeholder="/path/to/input.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" onclick={() => pickInput((v) => compressInput = v)} class="browse-btn">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
              </button>
            </div>
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <div class="flex">
              <input type="text" spellcheck="false" bind:value={compressOutput} placeholder="/path/to/output.mp4"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
              <button type="button" onclick={() => pickOutput((v) => compressOutput = v)} class="browse-btn">
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

      <!-- Run Button -->
      <div class="p-4 pt-0 flex-shrink-0">
        <button
          onclick={handleRun}
          disabled={isRunning}
          class="bg-primary text-primary-foreground font-mono text-[10px] tracking-[0.25em] uppercase font-semibold py-3 w-full border-0 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
        >
          {#if isRunning}
            <span class="spinner"></span>Processing
          {:else}
            <span class="text-[9px]">▶</span>Run
          {/if}
        </button>
      </div>
    </aside>

    <!-- Log Panel -->
    <section class="flex flex-col overflow-hidden">
      <div class="h-10 flex items-center justify-between px-5 border-b border-border flex-shrink-0">
        <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output Log</span>
        <span class="text-[9px] text-muted-foreground tabular-nums">{logs.length} lines</span>
      </div>

      <!-- Progress section -->
      {#if currentProgress}
        <div class="border-b border-border px-5 py-4 flex flex-col gap-3 flex-shrink-0">
          <!-- Bar -->
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
          <!-- Stats -->
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

      <!-- Log body -->
      <div class="flex-1 overflow-y-auto py-3" bind:this={logPanel}>
        {#if logs.length === 0 && !currentProgress}
          <p class="px-5 py-8 text-[11px] text-muted-foreground text-center tracking-widest">— awaiting process —</p>
        {:else}
          {#each logs as line, i}
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
