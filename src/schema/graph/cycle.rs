//! Detects cycles over the remaining nodes after topological sort.

use indexmap::IndexSet;

use super::{
    super::{SchemaName, SchemaNameRef},
    adjacency::{DenseIndex, SchemaAdjacency},
};

/// Finds every Schema participating in an `extends` cycle.
///
/// The detector skips nodes already resolved by topological sort, because
/// resolved nodes are provably acyclic. Over the remaining nodes, it runs in
/// `O(V + E)` time with two key choices:
///
/// - It uses an iterative strongly connected components search with an explicit
///   work stack instead of recursion.
/// - It walks `extends` backward through [`SchemaAdjacency::parents_of`] rather
///   than using the graph's forward adjacency.
pub(super) struct CycleDetector<'a, 'b> {
    adjacency: &'b SchemaAdjacency<'a>,
    /// Holds Schemas that topological sort already resolved. Only remaining
    /// nodes are explored.
    visited: &'b IndexSet<SchemaNameRef<'a>>,
    search: CycleSearchState,
    cyclic: Vec<SchemaName>,
}

impl<'a> CycleDetector<'a, '_> {
    pub(super) fn new<'b>(
        adjacency: &'b SchemaAdjacency<'a>,
        visited: &'b IndexSet<SchemaNameRef<'a>>,
    ) -> CycleDetector<'a, 'b> {
        CycleDetector {
            adjacency,
            visited,
            search: CycleSearchState::new(adjacency.node_count()),
            cyclic: Vec::new(),
        }
    }

    /// Returns every remaining Schema in a cycle in declaration order, or an
    /// empty vector if `visited` covers every node.
    pub(super) fn find(mut self) -> Vec<SchemaName> {
        if self.visited.len() == self.adjacency.node_count() {
            return Vec::new();
        }
        for start in 0..DenseIndex::saturating_u32(self.adjacency.node_count())
        {
            let start = DenseIndex::from(start);
            if self.is_kahn_visited(start) || self.search.is_discovered(start) {
                continue;
            }
            self.explore_from(start);
        }
        self.cyclic
    }

    /// Returns whether `node` was resolved by topological sort.
    fn is_kahn_visited(&self, node: DenseIndex) -> bool {
        self.adjacency
            .name_of(node)
            .is_some_and(|name| self.visited.contains(&name))
    }

    /// Runs an iterative depth-first search rooted at `start`, extracting every
    /// nontrivial strongly connected component (or self-loop) into
    /// `self.cyclic`.
    fn explore_from(&mut self, start: DenseIndex) {
        let mut work_stack: Vec<(DenseIndex, usize)> = vec![(start, 0)];
        while let Some(&(node, scan_pos)) = work_stack.last() {
            if scan_pos == 0 {
                self.search.discover(node);
            }
            match self.next_unvisited_parent(node, scan_pos) {
                Some((parent, next_pos)) => {
                    self.advance(&mut work_stack, node, parent, next_pos);
                }
                None => self.finish_node(&mut work_stack, node),
            }
        }
    }

    /// Scans `node`'s raw `extends` list from `from`, skipping unknown or
    /// already-resolved targets.
    ///
    /// Returns the first remaining known parent and the position to resume
    /// from. Runs in `O(V + E)` total with the monotonically advancing `from`.
    fn next_unvisited_parent(
        &self,
        node: DenseIndex,
        from: usize,
    ) -> Option<(DenseIndex, usize)> {
        let name = self.adjacency.name_of(node)?;
        let parents = self.adjacency.parents_of(name);
        let mut pos = from;
        while let Some(parent) = parents.get(pos) {
            pos = pos.saturating_add(1);
            if self.visited.contains(parent.as_str()) {
                continue;
            }
            if let Some(target) = self.adjacency.index_of(parent.as_ref()) {
                return Some((target, pos));
            }
        }
        None
    }

    /// Returns whether `node`'s raw `extends` list contains a valid, unresolved
    /// self-reference.
    fn has_self_extend(&self, node: DenseIndex) -> bool {
        let Some(name) = self.adjacency.name_of(node) else {
            return false;
        };
        self.adjacency.parents_of(name).iter().any(|parent| {
            !self.visited.contains(parent.as_str())
                && self.adjacency.index_of(parent.as_ref()) == Some(node)
        })
    }

    /// Advances `node`'s frame to `resume_at`, then visits `parent`.
    ///
    /// If `parent` is undiscovered, pushes it onto `work_stack`. If `parent` is
    /// a live back edge, folds its discovery order into `node`'s minimum
    /// reachable rank.
    fn advance(
        &mut self,
        work_stack: &mut Vec<(DenseIndex, usize)>,
        node: DenseIndex,
        parent: DenseIndex,
        resume_at: usize,
    ) {
        if let Some(frame) = work_stack.last_mut() {
            frame.1 = resume_at;
        }
        if !self.search.is_discovered(parent) {
            work_stack.push((parent, 0));
            return;
        }
        if !self.search.is_on_stack(parent) {
            return;
        }
        if let Some(parent_order) = self.search.discovery_order_of(parent) {
            self.search.lower_reachable(node, parent_order);
        }
    }

    /// Completes `node` by recording its strongly connected component into
    /// `self.cyclic` in insertion order and propagating its minimum reachable
    /// rank to its caller.
    fn finish_node(
        &mut self,
        work_stack: &mut Vec<(DenseIndex, usize)>,
        node: DenseIndex,
    ) {
        self.record_cyclic_component(node);
        work_stack.pop();
        let Some(&(parent, _)) = work_stack.last() else {
            return;
        };
        if let Some(node_lowest) = self.search.lowest_reachable_of(node) {
            self.search.lower_reachable(parent, node_lowest);
        }
    }

    /// Records a component rooted at `node` into `self.cyclic` when nontrivial
    /// or self-looping.
    ///
    /// Members are sorted into declaration order before extending
    /// `self.cyclic`.
    fn record_cyclic_component(&mut self, node: DenseIndex) {
        let node_order = self.search.discovery_order_of(node);
        if node_order.is_none()
            || node_order != self.search.lowest_reachable_of(node)
        {
            return;
        }
        let mut component = self.search.pop_component(node);
        let is_cyclic = component.len() > 1 || self.has_self_extend(node);
        if !is_cyclic {
            return;
        }
        // Insertion order, not stack pop order: matches
        // `SchemaBuilder::compute_hierarchy`'s error-reporting convention.
        component.sort_by_key(|idx| idx.index());
        let adjacency = &self.adjacency;
        self.cyclic.extend(
            component
                .iter()
                .filter_map(|&idx| adjacency.name_of(idx))
                .map(SchemaName::from),
        );
    }
}

