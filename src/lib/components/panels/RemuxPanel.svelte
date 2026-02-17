<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    output?: string;
    onpickinput: () => void;
    onpickoutput: () => void;
  }
  let { input = $bindable(""), output = $bindable(""), onpickinput, onpickoutput }: Props = $props();
</script>

<div class="flex flex-col gap-1.5 pb-1">
  <p class="text-[9px] text-muted-foreground tracking-wider">Copies all streams without re-encoding. Instant, lossless.</p>
  <div class="flex items-center gap-1.5 text-[9px] font-semibold text-foreground tracking-wider uppercase">
    <span class="inline-block w-1.5 h-1.5 bg-foreground"></span>Fast · No quality loss
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mkv" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>
