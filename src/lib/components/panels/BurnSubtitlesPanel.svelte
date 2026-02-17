<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";

  interface Props {
    input?: string;
    subtitleInput?: string;
    output?: string;
    onpickinput: () => void;
    onpicksubtitle: () => void;
    onpickoutput: () => void;
  }
  let {
    input = $bindable(""),
    subtitleInput = $bindable(""),
    output = $bindable(""),
    onpickinput,
    onpicksubtitle,
    onpickoutput
  }: Props = $props();
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mp4" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Subtitle (SRT/ASS)</span>
  <BrowseInput bind:value={subtitleInput} placeholder="/path/to/subtitles.srt" onbrowse={onpicksubtitle} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Burns subtitle text into video frames (hard subtitles).</p>
