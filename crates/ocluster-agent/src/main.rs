//! Optional node agent — stub implementation for 0.1.0.

use clap::Parser;

/// Node agent for local Ollama host management.
#[derive(Debug, Parser)]
#[command(name = "ocluster-agent", version, about = "Ollama Cluster node agent (stub)")]
struct Args {}

fn main() {
    let _ = Args::parse();
    eprintln!("ocluster-agent is not implemented in 0.1.0");
    eprintln!("Core routing and discovery work without an agent.");
    std::process::exit(2);
}