/// Tracks per-node bookkeeping for [`CycleDetector`]'s component search.
///
/// Sized to the full node count. Entries for nodes that are never reached stay
/// at their initial value.
struct CycleSearchState {
    /// Stores discovery order, assigned once per node on first visit.
    discovery_order: Vec<Option<u32>>,
    /// Stores the lowest discovery order reachable from each node through tree
    /// or back edges.
    lowest_reachable: Vec<u32>,
    /// Tracks whether each node is currently on `component_stack`.
    on_stack: Vec<bool>,
    /// Accumulates component members in discovery order.
    component_stack: Vec<DenseIndex>,
    /// Holds the next discovery order to assign.
    next_discovery_order: u32,
}

impl CycleSearchState {
    fn new(node_count: usize) -> Self {
        Self {
            discovery_order: vec![None; node_count],
            lowest_reachable: vec![0; node_count],
            on_stack: vec![false; node_count],
            component_stack: Vec::new(),
            next_discovery_order: 0,
        }
    }

    /// Returns `true` if `node` has a discovery order.
    fn is_discovered(&self, node: DenseIndex) -> bool {
        self.discovery_order.get(node.index()).copied().flatten().is_some()
    }

    /// Returns `node`'s discovery order, or `None` if undiscovered.
    fn discovery_order_of(&self, node: DenseIndex) -> Option<u32> {
        self.discovery_order.get(node.index()).copied().flatten()
    }

    /// Returns `node`'s current lowest-reachable rank, or `None` if
    /// undiscovered.
    fn lowest_reachable_of(&self, node: DenseIndex) -> Option<u32> {
        self.lowest_reachable.get(node.index()).copied()
    }

    /// Returns whether `node` is on the component stack (not the traversal
    /// work-stack).
    fn is_on_stack(&self, node: DenseIndex) -> bool {
        self.on_stack.get(node.index()).copied().unwrap_or(false)
    }

    /// Assigns `node` its discovery order and pushes it onto `component_stack`.
    fn discover(&mut self, node: DenseIndex) {
        let rank = self.next_discovery_order;
        self.next_discovery_order = self.next_discovery_order.saturating_add(1);
        if let Some(slot) = self.discovery_order.get_mut(node.index()) {
            *slot = Some(rank);
        }
        if let Some(slot) = self.lowest_reachable.get_mut(node.index()) {
            *slot = rank;
        }
        self.component_stack.push(node);
        if let Some(slot) = self.on_stack.get_mut(node.index()) {
            *slot = true;
        }
    }

    /// Folds `candidate` into `node`'s minimum reachable rank, keeping the
    /// smaller value.
    fn lower_reachable(&mut self, node: DenseIndex, candidate: u32) {
        if let Some(slot) = self.lowest_reachable.get_mut(node.index()) {
            *slot = (*slot).min(candidate);
        }
    }

