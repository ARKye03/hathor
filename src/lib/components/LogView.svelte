<script lang="ts">
  import type { QueueJob } from "$lib/types";

  interface Props {
    selectedJob: QueueJob | null;
    logCollapsed?: boolean;
    oncopycommand: (cmd: string) => void;
    ontoggle: () => void;
  }
  let { selectedJob, logCollapsed = $bindable(false), oncopycommand, ontoggle }: Props = $props();

  function jobLabel(job: QueueJob): string {
    if (job.operation.type === "merge") {
      const n = job.operation.inputs.length;
      return n > 0 ? `merge ${n} file${n === 1 ? "" : "s"}` : "merge";
    }
    const input = (job.operation as { input: string }).input;
    return input.split("/").pop() ?? input;
  }

  let logPanel = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const len = selectedJob?.logs.length ?? 0;
    if (len > 0 && !logCollapsed && logPanel) {
      // micro-task to let DOM update first
      Promise.resolve().then(() => {
        if (logPanel) logPanel.scrollTop = logPanel.scrollHeight;
      });
    }
  });
</script>

<!-- Log header -->
<div class="h-10 flex items-center justify-between px-5 border-b border-border flex-shrink-0">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground truncate">
    {selectedJob ? jobLabel(selectedJob) : "Output Log"}
  </span>
  <div class="flex items-center gap-3 flex-shrink-0 ml-3">
    <span class="text-[9px] text-muted-foreground tabular-nums">{selectedJob?.logs.length ?? 0} lines</span>
    <button
      type="button"
      onclick={ontoggle}
      class="log-toggle text-[8px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border border-border bg-transparent px-2 py-1"
    >
      {logCollapsed ? "Expand Logs" : "Collapse Logs"}
    </button>
  </div>
</div>

{#if !logCollapsed && selectedJob?.command}
  <div class="px-5 py-2 border-b border-border flex items-center gap-3">
    <code class="flex-1 text-[9px] text-muted-foreground truncate font-mono">{selectedJob.command}</code>
    <button
      type="button"
      onclick={() => oncopycommand(selectedJob!.command)}
      class="text-[8px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-0 bg-transparent"
    >Copy</button>
  </div>
{/if}

<!-- Log body -->
<div class="log-shell border-b border-border" class:log-shell-collapsed={logCollapsed}>
  {#if logCollapsed}
    <div class="h-10 px-5 flex items-center justify-between text-[10px]">
      <span class="text-muted-foreground tracking-widest uppercase">Logs collapsed</span>
      <span class="text-foreground truncate max-w-[60%] text-right">
        {selectedJob?.logs.at(-1) ?? "— awaiting process —"}
      </span>
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto py-3" bind:this={logPanel}>
      {#if !selectedJob || selectedJob.logs.length === 0}
        <p class="px-5 py-8 text-[11px] text-muted-foreground text-center tracking-widest">— awaiting process —</p>
      {:else}
        {#each selectedJob.logs as line, i}
          <div class="flex gap-4 px-5 py-px text-[11px] leading-relaxed font-mono hover:bg-muted/70">
            <span class="text-muted-foreground select-none shrink-0 w-9 text-right opacity-40 tabular-nums">{String(i + 1).padStart(4, "0")}</span>
            <span class="text-foreground break-all whitespace-pre-wrap">{line}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .log-toggle {
    border-radius: 2px;
  }

  .log-shell {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    background: color-mix(in oklab, var(--background) 88%, var(--card) 12%);
    transition: flex 180ms ease, max-height 180ms ease;
  }

  .log-shell-collapsed {
    flex: 0 0 auto;
    max-height: 40px;
    border-bottom: none;
  }
</style>
