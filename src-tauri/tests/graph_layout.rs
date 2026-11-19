use gix::ObjectId;
use graft_lib::git::graph::LaneLayout;
use graft_lib::git::walk::{Parent, WalkedCommit};
use graft_lib::git::wire::Edge;

fn oid(value: u32) -> ObjectId {
    ObjectId::from_hex(format!("{value:040x}").as_bytes()).expect("valid hex")
}

fn commit(id: u32, parents: &[u32]) -> WalkedCommit {
    WalkedCommit {
        id: oid(id),
        parents: parents.iter().map(|parent| Parent { id: oid(*parent), emitted_row: None }).collect(),
        time: 0,
    }
}

/// Places commits in order and returns each row's lane plus the edges ending on it.
fn layout(history: &[WalkedCommit]) -> (Vec<u16>, Vec<Vec<Edge>>, LaneLayout) {
    let mut layout = LaneLayout::default();
    let mut lanes = Vec::new();
    let mut buckets = Vec::new();
    for (row, commit) in history.iter().enumerate() {
        let mut edges = Vec::new();
        lanes.push(layout.place(row as u32, commit, &mut edges).lane);
        buckets.push(edges);
    }
    (lanes, buckets, layout)
}

#[test]
fn linear_history_stays_on_one_rail() {
    let (lanes, buckets, layout) = layout(&[commit(3, &[2]), commit(2, &[1]), commit(1, &[])]);

    assert_eq!(lanes, [0, 0, 0]);
    assert_eq!(layout.lane_count(), 1);
    assert!(layout.open_edges().is_empty());
    assert_eq!(buckets[1], [Edge { upper_row: 0, lane: 0, color: 0 }]);
    assert_eq!(buckets[2], [Edge { upper_row: 1, lane: 0, color: 0 }]);
}

#[test]
fn feature_branch_keeps_its_rail_until_the_fork_point() {
    // 5 (main) and 4 (feature) both descend from 3.
    let (lanes, buckets, layout) = layout(&[commit(5, &[3]), commit(4, &[3]), commit(3, &[2]), commit(2, &[])]);

    assert_eq!(lanes, [0, 1, 0, 0]);
    assert_eq!(buckets[2].len(), 2, "both children terminate on the fork point");
    assert!(buckets[2].contains(&Edge { upper_row: 0, lane: 0, color: 0 }));
    assert!(buckets[2].iter().any(|edge| edge.upper_row == 1 && edge.lane == 1));
    assert!(layout.open_edges().is_empty());
}

#[test]
fn merge_edge_reuses_the_lane_already_heading_for_its_parent() {
    // 6 merges 4 into main; 5 is a newer commit on the same feature branch as 4.
    let history = [commit(7, &[6]), commit(5, &[4]), commit(6, &[3, 4]), commit(4, &[3]), commit(3, &[])];
    let (lanes, buckets, layout) = layout(&history);

    assert_eq!(lanes, [0, 1, 0, 1, 0]);
    assert_eq!(layout.lane_count(), 2, "the merge edge must not open a third lane");
    let into_feature: Vec<u32> = buckets[3].iter().map(|edge| edge.upper_row).collect();
    assert_eq!(into_feature, [1, 2], "feature tip and merge commit both reach commit 4 through lane 1");
}

#[test]
fn unreached_parents_stay_open_for_the_next_chunk() {
    let (_, _, layout) = layout(&[commit(9, &[8, 7])]);

    let open = layout.open_edges();
    assert_eq!(open.len(), 2);
    assert!(open.iter().all(|edge| edge.upper_row == 0));
    assert_ne!(open[0].lane, open[1].lane);
}

#[test]
fn skewed_parent_is_linked_upwards_without_leaking_a_lane() {
    let mut child = commit(2, &[1]);
    child.parents[0].emitted_row = Some(0);
    let (_, buckets, layout) = layout(&[commit(1, &[]), child]);

    assert_eq!(buckets[1], [Edge { upper_row: 0, lane: 0, color: 1 }]);
    assert!(layout.open_edges().is_empty());
}