    /// Pops the component rooted at `root` off `component_stack`, most
    /// recently discovered first.
    fn pop_component(&mut self, root: DenseIndex) -> Vec<DenseIndex> {
        let mut component = Vec::new();
        while let Some(popped) = self.component_stack.pop() {
            if let Some(slot) = self.on_stack.get_mut(popped.index()) {
                *slot = false;
            }
            let is_root = popped == root;
            component.push(popped);
            if is_root {
                break;
            }
        }
        component
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::super::{
        super::{GLOBAL_SCHEMA_NAME, SchemaName, SchemaNameRef},
        builder::SchemaGraphBuilder,
    };

    fn schema(extends: &[&str]) -> super::super::super::RawSchema {
        super::super::super::RawSchema {
            extends: extends.iter().map(|&s| SchemaName::for_test(s)).collect(),
            ..super::super::super::RawSchema::default()
        }
    }

    #[test]
    fn rejects_a_direct_two_node_cycle() {
        let mut raw = IndexMap::new();
        raw.insert(SchemaName::for_test("a"), schema(&["b"]));
        raw.insert(SchemaName::for_test("b"), schema(&["a"]));
        let (builder, _warnings) = SchemaGraphBuilder::new(
            &raw,
            SchemaNameRef::from(GLOBAL_SCHEMA_NAME),
        );

        let err = builder.build().expect_err("cycle rejected");
        assert_eq!(err, vec![
            SchemaName::for_test("a"),
            SchemaName::for_test("b")
        ]);
    }

    #[test]
    fn excludes_a_schema_that_only_extends_into_the_cycle() {
        let mut raw = IndexMap::new();
        raw.insert(SchemaName::for_test("a"), schema(&["b"]));
        raw.insert(SchemaName::for_test("b"), schema(&["a"]));
        raw.insert(SchemaName::for_test("c"), schema(&["a"]));
        let (builder, _warnings) = SchemaGraphBuilder::new(
            &raw,
            SchemaNameRef::from(GLOBAL_SCHEMA_NAME),
        );

        let err = builder.build().expect_err("cycle rejected");
        assert_eq!(err, vec![
            SchemaName::for_test("a"),
            SchemaName::for_test("b")
        ]);
    }

    #[test]
    fn rejects_a_three_node_cycle_in_declaration_order() {
        let mut raw = IndexMap::new();
        raw.insert(SchemaName::for_test("a"), schema(&["b"]));
        raw.insert(SchemaName::for_test("b"), schema(&["c"]));
        raw.insert(SchemaName::for_test("c"), schema(&["a"]));
        let (builder, _warnings) = SchemaGraphBuilder::new(
            &raw,
            SchemaNameRef::from(GLOBAL_SCHEMA_NAME),
        );

        let err = builder.build().expect_err("cycle rejected");
        assert_eq!(err, vec![
            SchemaName::for_test("a"),
            SchemaName::for_test("b"),
            SchemaName::for_test("c")
        ]);
    }

    #[test]
    fn rejects_a_self_loop() {
        let mut raw = IndexMap::new();
        raw.insert(SchemaName::for_test("a"), schema(&["a"]));
        let (builder, _warnings) = SchemaGraphBuilder::new(
            &raw,
            SchemaNameRef::from(GLOBAL_SCHEMA_NAME),
        );

        let err = builder.build().expect_err("self-loop rejected");
        assert_eq!(err, vec![SchemaName::for_test("a")]);
    }

    #[test]
    fn returns_empty_when_no_cycle_exists() {
        let mut raw = IndexMap::new();
        raw.insert(SchemaName::for_test("root"), schema(&[]));
        raw.insert(SchemaName::for_test("child"), schema(&["root"]));
        let (builder, _warnings) = SchemaGraphBuilder::new(
            &raw,
            SchemaNameRef::from(GLOBAL_SCHEMA_NAME),
        );

        let graph = builder.build().expect("acyclic graph resolves");
        assert!(graph.topological_order().next().is_some());
    }

    #[test]
    fn rejects_multiple_independent_cycles() {
        let mut raw = IndexMap::new();
        raw.insert(SchemaName::for_test("a"), schema(&["b"]));
        raw.insert(SchemaName::for_test("b"), schema(&["a"]));
        raw.insert(SchemaName::for_test("x"), schema(&["y"]));
        raw.insert(SchemaName::for_test("y"), schema(&["x"]));
        let (builder, _warnings) = SchemaGraphBuilder::new(
            &raw,
            SchemaNameRef::from(GLOBAL_SCHEMA_NAME),
        );

        let err = builder.build().expect_err("cycles rejected");
        assert_eq!(err.len(), 4, "both cycles must be reported");
        assert!(err.contains(&SchemaName::for_test("a")));
        assert!(err.contains(&SchemaName::for_test("b")));
        assert!(err.contains(&SchemaName::for_test("x")));
        assert!(err.contains(&SchemaName::for_test("y")));
    }
}
