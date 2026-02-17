<script lang="ts">
  import { fmtDuration } from "$lib/format";
  import type { FfmpegProgress } from "$lib/ffmpeg";

  interface Props {
    currentProgress: FfmpegProgress;
    progressPct: number | null;
    etaSecs: number | null;
    encodeDuration: number;
  }
  let { currentProgress, progressPct, etaSecs, encodeDuration }: Props = $props();
</script>

<div class="border-b border-border px-5 py-4 flex flex-col gap-3 flex-shrink-0">
  <div class="flex items-center gap-3">
    <div class="flex-1 h-px bg-border relative overflow-visible">
      {#if progressPct !== null}
        <div
          class="absolute inset-y-0 left-0 bg-foreground transition-[width] duration-300"
          style="width: {progressPct.toFixed(1)}%; height: 1px;"
        ></div>
      {:else}
        <div class="progress-indeterminate" style="height: 1px; background: var(--foreground);"></div>
      {/if}
    </div>
    {#if progressPct !== null}
      <span class="text-[9px] text-foreground tabular-nums w-9 text-right shrink-0">{progressPct.toFixed(0)}%</span>
    {/if}
  </div>
  <div class="flex flex-wrap gap-x-5 gap-y-1 text-[9px]">
    {#if currentProgress.time}
      <span class="text-foreground tabular-nums">
        {currentProgress.time}{encodeDuration > 0 ? ` / ${fmtDuration(encodeDuration)}` : ""}
      </span>
    {/if}
    {#if currentProgress.speed != null}
      <span class="text-muted-foreground">{currentProgress.speed.toFixed(2)}x</span>
    {/if}
    {#if etaSecs != null}
      <span class="text-muted-foreground">ETA {fmtDuration(etaSecs)}</span>
    {/if}
    {#if currentProgress.fps != null && currentProgress.fps > 0}
      <span class="text-muted-foreground">{Math.round(currentProgress.fps)} fps</span>
    {/if}
    {#if currentProgress.bitrate}
      <span class="text-muted-foreground">{currentProgress.bitrate}</span>
    {/if}
    {#if currentProgress.frame != null}
      <span class="text-muted-foreground">frame {currentProgress.frame}</span>
    {/if}
  </div>
</div>

<style>
  .progress-indeterminate {
    position: absolute;
    top: 0;
    width: 30%;
    animation: indeterminate 1.4s ease-in-out infinite;
  }
  @keyframes indeterminate {
    0%   { left: -30%; }
    100% { left: 100%; }
  }
</style>
