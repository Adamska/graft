use gix::ObjectId;

pub const MAGIC: u32 = 0x5446_5247;
pub const HEADER_WORDS: usize = 12;
pub const FLAG_FINAL: u32 = 1;

#[derive(Debug, Clone, Copy, Default)]
pub struct Row {
    pub lane: u16,
    pub color: u8,
    pub parent_count: u8,
    pub time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// Row of the node at the upper end; the lower end is the row whose bucket holds the edge.
    pub upper_row: u32,
    pub lane: u16,
    pub color: u8,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub first_row: u32,
    pub rows: Vec<Row>,
    /// Prefix offsets into `edges`; edges ending at local row `i` are `edge_ends[i]..edge_ends[i + 1]`.
    pub edge_ends: Vec<u32>,
    pub edges: Vec<Edge>,
    /// Edges still waiting for their parent when the chunk was sealed.
    pub open_edges: Vec<Edge>,
    pub ref_marks: Vec<(u32, u32)>,
    pub oids: Vec<ObjectId>,
    pub strings: Vec<u8>,
    /// Two cumulative end offsets per row: summary, then author.
    pub string_ends: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Trailer {
    pub is_final: bool,
    pub lane_count: u32,
    pub elapsed_micros: u32,
    pub total_rows: u32,
}

// Chunk frame, little-endian, every section aligned to 4 bytes so the webview can
// alias it with Uint32Array views instead of copying:
//
//   header        u32 x 12   magic, first_row, row_count, edge_count, open_count, mark_count,
//                            strings_len, flags, lane_count, elapsed_micros, total_rows, reserved
//   rows          u32 x 2R   [lane | color << 16 | parent_count << 24, commit_time]
//   string_ends   u32 x 2R   [summary_end, author_end] cumulative into `strings`
//   edge_ends     u32 x R+1  prefix offsets, bucketed by the row an edge terminates at
//   edges         u32 x 2E   [upper_row, lane | color << 16]
//   open_edges    u32 x 2O   same encoding, parent not reached yet
//   ref_marks     u32 x 2M   [row, index into the snapshot ref list]
//   oids          u8  x 20R
//   strings       u8  x strings_len
pub fn encode(chunk: &Chunk, trailer: Trailer) -> Vec<u8> {
    let row_count = chunk.rows.len();
    let words = HEADER_WORDS
        + row_count * 4
        + chunk.edge_ends.len()
        + (chunk.edges.len() + chunk.open_edges.len() + chunk.ref_marks.len()) * 2;
    let mut out = Vec::with_capacity(words * 4 + row_count * 20 + chunk.strings.len());

    let header = [
        MAGIC,
        chunk.first_row,
        row_count as u32,
        chunk.edges.len() as u32,
        chunk.open_edges.len() as u32,
        chunk.ref_marks.len() as u32,
        chunk.strings.len() as u32,
        if trailer.is_final { FLAG_FINAL } else { 0 },
        trailer.lane_count,
        trailer.elapsed_micros,
        trailer.total_rows,
        0,
    ];
    header.iter().for_each(|word| put(&mut out, *word));

    for row in &chunk.rows {
        put(&mut out, pack(row.lane, row.color) | u32::from(row.parent_count) << 24);
        put(&mut out, row.time);
    }
    chunk.string_ends.iter().for_each(|end| put(&mut out, *end));
    chunk.edge_ends.iter().for_each(|end| put(&mut out, *end));
    for edge in chunk.edges.iter().chain(&chunk.open_edges) {
        put(&mut out, edge.upper_row);
        put(&mut out, pack(edge.lane, edge.color));
    }
    for (row, reference) in &chunk.ref_marks {
        put(&mut out, *row);
        put(&mut out, *reference);
    }
    for oid in &chunk.oids {
        out.extend_from_slice(&oid.as_bytes()[..20]);
    }
    out.extend_from_slice(&chunk.strings);
    out
}

fn pack(lane: u16, color: u8) -> u32 {
    u32::from(lane) | u32::from(color) << 16
}

fn put(out: &mut Vec<u8>, word: u32) {
    out.extend_from_slice(&word.to_le_bytes());
}
