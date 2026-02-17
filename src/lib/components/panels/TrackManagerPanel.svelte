<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";
  import type { StreamInfo } from "$lib/ffmpeg";

  interface Props {
    input?: string;
    output?: string;
    addAudioInput?: string;
    addSubtitleInput?: string;
    audioStreams?: StreamInfo[];
    subtitleStreams?: StreamInfo[];
    keepAudioIndices?: number[];
    keepSubtitleIndices?: number[];
    onpickinput: () => void;
    onpickoutput: () => void;
    onpickaddaudio: () => void;
    onpickaddsubtitle: () => void;
  }
  let {
    input = $bindable(""),
    output = $bindable(""),
    addAudioInput = $bindable(""),
    addSubtitleInput = $bindable(""),
    audioStreams = [],
    subtitleStreams = [],
    keepAudioIndices = $bindable<number[]>([]),
    keepSubtitleIndices = $bindable<number[]>([]),
    onpickinput,
    onpickoutput,
    onpickaddaudio,
    onpickaddsubtitle
  }: Props = $props();

  function toggleIndex(list: number[], idx: number): number[] {
    return list.includes(idx) ? list.filter((v) => v !== idx) : [...list, idx];
  }
</script>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Input</span>
  <BrowseInput bind:value={input} placeholder="/path/to/input.mkv" onbrowse={onpickinput} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mkv" onbrowse={onpickoutput} />
</label>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Keep Audio Tracks</span>
  {#if audioStreams.length === 0}
    <p class="text-[9px] text-muted-foreground">No audio streams detected.</p>
  {:else}
    <div class="border border-border">
      {#each audioStreams as s, i}
        <label class="flex items-center gap-2 px-2 py-1.5 border-b border-border last:border-b-0 text-[10px]">
          <input
            type="checkbox"
            checked={keepAudioIndices.includes(s.index)}
            onchange={() => keepAudioIndices = toggleIndex(keepAudioIndices, s.index)}
            class="accent-current w-3 h-3"
          />
          <span class="font-mono text-foreground">#{s.index}</span>
          <span class="text-muted-foreground">{s.codec_name}{s.language ? ` · ${s.language}` : ""}</span>
        </label>
      {/each}
    </div>
  {/if}
</div>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Keep Subtitle Tracks</span>
  {#if subtitleStreams.length === 0}
    <p class="text-[9px] text-muted-foreground">No subtitle streams detected.</p>
  {:else}
    <div class="border border-border">
      {#each subtitleStreams as s, i}
        <label class="flex items-center gap-2 px-2 py-1.5 border-b border-border last:border-b-0 text-[10px]">
          <input
            type="checkbox"
            checked={keepSubtitleIndices.includes(s.index)}
            onchange={() => keepSubtitleIndices = toggleIndex(keepSubtitleIndices, s.index)}
            class="accent-current w-3 h-3"
          />
          <span class="font-mono text-foreground">#{s.index}</span>
          <span class="text-muted-foreground">{s.codec_name}{s.language ? ` · ${s.language}` : ""}</span>
        </label>
      {/each}
    </div>
  {/if}
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Add Audio Track (Optional)</span>
  <BrowseInput bind:value={addAudioInput} placeholder="/path/to/new-audio.aac" onbrowse={onpickaddaudio} />
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Add Subtitle Track (Optional)</span>
  <BrowseInput bind:value={addSubtitleInput} placeholder="/path/to/new-subtitle.srt" onbrowse={onpickaddsubtitle} />
</label>

<p class="text-[9px] text-muted-foreground">Uncheck tracks to remove. Checked tracks are kept and optional new tracks are appended.</p>
