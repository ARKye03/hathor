<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";
  import type { Container, QualityMode, Resolution, Fps } from "$lib/types";

  interface Props {
    input?: string;
    output?: string;
    container?: Container;
    qualityMode?: QualityMode;
    crf?: number;
    bitrate?: string;
    resolution?: Resolution;
    fps?: Fps;
    compatibilityWarning?: string | null;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    container = $bindable<Container>("mp4"),
    qualityMode = $bindable<QualityMode>("crf"),
    crf = $bindable(23),
    bitrate = $bindable("2000k"),
    resolution = $bindable<Resolution>("keep"),
    fps = $bindable<Fps>("keep"),
    compatibilityWarning = null,
    onpickinput,
    onpickoutput,
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mkv" onbrowse={onpickinput} />
</label>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Container</span>
  <div class="flex border border-border">
    {#each (["mp4", "mkv", "mov", "webm", "gif"] as Container[]) as c, i}
      <button type="button" onclick={() => container = c}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={container === c}
        class:text-primary-foreground={container === c}
        class:bg-transparent={container !== c}
        class:text-muted-foreground={container !== c}
        class:border-r-0={i === 4}
      >{c}</button>
    {/each}
  </div>
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Quality</span>
  <div class="flex border border-border">
    {#each ([["crf", "CRF"], ["bitrate", "Bitrate"]] as [QualityMode, string][]) as [mode, label]}
      <button type="button" onclick={() => qualityMode = mode}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase font-mono border-0 border-r border-border last:border-r-0 cursor-pointer transition-colors"
        class:bg-primary={qualityMode === mode}
        class:text-primary-foreground={qualityMode === mode}
        class:bg-transparent={qualityMode !== mode}
        class:text-muted-foreground={qualityMode !== mode}
      >{label}</button>
    {/each}
  </div>
  {#if qualityMode === "crf"}
    <div class="flex items-baseline justify-between text-[9px] mt-1">
      <span class="text-muted-foreground">CRF</span>
      <span class="text-foreground tabular-nums">{crf}</span>
    </div>
    <input type="range" min="0" max="51" bind:value={crf} class="slider w-full"
      style="--fill: {((crf / 51) * 100).toFixed(1)}%" />
    <div class="flex justify-between text-[9px] text-muted-foreground">
      <span>0 · Lossless</span><span>51 · Worst</span>
    </div>
  {:else}
    <input type="text" spellcheck="false" bind:value={bitrate} placeholder="2000k"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  {/if}
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Resolution</span>
  <div class="flex border border-border">
    {#each (["keep", "1080p", "720p", "480p"] as Resolution[]) as r, i}
      <button type="button" onclick={() => resolution = r}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={resolution === r}
        class:text-primary-foreground={resolution === r}
        class:bg-transparent={resolution !== r}
        class:text-muted-foreground={resolution !== r}
        class:border-r-0={i === 3}
      >{r}</button>
    {/each}
  </div>
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Frame Rate</span>
  <div class="flex border border-border">
    {#each (["keep", "24", "30", "60"] as Fps[]) as f, i}
      <button type="button" onclick={() => fps = f}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={fps === f}
        class:text-primary-foreground={fps === f}
        class:bg-transparent={fps !== f}
        class:text-muted-foreground={fps !== f}
        class:border-r-0={i === 3}
      >{f === "keep" ? f : `${f} fps`}</button>
    {/each}
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

{#if compatibilityWarning}
  <p class="text-[9px] text-destructive">{compatibilityWarning}</p>
{/if}
