<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";
  import type { TrimMode } from "$lib/types";

  interface Props {
    input?: string;
    output?: string;
    trimMode?: TrimMode;
    trimStart?: string;
    trimDuration?: string;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    trimMode = $bindable<TrimMode>("accurate"),
    trimStart = $bindable("00:00:00"),
    trimDuration = $bindable("00:00:30"),
    onpickinput,
    onpickoutput,
  }: Props = $props();
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
