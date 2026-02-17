export function fmtDuration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function fmtBytes(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
  return `${Math.round(bytes / 1e3)} KB`;
}

export function fmtFps(fr: string | null): string | null {
  if (!fr) return null;
  const [n, d] = fr.split("/").map(Number);
  if (!n || !d) return null;
  const fps = n / d;
  return Number.isInteger(fps) ? `${fps}` : fps.toFixed(3).replace(/\.?0+$/, "");
}

export function fmtChannels(ch: number | null): string {
  if (ch === 1) return "Mono";
  if (ch === 2) return "Stereo";
  return ch ? `${ch}ch` : "";
}
