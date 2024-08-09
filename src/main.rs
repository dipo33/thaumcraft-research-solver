mod aspect;
mod graph;
mod solver;

use aspect::{Aspect, AspectInventory};
use clap::Parser;
use ftp::FtpStream;
use nbt::Blob;
use solver::Solver;
use std::{cmp::min, io::Cursor};

/// ThaumCraft Research Solver using weighted paths with your actual aspect inventory
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Actual MineCraft username
    #[arg(short, long)]
    username: String,

    /// MineCraft server FTP address
    #[arg(short = 'a', long)]
    ftp_address: String,

    /// MineCraft server FTP username
    #[arg(short, long)]
    ftp_username: String,

    /// MineCraft server FTP password
    #[arg(short = 'p', long)]
    ftp_password: String,
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

fn read_u8(msg: &str, max: u8) -> u8 {
    use std::io::{self, Write};

    let mut value_str = String::new();
    let mut value: Option<u8> = None;
    while value.is_none() {
        value_str.clear();

        print!("{}", msg);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut value_str).unwrap();
        value_str = value_str.trim().to_string();

        value = match value_str.parse() {
            Ok(value) if value <= max => Some(value),
            _ => {
                print!("'{}' is not a valid integer between 0 and {}! ", value_str, max);
                None
            }
        }
    }

    value.unwrap()
}

fn download_aspect_inventory_from_ftp(args: &Args) -> Cursor<Vec<u8>> {
    let mut ftp_stream = FtpStream::connect(args.ftp_address.as_str()).expect("Should connect to FTP");
    let _ = ftp_stream.login(args.ftp_username.as_str(), args.ftp_password.as_str()).expect("Should login to FTP");

    ftp_stream
        .simple_retr(format!("/World/playerdata/{}.thaum", args.username).as_str())
        .expect("Should retrieve thaum file from FTP")
}

fn main_loop(solver: &Solver) {
    let aspect_a = find_aspect("Enter the first aspect: ");
    let aspect_b = find_aspect("Enter the second aspect: ");

    let target_distance: u8 = read_u8("Enter the minimal distance between the two aspects: ", 8) + 2;
    let max_distance_increase = min(12 - target_distance, 3);

    println!("\n");

    let best_paths = solver.find_paths(aspect_a, aspect_b, target_distance, max_distance_increase);
    let mut shortest_price: Option<u32> = None;
    for increase in 0..max_distance_increase {
        let paths = best_paths.get(&increase);
        if let Some(paths) = paths {
            if paths.paths.len() > 0 {
                if shortest_price.is_none() {
                    shortest_price = Some(paths.price);
                    println!("Shortest paths from {:?} to {:?} are of length {}!", aspect_a, aspect_b, target_distance + increase);
                }

                if paths.price > shortest_price.unwrap() {
                    continue;
                }

                println!("Paths from {:?} to {:?} of length {}:", aspect_a, aspect_b, target_distance + increase);
                for path in &paths.paths {
                    println!("\tScore [{}]: {:?}", paths.price, path);
                }
            }
        }
    }

    println!("\n");
}

fn main() {
    let args = Args::parse();
    let mut aspect_inventory_file = download_aspect_inventory_from_ftp(&args);
    let blob = Blob::from_gzip_reader(&mut aspect_inventory_file).unwrap();
    let aspect_inventory = AspectInventory::from_nbt(blob).unwrap();
    let solver = Solver::new(aspect_inventory);

    loop {
        main_loop(&solver);
    }
}
