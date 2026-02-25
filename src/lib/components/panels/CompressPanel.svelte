<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";
  type CompressPreset = "iphone_ipad" | "android" | "youtube" | "tiktok" | "instagram";

  interface Props {
    input?: string;
    output?: string;
    crf?: number;
    preset?: CompressPreset;
    sizeTargetEnabled?: boolean;
    sizeTargetMb?: number;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    crf = $bindable(23),
    preset = $bindable<CompressPreset>("youtube"),
    sizeTargetEnabled = $bindable(false),
    sizeTargetMb = $bindable(25),
    onpickinput,
    onpickoutput
  }: Props = $props();

  const crfLabel = $derived(
    crf <= 17 ? "Lossless" :
    crf <= 23 ? "High Quality" :
    crf <= 28 ? "Good" :
    crf <= 35 ? "Compressed" : "Low Quality"
  );
  const crfFill = $derived(`${((crf / 51) * 100).toFixed(1)}%`);
  const PRESETS: [CompressPreset, string, string][] = [
    ["iphone_ipad", "iPhone/iPad", "H.264 + AAC"],
    ["android", "Android", "H.264 + AAC"],
    ["youtube", "YouTube", "Upload-safe"],
    ["tiktok", "TikTok", "Social feed"],
    ["instagram", "Instagram", "Reels/Feed"]
  ];
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Delivery Preset</span>
  <div class="grid grid-cols-2 gap-2">
    {#each PRESETS as [id, label, hint]}
      <button
        type="button"
        onclick={() => preset = id}
        class="flex flex-col items-start gap-1 border px-2 py-2 text-left transition-colors"
        class:bg-primary={preset === id}
        class:text-primary-foreground={preset === id}
        class:border-primary={preset === id}
        class:bg-input={preset !== id}
        class:text-foreground={preset !== id}
        class:border-border={preset !== id}
      >
        <span class="text-[10px] font-semibold">{label}</span>
        <span class="text-[9px] opacity-80">{hint}</span>
      </button>
    {/each}
  </div>
</div>

<div class="flex flex-col gap-3">
  <div class="flex items-baseline justify-between">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">CRF</span>
    <span class="text-[10px] text-foreground">{crf} — {crfLabel}</span>
  </div>
  <input type="range" min="0" max="51" bind:value={crf} class="slider w-full" style="--fill: {crfFill}" />
  <div class="flex justify-between text-[9px] text-muted-foreground">
    <span>0 · Lossless</span>
    <span>51 · Worst</span>
  </div>
</div>

<div class="flex flex-col gap-2 border border-border bg-input px-3 py-3">
  <label class="flex items-center justify-between gap-3">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Fit To Size</span>
    <input type="checkbox" class="toggle toggle-sm" bind:checked={sizeTargetEnabled} />
  </label>
  {#if sizeTargetEnabled}
    <label class="flex items-center justify-between gap-2">
      <span class="text-[10px] text-muted-foreground">Target MB</span>
      <input
        type="number"
        min="1"
        step="1"
        bind:value={sizeTargetMb}
        class="w-24 bg-background border border-border px-2 py-1 text-[10px] text-right text-foreground outline-none focus:border-foreground"
      />
    </label>
    <p class="text-[9px] text-muted-foreground">Uses two-pass bitrate estimation from source duration.</p>
  {/if}
</div>
