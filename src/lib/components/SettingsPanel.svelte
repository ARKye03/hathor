<script lang="ts">
  import { theme, type ThemePref } from "$lib/theme.svelte";

  interface Props {
    defaultOutputDir?: string;
    outputNameTemplate?: string;
    cancelCleanupEnabled?: boolean;
    onpickdir: () => void;
  }
  let {
    defaultOutputDir = $bindable(""),
    outputNameTemplate = $bindable("{name}_out"),
    cancelCleanupEnabled = $bindable(true),
    onpickdir,
  }: Props = $props();
</script>

<div class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Theme</span>
  <div class="flex border border-border">
    {#each (["light", "dark", "system"] as ThemePref[]) as opt, i}
      <button
        type="button"
        onclick={() => theme.pref = opt}
        class="flex-1 py-1.5 text-[9px] font-semibold tracking-[0.12em] uppercase border-0 border-r border-border cursor-pointer transition-colors"
        class:bg-primary={theme.pref === opt}
        class:text-primary-foreground={theme.pref === opt}
        class:bg-transparent={theme.pref !== opt}
        class:text-muted-foreground={theme.pref !== opt}
        class:border-r-0={i === 2}
      >{opt}</button>
    {/each}
  </div>
</div>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Default Output Folder</span>
  <div class="flex">
    <input type="text" spellcheck="false" bind:value={defaultOutputDir} placeholder="Use source folder by default"
      class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
    <button type="button" aria-label="Browse" onclick={onpickdir} class="browse-btn">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/></svg>
    </button>
  </div>
  <button type="button" onclick={() => defaultOutputDir = ""}
    class="self-start text-[8px] font-semibold tracking-[0.15em] uppercase text-muted-foreground hover:text-foreground transition-colors border border-border px-2 py-1 bg-transparent cursor-pointer"
  >Use Source Folder</button>
</label>

<label class="flex items-center gap-2 text-[9px] font-semibold tracking-[0.18em] uppercase text-muted-foreground">
  <input type="checkbox" bind:checked={cancelCleanupEnabled} class="accent-current w-3 h-3" />
  Cleanup Partial Output By Default
</label>

<label class="flex flex-col gap-2">
  <span class="text-[9px] font-semibold tracking-[0.2em] uppercase text-muted-foreground">Output Naming Template</span>
  <input type="text" spellcheck="false" bind:value={outputNameTemplate} placeholder="{name}_out"
    class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground" />
  <p class="text-[9px] text-muted-foreground">Placeholders: {"{name}"}, {"{mode}"}, {"{ext}"}, {"{ts}"}</p>
</label>

<p class="text-[9px] text-muted-foreground">These preferences apply to newly created queue jobs.</p>
