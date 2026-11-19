import { fileName } from "../../lib/format";
import type { Block, ChangeKind, EntityKind, Location } from "../../lib/types";

export const ENTITY_LABELS: Record<EntityKind, string> = {
  function: "Function",
  method: "Method",
  struct: "Struct",
  enum: "Enum",
  trait: "Trait",
  impl: "Impl",
  class: "Class",
  interface: "Interface",
  type_alias: "Type",
  module: "Module",
  constant: "Constant",
  macro: "Macro",
};

type Tone = "moved" | "edited" | "add" | "del" | "neutral";

export const TONE_CLASSES: Record<Tone, string> = {
  moved: "border-moved/50 text-moved",
  edited: "border-edited/50 text-edited",
  add: "border-add/50 text-add",
  del: "border-del/50 text-del",
  neutral: "border-line-strong text-soft",
};

const TONES: Record<ChangeKind, Tone> = {
  moved: "moved",
  origin: "moved",
  moved_edited: "edited",
  signature: "neutral",
  body: "neutral",
  modified: "neutral",
  lines: "neutral",
  added: "add",
  deleted: "del",
};

export function toneOf(kind: ChangeKind): Tone {
  return TONES[kind];
}

function place(location: Location | null): string {
  if (!location) return "";
  return location.path ? `${fileName(location.path)}:L${location.start}` : `L${location.start}`;
}

/** Badge text such as `MOVED L42 -> L189` or `MOVED + EDITED 88%`. */
export function badgeFor(block: Block): string {
  switch (block.kind) {
    case "moved":
      return `MOVED ${place(block.old)} -> ${place(block.new)}`;
    case "moved_edited":
      return `MOVED + EDITED ${block.similarity}%`;
    case "signature":
      return block.renamed_from ? "RENAMED" : "SIGNATURE";
    case "body":
      return "BODY";
    case "modified":
      return "SIGNATURE + BODY";
    case "added":
      return "ADDED";
    case "deleted":
      return "DELETED";
    case "lines":
      return "LINES";
    case "origin":
      return `MOVED TO ${place(block.new)}`;
  }
}

/** Secondary detail shown next to the badge. */
export function detailFor(block: Block): string | null {
  if (block.kind === "moved_edited") return `${place(block.old)} -> ${place(block.new)}`;
  if (block.renamed_from) return `was ${block.renamed_from}`;
  return null;
}

export function lineRange(location: Location | null): string {
  if (!location) return "";
  const lines = location.start === location.end ? `L${location.start}` : `L${location.start}–${location.end}`;
  return location.path ? `${fileName(location.path)} ${lines}` : lines;
}

/** Container prefix of a qualified name, e.g. `AuthToken::` for `AuthToken::validate`. */
export function qualifier(block: Block): string {
  return block.qualified.endsWith(block.name) ? block.qualified.slice(0, block.qualified.length - block.name.length) : "";
}
