const INTEGER = new Intl.NumberFormat("en-US");

export function formatCount(value: number): string {
  return INTEGER.format(value);
}

export function formatDuration(micros: number): string {
  if (micros < 1000) return `${micros}µs`;
  const millis = micros / 1000;
  if (millis < 10) return `${millis.toFixed(1)}ms`;
  if (millis < 1000) return `${Math.round(millis)}ms`;
  return `${(millis / 1000).toFixed(2)}s`;
}

export function formatMegabytes(bytes: number): string {
  return `${Math.round(bytes / 1_048_576)} MB`;
}

const UNITS: [limit: number, seconds: number, suffix: string][] = [
  [60, 1, "s"],
  [3600, 60, "m"],
  [86_400, 3600, "h"],
  [604_800, 86_400, "d"],
  [2_629_800, 604_800, "w"],
  [31_557_600, 2_629_800, "mo"],
];

/** Compact age such as `4h` or `3mo`, relative to `now` in seconds. */
export function formatAge(epochSeconds: number, now: number): string {
  const elapsed = Math.max(0, now - epochSeconds);
  for (const [limit, seconds, suffix] of UNITS) {
    if (elapsed < limit) return `${Math.max(1, Math.floor(elapsed / seconds))}${suffix}`;
  }
  return `${Math.floor(elapsed / 31_557_600)}y`;
}

export function formatTimestamp(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toLocaleString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function shortOid(oid: string): string {
  return oid.slice(0, 7);
}

export function fileName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

export function directory(path: string): string {
  const cut = path.lastIndexOf("/");
  return cut < 0 ? "" : path.slice(0, cut + 1);
}
