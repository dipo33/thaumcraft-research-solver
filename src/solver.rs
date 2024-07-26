use std::collections::{HashMap, VecDeque};

use crate::{
    aspect::{Aspect, AspectInventory},
    graph::Graph,
};

struct SolverState {
    node: Aspect,
    price: u32,
    distance: u8,
    path: Vec<Aspect>,
}

pub struct Solver {
    aspect_graph: Graph<Aspect>,
    aspect_inventory: AspectInventory,
}

impl Solver {
    pub fn new(aspect_inventory: AspectInventory) -> Self {
        Solver {
            aspect_graph: Solver::build_aspect_graph(),
            aspect_inventory,
        }
    }

    pub fn find_paths(&self, start: Aspect, end: Aspect, distance: u8, max_distance_increase: u8) -> HashMap<u8, AspectPaths> {
        let mut best_paths = HashMap::new();
        for increase in 0..max_distance_increase {
            let paths = self.find_paths_with_length(start, end, distance + increase);
            best_paths.insert(increase, paths);
        }

        best_paths
    }

    fn find_paths_with_length(&self, start: Aspect, end: Aspect, desired_distance: u8) -> AspectPaths {
        let mut queue = VecDeque::new();

        let mut lowest_price = u32::MAX;
        let mut paths = Vec::new();
        queue.push_back(SolverState {
            node: start,
            price: 0,
            distance: 1,
            path: vec![start],
        });

        while let Some(SolverState { node, price, distance, path }) = queue.pop_front() {
            if distance == desired_distance {
                if node == end && price <= lowest_price {
                    if price < lowest_price {
                        paths.clear();
                        lowest_price = price;
                    }
                    paths.push(path.clone());
                }
                continue;
            }

            for neighbor in self.aspect_graph.neighbours_cloned_iter(node) {
                let neighbor_price: u32 = self.aspect_inventory.price_of(neighbor).into();
                let new_price = neighbor_price + price;
                if new_price <= lowest_price {
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    queue.push_back(SolverState {
                        node: neighbor,
                        price: new_price,
                        distance: distance + 1,
                        path: new_path,
                    });
                }
            }
        }

        AspectPaths::new(paths, lowest_price)
    }

    fn add_composite_edges(graph: &mut Graph<Aspect>, composite: Aspect, primal_a: Aspect, primal_b: Aspect) {
        graph.add_indirectional_edge(composite, primal_a);
        graph.add_indirectional_edge(composite, primal_b);
    }

