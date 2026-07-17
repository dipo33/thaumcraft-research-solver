# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                  # debug build
cargo build --release        # optimized build
cargo run -- ftp --username <mc_user> --ftp-address <host> --ftp-username <user> --ftp-password <pwd>
cargo run -- ssh --username <mc_user> --host-alias <alias>
cargo run -- simple          # test mode — uses 100 of each aspect
cargo fmt                    # format (max_width = 180, configured in rustfmt.toml)
cargo clippy
```

## Architecture

Single-binary CLI with four modules:

- **`aspect.rs`** — `Aspect` enum (69 variants, all Thaumcraft aspects), `AspectInventory` (player counts loaded from NBT). `price_of(aspect)` returns `max_amount + 1 - current_amount`; missing aspects return `u16::MAX`.
- **`graph.rs`** — Generic undirected adjacency-list graph used to model aspect transformation rules.
- **`solver.rs`** — Weighted BFS over the aspect graph. `find_paths_with_length()` explores all paths between two aspects at a given distance, pruning branches when cumulative cost exceeds the best found so far. Returns all minimum-cost paths.
- **`main.rs`** — CLI parsing (clap), inventory loading from three sources (FTP/SSH/simple), interactive REPL.

## Aspect Graph

50 hardcoded edges in `Solver::build_aspect_graph()` encode all Thaumcraft transformation rules (e.g., `Arbor = Aer + Herba`). Adding a new aspect requires: a new `Aspect` enum variant in `aspect.rs`, its key/display name, and the relevant graph edges in `solver.rs`.

## Inventory Loading

Player inventory is a gzip-compressed NBT file (`.thaum` extension, Minecraft mod format):
- **FTP**: downloads `/World/playerdata/{username}.thaum` from the server
- **SSH**: reads `/opt/gtnh/gtnh_server/World/playerdata/{username}.thaum` via SSH; uses `~/.ssh/config` for host resolution and supports agent auth or pubkey files
- **Simple**: skips loading; fills all 69 aspects with count=100

Five custom aspects map to `custom1`–`custom5` NBT keys rather than their display names.
