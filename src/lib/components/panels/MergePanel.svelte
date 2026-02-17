<script lang="ts">
  import BrowseInput from "$lib/components/BrowseInput.svelte";
  import type { MediaInfo } from "$lib/ffmpeg";

  interface Props {
    inputs?: string[];
    output?: string;
    infos?: (MediaInfo | null)[];
    onaddfiles: () => void;
    onpickoutput: () => void;
    onremoveinput: (idx: number) => void;
    onmoveinput: (idx: number, dir: -1 | 1) => void;
    onclearinputs: () => void;
  }
  let {
    inputs = $bindable<string[]>([]),
    output = $bindable(""),
    infos = $bindable<(MediaInfo | null)[]>([]),
    onaddfiles,
    onpickoutput,
    onremoveinput,
    onmoveinput,
    onclearinputs,
  }: Props = $props();

  const mergeMismatchWarning = $derived((() => {
    if (inputs.length < 2 || infos.length !== inputs.length) return null;
    const first = infos[0];
    if (!first) return "Could not read metadata for the first file.";
    const v0 = first.streams.find((s) => s.codec_type === "video");
    const a0 = first.streams.find((s) => s.codec_type === "audio");
    for (let i = 1; i < infos.length; i += 1) {
      const cur = infos[i];
      if (!cur) return "Could not read metadata for one or more files.";
      const v = cur.streams.find((s) => s.codec_type === "video");
      const a = cur.streams.find((s) => s.codec_type === "audio");
      if (!!v0 !== !!v || !!a0 !== !!a) return "Stream layout mismatch detected (video/audio presence differs).";
      if (v0 && v && (v.codec_name !== v0.codec_name || v.width !== v0.width || v.height !== v0.height)) {
        return "Video codec or resolution mismatch detected; concat copy may fail.";
      }
      if (a0 && a && (a.codec_name !== a0.codec_name || a.channels !== a0.channels || a.sample_rate !== a0.sample_rate)) {
        return "Audio stream mismatch detected; concat copy may fail.";
      }
    }
    return null;
  })());
</script>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Inputs</span>
  <div class="flex gap-2">
    <button type="button" onclick={onaddfiles}
      class="bg-input border border-border text-foreground text-[10px] tracking-[0.14em] uppercase font-semibold px-3 py-2 cursor-pointer hover:bg-muted transition-colors"
    >+ Add Files</button>
    <button type="button" onclick={onclearinputs}
      class="bg-transparent border border-border text-muted-foreground text-[10px] tracking-[0.14em] uppercase font-semibold px-3 py-2 cursor-pointer hover:text-foreground transition-colors"
    >Clear</button>
  </div>
  <div class="border border-border max-h-36 overflow-y-auto">
    {#if inputs.length === 0}
      <p class="px-3 py-3 text-[10px] text-muted-foreground">Select 2+ files in playback order.</p>
    {:else}
      {#each inputs as p, i}
        <div class="flex items-center gap-2 px-2 py-1.5 border-b border-border last:border-b-0">
          <span class="w-5 text-[9px] text-muted-foreground text-right tabular-nums">{String(i + 1).padStart(2, "0")}</span>
          <span class="flex-1 min-w-0 truncate text-[10px] font-mono text-foreground">{p.split("/").pop() ?? p}</span>
          <button type="button" onclick={() => onmoveinput(i, -1)} class="text-[9px] text-muted-foreground hover:text-foreground">↑</button>
          <button type="button" onclick={() => onmoveinput(i, 1)} class="text-[9px] text-muted-foreground hover:text-foreground">↓</button>
          <button type="button" onclick={() => onremoveinput(i)} class="text-[9px] text-muted-foreground hover:text-destructive">×</button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output</span>
  <BrowseInput bind:value={output} placeholder="/path/to/output.mp4" onbrowse={onpickoutput} />
</label>

<p class="text-[9px] text-muted-foreground">Uses concat demuxer with stream copy for matching files.</p>

{#if mergeMismatchWarning}
  <p class="text-[9px] text-destructive">{mergeMismatchWarning}</p>
{/if}
