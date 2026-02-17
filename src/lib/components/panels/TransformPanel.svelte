<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";
  import type { Rotate, Flip } from "$lib/types";

  interface Props {
    input?: string;
    output?: string;
    cropEnabled?: boolean;
    cropX?: number;
    cropY?: number;
    cropWidth?: number;
    cropHeight?: number;
    padEnabled?: boolean;
    padWidth?: number;
    padHeight?: number;
    padColor?: string;
    rotate?: Rotate;
    flip?: Flip;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    cropEnabled = $bindable(false),
    cropX = $bindable(0),
    cropY = $bindable(0),
    cropWidth = $bindable(640),
    cropHeight = $bindable(360),
    padEnabled = $bindable(false),
    padWidth = $bindable(1280),
    padHeight = $bindable(720),
    padColor = $bindable("#000000"),
    rotate = $bindable<Rotate>("keep"),
    flip = $bindable<Flip>("none"),
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
  <label class="flex items-center gap-2 text-[9px] font-semibold tracking-[0.18em] uppercase text-muted-foreground">
    <input type="checkbox" bind:checked={cropEnabled} class="accent-current w-3 h-3" />
    Crop
  </label>
  {#if cropEnabled}
    <div class="grid grid-cols-2 gap-2">
      <input type="number" min="0" bind:value={cropX} placeholder="x"
        class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
      <input type="number" min="0" bind:value={cropY} placeholder="y"
        class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
      <input type="number" min="1" bind:value={cropWidth} placeholder="width"
        class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
      <input type="number" min="1" bind:value={cropHeight} placeholder="height"
        class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
    </div>
  {/if}
</div>

<div class="flex flex-col gap-2">
  <label class="flex items-center gap-2 text-[9px] font-semibold tracking-[0.18em] uppercase text-muted-foreground">
    <input type="checkbox" bind:checked={padEnabled} class="accent-current w-3 h-3" />
    Pad
  </label>
  {#if padEnabled}
    <div class="grid grid-cols-2 gap-2">
      <input type="number" min="1" bind:value={padWidth} placeholder="width"
        class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
      <input type="number" min="1" bind:value={padHeight} placeholder="height"
        class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
    </div>
    <input type="text" spellcheck="false" bind:value={padColor} placeholder="#000000"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-2 py-1.5 outline-none focus:border-foreground" />
    <p class="text-[9px] text-muted-foreground">Pads to target size and centers source (letterbox/pillarbox).</p>
  {/if}
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Rotate</span>
  <div class="flex border border-border">
    {#each (["keep", "90", "180", "270"] as Rotate[]) as r, i}
      <button type="button" onclick={() => rotate = r}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={rotate === r}
        class:text-primary-foreground={rotate === r}
        class:bg-transparent={rotate !== r}
        class:text-muted-foreground={rotate !== r}
        class:border-r-0={i === 3}
      >{r}</button>
    {/each}
  </div>
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Flip</span>
  <div class="flex border border-border">
    {#each (["none", "horizontal", "vertical", "both"] as Flip[]) as f, i}
      <button type="button" onclick={() => flip = f}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={flip === f}
        class:text-primary-foreground={flip === f}
        class:bg-transparent={flip !== f}
        class:text-muted-foreground={flip !== f}
        class:border-r-0={i === 3}
      >{f}</button>
    {/each}
  </div>
</div>
