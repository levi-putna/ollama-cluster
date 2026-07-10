mod cli;
mod output;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e:#}");
        std::process::exit(exit_code(&e));
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(ref cmd) => cli::execute(cmd.clone(), &cli).await,
        None => cli::cmd_dashboard(&cli).await,
    }
}

fn exit_code(err: &anyhow::Error) -> i32 {
    let msg = format!("{err:#}");
    if msg.contains("configuration") {
        3
    } else if msg.contains("unavailable") || msg.contains("connect") {
        4
    } else if msg.contains("not found") {
        5
    } else if msg.contains("rejected") {
        6
    } else {
        1
    }
}
