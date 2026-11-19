export interface FuzzyMatch {
  score: number;
  /** Indices of the matched characters, for highlighting. */
  positions: number[];
}

const BOUNDARY = /[\s/_\-.:(]/;

/**
 * Subsequence match that favors consecutive characters and word starts, so `cob`
 * ranks "Checkout branch" above "Toggle sidebar". Returns `null` when the query
 * is not a subsequence of the text.
 */
export function fuzzyMatch(query: string, text: string): FuzzyMatch | null {
  const needle = query.toLowerCase().replace(/\s+/g, "");
  if (needle === "") return { score: 0, positions: [] };
  const haystack = text.toLowerCase();

  const positions: number[] = [];
  let score = 0;
  let from = 0;
  for (const character of needle) {
    const at = haystack.indexOf(character, from);
    if (at < 0) return null;
    const previous = positions[positions.length - 1];
    if (previous !== undefined && at === previous + 1) score += 8;
    if (at === 0 || BOUNDARY.test(haystack[at - 1] ?? "")) score += 6;
    score -= Math.min(4, at - from);
    positions.push(at);
    from = at + 1;
  }
  return { score: score - haystack.length * 0.02, positions };
}
