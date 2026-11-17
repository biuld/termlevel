//! Incremental history layout. Slots stay stable; only a bounded set is drawn/transferred.
use super::*;
use std::collections::{BTreeSet, HashMap, HashSet};

pub(super) const VISIBLE_LANES: u32 = 24;
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct GraphEdge {
    pub from: u32,
    pub to: u32,
}
#[derive(Clone, Debug, PartialEq, JsObject)]
pub struct GraphRow {
    pub incoming: bool,
    pub lane: u32,
    pub through: Vec<u32>,
    pub edges: Vec<GraphEdge>,
}
#[derive(Clone, Default)]
struct Layout {
    targets: HashMap<String, u32>,
    slots: Vec<Option<String>>,
    free: BTreeSet<u32>,
    max_lanes: u32,
    clipped: bool,
}
impl Layout {
    fn insert(&mut self, oid: &str, preferred: Option<u32>) -> u32 {
        if let Some(lane) = self.targets.get(oid) {
            return *lane;
        }
        let lane = preferred
            .filter(|lane| self.free.contains(lane))
            .or_else(|| self.free.first().copied())
            .unwrap_or(self.slots.len() as u32);
        self.free.remove(&lane);
        if lane as usize == self.slots.len() {
            self.slots.push(None);
        }
        self.slots[lane as usize] = Some(oid.to_string());
        self.targets.insert(oid.to_string(), lane);
        self.max_lanes = self.max_lanes.max((lane + 1).min(VISIBLE_LANES + 1));
        self.clipped |= lane >= VISIBLE_LANES;
        lane
    }
    fn row(&mut self, commit: &RepoCommit) -> GraphRow {
        let incoming = self.targets.contains_key(&commit.oid);
        let lane = self.insert(&commit.oid, None);
        let through = self
            .slots
            .iter()
            .take(VISIBLE_LANES as usize)
            .enumerate()
            .filter(|(index, oid)| *index as u32 != lane && oid.is_some())
            .map(|(index, _)| index as u32)
            .collect();
        self.targets.remove(&commit.oid);
        self.slots[lane as usize] = None;
        self.free.insert(lane);
        let edges = commit
            .parents
            .iter()
            .enumerate()
            .map(|(index, parent)| {
                let target = self.insert(parent, (index == 0).then_some(lane));
                GraphEdge {
                    from: lane.min(VISIBLE_LANES),
                    to: target.min(VISIBLE_LANES),
                }
            })
            .collect();
        GraphRow {
            incoming,
            lane: lane.min(VISIBLE_LANES),
            through,
            edges,
        }
    }
}

#[derive(Default)]
pub(super) struct HistoryGraph {
    root: String,
    scope: String,
    next_skip: u32,
    layout: Layout,
    tips: Option<Vec<String>>,
    pub loaded: HashSet<String>,
}
impl HistoryGraph {
    pub fn matches(&self, root: &str, scope: &str, count: u32) -> bool {
        self.root == root && self.scope == scope && self.next_skip == count
    }
    pub fn pin(&mut self, tips: Option<Vec<String>>) {
        self.tips = tips;
    }
    pub fn tips(&self) -> Option<&[String]> {
        self.tips.as_deref()
    }
    pub fn apply(
        &mut self,
        root: &str,
        scope: &str,
        skip: u32,
        log: &mut RepoLog,
    ) -> Result<(), String> {
        if skip == 0 {
            *self = Self {
                tips: self.tips.take(),
                root: root.into(),
                scope: scope.into(),
                ..Default::default()
            };
        }
        if root != self.root || scope != self.scope || skip != self.next_skip {
            return Err(
                "History changed while loading. Refresh before loading more commits.".into(),
            );
        }
        let mut layout = self.layout.clone();
        let mut rows = Vec::with_capacity(log.commits.len());
        for commit in &log.commits {
            requests::check()?;
            rows.push(layout.row(commit));
        }
        requests::check()?;
        self.loaded
            .extend(log.commits.iter().map(|commit| commit.oid.clone()));
        self.next_skip += log.commits.len() as u32;
        log.graph = rows;
        log.graph_max_lanes = layout.max_lanes;
        log.graph_clipped = layout.clipped;
        self.layout = layout;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn commit(oid: &str, parents: &[&str]) -> RepoCommit {
        RepoCommit {
            parents: parents.iter().map(|s| s.to_string()).collect(),
            ..RepoCommit::empty(oid.into())
        }
    }
    #[test]
    fn first_parent_and_shared_ancestors_keep_their_slots() {
        let mut layout = Layout::default();
        assert_eq!(
            layout.row(&commit("m", &["b", "c"])).edges,
            vec![GraphEdge { from: 0, to: 0 }, GraphEdge { from: 0, to: 1 }]
        );
        assert_eq!(layout.row(&commit("c", &["a"])).lane, 1);
        let b = layout.row(&commit("b", &["a"]));
        assert_eq!(b.lane, 0);
        assert_eq!(b.through, vec![1]);
        assert!(b.incoming);
        assert_eq!(b.edges, vec![GraphEdge { from: 0, to: 1 }]);
        assert_eq!(layout.row(&commit("a", &[])).lane, 1);
        assert!(layout.targets.is_empty());
    }
    #[test]
    fn broad_history_has_bounded_row_payload_and_reuses_slots() {
        let mut layout = Layout::default();
        for i in 0..10_000 {
            let row = layout.row(&commit(&format!("tip-{i}"), &[&format!("parent-{i}")]));
            assert!(!row.incoming);
            assert!(row.through.len() <= VISIBLE_LANES as usize);
            assert!(row.lane <= VISIBLE_LANES);
        }
        assert!(layout.clipped);
        for i in 0..10_000 {
            layout.row(&commit(&format!("parent-{i}"), &[]));
        }
        assert_eq!(layout.row(&commit("new", &["next"])).lane, 0);
    }
    #[test]
    fn paged_history_matches_one_pass_and_rejects_stale_offsets() {
        let commits = vec![
            commit("m", &["b", "c"]),
            commit("c", &["a"]),
            commit("b", &["a"]),
            commit("a", &[]),
        ];
        let mut paged = HistoryGraph::default();
        let mut one = RepoLog {
            commits: commits.clone(),
            ..Default::default()
        };
        HistoryGraph::default()
            .apply("repo", "", 0, &mut one)
            .unwrap();
        let mut first = RepoLog {
            commits: commits[..2].to_vec(),
            ..Default::default()
        };
        paged.apply("repo", "", 0, &mut first).unwrap();
        let mut second = RepoLog {
            commits: commits[2..].to_vec(),
            ..Default::default()
        };
        paged.apply("repo", "", 2, &mut second).unwrap();
        assert_eq!([first.graph, second.graph.clone()].concat(), one.graph);
        assert!(paged.apply("repo", "", 2, &mut second).is_err());
    }
}
