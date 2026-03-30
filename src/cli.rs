use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "spin", about = "Local development orchestrator")]
pub struct Cli {
    /// Show plumbing commands in help output
    #[arg(long, global = true, hide = true)]
    pub plumbing: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Bring up an application and all its dependencies
    Up,
    /// Tear down a running application and all its dependencies
    Down,
    /// Run static analysis on .spin files
    Check,
    /// Internal plumbing commands (use --plumbing to see in help)
    #[command(hide = true)]
    Plumbing {
        #[command(subcommand)]
        command: PlumbingCommand,
    },
}

#[derive(Subcommand)]
pub enum PlumbingCommand {
    /// Launch and monitor a resource
    Supervise {
        /// Name of the resource to supervise
        resource: String,
    },
    /// Tear down a resource
    Kill {
        /// Name of the resource to kill
        resource: String,
    },
}
