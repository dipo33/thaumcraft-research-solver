mod aspect;
mod graph;
mod solver;

use anyhow::{anyhow, Context, Result};
use aspect::{Aspect, AspectInventory};
use clap::{Args as ClapArgs, Parser, Subcommand};
use colored::Colorize;
use ftp::FtpStream;
use nbt::Blob;
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Editor, Helper};
use solver::Solver;
use ssh2::Session;
use ssh2_config::{HostParams, ParseRule, SshConfig};
use std::cmp::min;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};

/// ThaumCraft Research Solver using weighted paths with your actual aspect inventory
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(ClapArgs, Debug)]
struct FtpConfig {
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

#[derive(ClapArgs, Debug)]
pub struct SshConfigRef {
    /// Actual Minecraft username
    #[arg(short, long)]
    username: String,

    /// Host alias in ~/.ssh/config
    #[arg(short = 'a', long)]
    pub host_alias: String,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Use FTP to connect to the server
    Ftp(FtpConfig),

    /// Use SSH to connect to the server
    Ssh(SshConfigRef),

    /// Run without connecting to a server (all aspects at 100)
    Simple,
}

// ─── Rustyline helper: tab-completion for aspect names ────────────────────────

struct AspectHelper;

impl Completer for AspectHelper {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<String>)> {
        let prefix = line[..pos].trim().to_lowercase();
        if prefix.is_empty() {
            return Ok((0, vec![]));
        }
        let candidates = Aspect::values()
            .iter()
            .filter(|a| a.display_name().starts_with(&prefix))
            .map(|a| a.display_name())
            .collect();
        Ok((0, candidates))
    }
}

impl Hinter for AspectHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        None
    }
}
impl Highlighter for AspectHelper {}
impl Validator for AspectHelper {}
impl Helper for AspectHelper {}

type Rl = Editor<AspectHelper, DefaultHistory>;

// ─── SSH helpers ──────────────────────────────────────────────────────────────

fn expand_tilde(p: &Path) -> Result<PathBuf> {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        let home = dirs::home_dir().context("Unable to determine home directory")?;
        Ok(home.join(rest))
    } else {
        Ok(p.to_path_buf())
    }
}

pub fn download_aspect_inventory_from_ssh(cfg: &SshConfigRef) -> Result<Cursor<Vec<u8>>> {
    let ssh_cfg = load_user_ssh_config(ParseRule::STRICT)?;
    let params = resolve_host_params(&ssh_cfg, &cfg.host_alias);
    let mut session = connect_ssh_session(&params, &cfg.host_alias)?;
    authenticate_session(&mut session, &params)?;
    sftp_read_to_cursor(&session, &remote_thaum_path(&cfg.username))
}

fn load_user_ssh_config(rules: ParseRule) -> Result<SshConfig> {
    let path = default_ssh_config_path()?;
    let file = File::open(&path).with_context(|| format!("Failed to open {:?}", path))?;
    let mut reader = BufReader::new(file);
    SshConfig::default()
        .parse(&mut reader, rules)
        .with_context(|| format!("Failed to parse SSH config {:?}", path))
}

fn default_ssh_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Unable to determine home directory")?;
    Ok(home.join(".ssh").join("config"))
}

fn resolve_host_params(cfg: &SshConfig, alias: &str) -> HostParams {
    cfg.query(alias)
}

fn connect_ssh_session(params: &HostParams, alias_fallback: &str) -> Result<Session> {
    let hostname = params.host_name.clone().unwrap_or_else(|| alias_fallback.to_string());
    let port = params.port.unwrap_or(22);
    let addr = format!("{}:{}", hostname, port);
    let tcp = TcpStream::connect(&addr).with_context(|| format!("Failed to connect to {}", addr))?;
    let mut sess = Session::new().context("Failed to create SSH session")?;
    sess.set_tcp_stream(tcp);
    sess.handshake().context("SSH handshake failed")?;
    Ok(sess)
}

fn authenticate_session(session: &mut Session, params: &HostParams) -> Result<()> {
    let user = params
        .user
        .as_deref()
        .ok_or_else(|| anyhow!("No `User` resolved from SSH config for this host"))?;

    if session.userauth_agent(user).is_ok() && session.authenticated() {
        return Ok(());
    }
    for identity_file in params.identity_file.clone().unwrap_or_default().iter() {
        let path = expand_tilde(identity_file)?;
        if session.userauth_pubkey_file(user, None, &path, None).is_ok() && session.authenticated() {
            return Ok(());
        }
    }
    Err(anyhow!("SSH authentication failed (agent and IdentityFile fallback both failed)"))
}

