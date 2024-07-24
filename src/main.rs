use clap::Parser;
use ftp::FtpStream;
use nbt::{Blob, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::Read,
};
use strsim::normalized_levenshtein;

/// Thaumcraft Research Solver using weighted paths with your actual aspect inventory
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Actual Minecraft username
    #[arg(short, long)]
    username: String,

    /// Minecraft server FTP address
    #[arg(short = 'a', long)]
    ftp_address: String,

    /// Minecraft server FTP username
    #[arg(short, long)]
    ftp_username: String,

    /// Minecraft server FTP password
    #[arg(short = 'p', long)]
    ftp_password: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum Aspect {
    Aer,
    Alienis,
    Aqua,
    Arbor,
    Auram,
    Bestia,
    Caelum,
    Cognitio,
    Corpus,
    Desidia,
    Electrum,
    Exanimis,
    Fabrico,
    Fames,
    Gelum,
    Gula,
    Herba,
    Humanus,
    Ignis,
    Infernus,
    Instrumentum,
    Invidia,
    Ira,
    Iter,
    Limus,
    Lucrum,
    Lux,
    Luxuria,
    Machina,
    Magneto,
    Messis,
    Metallum,
    Meto,
    Mortuus,
    Motus,
    Nebrisum,
    Ordo,
    Pannus,
    Perditio,
    Perfodio,
    Permutatio,
    Potentia,
    Praecantatio,
    Radio,
    Sano,
    Sensus,
    Spiritus,
    Strontio,
    Superbia,
    Tabernus,
    Telum,
    Tempestas,
    Tempus,
    Tenebrae,
    Terra,
    Tutamen,
    Vacuos,
    Venenum,
    Victus,
    Vinculum,
    Vitium,
    Vitreus,
    Volatus,
}

