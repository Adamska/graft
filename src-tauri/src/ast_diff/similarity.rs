use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use similar::{capture_diff_slices, diff_ratio, Algorithm};

use super::matcher::EntityRef;
use super::model::{ParsedFile, TokenClass};
use super::tokens::text_hash;
use super::FilePair;

struct Print {
    sequence: Vec<u64>,
    sorted: Vec<u64>,
}

/// Which tokens of a declaration take part in a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Whole,
    /// Body only, so a shared signature shape cannot pair two unrelated short functions.
    Body,
}

/// Lazily computed token sequences for one side of the changeset.
pub struct Fingerprints<'a> {
    files: &'a [FilePair],
    side: fn(&FilePair) -> &ParsedFile,
    cache: RefCell<HashMap<(EntityRef, Scope), Rc<Print>>>,
}

impl<'a> Fingerprints<'a> {
    pub fn new(files: &'a [FilePair], side: fn(&FilePair) -> &ParsedFile) -> Self {
        Self { files, side, cache: RefCell::default() }
    }

    fn print(&self, at: EntityRef, scope: Scope) -> Rc<Print> {
        let mut cache = self.cache.borrow_mut();
        Rc::clone(cache.entry((at, scope)).or_insert_with(|| {
            let file = (self.side)(&self.files[at.file]);
            let entity = &file.entities[at.index];
            let range = match (scope, &entity.body_tokens) {
                (Scope::Body, Some(body)) => body.clone(),
                _ => entity.tokens.clone(),
            };
            let sequence: Vec<u64> = file.tokens[range]
                .iter()
                .filter(|token| token.class != TokenClass::Comment)
                .map(|token| text_hash(file.token_text(token)))
                .collect();
            let mut sorted = sequence.clone();
            sorted.sort_unstable();
            Rc::new(Print { sequence, sorted })
        }))
    }

    pub fn len(&self, at: EntityRef, scope: Scope) -> usize {
        self.print(at, scope).sequence.len()
    }

    /// Token-level similarity in `0.0..=1.0`, or `None` when it falls below `floor`.
    ///
    /// The shared-token count bounds the longest common subsequence from above, so most
    /// unrelated pairs are rejected by a linear merge without running the diff at all.
    pub fn similarity(
        &self,
        at: EntityRef,
        other: &Self,
        other_at: EntityRef,
        scope: Scope,
        floor: f32,
    ) -> Option<f32> {
        let (left, right) = (self.print(at, scope), other.print(other_at, scope));
        let total = left.sequence.len() + right.sequence.len();
        if total == 0 {
            return (floor <= 1.0).then_some(1.0);
        }
        let bound = 2.0 * shared(&left.sorted, &right.sorted) as f32 / total as f32;
        if bound < floor {
            return None;
        }
        let ops = capture_diff_slices(Algorithm::Myers, &left.sequence, &right.sequence);
        let ratio = diff_ratio(&ops, left.sequence.len(), right.sequence.len());
        (ratio >= floor).then_some(ratio)
    }
}

fn shared(left: &[u64], right: &[u64]) -> usize {
    let (mut l, mut r, mut count) = (0, 0, 0);
    while l < left.len() && r < right.len() {
        match left[l].cmp(&right[r]) {
            std::cmp::Ordering::Less => l += 1,
            std::cmp::Ordering::Greater => r += 1,
            std::cmp::Ordering::Equal => {
                count += 1;
                l += 1;
                r += 1;
            }
        }
    }
    count
}
