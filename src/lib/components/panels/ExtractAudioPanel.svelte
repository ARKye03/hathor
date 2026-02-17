<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  type AudioFormat = "mp3" | "aac" | "opus" | "wav";

  interface Props {
    input?: string;
    output?: string;
    format?: AudioFormat;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    format = $bindable<AudioFormat>("mp3"),
    onpickinput,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Format</span>
  <div class="flex border border-border">
    {#each (["mp3", "aac", "opus", "wav"] as AudioFormat[]) as f, i}
      <button type="button" onclick={() => format = f}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.1em] uppercase font-mono border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={format === f}
        class:text-primary-foreground={format === f}
        class:bg-transparent={format !== f}
        class:text-muted-foreground={format !== f}
        class:border-r-0={i === 3}
      >{f}</button>
    {/each}
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp3" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Extracts audio track from input media and re-encodes to selected format.</p>
