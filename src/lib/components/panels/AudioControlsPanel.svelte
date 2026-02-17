<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    output?: string;
    volume?: number;
    fadeInSecs?: number;
    fadeOutSecs?: number;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    volume = $bindable(1),
    fadeInSecs = $bindable(0),
    fadeOutSecs = $bindable(0),
    onpickinput,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<div class="grid grid-cols-1 gap-3">
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Volume Multiplier</span>
    <input type="number" min="0" step="0.05" bind:value={volume}
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  </label>
  <div class="grid grid-cols-2 gap-3">
    <label class="flex flex-col gap-2">
      <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Fade In (s)</span>
      <input type="number" min="0" step="0.1" bind:value={fadeInSecs}
        class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
    </label>
    <label class="flex flex-col gap-2">
      <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Fade Out (s)</span>
      <input type="number" min="0" step="0.1" bind:value={fadeOutSecs}
        class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
    </label>
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Applies volume and optional fades while preserving video stream copy when possible.</p>
