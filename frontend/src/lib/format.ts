export function fmt(n: number | null | undefined): string {
  if (n == null || isNaN(n)) return "--";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  return Number.isInteger(n) ? n.toString() : n.toFixed(1);
}

export function fmtRound(n: number | null | undefined): string {
  return fmt(n != null ? Math.round(n) : n);
}

export function fmtMs(ms: number | null | undefined): string {
  if (ms == null || isNaN(ms)) return "--";
  if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
  return ms + "ms";
}
