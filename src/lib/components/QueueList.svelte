<script lang="ts">
  import type { QueueJob } from "$lib/types";

  interface Props {
    queue: QueueJob[];
    selectedJobId: string | null;
    queueRunning: boolean;
    onselect: (id: string) => void;
    onmove: (id: string, dir: -1 | 1) => void;
    onretry: (id: string) => void;
    onremove: (id: string) => void;
  }
  let { queue, selectedJobId, queueRunning, onselect, onmove, onretry, onremove }: Props = $props();

  function jobLabel(job: QueueJob): string {
    if (job.operation.type === "merge") {
      const n = job.operation.inputs.length;
      return n > 0 ? `merge ${n} file${n === 1 ? "" : "s"}` : "merge";
    }
    const input = (job.operation as { input: string }).input;
    return input.split("/").pop() ?? input;
  }
</script>

{#if queue.length === 0}
  <div class="px-5 py-5">
    <div class="queue-empty">
      <div class="queue-empty-grid"></div>
      <p class="text-[11px] text-muted-foreground text-center tracking-widest">— add jobs using the form —</p>
    </div>
  </div>
{:else}
  {#each queue as job}
    <div
      role="button"
      tabindex="0"
      onclick={() => onselect(job.id)}
      onkeydown={(e) => e.key === "Enter" && onselect(job.id)}
      class="flex items-center gap-3 px-5 py-2.5 border-b border-border last:border-b-0 cursor-pointer hover:bg-muted transition-colors"
      class:bg-muted={selectedJobId === job.id}
    >
      <span
        class="w-[5px] h-[5px] flex-shrink-0 bg-current"
        class:text-muted-foreground={job.status === "pending"}
        class:text-foreground={job.status === "running" || job.status === "done"}
        class:text-destructive={job.status === "error" || job.status === "cancelled"}
        class:dot-pulse={job.status === "running"}
      ></span>
      <span class="text-[8px] uppercase tracking-wider text-muted-foreground w-14 flex-shrink-0">{job.operation.type}</span>
      <span class="flex-1 text-[11px] text-foreground truncate">{jobLabel(job)}</span>
      {#if !queueRunning}
        <button
          type="button"
          aria-label="Move job up"
          onclick={(e) => { e.stopPropagation(); onmove(job.id, -1); }}
          class="text-[9px] text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
        >↑</button>
        <button
          type="button"
          aria-label="Move job down"
          onclick={(e) => { e.stopPropagation(); onmove(job.id, 1); }}
          class="text-[9px] text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
        >↓</button>
      {/if}
      {#if job.status === "running"}
        <span class="spinner flex-shrink-0"></span>
      {:else if job.status === "done"}
        <span class="text-[9px] text-foreground flex-shrink-0">✓</span>
      {:else if job.status === "error" || job.status === "cancelled"}
        <button
          type="button"
          aria-label="Retry job"
          onclick={(e) => { e.stopPropagation(); onretry(job.id); }}
          class="text-[9px] text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
        >↺</button>
      {:else if job.status === "pending"}
        <button
          type="button"
          aria-label="Remove job"
          onclick={(e) => { e.stopPropagation(); onremove(job.id); }}
          class="text-[9px] text-muted-foreground hover:text-destructive transition-colors cursor-pointer border-0 bg-transparent flex-shrink-0"
        >×</button>
      {/if}
    </div>
  {/each}
{/if}

<style>
  .queue-empty {
    border: 1px solid var(--border);
    border-radius: 2px;
    min-height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    background: color-mix(in oklab, var(--card) 84%, var(--background) 16%);
  }

  .queue-empty-grid {
    position: absolute;
    inset: 0;
    opacity: 0.25;
    background-image:
      linear-gradient(to right, color-mix(in oklab, var(--border) 60%, transparent) 1px, transparent 1px),
      linear-gradient(to bottom, color-mix(in oklab, var(--border) 60%, transparent) 1px, transparent 1px);
    background-size: 18px 18px;
  }
</style>
