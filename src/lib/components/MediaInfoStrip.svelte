<script lang="ts">
  import { fmtDuration, fmtBytes, fmtFps, fmtChannels } from "$lib/format";
  import type { MediaInfo } from "$lib/ffmpeg";

  interface Props {
    probing: boolean;
    mediaInfo: MediaInfo | null;
  }
  let { probing, mediaInfo }: Props = $props();
</script>

{#if probing}
  <div class="border-t border-border pt-4 mt-1">
    <p class="text-[9px] text-muted-foreground tracking-widest uppercase animate-pulse">Probing…</p>
  </div>
{:else if mediaInfo}
  {@const video = mediaInfo.streams.find(s => s.codec_type === "video")}
  {@const audios = mediaInfo.streams.filter(s => s.codec_type === "audio")}
  <div class="border-t border-border pt-4 mt-1 flex flex-col gap-1.5">
    <div class="flex justify-between text-[9px]">
      <span class="text-muted-foreground tracking-wider uppercase">Duration</span>
      <span class="text-foreground tabular-nums">{fmtDuration(mediaInfo.duration_secs)}</span>
    </div>
    {#if mediaInfo.size_bytes}
      <div class="flex justify-between text-[9px]">
        <span class="text-muted-foreground tracking-wider uppercase">Size</span>
        <span class="text-foreground">{fmtBytes(mediaInfo.size_bytes)}</span>
      </div>
    {/if}
    {#if video}
      <div class="flex justify-between text-[9px]">
        <span class="text-muted-foreground tracking-wider uppercase">Video</span>
        <span class="text-foreground">
          {video.codec_name.toUpperCase()}
          {#if video.width && video.height} · {video.width}×{video.height}{/if}
          {#if fmtFps(video.frame_rate)} · {fmtFps(video.frame_rate)} fps{/if}
        </span>
      </div>
    {/if}
    {#each audios as a}
      <div class="flex justify-between text-[9px]">
        <span class="text-muted-foreground tracking-wider uppercase">Audio{a.language ? ` (${a.language})` : ""}</span>
        <span class="text-foreground">{a.codec_name.toUpperCase()} · {fmtChannels(a.channels)}</span>
      </div>
    {/each}
  </div>
{/if}
