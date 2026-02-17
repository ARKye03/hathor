<script lang="ts">
  interface Props {
    value?: string;
    placeholder?: string;
    onbrowse: () => void;
  }
  let { value = $bindable(""), placeholder = "", onbrowse }: Props = $props();
</script>

<div class="flex">
  <input
    type="text"
    spellcheck="false"
    bind:value
    {placeholder}
    class="bg-input border border-border text-foreground font-mono text-[11px] px-3 py-2 flex-1 min-w-0 outline-none transition-colors placeholder:text-muted-foreground focus:border-foreground"
  />
  <button type="button" aria-label="Browse" onclick={onbrowse} class="browse-btn">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
      <path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"/>
    </svg>
  </button>
</div>