fn remote_thaum_path(mc_username: &str) -> String {
    format!("/opt/gtnh/gtnh_server/World/playerdata/{}.thaum", mc_username)
}

fn sftp_read_to_cursor(session: &Session, remote_path: &str) -> Result<Cursor<Vec<u8>>> {
    let sftp = session.sftp().context("Failed to create SFTP session")?;
    let mut file = sftp.open(remote_path).with_context(|| format!("Failed to open remote file {}", remote_path))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).with_context(|| format!("Failed to read remote file {}", remote_path))?;
    Ok(Cursor::new(buf))
}

// ─── FTP helpers ──────────────────────────────────────────────────────────────

fn download_aspect_inventory_from_ftp(config: &FtpConfig) -> Result<Cursor<Vec<u8>>> {
    let mut ftp_stream = FtpStream::connect(config.ftp_address.as_str())
        .map_err(|e| anyhow!("Failed to connect to FTP at {}: {}", config.ftp_address, e))?;
    ftp_stream
        .login(config.ftp_username.as_str(), config.ftp_password.as_str())
        .map_err(|e| anyhow!("Failed to login to FTP: {}", e))?;
    ftp_stream
        .simple_retr(&format!("/World/playerdata/{}.thaum", config.username))
        .map_err(|e| anyhow!("Failed to retrieve .thaum file: {}", e))
}

// ─── Inventory loading ────────────────────────────────────────────────────────

fn load_ftp_inventory(config: &FtpConfig) -> Result<AspectInventory> {
    let mut cursor = download_aspect_inventory_from_ftp(config)?;
    let blob = Blob::from_gzip_reader(&mut cursor).context("Failed to parse NBT data")?;
    AspectInventory::from_nbt(blob).map_err(|e| anyhow!("{}", e))
}

fn load_ssh_inventory(config: &SshConfigRef) -> Result<AspectInventory> {
    let mut cursor = download_aspect_inventory_from_ssh(config)?;
    let blob = Blob::from_gzip_reader(&mut cursor).context("Failed to parse NBT data")?;
    AspectInventory::from_nbt(blob).map_err(|e| anyhow!("{}", e))
}

// ─── Display helpers ──────────────────────────────────────────────────────────

fn divider() {
    println!("{}", "─".repeat(56).dimmed());
}

fn print_inventory_summary(inventory: &AspectInventory) {
    let known = Aspect::values().iter().filter(|&&a| inventory.amount_of(a) > 0).count();
    let total = Aspect::values().len();
    let max_amount = Aspect::values().iter().map(|&a| inventory.amount_of(a)).max().unwrap_or(0);
    println!(
        "  Inventory: {} / {} aspects known, highest stack {}",
        known.to_string().cyan().bold(),
        total.to_string().dimmed(),
        max_amount.to_string().cyan()
    );
}

fn format_path(path: &[Aspect]) -> String {
    let arrow = " → ".dimmed().to_string();
    path.iter()
        .map(|a| a.display_name().cyan().to_string())
        .collect::<Vec<_>>()
        .join(&arrow)
}

// ─── Input helpers ────────────────────────────────────────────────────────────

fn is_quit(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "q" | "quit" | "exit")
}

fn yes_or_no(label: &str) -> bool {
    print!("  {} ", label.dimmed());
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    matches!(s.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Reads an aspect name with fuzzy matching and tab completion.
/// Returns None if the user wants to quit.
fn find_aspect(rl: &mut Rl, label: &str) -> Option<Aspect> {
    let prompt = format!("  {} ", label.yellow().bold());
    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim().to_owned();
                if input.is_empty() {
                    continue;
                }
                if is_quit(&input) {
                    return None;
                }
                rl.add_history_entry(&input).ok();

                match Aspect::from_str_fuzzy(&input) {
                    Some((aspect, score)) if score >= 0.99 => return Some(aspect),
                    Some((aspect, score)) if score >= 0.5 => {
                        println!(
                            "  {} {}? ",
                            "Did you mean".yellow(),
                            aspect.display_name().cyan().bold()
                        );
                        if yes_or_no("(y/n):") {
                            return Some(aspect);
                        }
                    }
                    _ => println!(
                        "  {} Unknown aspect '{}'. Tab to autocomplete, 'quit' to exit.",
                        "✗".red(),
                        input.yellow()
                    ),
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) | Err(rustyline::error::ReadlineError::Eof) => {
                return None;
            }
            Err(e) => {
                eprintln!("{} {}", "Read error:".red(), e);
                return None;
            }
        }
    }
}

