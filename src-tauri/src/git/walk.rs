use std::collections::BinaryHeap;

use gix::hashtable::HashMap;
use gix::objs::commit::ref_iter::Token;
use gix::objs::FindExt;
use gix::ObjectId;
use smallvec::SmallVec;

const QUEUED: u32 = u32::MAX;
const SUMMARY_LIMIT: usize = 240;

#[derive(Debug, Clone)]
pub struct Parent {
    pub id: ObjectId,
    /// Set when clock skew caused the parent to be emitted before this child.
    pub emitted_row: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct WalkedCommit {
    pub id: ObjectId,
    pub parents: SmallVec<[Parent; 2]>,
    pub time: i64,
}

#[derive(Debug, Clone, Default)]
pub struct CommitMeta {
    pub summary: Vec<u8>,
    pub author: Vec<u8>,
}

struct Pending {
    time: i64,
    sequence: u32,
    id: ObjectId,
    parents: SmallVec<[ObjectId; 2]>,
    meta: Option<CommitMeta>,
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.sequence == other.sequence
    }
}

impl Eq for Pending {}

impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pending {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time).then_with(|| other.sequence.cmp(&self.sequence))
    }
}

/// Newest-first commit traversal that decodes every commit exactly once.
///
/// With a commit-graph file the topology comes straight from the memory map and
/// author/summary decoding is deferred to the caller, which batches it across cores.
pub struct Walker<'repo> {
    repo: &'repo gix::Repository,
    commit_graph: Option<gix::commitgraph::Graph>,
    queue: BinaryHeap<Pending>,
    rows: HashMap<ObjectId, u32>,
    buffer: Vec<u8>,
    next_row: u32,
    sequence: u32,
}

impl<'repo> Walker<'repo> {
    pub fn new(repo: &'repo gix::Repository, tips: impl IntoIterator<Item = ObjectId>) -> Self {
        let mut walker = Self {
            repo,
            commit_graph: repo.commit_graph_if_enabled().ok().flatten(),
            queue: BinaryHeap::new(),
            rows: HashMap::default(),
            buffer: Vec::with_capacity(1024),
            next_row: 0,
            sequence: 0,
        };
        for tip in tips {
            walker.enqueue(tip);
        }
        walker
    }

    /// Appends up to `limit` commits and returns `false` once the history is exhausted.
    pub fn fill(&mut self, limit: usize, commits: &mut Vec<WalkedCommit>, metas: &mut Vec<Option<CommitMeta>>) -> bool {
        while commits.len() < limit {
            let Some(pending) = self.queue.pop() else {
                return false;
            };
            self.rows.insert(pending.id, self.next_row);
            self.next_row += 1;

            let mut parents = SmallVec::new();
            for id in pending.parents {
                let emitted_row = self.rows.get(&id).copied().filter(|row| *row != QUEUED);
                if self.enqueue(id) {
                    parents.push(Parent { id, emitted_row });
                }
            }
            commits.push(WalkedCommit { id: pending.id, parents, time: pending.time });
            metas.push(pending.meta);
        }
        !self.queue.is_empty()
    }

    /// Returns `false` when the object is missing, which marks a shallow boundary.
    fn enqueue(&mut self, id: ObjectId) -> bool {
        if self.rows.contains_key(&id) {
            return true;
        }
        let Some(mut pending) = self.load(id) else {
            return false;
        };
        pending.sequence = self.sequence;
        self.sequence += 1;
        self.rows.insert(id, QUEUED);
        self.queue.push(pending);
        true
    }

    fn load(&mut self, id: ObjectId) -> Option<Pending> {
        if let Some(commit) = self.commit_graph.as_ref().and_then(|graph| graph.commit_by_id(id)) {
            let graph = self.commit_graph.as_ref()?;
            let time = commit.committer_timestamp() as i64;
            let parents = commit
                .iter_parents()
                .filter_map(|position| position.ok())
                .map(|position| graph.id_at(position).to_owned())
                .collect();
            return Some(Pending { time, sequence: 0, id, parents, meta: None });
        }

        let mut pending = Pending { time: 0, sequence: 0, id, parents: SmallVec::new(), meta: None };
        let mut meta = CommitMeta::default();
        for token in self.repo.objects.find_commit_iter(&id, &mut self.buffer).ok()? {
            match token.ok()? {
                Token::Parent { id } => pending.parents.push(id),
                Token::Author { signature } => meta.author = signature.name.to_vec(),
                Token::Committer { signature } => pending.time = signature.seconds(),
                Token::Message(message) => {
                    meta.summary = summary_of(message);
                    break;
                }
                _ => {}
            }
        }
        pending.meta = Some(meta);
        Some(pending)
    }
}

pub fn decode_meta(repo: &gix::Repository, id: &ObjectId, buffer: &mut Vec<u8>) -> CommitMeta {
    let mut meta = CommitMeta::default();
    let Ok(tokens) = repo.objects.find_commit_iter(id, buffer) else {
        return meta;
    };
    for token in tokens.flatten() {
        match token {
            Token::Author { signature } => meta.author = signature.name.to_vec(),
            Token::Message(message) => {
                meta.summary = summary_of(message);
                break;
            }
            _ => {}
        }
    }
    meta
}

fn summary_of(message: &[u8]) -> Vec<u8> {
    let line = message.split(|byte| *byte == b'\n').next().unwrap_or_default();
    let mut end = line.len().min(SUMMARY_LIMIT);
    while end > 0 && end < line.len() && (line[end] & 0xC0) == 0x80 {
        end -= 1;
    }
    line[..end].trim_ascii().to_vec()
}
