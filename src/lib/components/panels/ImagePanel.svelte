<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  type ImageFormat = "png" | "jpg" | "webp" | "avif" | "ico";

  interface Props {
    input?: string;
    output?: string;
    format?: ImageFormat;
    quality?: number;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    format = $bindable<ImageFormat>("png"),
    quality = $bindable(82),
    onpickinput,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.png" onbrowse={onpickinput} />
</label>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Format</span>
  <div class="flex border border-border">
    {#each (["png", "jpg", "webp", "avif", "ico"] as ImageFormat[]) as f, i}
      <button type="button" onclick={() => format = f}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={format === f}
        class:text-primary-foreground={format === f}
        class:bg-transparent={format !== f}
        class:text-muted-foreground={format !== f}
        class:border-r-0={i === 4}
      >{f}</button>
    {/each}
  </div>
</div>

{#if format === "jpg" || format === "webp" || format === "avif"}
  <div class="flex flex-col gap-2">
    <div class="flex items-baseline justify-between text-[9px]">
      <span class="text-muted-foreground">Quality</span>
      <span class="text-foreground tabular-nums">{quality}</span>
    </div>
    <input type="range" min="1" max="100" bind:value={quality} class="slider w-full"
      style="--fill: {quality.toFixed(1)}%" />
  </div>
{/if}

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.webp" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Converts one image format to another. Supports PNG, JPG, WebP, AVIF, and ICO.</p>
