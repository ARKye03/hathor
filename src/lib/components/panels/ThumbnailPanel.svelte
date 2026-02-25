<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    output?: string;
    time?: string;
    onpickinput: () => void;
    onpickoutput: () => void;
  }

  let {
    input = $bindable(""),
    output = $bindable(""),
    time = $bindable("00:00:01"),
    onpickinput,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Capture Time</span>
  <input type="text" spellcheck="false" bind:value={time} placeholder="00:00:03"
    class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 w-full outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/thumbnail.png" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Extracts a single frame from the selected timestamp.</p>
