<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  type SequenceFormat = "png" | "jpg" | "webp";

  interface Props {
    input?: string;
    outputPattern?: string;
    start?: string;
    duration?: string;
    fps?: number;
    scaleWidth?: number;
    format?: SequenceFormat;
    onpickinput: () => void;
    onpickoutput: () => void;
  }

  let {
    input = $bindable(""),
    outputPattern = $bindable(""),
    start = $bindable(""),
    duration = $bindable(""),
    fps = $bindable(1),
    scaleWidth = $bindable(0),
    format = $bindable<SequenceFormat>("png"),
    onpickinput,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<div class="grid grid-cols-2 gap-3">
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Start (Optional)</span>
    <input type="text" spellcheck="false" bind:value={start} placeholder="00:00:00"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  </label>
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Duration (Optional)</span>
    <input type="text" spellcheck="false" bind:value={duration} placeholder="00:00:10"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  </label>
</div>

<div class="grid grid-cols-2 gap-3">
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">FPS</span>
    <input type="number" min="1" max="60" step="1" bind:value={fps}
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors focus:border-foreground" />
  </label>
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Scale Width (0 = keep)</span>
    <input type="number" min="0" step="2" bind:value={scaleWidth}
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors focus:border-foreground" />
  </label>
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Format</span>
  <div class="flex border border-border">
    {#each (["png", "jpg", "webp"] as SequenceFormat[]) as f, i}
      <button type="button" onclick={() => format = f}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={format === f}
        class:text-primary-foreground={format === f}
        class:bg-transparent={format !== f}
        class:text-muted-foreground={format !== f}
        class:border-r-0={i === 2}
      >{f}</button>
    {/each}
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output Pattern</span>
  <BrowseInput bind:value={outputPattern} placeholder="/path/to/frame_%05d.png" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Exports frames using FFmpeg sequence pattern (for example `%05d`).</p>
