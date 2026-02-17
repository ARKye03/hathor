export type ThemePref = "light" | "dark" | "system";
export const THEME_KEY = "hathor-theme";

function createThemeStore() {
  let pref = $state<ThemePref>("system");
  return {
    get pref() { return pref; },
    set pref(v: ThemePref) { pref = v; }
  };
}

export const theme = createThemeStore();
