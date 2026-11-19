const MAGIC = 0x54465247;
const HEADER_WORDS = 12;
const OID_BYTES = 20;
export const CHUNK_SHIFT = 11;
const CHUNK_MASK = (1 << CHUNK_SHIFT) - 1;
const TEXT_CACHE_LIMIT = 4096;
const HEX = Array.from({ length: 256 }, (_, byte) => byte.toString(16).padStart(2, "0"));

/** Zero-copy views over one frame produced by `git::wire::encode`. */
export interface Chunk {
  firstRow: number;
  rowCount: number;
  rows: Uint32Array;
  stringEnds: Uint32Array;
  edgeEnds: Uint32Array;
  edges: Uint32Array;
  openEdges: Uint32Array;
  oids: Uint8Array;
  strings: Uint8Array;
}

export interface RowText {
  summary: string;
  author: string;
}

/**
 * Commit graph storage. Frames are kept as received and addressed by `row >> CHUNK_SHIFT`,
 * so appending a chunk never copies or re-indexes the rows that are already on screen.
 */
export class GraphModel {
  chunks: Chunk[] = [];
  total = 0;
  laneCount = 1;
  complete = false;
  elapsedMicros = 0;
  /** Ref list indices decorating each row. */
  refsByRow = new Map<number, number[]>();
  rowByRef = new Map<number, number>();

  private decoder = new TextDecoder();
  private text = new Map<number, RowText>();

  append(buffer: ArrayBuffer): void {
    const header = new Uint32Array(buffer, 0, HEADER_WORDS);
    if (header[0] !== MAGIC) throw new Error("Unrecognized graph frame");
    const [, firstRow = 0, rowCount = 0, edgeCount = 0, openCount = 0, markCount = 0, stringsLength = 0, flags = 0] = header;

    let offset = HEADER_WORDS * 4;
    const words = (length: number) => {
      const view = new Uint32Array(buffer, offset, length);
      offset += length * 4;
      return view;
    };
    const rows = words(rowCount * 2);
    const stringEnds = words(rowCount * 2);
    const edgeEnds = words(rowCount + 1);
    const edges = words(edgeCount * 2);
    const openEdges = words(openCount * 2);
    const marks = words(markCount * 2);
    const oids = new Uint8Array(buffer, offset, rowCount * OID_BYTES);
    const strings = new Uint8Array(buffer, offset + rowCount * OID_BYTES, stringsLength);

    for (let index = 0; index < markCount; index++) {
      const row = marks[index * 2] ?? 0;
      const ref = marks[index * 2 + 1] ?? 0;
      const decorated = this.refsByRow.get(row);
      if (decorated) decorated.push(ref);
      else this.refsByRow.set(row, [ref]);
      this.rowByRef.set(ref, row);
    }

    this.chunks.push({ firstRow, rowCount, rows, stringEnds, edgeEnds, edges, openEdges, oids, strings });
    this.total = header[10] ?? this.total + rowCount;
    this.laneCount = Math.max(1, header[8] ?? 1);
    this.elapsedMicros = header[9] ?? 0;
    this.complete = (flags & 1) === 1;
  }

  chunkOf(row: number): Chunk | undefined {
    return this.chunks[row >> CHUNK_SHIFT];
  }

  private word(row: number, slot: number): number {
    return this.chunkOf(row)?.rows[(row & CHUNK_MASK) * 2 + slot] ?? 0;
  }

  lane(row: number): number {
    return this.word(row, 0) & 0xffff;
  }

  color(row: number): number {
    return (this.word(row, 0) >>> 16) & 0xff;
  }

  parentCount(row: number): number {
    return this.word(row, 0) >>> 24;
  }

  time(row: number): number {
    return this.word(row, 1);
  }

  oid(row: number): string {
    const chunk = this.chunkOf(row);
    if (!chunk) return "";
    const start = (row & CHUNK_MASK) * OID_BYTES;
    let hex = "";
    for (let index = start; index < start + OID_BYTES; index++) hex += HEX[chunk.oids[index] ?? 0];
    return hex;
  }

  /** Decodes summary and author on first use; only rows that scroll into view pay for it. */
  textOf(row: number): RowText {
    const cached = this.text.get(row);
    if (cached) return cached;
    const chunk = this.chunkOf(row);
    if (!chunk) return { summary: "", author: "" };

    const local = row & CHUNK_MASK;
    const start = local === 0 ? 0 : (chunk.stringEnds[local * 2 - 1] ?? 0);
    const middle = chunk.stringEnds[local * 2] ?? start;
    const end = chunk.stringEnds[local * 2 + 1] ?? middle;
    const decoded = {
      summary: this.decoder.decode(chunk.strings.subarray(start, middle)),
      author: this.decoder.decode(chunk.strings.subarray(middle, end)),
    };
    if (this.text.size >= TEXT_CACHE_LIMIT) this.text.clear();
    this.text.set(row, decoded);
    return decoded;
  }
}
