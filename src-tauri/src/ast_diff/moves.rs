use std::collections::{HashMap, HashSet};

use super::matcher::EntityRef;

type Scope = (usize, Option<usize>);

/// A pair has moved when it changed scope, or when it left the longest run of siblings
/// that kept their relative order. Members of a relocated container are not moved themselves.
pub fn moved_pairs(
    forward: &HashMap<EntityRef, EntityRef>,
    old_parent: impl Fn(EntityRef) -> Option<usize>,
    new_parent: impl Fn(EntityRef) -> Option<usize>,
) -> HashSet<EntityRef> {
    let mut moved = HashSet::new();
    let mut groups: HashMap<Scope, Vec<(usize, EntityRef)>> = HashMap::new();
    for (&old, &new) in forward {
        let same_scope = match (old_parent(old), new_parent(new)) {
            (None, None) => old.file == new.file,
            (Some(left), Some(right)) => {
                forward.get(&EntityRef { file: old.file, index: left })
                    == Some(&EntityRef { file: new.file, index: right })
            }
            _ => false,
        };
        if same_scope {
            groups.entry((new.file, new_parent(new))).or_default().push((new.index, old));
        } else {
            moved.insert(old);
        }
    }
    for siblings in groups.values_mut() {
        siblings.sort_unstable();
        let order: Vec<usize> = siblings.iter().map(|(_, old)| old.index).collect();
        let kept = in_order(&order);
        moved.extend(siblings.iter().zip(kept).filter(|(_, kept)| !kept).map(|((_, old), _)| *old));
    }
    moved
}

/// Marks the members of one longest increasing subsequence of `sequence`.
pub fn in_order(sequence: &[usize]) -> Vec<bool> {
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![usize::MAX; sequence.len()];
    for (at, value) in sequence.iter().enumerate() {
        let position = tails.partition_point(|tail| sequence[*tail] < *value);
        if position > 0 {
            previous[at] = tails[position - 1];
        }
        if position == tails.len() {
            tails.push(at);
        } else {
            tails[position] = at;
        }
    }

    let mut kept = vec![false; sequence.len()];
    let mut cursor = tails.last().copied();
    while let Some(at) = cursor {
        kept[at] = true;
        cursor = Some(previous[at]).filter(|previous| *previous != usize::MAX);
    }
    kept
}
