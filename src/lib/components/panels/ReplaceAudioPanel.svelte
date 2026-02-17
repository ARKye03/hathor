<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    audioInput?: string;
    output?: string;
    warning?: string | null;
    onpickinput: () => void;
    onpickaudio: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    audioInput = $bindable(""),
    output = $bindable(""),
    warning = null,
    onpickinput,
    onpickaudio,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Video Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Audio Track</span>
  <BrowseInput bind:value={audioInput} placeholder="/path/to/track.wav" onbrowse={onpickaudio} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Replaces source audio with the selected track and keeps video stream copy when possible.</p>

{#if warning}
  <p class="text-[9px] text-destructive">{warning}</p>
{/if}
