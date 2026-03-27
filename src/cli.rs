use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "spin", about = "Local development orchestrator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Bring up an application and all its dependencies
    Up,
    /// Tear down a running application and all its dependencies
    Down,
}
