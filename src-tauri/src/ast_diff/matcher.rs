use std::collections::HashMap;

use super::model::{ChangeKind, Entity, EntityKind, ParsedFile};
use super::moves;
use super::similarity::{Fingerprints, Scope};
use super::tokens::text_hash;
use super::FilePair;

/// A same-named declaration found elsewhere must overlap at least this much to count as moved.
const SAME_NAME_FLOOR: f32 = 0.4;
/// Renamed or heavily edited declarations need a stronger overlap to be paired at all.
const FUZZY_FLOOR: f32 = 0.6;
/// Bodies shorter than this are too generic to identify a renamed declaration.
const FUZZY_MIN_TOKENS: usize = 8;
/// Upper bound on candidate comparisons per pass, so pathological changesets stay interactive.
const PAIR_BUDGET: usize = 40_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityRef {
    pub file: usize,
    pub index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Change {
    pub kind: ChangeKind,
    pub old: Option<EntityRef>,
    pub new: Option<EntityRef>,
    pub similarity: f32,
}

pub struct Matcher<'a> {
    files: &'a [FilePair],
    forward: HashMap<EntityRef, EntityRef>,
    backward: HashMap<EntityRef, EntityRef>,
    old_prints: Fingerprints<'a>,
    new_prints: Fingerprints<'a>,
}

/// Pairs declarations across the whole changeset, strongest evidence first: same file
/// and qualified name, identical structural hash anywhere, same name elsewhere, and
/// finally token similarity. Pairing a container also pairs its members by name, so a
/// relocated class does not scatter its methods across unrelated candidates.
/// Returns the changes plus every pairing, keyed by the new-side declaration.
pub fn changes(files: &[FilePair]) -> (Vec<Change>, HashMap<EntityRef, EntityRef>) {
    let mut matcher = Matcher {
        files,
        forward: HashMap::new(),
        backward: HashMap::new(),
        old_prints: Fingerprints::new(files, |pair| &pair.old),
        new_prints: Fingerprints::new(files, |pair| &pair.new),
    };
    for file in 0..files.len() {
        matcher.pair_members(file, None, file, None, true);
    }
    matcher.pair_remaining(Scope::Whole, 1.0, |entity| entity.hash);
    matcher.pair_remaining(Scope::Whole, SAME_NAME_FLOOR, |entity| text_hash(&entity.name));
    matcher.pair_remaining(Scope::Body, FUZZY_FLOOR, |_| 0);
    (matcher.classify(), matcher.backward)
}

