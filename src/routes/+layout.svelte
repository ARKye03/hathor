<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { theme, type ThemePref, THEME_KEY } from "$lib/theme.svelte";
  import "../app.css";

  let { children } = $props();

  function applyTheme(pref: ThemePref) {
    const isDark =
      pref === "dark" ||
      (pref === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.classList.toggle("dark", isDark);
  }

  let removeMqListener: (() => void) | null = null;

  onMount(() => {
    const saved = localStorage.getItem(THEME_KEY) as ThemePref | null;
    if (saved) theme.pref = saved;

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => { if (theme.pref === "system") applyTheme("system"); };
    mq.addEventListener("change", handler);
    removeMqListener = () => mq.removeEventListener("change", handler);
  });

  onDestroy(() => removeMqListener?.());

  $effect(() => {
    applyTheme(theme.pref);
    try { localStorage.setItem(THEME_KEY, theme.pref); } catch {}
  });
</script>

{@render children()}
