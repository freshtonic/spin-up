mod cli;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Up => {
            println!("spin up: not yet implemented");
        }
        Command::Down => {
            println!("spin down: not yet implemented");
        }
    }
}