impl Aspect {
    fn values() -> &'static [Aspect] {
        static VALUES: [Aspect; 63] = [
            Aspect::Aer,
            Aspect::Alienis,
            Aspect::Aqua,
            Aspect::Arbor,
            Aspect::Auram,
            Aspect::Bestia,
            Aspect::Caelum,
            Aspect::Cognitio,
            Aspect::Corpus,
            Aspect::Desidia,
            Aspect::Electrum,
            Aspect::Exanimis,
            Aspect::Fabrico,
            Aspect::Fames,
            Aspect::Gelum,
            Aspect::Gula,
            Aspect::Herba,
            Aspect::Humanus,
            Aspect::Ignis,
            Aspect::Infernus,
            Aspect::Instrumentum,
            Aspect::Invidia,
            Aspect::Ira,
            Aspect::Iter,
            Aspect::Limus,
            Aspect::Lucrum,
            Aspect::Lux,
            Aspect::Luxuria,
            Aspect::Machina,
            Aspect::Magneto,
            Aspect::Messis,
            Aspect::Metallum,
            Aspect::Meto,
            Aspect::Mortuus,
            Aspect::Motus,
            Aspect::Nebrisum,
            Aspect::Ordo,
            Aspect::Pannus,
            Aspect::Perditio,
            Aspect::Perfodio,
            Aspect::Permutatio,
            Aspect::Potentia,
            Aspect::Praecantatio,
            Aspect::Radio,
            Aspect::Sano,
            Aspect::Sensus,
            Aspect::Spiritus,
            Aspect::Strontio,
            Aspect::Superbia,
            Aspect::Tabernus,
            Aspect::Telum,
            Aspect::Tempestas,
            Aspect::Tempus,
            Aspect::Tenebrae,
            Aspect::Terra,
            Aspect::Tutamen,
            Aspect::Vacuos,
            Aspect::Venenum,
            Aspect::Victus,
            Aspect::Vinculum,
            Aspect::Vitium,
            Aspect::Vitreus,
            Aspect::Volatus,
        ];
        &VALUES
    }

    fn from_str_fuzzy(name: &String) -> Option<(Aspect, f64)> {
        let mut highest_score = 0.0;
        let mut best_match = None;

        for variant in Aspect::values().iter() {
            let variant_name = format!("{:?}", variant).to_lowercase();
            let input_name = name.to_lowercase();
            let score = normalized_levenshtein(&variant_name, &input_name);

            if score > highest_score {
                highest_score = score;
                best_match = Some(variant.clone());
            }
        }
        if best_match.is_some() {
            Some((best_match.unwrap(), highest_score))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct Graph {
    // Maps each node to a list of (neighbor, weight) pairs
    edges: HashMap<Aspect, HashSet<Aspect>>,
    aspect_inv: HashMap<Aspect, i16>,
}

impl Graph {
    // Initialize the graph with static data
    fn new(aspect_inv: HashMap<Aspect, i16>) -> Self {
        let mut edges = HashMap::new();
        Graph::add_composite(&mut edges, Aspect::Alienis, Aspect::Vacuos, Aspect::Tenebrae);
        Graph::add_composite(&mut edges, Aspect::Arbor, Aspect::Aer, Aspect::Herba);
        Graph::add_composite(&mut edges, Aspect::Auram, Aspect::Praecantatio, Aspect::Aer);
        Graph::add_composite(&mut edges, Aspect::Bestia, Aspect::Motus, Aspect::Victus);
        Graph::add_composite(&mut edges, Aspect::Caelum, Aspect::Vitreus, Aspect::Metallum);
        Graph::add_composite(&mut edges, Aspect::Cognitio, Aspect::Ignis, Aspect::Spiritus);
        Graph::add_composite(&mut edges, Aspect::Corpus, Aspect::Mortuus, Aspect::Bestia);
        Graph::add_composite(&mut edges, Aspect::Desidia, Aspect::Vinculum, Aspect::Spiritus);
        Graph::add_composite(&mut edges, Aspect::Electrum, Aspect::Potentia, Aspect::Machina);
        Graph::add_composite(&mut edges, Aspect::Exanimis, Aspect::Motus, Aspect::Mortuus);
        Graph::add_composite(&mut edges, Aspect::Fabrico, Aspect::Humanus, Aspect::Instrumentum);
        Graph::add_composite(&mut edges, Aspect::Fames, Aspect::Victus, Aspect::Vacuos);
        Graph::add_composite(&mut edges, Aspect::Gelum, Aspect::Ignis, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Gula, Aspect::Fames, Aspect::Vacuos);
        Graph::add_composite(&mut edges, Aspect::Herba, Aspect::Victus, Aspect::Terra);
        Graph::add_composite(&mut edges, Aspect::Humanus, Aspect::Bestia, Aspect::Cognitio);
        Graph::add_composite(&mut edges, Aspect::Infernus, Aspect::Ignis, Aspect::Praecantatio);
        Graph::add_composite(&mut edges, Aspect::Instrumentum, Aspect::Humanus, Aspect::Ordo);
        Graph::add_composite(&mut edges, Aspect::Invidia, Aspect::Sensus, Aspect::Fames);
        Graph::add_composite(&mut edges, Aspect::Ira, Aspect::Telum, Aspect::Ignis);
        Graph::add_composite(&mut edges, Aspect::Iter, Aspect::Motus, Aspect::Terra);
        Graph::add_composite(&mut edges, Aspect::Limus, Aspect::Victus, Aspect::Aqua);
        Graph::add_composite(&mut edges, Aspect::Lucrum, Aspect::Humanus, Aspect::Fames);
        Graph::add_composite(&mut edges, Aspect::Lux, Aspect::Aer, Aspect::Ignis);
        Graph::add_composite(&mut edges, Aspect::Luxuria, Aspect::Corpus, Aspect::Fames);
        Graph::add_composite(&mut edges, Aspect::Machina, Aspect::Motus, Aspect::Instrumentum);
        Graph::add_composite(&mut edges, Aspect::Magneto, Aspect::Metallum, Aspect::Iter);
        Graph::add_composite(&mut edges, Aspect::Messis, Aspect::Herba, Aspect::Humanus);
        Graph::add_composite(&mut edges, Aspect::Metallum, Aspect::Terra, Aspect::Vitreus);
        Graph::add_composite(&mut edges, Aspect::Meto, Aspect::Messis, Aspect::Instrumentum);
        Graph::add_composite(&mut edges, Aspect::Mortuus, Aspect::Victus, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Motus, Aspect::Aer, Aspect::Ordo);
        Graph::add_composite(&mut edges, Aspect::Nebrisum, Aspect::Perfodio, Aspect::Lucrum);
        Graph::add_composite(&mut edges, Aspect::Pannus, Aspect::Instrumentum, Aspect::Bestia);
        Graph::add_composite(&mut edges, Aspect::Perfodio, Aspect::Humanus, Aspect::Terra);
        Graph::add_composite(&mut edges, Aspect::Permutatio, Aspect::Perditio, Aspect::Ordo);
        Graph::add_composite(&mut edges, Aspect::Potentia, Aspect::Ordo, Aspect::Ignis);
        Graph::add_composite(&mut edges, Aspect::Praecantatio, Aspect::Vacuos, Aspect::Potentia);
        Graph::add_composite(&mut edges, Aspect::Radio, Aspect::Lux, Aspect::Potentia);
        Graph::add_composite(&mut edges, Aspect::Sano, Aspect::Victus, Aspect::Ordo);
        Graph::add_composite(&mut edges, Aspect::Sensus, Aspect::Aer, Aspect::Spiritus);
        Graph::add_composite(&mut edges, Aspect::Spiritus, Aspect::Victus, Aspect::Mortuus);
        Graph::add_composite(&mut edges, Aspect::Strontio, Aspect::Cognitio, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Superbia, Aspect::Volatus, Aspect::Vacuos);
        Graph::add_composite(&mut edges, Aspect::Tabernus, Aspect::Tutamen, Aspect::Iter);
        Graph::add_composite(&mut edges, Aspect::Telum, Aspect::Instrumentum, Aspect::Ignis);
        Graph::add_composite(&mut edges, Aspect::Tempestas, Aspect::Aer, Aspect::Aqua);
        Graph::add_composite(&mut edges, Aspect::Tempus, Aspect::Vacuos, Aspect::Ordo);
        Graph::add_composite(&mut edges, Aspect::Tenebrae, Aspect::Vacuos, Aspect::Lux);
        Graph::add_composite(&mut edges, Aspect::Tutamen, Aspect::Instrumentum, Aspect::Terra);
        Graph::add_composite(&mut edges, Aspect::Vacuos, Aspect::Aer, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Venenum, Aspect::Aqua, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Victus, Aspect::Aqua, Aspect::Terra);
        Graph::add_composite(&mut edges, Aspect::Vinculum, Aspect::Motus, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Vitium, Aspect::Praecantatio, Aspect::Perditio);
        Graph::add_composite(&mut edges, Aspect::Vitreus, Aspect::Terra, Aspect::Ordo);
        Graph::add_composite(&mut edges, Aspect::Volatus, Aspect::Aer, Aspect::Motus);

        Graph { edges, aspect_inv }
    }

    fn get_price_of(&self, aspect: Aspect) -> f64 {
        let amount = self.aspect_inv.get(&aspect).unwrap_or(&0).to_owned();
        if amount == 0 {
            return f64::MAX;
        }

        return 1.0 / (amount as f64);
    }

    fn add_edge(edges: &mut HashMap<Aspect, HashSet<Aspect>>, key: Aspect, aspects: Vec<Aspect>) {
        edges.entry(key).or_insert_with(HashSet::new).extend(aspects)
    }

    fn add_composite(edges: &mut HashMap<Aspect, HashSet<Aspect>>, result: Aspect, aspect_a: Aspect, aspect_b: Aspect) {
        Graph::add_edge(edges, result, vec![aspect_a, aspect_b]);
        Graph::add_edge(edges, aspect_a, vec![result]);
        Graph::add_edge(edges, aspect_b, vec![result]);
    }

    fn find_paths_longer(
        &self,
        start: Aspect,
        end: Aspect,
        distance: usize,
    ) -> ((Vec<Vec<Aspect>>, u64), (Vec<Vec<Aspect>>, u64)) {
        let (desired_paths, desired_paths_score) = self.find_paths(start, end, distance);
        let mut best_paths = desired_paths.clone();
        let mut lowest_score = desired_paths_score;
        for inc in 1..3 {
            let (paths, score) = self.find_paths(start, end, distance + inc);
            if score < lowest_score {
                lowest_score = score;
                best_paths = paths;
            }
        }

        let desired_paths_score = (desired_paths_score * 100000.0) as u64;
        let lowest_score = (lowest_score * 100000.0) as u64;
        if lowest_score < desired_paths_score {
            ((desired_paths, desired_paths_score), (best_paths, lowest_score))
        } else {
            ((desired_paths, desired_paths_score), (Vec::new(), 0))
        }
    }

    fn find_paths(&self, start: Aspect, end: Aspect, distance: usize) -> (Vec<Vec<Aspect>>, f64) {
        let mut results = Vec::new();
        let mut lowest_score = f64::MAX;
        let mut queue = VecDeque::new();
        queue.push_back((start, 0.0, 1, vec![start]));

        while let Some((current, current_score, current_distance, path)) = queue.pop_front() {
            if current_distance == distance {
                if current == end && current_score <= lowest_score {
                    if current_score < lowest_score {
                        results.clear();
                        lowest_score = current_score;
                    }
                    results.push(path.clone());
                }
                continue;
            }

            if let Some(neighbors) = self.edges.get(&current) {
                for neighbor in neighbors {
                    let price = self.get_price_of(*neighbor);
                    if current_score + price <= lowest_score {
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push_back((neighbor.clone(), current_score + price, current_distance + 1, new_path));
                    }
                }
            }
        }

        (results, lowest_score)
    }
}

fn yes_or_no() -> bool {
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => {
            let normalized_input = input.trim().to_lowercase();
            match normalized_input.as_str() {
                "yes" | "y" => true,
                _ => false,
            }
        }
        Err(error) => panic!("Error reading input: {}", error),
    }
}

fn find_aspect(msg: &str) -> Aspect {
    use std::io::{self, Write};

    let mut aspect_str = String::new();
    let mut aspect: Option<Aspect> = None;

    while aspect.is_none() {
        aspect_str.clear();

        print!("{}", msg);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut aspect_str).unwrap();
        aspect_str = aspect_str.trim().to_owned();

        aspect = match Aspect::from_str_fuzzy(&aspect_str) {
            Some((aspect, 1.0)) => Some(aspect),
            Some((aspect, _)) => {
                println!("Did you mean '{:?}'? y/n", aspect);
                if yes_or_no() {
                    Some(aspect)
                } else {
                    None
                }
            }
            None => {
                println!("Aspect does not exist!");
                None
            }
        };
    }

    aspect.unwrap()
}

