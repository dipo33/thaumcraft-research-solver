mod aspect;
mod graph;
mod solver;

use anyhow::{anyhow, Context, Result};
use aspect::{Aspect, AspectInventory};
use clap::{Args as ClapArgs, Parser, Subcommand};
use ftp::FtpStream;
use nbt::Blob;
use solver::Solver;
use ssh2::Session;
use ssh2_config::{HostParams, ParseRule, SshConfig};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::net::TcpStream;
use std::{
    cmp::min,
    path::{Path, PathBuf},
};

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

    /// Host alias in ~/.ssh/config, e.g. "nuremberg"
    #[arg(short = 'a', long)]
    pub host_alias: String,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Use FTP to connect to the server
    Ftp(FtpConfig),

    /// Use SSH to connect to the server
    Ssh(SshConfigRef),

    /// Run without FTP
    Simple,
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

fn expand_tilde(p: &Path) -> Result<PathBuf> {
    // ssh2-config yields PathBufs; if the key is "~/.ssh/id_ed25519", libssh2 won’t expand it for you.
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

    let remote_path = remote_thaum_path(&cfg.username);
    sftp_read_to_cursor(&session, &remote_path)
}

fn load_user_ssh_config(rules: ParseRule) -> Result<SshConfig> {
    let path = default_ssh_config_path()?;
    let file = File::open(&path).with_context(|| format!("Failed to open {:?}", path))?;
    let mut reader = BufReader::new(file);

    // This matches ssh2-config’s API and documentation examples. :contentReference[oaicite:2]{index=2}
    let cfg = SshConfig::default()
        .parse(&mut reader, rules)
        .with_context(|| format!("Failed to parse SSH config {:?}", path))?;

    Ok(cfg)
}

fn default_ssh_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Unable to determine home directory")?;
    Ok(home.join(".ssh").join("config"))
}

fn resolve_host_params<'a>(cfg: &'a SshConfig, alias: &str) -> HostParams {
    // Per docs: if no rule matches, defaults are returned. :contentReference[oaicite:3]{index=3}
    cfg.query(alias)
}

fn connect_ssh_session(params: &HostParams, alias_fallback: &str) -> Result<Session> {
    let hostname = params.host_name.clone().unwrap_or_else(|| alias_fallback.to_string());

    let port: u16 = params.port.unwrap_or(22) as u16;
    let addr = format!("{}:{}", hostname, port);

    let tcp = TcpStream::connect(&addr).with_context(|| format!("Failed to connect to {}", addr))?;

    let mut sess = Session::new().context("Failed to create SSH session")?;
    sess.set_tcp_stream(tcp);
    sess.handshake().context("SSH handshake failed")?;

    Ok(sess)
}

fn authenticate_session(session: &mut Session, params: &HostParams) -> Result<()> {
    let user = params.user.as_deref().ok_or_else(|| anyhow!("No `User` resolved from SSH config for this host"))?;

    // 1) Prefer agent (common when you use ssh config aliases)
    if session.userauth_agent(user).is_ok() && session.authenticated() {
        return Ok(());
    }

    // 2) Fall back to IdentityFile list, as shown in ssh2-config docs. :contentReference[oaicite:4]{index=4}
    for identity_file in params.identity_file.clone().unwrap_or_default().iter() {
        let identity_file = expand_tilde(identity_file)?;
        if session.userauth_pubkey_file(user, None, &identity_file, None).is_ok() && session.authenticated() {
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

fn download_aspect_inventory_from_ftp(config: &FtpConfig) -> Cursor<Vec<u8>> {
    let mut ftp_stream = FtpStream::connect(config.ftp_address.as_str()).expect("Should connect to FTP");
    let _ = ftp_stream.login(config.ftp_username.as_str(), config.ftp_password.as_str()).expect("Should login to FTP");

    ftp_stream
        .simple_retr(format!("/World/playerdata/{}.thaum", config.username).as_str())
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
    let aspect_inventory = match args.mode {
        Mode::Ftp(ftp_config) => {
            let mut aspect_inventory_file = download_aspect_inventory_from_ftp(&ftp_config);
            let blob = Blob::from_gzip_reader(&mut aspect_inventory_file).unwrap();
            AspectInventory::from_nbt(blob).unwrap()
        }
        Mode::Ssh(ssh_config) => {
            let mut aspect_inventory_file = download_aspect_inventory_from_ssh(&ssh_config).unwrap();
            let blob = Blob::from_gzip_reader(&mut aspect_inventory_file).unwrap();
            AspectInventory::from_nbt(blob).unwrap()
        }
        Mode::Simple => AspectInventory::default(),
    };
    let solver = Solver::new(aspect_inventory);

    loop {
        main_loop(&solver);
    }
}
