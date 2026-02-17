<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    output?: string;
    crf?: number;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let { input = $bindable(""), output = $bindable(""), crf = $bindable(23), onpickinput, onpickoutput }: Props = $props();

  const crfLabel = $derived(
    crf <= 17 ? "Lossless" :
    crf <= 23 ? "High Quality" :
    crf <= 28 ? "Good" :
    crf <= 35 ? "Compressed" : "Low Quality"
  );
  const crfFill = $derived(`${((crf / 51) * 100).toFixed(1)}%`);
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

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