/// Reads the number of intermediate aspects (blank hexes to fill on the board).
/// Returns None if the user wants to quit.
fn read_steps(rl: &mut Rl) -> Option<u8> {
    let prompt = format!("  {} ", "Steps (1–8):".yellow().bold());
    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim().to_owned();
                if input.is_empty() {
                    continue;
                }
                if is_quit(&input) {
                    return None;
                }
                rl.add_history_entry(&input).ok();
                match input.parse::<u8>() {
                    Ok(v) if (1..=8).contains(&v) => return Some(v),
                    _ => println!("  {} Enter a number from 1 to 8.", "✗".red()),
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) | Err(rustyline::error::ReadlineError::Eof) => {
                return None;
            }
            Err(e) => {
                eprintln!("{} {}", "Read error:".red(), e);
                return None;
            }
        }
    }
}

// ─── REPL ─────────────────────────────────────────────────────────────────────

/// Runs one search. Returns false when the user wants to quit.
fn main_loop(solver: &Solver, rl: &mut Rl) -> bool {
    println!();
    divider();

    let aspect_a = match find_aspect(rl, "Start aspect:") {
        Some(a) => a,
        None => return false,
    };

    let aspect_b = match find_aspect(rl, "End aspect:  ") {
        Some(a) => a,
        None => return false,
    };

    println!(
        "\n  {} count the blank hexes between {} and {} on the research board",
        "Steps:".yellow().bold(),
        aspect_a.display_name().cyan(),
        aspect_b.display_name().cyan()
    );

    let steps = match read_steps(rl) {
        Some(s) => s,
        None => return false,
    };

    println!();

    let target_distance: u8 = steps + 2;
    let max_distance_increase = min(12u8.saturating_sub(target_distance), 3);

    let best_paths = solver.find_paths(aspect_a, aspect_b, target_distance, max_distance_increase);

    let mut best_price: Option<u32> = None;
    let mut found_any = false;

    for increase in 0..max_distance_increase {
        if let Some(paths) = best_paths.get(&increase) {
            if paths.paths.is_empty() {
                continue;
            }

            let chain_len = target_distance + increase;

            if let Some(bp) = best_price {
                if paths.price > bp {
                    continue;
                }
                println!(
                    "  {} chain {}, cost {}:",
                    "Also at".yellow(),
                    chain_len.to_string().cyan(),
                    paths.price.to_string().yellow()
                );
            } else {
                best_price = Some(paths.price);
                println!(
                    "  {} {} {} {} — chain {}, cost {}:",
                    "Best paths:".green().bold(),
                    aspect_a.display_name().cyan().bold(),
                    "→".dimmed(),
                    aspect_b.display_name().cyan().bold(),
                    chain_len.to_string().cyan().bold(),
                    paths.price.to_string().yellow()
                );
            }

            for path in &paths.paths {
                println!("    {}", format_path(path));
            }

            found_any = true;
        }
    }

    if !found_any {
        println!(
            "  {} No path from {} to {} with {} intermediate step(s). Try a different step count.",
            "✗".red(),
            aspect_a.display_name().cyan(),
            aspect_b.display_name().cyan(),
            steps
        );
    }

    true
}

// ─── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    println!("{}", "ThaumCraft Research Solver".cyan().bold());
    println!("{}", "━".repeat(40).dimmed());

    let aspect_inventory = match args.mode {
        Mode::Ftp(config) => {
            print!("  Connecting via FTP... ");
            io::stdout().flush().ok();
            match load_ftp_inventory(&config) {
                Ok(inv) => {
                    println!("{}", "done".green());
                    inv
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
        Mode::Ssh(config) => {
            print!("  Connecting via SSH ({})... ", config.host_alias.cyan());
            io::stdout().flush().ok();
            match load_ssh_inventory(&config) {
                Ok(inv) => {
                    println!("{}", "done".green());
                    inv
                }
                Err(e) => {
                    eprintln!("{} {}", "Error:".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
        Mode::Simple => {
            println!("  {} simple mode — all aspects at 100", "ℹ".blue());
            AspectInventory::default()
        }
    };

    print_inventory_summary(&aspect_inventory);
    println!(
        "  {}",
        "Tab: complete aspect names  •  ↑↓: history  •  'quit': exit".dimmed()
    );

    let solver = Solver::new(aspect_inventory);

    let config = Config::builder().completion_type(CompletionType::List).build();
    let mut rl: Rl = match Editor::with_config(config) {
        Ok(ed) => ed,
        Err(e) => {
            eprintln!("{} {}", "Failed to initialise line editor:".red(), e);
            std::process::exit(1);
        }
    };
    rl.set_helper(Some(AspectHelper));

    loop {
        if !main_loop(&solver, &mut rl) {
            println!("\n{}", "Goodbye!".cyan().bold());
            break;
        }
    }
}