fn read_aspect(aspect: &Value) -> (Aspect, i16) {
    if let Value::Compound(aspect) = aspect {
        let aspect_key = aspect.get("key");
        let aspect_amount = aspect.get("amount");

        match (aspect_key, aspect_amount) {
            (Some(Value::String(key)), Some(Value::Short(amount))) => {
                if let Some((aspect, 1.0)) = Aspect::from_str_fuzzy(key) {
                    (aspect, amount.to_owned())
                } else {
                    panic!("Aspect inventory contains unknown aspect '{}'.", key);
                }
            }
            _ => {
                panic!("Aspect inventory does not contain expected data.");
            }
        }
    } else {
        panic!("Aspect inventory contains unexpected NBT element.");
    }
}

fn read_aspect_inventory<R>(file: &mut R) -> HashMap<Aspect, i16>
where
    R: Read,
{
    let mut aspects = HashMap::new();
    let blob: Blob = Blob::from_gzip_reader(file).unwrap();
    match blob.get("THAUMCRAFT.ASPECTS") {
        Some(Value::List(owned_aspects)) => {
            for owned_aspect in owned_aspects {
                let (aspect, amount) = read_aspect(owned_aspect);
                aspects.insert(aspect, amount);
            }
        }
        _ => {}
    }

    aspects
}

