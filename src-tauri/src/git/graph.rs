use std::sync::RwLock;
use std::time::Instant;

use gix::hashtable::HashMap;
use gix::ObjectId;
use rayon::prelude::*;
use serde::Serialize;
use smallvec::{smallvec, SmallVec};

use super::search::GraphIndex;
use super::walk::{decode_meta, CommitMeta, WalkedCommit, Walker};
use super::wire::{self, Chunk, Edge, Row, Trailer};

pub const CHUNK_ROWS: usize = 2048;
const LANE_COLORS: u8 = 8;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct GraphStats {
    pub commits: u32,
    pub lanes: u32,
    pub micros: u64,
}

struct Lane {
    color: u8,
    children: SmallVec<[u32; 2]>,
}

/// Assigns every commit a rail and routes its edges.
///
/// A lane is reserved for the parent a child is waiting on and stays put until that
/// parent shows up, so first-parent chains render as straight rails. When several lanes
/// wait on the same commit it takes the leftmost and the others bend into it. Merge
/// edges reuse a lane already heading for the same parent instead of opening a new one,
/// which keeps wide histories from fanning out.
#[derive(Default)]
pub struct LaneLayout {
    lanes: Vec<Option<Lane>>,
    waiting: HashMap<ObjectId, SmallVec<[u16; 2]>>,
    next_color: u8,
}

impl LaneLayout {
    /// Places `commit` on `row` and appends every edge that terminates there.
    pub fn place(&mut self, row: u32, commit: &WalkedCommit, edges: &mut Vec<Edge>) -> Row {
        let mut incoming = self.waiting.remove(&commit.id).unwrap_or_default();
        incoming.sort_unstable();

        let (lane, color) = match incoming.first() {
            Some(&lane) => (lane, self.lanes[lane as usize].as_ref().map_or(0, |lane| lane.color)),
            None => (self.free_lane(), self.fresh_color()),
        };
        for &merged in &incoming {
            if let Some(state) = self.lanes[merged as usize].take() {
                let color = state.color;
                edges.extend(state.children.into_iter().map(|upper_row| Edge { upper_row, lane: merged, color }));
            }
        }

        for (position, parent) in commit.parents.iter().enumerate() {
            let shared = self.waiting.get(&parent.id).and_then(|lanes| lanes.first().copied());
            match (parent.emitted_row, position, shared) {
                (Some(upper_row), _, _) => edges.push(Edge { upper_row, lane, color }),
                (None, 0, _) => self.occupy(lane, parent.id, color, row),
                (None, _, Some(shared)) => {
                    if let Some(state) = self.lanes[shared as usize].as_mut() {
                        state.children.push(row);
                    }
                }
                (None, _, None) => {
                    let (fresh, color) = (self.free_lane(), self.fresh_color());
                    self.occupy(fresh, parent.id, color, row);
                }
            }
        }

        Row {
            lane,
            color,
            parent_count: commit.parents.len().min(usize::from(u8::MAX)) as u8,
            time: commit.time.clamp(0, i64::from(u32::MAX)) as u32,
        }
    }

    pub fn lane_count(&self) -> u32 {
        self.lanes.len() as u32
    }

    pub fn open_edges(&self) -> Vec<Edge> {
        let mut open = Vec::new();
        for (lane, state) in self.lanes.iter().enumerate() {
            let Some(state) = state else { continue };
            open.extend(state.children.iter().map(|&upper_row| Edge {
                upper_row,
                lane: lane as u16,
                color: state.color,
            }));
        }
        open
    }

    fn occupy(&mut self, lane: u16, target: ObjectId, color: u8, child_row: u32) {
        self.lanes[lane as usize] = Some(Lane { color, children: smallvec![child_row] });
        self.waiting.entry(target).or_default().push(lane);
    }

    fn free_lane(&mut self) -> u16 {
        let lane = self.lanes.iter().position(Option::is_none).unwrap_or_else(|| {
            self.lanes.push(None);
            self.lanes.len() - 1
        });
        lane as u16
    }

