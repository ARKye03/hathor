<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  type ImageFormat = "png" | "jpg" | "webp" | "avif" | "ico";
  type ResizeMethod = "lanczos" | "bicubic" | "bilinear" | "neighbor";

  interface Props {
    input?: string;
    output?: string;
    format?: ImageFormat;
    quality?: number;
    resizeEnabled?: boolean;
    resizePercent?: number;
    resizeMethod?: ResizeMethod;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    format = $bindable<ImageFormat>("png"),
    quality = $bindable(82),
    resizeEnabled = $bindable(false),
    resizePercent = $bindable(100),
    resizeMethod = $bindable<ResizeMethod>("lanczos"),
    onpickinput,
    onpickoutput
  }: Props = $props();

  const percentPresets = [100, 75, 50, 25];
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

<div class="flex flex-col gap-2 border border-border bg-input px-3 py-3">
  <label class="flex items-center justify-between gap-3">
    <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Resize</span>
    <input type="checkbox" class="toggle toggle-sm" bind:checked={resizeEnabled} />
  </label>
  {#if resizeEnabled}
    <label class="flex flex-col gap-2">
      <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Method</span>
      <select bind:value={resizeMethod}
        class="bg-background border border-border text-foreground text-[11px] px-2 py-2 outline-none focus:border-foreground">
        <option value="lanczos">Lanczos3</option>
        <option value="bicubic">Bicubic</option>
        <option value="bilinear">Bilinear</option>
        <option value="neighbor">Nearest</option>
      </select>
    </label>
    <div class="flex flex-col gap-2">
      <div class="flex items-baseline justify-between text-[9px]">
        <span class="text-muted-foreground">Scale</span>
        <span class="text-foreground tabular-nums">{resizePercent}%</span>
      </div>
      <div class="grid grid-cols-4 gap-1">
        {#each percentPresets as preset}
          <button
            type="button"
            onclick={() => resizePercent = preset}
            class="py-1 text-[9px] border transition-colors"
            class:bg-primary={resizePercent === preset}
            class:text-primary-foreground={resizePercent === preset}
            class:border-primary={resizePercent === preset}
            class:bg-background={resizePercent !== preset}
            class:text-muted-foreground={resizePercent !== preset}
            class:border-border={resizePercent !== preset}
          >{preset}%</button>
        {/each}
      </div>
      <input type="range" min="1" max="400" bind:value={resizePercent} class="slider w-full"
        style="--fill: {Math.min(100, resizePercent).toFixed(1)}%" />
      <input type="number" min="1" max="400" bind:value={resizePercent}
        class="bg-background border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors focus:border-foreground" />
    </div>
    {#if format === "ico"}
      <p class="text-[9px] text-muted-foreground">ICO output is still normalized to 256x256.</p>
    {/if}
  {/if}
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.webp" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Converts one image format to another. Supports PNG, JPG, WebP, AVIF, and ICO.</p>