    fn build_aspect_graph() -> Graph<Aspect> {
        let mut graph = Graph::new();
        Solver::add_composite_edges(&mut graph, Aspect::Alienis, Aspect::Vacuos, Aspect::Tenebrae);
        Solver::add_composite_edges(&mut graph, Aspect::Arbor, Aspect::Aer, Aspect::Herba);
        Solver::add_composite_edges(&mut graph, Aspect::Auram, Aspect::Praecantatio, Aspect::Aer);
        Solver::add_composite_edges(&mut graph, Aspect::Bestia, Aspect::Motus, Aspect::Victus);
        Solver::add_composite_edges(&mut graph, Aspect::Caelum, Aspect::Vitreus, Aspect::Metallum);
        Solver::add_composite_edges(&mut graph, Aspect::Cognitio, Aspect::Ignis, Aspect::Spiritus);
        Solver::add_composite_edges(&mut graph, Aspect::Corpus, Aspect::Mortuus, Aspect::Bestia);
        Solver::add_composite_edges(&mut graph, Aspect::Desidia, Aspect::Vinculum, Aspect::Spiritus);
        Solver::add_composite_edges(&mut graph, Aspect::Electrum, Aspect::Potentia, Aspect::Machina);
        Solver::add_composite_edges(&mut graph, Aspect::Exanimis, Aspect::Motus, Aspect::Mortuus);
        Solver::add_composite_edges(&mut graph, Aspect::Fabrico, Aspect::Humanus, Aspect::Instrumentum);
        Solver::add_composite_edges(&mut graph, Aspect::Fames, Aspect::Victus, Aspect::Vacuos);
        Solver::add_composite_edges(&mut graph, Aspect::Gelum, Aspect::Ignis, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Gula, Aspect::Fames, Aspect::Vacuos);
        Solver::add_composite_edges(&mut graph, Aspect::Herba, Aspect::Victus, Aspect::Terra);
        Solver::add_composite_edges(&mut graph, Aspect::Humanus, Aspect::Bestia, Aspect::Cognitio);
        Solver::add_composite_edges(&mut graph, Aspect::Infernus, Aspect::Ignis, Aspect::Praecantatio);
        Solver::add_composite_edges(&mut graph, Aspect::Instrumentum, Aspect::Humanus, Aspect::Ordo);
        Solver::add_composite_edges(&mut graph, Aspect::Invidia, Aspect::Sensus, Aspect::Fames);
        Solver::add_composite_edges(&mut graph, Aspect::Ira, Aspect::Telum, Aspect::Ignis);
        Solver::add_composite_edges(&mut graph, Aspect::Iter, Aspect::Motus, Aspect::Terra);
        Solver::add_composite_edges(&mut graph, Aspect::Limus, Aspect::Victus, Aspect::Aqua);
        Solver::add_composite_edges(&mut graph, Aspect::Lucrum, Aspect::Humanus, Aspect::Fames);
        Solver::add_composite_edges(&mut graph, Aspect::Lux, Aspect::Aer, Aspect::Ignis);
        Solver::add_composite_edges(&mut graph, Aspect::Luxuria, Aspect::Corpus, Aspect::Fames);
        Solver::add_composite_edges(&mut graph, Aspect::Machina, Aspect::Motus, Aspect::Instrumentum);
        Solver::add_composite_edges(&mut graph, Aspect::Magneto, Aspect::Metallum, Aspect::Iter);
        Solver::add_composite_edges(&mut graph, Aspect::Messis, Aspect::Herba, Aspect::Humanus);
        Solver::add_composite_edges(&mut graph, Aspect::Metallum, Aspect::Terra, Aspect::Vitreus);
        Solver::add_composite_edges(&mut graph, Aspect::Meto, Aspect::Messis, Aspect::Instrumentum);
        Solver::add_composite_edges(&mut graph, Aspect::Mortuus, Aspect::Victus, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Motus, Aspect::Aer, Aspect::Ordo);
        Solver::add_composite_edges(&mut graph, Aspect::Nebrisum, Aspect::Perfodio, Aspect::Lucrum);
        Solver::add_composite_edges(&mut graph, Aspect::Pannus, Aspect::Instrumentum, Aspect::Bestia);
        Solver::add_composite_edges(&mut graph, Aspect::Perfodio, Aspect::Humanus, Aspect::Terra);
        Solver::add_composite_edges(&mut graph, Aspect::Permutatio, Aspect::Perditio, Aspect::Ordo);
        Solver::add_composite_edges(&mut graph, Aspect::Potentia, Aspect::Ordo, Aspect::Ignis);
        Solver::add_composite_edges(&mut graph, Aspect::Praecantatio, Aspect::Vacuos, Aspect::Potentia);
        Solver::add_composite_edges(&mut graph, Aspect::Radio, Aspect::Lux, Aspect::Potentia);
        Solver::add_composite_edges(&mut graph, Aspect::Sano, Aspect::Victus, Aspect::Ordo);
        Solver::add_composite_edges(&mut graph, Aspect::Sensus, Aspect::Aer, Aspect::Spiritus);
        Solver::add_composite_edges(&mut graph, Aspect::Spiritus, Aspect::Victus, Aspect::Mortuus);
        Solver::add_composite_edges(&mut graph, Aspect::Strontio, Aspect::Cognitio, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Superbia, Aspect::Volatus, Aspect::Vacuos);
        Solver::add_composite_edges(&mut graph, Aspect::Tabernus, Aspect::Tutamen, Aspect::Iter);
        Solver::add_composite_edges(&mut graph, Aspect::Telum, Aspect::Instrumentum, Aspect::Ignis);
        Solver::add_composite_edges(&mut graph, Aspect::Tempestas, Aspect::Aer, Aspect::Aqua);
        Solver::add_composite_edges(&mut graph, Aspect::Tempus, Aspect::Vacuos, Aspect::Ordo);
        Solver::add_composite_edges(&mut graph, Aspect::Tenebrae, Aspect::Vacuos, Aspect::Lux);
        Solver::add_composite_edges(&mut graph, Aspect::Tutamen, Aspect::Instrumentum, Aspect::Terra);
        Solver::add_composite_edges(&mut graph, Aspect::Vacuos, Aspect::Aer, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Venenum, Aspect::Aqua, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Victus, Aspect::Aqua, Aspect::Terra);
        Solver::add_composite_edges(&mut graph, Aspect::Vinculum, Aspect::Motus, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Vitium, Aspect::Praecantatio, Aspect::Perditio);
        Solver::add_composite_edges(&mut graph, Aspect::Vitreus, Aspect::Terra, Aspect::Ordo);
        Solver::add_composite_edges(&mut graph, Aspect::Volatus, Aspect::Aer, Aspect::Motus);

        graph
    }
}

pub type AspectPath = Vec<Aspect>;

pub struct AspectPaths {
    pub paths: Vec<Vec<Aspect>>,
    pub price: u32,
}

impl AspectPaths {
    pub fn new(paths: Vec<AspectPath>, price: u32) -> Self {
        Self { paths, price }
    }
}