    fn fresh_color(&mut self) -> u8 {
        let color = self.next_color;
        self.next_color = (self.next_color + 1) % LANE_COLORS;
        color
    }
}

/// Walks the history from `tips` and hands each encoded chunk to `sink` as soon as it is
/// ready. Lane assignment is sequential by nature (a row depends on the lanes left open
/// above it), so it runs concurrently with the data-parallel commit metadata decode.
/// The walk stops early when `sink` returns `false`.
pub fn stream(
    sync: &gix::ThreadSafeRepository,
    tips: &[ObjectId],
    ref_marks: &HashMap<ObjectId, Vec<u32>>,
    index: &RwLock<GraphIndex>,
    mut sink: impl FnMut(Vec<u8>) -> bool,
) -> GraphStats {
    let started = Instant::now();
    let repo = sync.to_thread_local();
    let mut walker = Walker::new(&repo, tips.iter().copied());
    let mut layout = LaneLayout::default();
    let mut commits = Vec::with_capacity(CHUNK_ROWS);
    let mut metas = Vec::with_capacity(CHUNK_ROWS);
    let mut total = 0u32;

    loop {
        commits.clear();
        metas.clear();
        let has_more = walker.fill(CHUNK_ROWS, &mut commits, &mut metas);

        let mut chunk = Chunk { first_row: total, ..Chunk::default() };
        rayon::join(|| place_chunk(&mut layout, &commits, &mut chunk), || decode_missing(sync, &commits, &mut metas));
        for (offset, (commit, meta)) in commits.iter().zip(&metas).enumerate() {
            if let Some(meta) = meta {
                chunk.strings.extend_from_slice(&meta.summary);
                chunk.string_ends.push(chunk.strings.len() as u32);
                chunk.strings.extend_from_slice(&meta.author);
            } else {
                chunk.string_ends.push(chunk.strings.len() as u32);
            }
            chunk.string_ends.push(chunk.strings.len() as u32);
            chunk.oids.push(commit.id);
            if let Some(marks) = ref_marks.get(&commit.id) {
                let row = total + offset as u32;
                chunk.ref_marks.extend(marks.iter().map(|mark| (row, *mark)));
            }
        }
        total += commits.len() as u32;
        chunk.open_edges = layout.open_edges();
        if let Ok(mut index) = index.write() {
            index.append(&chunk);
        }

        let trailer = Trailer {
            is_final: !has_more,
            lane_count: layout.lane_count(),
            elapsed_micros: started.elapsed().as_micros().min(u128::from(u32::MAX)) as u32,
            total_rows: total,
        };
        if !sink(wire::encode(&chunk, trailer)) || !has_more {
            break;
        }
    }

    GraphStats { commits: total, lanes: layout.lane_count(), micros: started.elapsed().as_micros() as u64 }
}

fn place_chunk(layout: &mut LaneLayout, commits: &[WalkedCommit], chunk: &mut Chunk) {
    chunk.edge_ends.push(0);
    for (offset, commit) in commits.iter().enumerate() {
        let row = layout.place(chunk.first_row + offset as u32, commit, &mut chunk.edges);
        chunk.rows.push(row);
        chunk.edge_ends.push(chunk.edges.len() as u32);
    }
}

fn decode_missing(sync: &gix::ThreadSafeRepository, commits: &[WalkedCommit], metas: &mut [Option<CommitMeta>]) {
    if metas.iter().all(Option::is_some) {
        return;
    }
    commits.par_iter().zip(metas.par_iter_mut()).with_min_len(64).for_each_init(
        || (sync.to_thread_local(), Vec::with_capacity(1024)),
        |(repo, buffer), (commit, meta)| {
            if meta.is_none() {
                *meta = Some(decode_meta(repo, &commit.id, buffer));
            }
        },
    );
}
