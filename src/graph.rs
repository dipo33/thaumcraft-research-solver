use std::collections::hash_set::Iter;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::Cloned;

#[derive(Debug, Clone)]
pub struct Graph<T> {
    edges: HashMap<T, HashSet<T>>,
    empty_set: HashSet<T>,
}

impl<T: Eq + Hash + Copy> Graph<T> {
    pub fn new() -> Self {
        Graph {
            edges: HashMap::new(),
            empty_set: HashSet::new(),
        }
    }

    pub fn add_edge(&mut self, from: T, to: T) {
        self.edges.entry(from).or_default().insert(to);
    }

    pub fn add_indirectional_edge(&mut self, node_a: T, node_b: T) {
        self.add_edge(node_a, node_b);
        self.add_edge(node_b, node_a);
    }

    #[allow(dead_code)]
    pub fn neighbours(&self, node: T) -> &HashSet<T> {
        self.edges.get(&node).unwrap_or(&self.empty_set)
    }

    pub fn neighbours_cloned_iter(&self, node: T) -> Cloned<Iter<'_, T>> {
        self.edges.get(&node).unwrap_or(&self.empty_set).iter().cloned()
    }
}
