<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    output?: string;
    start?: string;
    duration?: string;
    fps?: number;
    width?: number;
    usePalette?: boolean;
    onpickinput: () => void;
    onpickoutput: () => void;
  }

  let {
    input = $bindable(""),
    output = $bindable(""),
    start = $bindable("00:00:00"),
    duration = $bindable("00:00:06"),
    fps = $bindable(15),
    width = $bindable(480),
    usePalette = $bindable(true),
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
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Start</span>
    <input type="text" spellcheck="false" bind:value={start} placeholder="00:00:00"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  </label>
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Duration</span>
    <input type="text" spellcheck="false" bind:value={duration} placeholder="00:00:06"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  </label>
</div>

<div class="grid grid-cols-2 gap-3">
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">FPS</span>
    <input type="number" min="1" max="30" step="1" bind:value={fps}
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors focus:border-foreground" />
  </label>
  <label class="flex flex-col gap-2">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Width</span>
    <input type="number" min="80" step="2" bind:value={width}
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors focus:border-foreground" />
  </label>
</div>

<label class="flex items-center justify-between border border-border bg-input px-3 py-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Palette Optimization</span>
  <input type="checkbox" class="toggle toggle-sm" bind:checked={usePalette} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.gif" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Trim, scale, and convert to GIF with optional palette optimization.</p>