fn download_aspect_inventory(args: &Args) -> HashMap<Aspect, i16> {
    let mut ftp_stream = FtpStream::connect(args.ftp_address.as_str()).expect("Should connect to FTP");
    let _ = ftp_stream
        .login(args.ftp_username.as_str(), args.ftp_password.as_str())
        .expect("Should login to FTP");
    let mut reader = ftp_stream
        .simple_retr(format!("/World/playerdata/{}.thaum", args.username).as_str())
        .expect("Should retrieve thaum file from FTP");

    read_aspect_inventory(&mut reader)
}

fn main_loop(graph: &Graph) {
    // Input handling
    use std::io::{self, Write};
    let aspect_a = find_aspect("Enter the first aspect: ");
    let aspect_b = find_aspect("Enter the second aspect: ");

    let mut distance_str = String::new();
    print!("Enter the desired distance: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut distance_str).unwrap();

    let mut target_distance: usize = distance_str.trim().parse().expect("Please enter a valid number");
    target_distance += 2;
    let mut desired_paths = Vec::new();
    let mut best_paths = Vec::new();
    let mut desired_paths_score = 0;
    let mut best_score = 0;

    println!("\n");

    while desired_paths.is_empty() {
        // Find paths
        ((desired_paths, desired_paths_score), (best_paths, best_score)) =
            graph.find_paths_longer(aspect_a, aspect_b, target_distance);
        if desired_paths.len() == 0 {
            println!(
                "There is no such path of length {} between aspects {:?} and {:?}.",
                target_distance, aspect_b, aspect_b
            );

            target_distance += 1;
            println!("Trying to find a path with length of {}.", target_distance);
        }
    }
    // Display results
    println!(
        "Paths from {:?} to {:?} with length {}:",
        aspect_a, aspect_b, target_distance
    );
    for path in &desired_paths {
        println!("\tScore [{}] {:?}", desired_paths_score, path);
    }

    if best_paths.len() > 0 {
        println!(
            "A paths from {:?} to {:?} of length {} were found that are cheaper than cheapest path of length {}.",
            aspect_a,
            aspect_b,
            best_paths[0].len(),
            target_distance
        );

        for path in &best_paths {
            println!("\tScore [{}] {:?}", best_score, path);
        }
    }

    println!("\n");
}

fn main() {
    let args = Args::parse();
    let aspect_inv = download_aspect_inventory(&args);
    let graph = Graph::new(aspect_inv);

    loop {
        main_loop(&graph);
    }
}
