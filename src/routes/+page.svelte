<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { runFfmpeg, onLog, onDone, type FfmpegOperation } from "$lib/ffmpeg";
  import type { UnlistenFn } from "@tauri-apps/api/event";

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

  let unlistenLog: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  onMount(async () => {
    unlistenLog = await onLog(async (line) => {
      logs = [...logs, line];
      await tick();
      if (logPanel) logPanel.scrollTop = logPanel.scrollHeight;
    });
    unlistenDone = await onDone((code) => {
      status = code === 0 ? "done" : "error";
    });
  });

  onDestroy(() => {
    unlistenLog?.();
    unlistenDone?.();
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
    try {
      await runFfmpeg(buildOperation());
    } catch (e) {
      logs = [...logs, `[error] ${e}`];
      status = "error";
    }
  }

  const isRunning = $derived(status === "running");
  const crfLabel = $derived(
    compressCrf <= 17 ? "Lossless" :
    compressCrf <= 23 ? "High Quality" :
    compressCrf <= 28 ? "Good" :
    compressCrf <= 35 ? "Compressed" : "Low Quality"
  );
  const crfFill = $derived(`${((compressCrf / 51) * 100).toFixed(1)}%`);
</script>

<div class="h-screen flex flex-col overflow-hidden bg-background text-foreground font-mono">

  <!-- Header -->
  <header class="h-12 flex items-center justify-between px-6 border-b border-border flex-shrink-0">
    <span class="text-[11px] font-bold tracking-[0.35em] uppercase select-none">HATHOR</span>

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
  </header>

  <!-- Workspace -->
  <div class="grid grid-cols-[340px_1fr] flex-1 overflow-hidden">

    <!-- Config Panel -->
    <aside class="flex flex-col border-r border-border overflow-hidden bg-card">

      <!-- Tabs -->
      <div class="flex border-b border-border flex-shrink-0">
        {#each (["convert", "trim", "compress"] as Tab[]) as tab, i}
          <button
            onclick={() => activeTab = tab}
            class="flex-1 py-3 text-[9px] font-semibold tracking-[0.18em] uppercase cursor-pointer bg-transparent border-0 border-r border-border transition-colors"
            class:text-foreground={activeTab === tab}
            class:tab-active={activeTab === tab}
            class:text-muted-foreground={activeTab !== tab}
            class:hover:text-foreground={activeTab !== tab}
            class:border-r-0={i === 2}
          >
            {tab}
          </button>
        {/each}
      </div>

      <!-- Fields -->
      <div class="flex-1 px-5 py-6 flex flex-col gap-5 overflow-y-auto">

        {#if activeTab === "convert"}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <input
              type="text" spellcheck="false"
              bind:value={convertInput}
              placeholder="/path/to/input.mkv"
              class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
            />
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <input
              type="text" spellcheck="false"
              bind:value={convertOutput}
              placeholder="/path/to/output.mp4"
              class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
            />
          </label>

        {:else if activeTab === "trim"}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <input
              type="text" spellcheck="false"
              bind:value={trimInput}
              placeholder="/path/to/input.mp4"
              class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
            />
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <input
              type="text" spellcheck="false"
              bind:value={trimOutput}
              placeholder="/path/to/output.mp4"
              class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
            />
          </label>
          <div class="grid grid-cols-2 gap-3">
            <label class="flex flex-col gap-2">
              <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Start</span>
              <input
                type="text" spellcheck="false"
                bind:value={trimStart}
                placeholder="00:00:10"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
              />
            </label>
            <label class="flex flex-col gap-2">
              <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Duration</span>
              <input
                type="text" spellcheck="false"
                bind:value={trimDuration}
                placeholder="00:00:30"
                class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
              />
            </label>
          </div>

        {:else}
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
            <input
              type="text" spellcheck="false"
              bind:value={compressInput}
              placeholder="/path/to/input.mp4"
              class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
            />
          </label>
          <label class="flex flex-col gap-2">
            <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
            <input
              type="text" spellcheck="false"
              bind:value={compressOutput}
              placeholder="/path/to/output.mp4"
              class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
            />
          </label>
          <div class="flex flex-col gap-3">
            <div class="flex items-baseline justify-between">
              <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">CRF</span>
              <span class="text-[10px] text-foreground">{compressCrf} — {crfLabel}</span>
            </div>
            <input
              type="range" min="0" max="51"
              bind:value={compressCrf}
              class="slider w-full"
              style="--fill: {crfFill}"
            />
            <div class="flex justify-between text-[9px] text-muted-foreground">
              <span>0 · Lossless</span>
              <span>51 · Worst</span>
            </div>
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
      <div class="flex-1 overflow-y-auto py-3" bind:this={logPanel}>
        {#if logs.length === 0}
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
  .dot-pulse {
    animation: pulse 1s ease-in-out infinite;
  }

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

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Scrollbars */
  ::-webkit-scrollbar { width: 3px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border); }
  ::-webkit-scrollbar-thumb:hover { background: var(--foreground); }
</style>
