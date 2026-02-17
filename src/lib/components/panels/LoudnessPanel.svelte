<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  type LoudnessPreset = "broadcast" | "streaming" | "podcast";

  interface Props {
    input?: string;
    output?: string;
    preset?: LoudnessPreset;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    preset = $bindable<LoudnessPreset>("broadcast"),
    onpickinput,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">EBU R128 Preset</span>
  <div class="flex border border-border">
    {#each (["broadcast", "streaming", "podcast"] as LoudnessPreset[]) as p, i}
      <button type="button" onclick={() => preset = p}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={preset === p}
        class:text-primary-foreground={preset === p}
        class:bg-transparent={preset !== p}
        class:text-muted-foreground={preset !== p}
        class:border-r-0={i === 2}
      >{p}</button>
    {/each}
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Applies loudness normalization using FFmpeg loudnorm filter presets.</p>
