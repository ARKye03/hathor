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

<main>
  <header>
    <div class="wordmark">
      <svg class="logo-mark" viewBox="0 0 20 20" width="18" height="18">
        <polygon points="10,1 18,5.5 18,14.5 10,19 2,14.5 2,5.5" fill="none" stroke="currentColor" stroke-width="1.5"/>
        <polygon points="10,5 15,7.75 15,13.25 10,16 5,13.25 5,7.75" fill="currentColor" opacity="0.3"/>
      </svg>
      HATHOR
    </div>
    <div class="status-chip {status}">
      <span class="dot"></span>
      {status.toUpperCase()}
    </div>
  </header>

  <div class="workspace">
    <aside class="config-panel">
      <div class="tabs">
        {#each (["convert", "trim", "compress"] as Tab[]) as tab}
          <button
            class="tab"
            class:active={activeTab === tab}
            onclick={() => activeTab = tab}
          >
            {tab}
          </button>
        {/each}
      </div>

      <div class="fields">
        {#if activeTab === "convert"}
          <label>
            <span class="field-label">INPUT</span>
            <input type="text" bind:value={convertInput} placeholder="/path/to/input.mkv" spellcheck="false" />
          </label>
          <label>
            <span class="field-label">OUTPUT</span>
            <input type="text" bind:value={convertOutput} placeholder="/path/to/output.mp4" spellcheck="false" />
          </label>

        {:else if activeTab === "trim"}
          <label>
            <span class="field-label">INPUT</span>
            <input type="text" bind:value={trimInput} placeholder="/path/to/input.mp4" spellcheck="false" />
          </label>
          <label>
            <span class="field-label">OUTPUT</span>
            <input type="text" bind:value={trimOutput} placeholder="/path/to/output.mp4" spellcheck="false" />
          </label>
          <div class="row-2">
            <label>
              <span class="field-label">START</span>
              <input type="text" bind:value={trimStart} placeholder="00:00:10" spellcheck="false" />
            </label>
            <label>
              <span class="field-label">DURATION</span>
              <input type="text" bind:value={trimDuration} placeholder="00:00:30" spellcheck="false" />
            </label>
          </div>

        {:else}
          <label>
            <span class="field-label">INPUT</span>
            <input type="text" bind:value={compressInput} placeholder="/path/to/input.mp4" spellcheck="false" />
          </label>
          <label>
            <span class="field-label">OUTPUT</span>
            <input type="text" bind:value={compressOutput} placeholder="/path/to/output.mp4" spellcheck="false" />
          </label>
          <div class="crf-group">
            <div class="crf-header">
              <span class="field-label">CRF</span>
              <span class="crf-readout">{compressCrf} — {crfLabel}</span>
            </div>
            <input
              type="range" min="0" max="51"
              bind:value={compressCrf}
              class="slider"
              style="--fill: {crfFill}"
            />
            <div class="crf-scale">
              <span>0 · Lossless</span>
              <span>51 · Worst</span>
            </div>
          </div>
        {/if}
      </div>

      <button class="run-btn" onclick={handleRun} disabled={isRunning}>
        {#if isRunning}
          <span class="spinner"></span>PROCESSING
        {:else}
          <span class="run-icon">▶</span>RUN
        {/if}
      </button>
    </aside>

    <section class="log-panel">
      <div class="log-header">
        <span>OUTPUT LOG</span>
        <span class="log-count">{logs.length} lines</span>
      </div>
      <div class="log-body" bind:this={logPanel}>
        {#if logs.length === 0}
          <span class="log-empty">— Awaiting process —</span>
        {:else}
          {#each logs as line, i}
            <div class="log-line">
              <span class="line-num">{String(i + 1).padStart(4, "0")}</span>
              <span class="line-text">{line}</span>
            </div>
          {/each}
        {/if}
      </div>
    </section>
  </div>
</main>

<style>
  @import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@300;400;500;600&display=swap');

  :root {
    --bg: #070910;
    --surface: #0c0e16;
    --surface-2: #10131e;
    --border: #1a2030;
    --border-hi: #252d42;
    --accent: #00d4b8;
    --accent-dim: rgba(0, 212, 184, 0.08);
    --accent-glow: rgba(0, 212, 184, 0.25);
    --text: #9db4cc;
    --text-hi: #d6e8f8;
    --muted: #334055;
    --label: #4a6080;
    --done: #22c55e;
    --error: #ef4444;
    --amber: #f0a040;
    --font: 'IBM Plex Mono', 'Courier New', monospace;
  }

  * { box-sizing: border-box; margin: 0; padding: 0; }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    background-image:
      linear-gradient(rgba(0, 212, 184, 0.018) 1px, transparent 1px),
      linear-gradient(90deg, rgba(0, 212, 184, 0.018) 1px, transparent 1px);
    background-size: 36px 36px;
    font-family: var(--font);
    color: var(--text);
    overflow: hidden;
  }

  /* ── Header ── */
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    height: 48px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .wordmark {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.3em;
    color: var(--text-hi);
  }

  .logo-mark { color: var(--accent); flex-shrink: 0; }

  .status-chip {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 10px;
    border: 1px solid currentColor;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.2em;
  }

  .status-chip.idle  { color: var(--muted); border-color: var(--border); }
  .status-chip.running { color: var(--amber); }
  .status-chip.done  { color: var(--done); }
  .status-chip.error { color: var(--error); }

  .dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: currentColor;
  }

  .status-chip.running .dot {
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.2; transform: scale(0.6); }
  }

  /* ── Layout ── */
  .workspace {
    display: grid;
    grid-template-columns: 340px 1fr;
    flex: 1;
    overflow: hidden;
  }

  /* ── Config panel ── */
  .config-panel {
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    background: var(--surface);
    overflow: hidden;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .tab {
    flex: 1;
    padding: 11px 0;
    background: none;
    border: none;
    border-right: 1px solid var(--border);
    color: var(--label);
    font-family: var(--font);
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    cursor: pointer;
    transition: color 0.12s, background 0.12s;
  }

  .tab:last-child { border-right: none; }
  .tab:hover { color: var(--text); background: rgba(255,255,255,0.015); }

  .tab.active {
    color: var(--accent);
    background: var(--accent-dim);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  /* ── Fields ── */
  .fields {
    flex: 1;
    padding: 22px 18px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    overflow-y: auto;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label {
    font-size: 8px;
    font-weight: 600;
    letter-spacing: 0.25em;
    color: var(--label);
  }

  input[type="text"] {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-hi);
    font-family: var(--font);
    font-size: 11px;
    padding: 9px 11px;
    outline: none;
    transition: border-color 0.12s, box-shadow 0.12s;
    width: 100%;
  }

  input[type="text"]:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-glow);
  }

  input[type="text"]::placeholder { color: var(--muted); }

  .row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  /* ── CRF ── */
  .crf-group { display: flex; flex-direction: column; gap: 10px; }

  .crf-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .crf-readout {
    font-size: 10px;
    color: var(--accent);
    letter-spacing: 0.03em;
  }

  .slider {
    -webkit-appearance: none;
    width: 100%;
    height: 2px;
    background: linear-gradient(
      to right,
      var(--accent) var(--fill, 45%),
      var(--border-hi) var(--fill, 45%)
    );
    outline: none;
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 12px;
    height: 12px;
    background: var(--accent);
    cursor: pointer;
    box-shadow: 0 0 10px var(--accent-glow);
  }

  .crf-scale {
    display: flex;
    justify-content: space-between;
    font-size: 8px;
    color: var(--muted);
    letter-spacing: 0.08em;
  }

  /* ── Run button ── */
  .run-btn {
    margin: 16px 18px 18px;
    padding: 13px;
    background: var(--accent);
    border: none;
    color: #000;
    font-family: var(--font);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.25em;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    transition: background 0.12s, box-shadow 0.12s;
    flex-shrink: 0;
  }

  .run-btn:hover:not(:disabled) {
    background: #00ecd4;
    box-shadow: 0 0 20px var(--accent-glow);
  }

  .run-btn:disabled {
    background: var(--surface-2);
    color: var(--muted);
    border: 1px solid var(--border);
    cursor: not-allowed;
    box-shadow: none;
  }

  .run-icon { font-size: 9px; }

  .spinner {
    width: 10px; height: 10px;
    border: 2px solid rgba(0,0,0,0.2);
    border-top-color: #000;
    border-radius: 50%;
    animation: spin 0.55s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Log panel ── */
  .log-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }

  .log-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 20px;
    height: 40px;
    border-bottom: 1px solid var(--border);
    font-size: 8px;
    font-weight: 600;
    letter-spacing: 0.2em;
    color: var(--label);
    flex-shrink: 0;
  }

  .log-count { color: var(--muted); }

  .log-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 0;
  }

  .log-empty {
    display: block;
    padding: 24px 20px;
    font-size: 11px;
    color: var(--muted);
    text-align: center;
    letter-spacing: 0.1em;
  }

  .log-line {
    display: flex;
    gap: 14px;
    padding: 1px 20px;
    font-size: 10px;
    line-height: 1.7;
  }

  .log-line:hover { background: rgba(255,255,255,0.015); }

  .line-num {
    color: var(--muted);
    user-select: none;
    flex-shrink: 0;
    opacity: 0.6;
  }

  .line-text {
    color: var(--text);
    word-break: break-all;
    white-space: pre-wrap;
  }

  /* Scrollbars */
  ::-webkit-scrollbar { width: 3px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--border-hi); }
  ::-webkit-scrollbar-thumb:hover { background: var(--muted); }
</style>