impl<'a> Matcher<'a> {
    fn old_entity(&self, at: EntityRef) -> &'a Entity {
        &self.files[at.file].old.entities[at.index]
    }

    fn new_entity(&self, at: EntityRef) -> &'a Entity {
        &self.files[at.file].new.entities[at.index]
    }

    fn link(&mut self, old: EntityRef, new: EntityRef) {
        self.forward.insert(old, new);
        self.backward.insert(new, old);
        if self.old_entity(old).is_container && self.new_entity(new).is_container {
            self.pair_members(old.file, Some(old.index), new.file, Some(new.index), false);
        }
    }

    /// Pairs the children of two matched scopes by kind and name, in declaration order.
    fn pair_members(
        &mut self,
        old_file: usize,
        old_parent: Option<usize>,
        new_file: usize,
        new_parent: Option<usize>,
        qualified: bool,
    ) {
        let key = |entity: &Entity| {
            (entity.kind.family(), if qualified { entity.qualified.clone() } else { entity.name.clone() })
        };
        let (old_side, new_side) = (&self.files[old_file].old, &self.files[new_file].new);
        let mut candidates: HashMap<(EntityKind, String), Vec<usize>> = HashMap::new();
        for index in new_side
            .children_of(new_parent)
            .filter(|index| !self.backward.contains_key(&EntityRef { file: new_file, index: *index }))
        {
            candidates.entry(key(&new_side.entities[index])).or_default().push(index);
        }
        candidates.values_mut().for_each(|indices| indices.reverse());

        let members: Vec<usize> = old_side.children_of(old_parent).collect();
        for index in members {
            let old = EntityRef { file: old_file, index };
            if self.forward.contains_key(&old) {
                continue;
            }
            if let Some(new) = candidates.get_mut(&key(&old_side.entities[index])).and_then(Vec::pop) {
                self.link(old, EntityRef { file: new_file, index: new });
            }
        }
    }

    fn unmatched(&self, side: fn(&FilePair) -> &ParsedFile, taken: &HashMap<EntityRef, EntityRef>) -> Vec<EntityRef> {
        let all = self
            .files
            .iter()
            .enumerate()
            .flat_map(|(file, pair)| (0..side(pair).entities.len()).map(move |index| EntityRef { file, index }));
        all.filter(|at| !taken.contains_key(at)).collect()
    }

    /// Greedily links the best-scoring unmatched pairs that share a `bucket` and reach `floor`.
    fn pair_remaining(&mut self, scope: Scope, floor: f32, bucket: impl Fn(&Entity) -> u64) {
        let mut buckets: HashMap<(EntityKind, u64), Vec<EntityRef>> = HashMap::new();
        for new in self.unmatched(|pair| &pair.new, &self.backward) {
            let entity = self.new_entity(new);
            buckets.entry((entity.kind.family(), bucket(entity))).or_default().push(new);
        }

        let mut budget = PAIR_BUDGET;
        let mut scored = Vec::new();
        for old in self.unmatched(|pair| &pair.old, &self.forward) {
            let left = self.old_entity(old);
            let Some(candidates) = buckets.get(&(left.kind.family(), bucket(left))) else { continue };
            if candidates.len() > budget {
                break;
            }
            budget -= candidates.len();
            for &new in candidates {
                let too_small = scope == Scope::Body
                    && self.old_prints.len(old, scope).min(self.new_prints.len(new, scope)) < FUZZY_MIN_TOKENS;
                if too_small {
                    continue;
                }
                if let Some(score) = self.old_prints.similarity(old, &self.new_prints, new, scope, floor) {
                    scored.push((score, old, new));
                }
            }
        }
        scored
            .sort_by(|left, right| right.0.total_cmp(&left.0).then_with(|| (left.1, left.2).cmp(&(right.1, right.2))));
        for (_, old, new) in scored {
            if !self.forward.contains_key(&old) && !self.backward.contains_key(&new) {
                self.link(old, new);
            }
        }
    }

    fn classify(&self) -> Vec<Change> {
        let moved = moves::moved_pairs(&self.forward, |at| self.old_entity(at).parent, |at| self.new_entity(at).parent);
        let mut out = Vec::new();
        for (&old, &new) in &self.forward {
            let (left, right) = (self.old_entity(old), self.new_entity(new));
            let kind = match (moved.contains(&old), left.hash == right.hash) {
                (true, true) => ChangeKind::Moved,
                (true, false) => ChangeKind::MovedEdited,
                (false, true) => continue,
                (false, false)
                    if left.is_container && right.is_container && left.shallow_hash == right.shallow_hash =>
                {
                    continue
                }
                (false, false) => {
                    match (left.signature_hash == right.signature_hash, left.body_hash == right.body_hash) {
                        (false, true) => ChangeKind::Signature,
                        (true, false) => ChangeKind::Body,
                        _ if left.is_container => ChangeKind::Body,
                        _ => ChangeKind::Modified,
                    }
                }
            };
            let similarity = self.old_prints.similarity(old, &self.new_prints, new, Scope::Whole, 0.0).unwrap_or(0.0);
            out.push(Change { kind, old: Some(old), new: Some(new), similarity });
        }

        let orphan = |parent: Option<usize>, file: usize, taken: &HashMap<EntityRef, EntityRef>| {
            parent.is_none_or(|index| taken.contains_key(&EntityRef { file, index }))
        };
        for old in self.unmatched(|pair| &pair.old, &self.forward) {
            if orphan(self.old_entity(old).parent, old.file, &self.forward) {
                out.push(Change { kind: ChangeKind::Deleted, old: Some(old), new: None, similarity: 0.0 });
            }
        }
        for new in self.unmatched(|pair| &pair.new, &self.backward) {
            if orphan(self.new_entity(new).parent, new.file, &self.backward) {
                out.push(Change { kind: ChangeKind::Added, old: None, new: Some(new), similarity: 0.0 });
            }
        }
        out.sort_by_key(|change| (change.new.or(change.old), change.old));
        out
    }
}
