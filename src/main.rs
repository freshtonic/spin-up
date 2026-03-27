mod cli;

use clap::Parser;
use cli::{Cli, Command, PlumbingCommand};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Up => {
            println!("spin up: not yet implemented");
        }
        Command::Down => {
            println!("spin down: not yet implemented");
        }
        Command::Plumbing { command } => match command {
            PlumbingCommand::Supervise { resource } => {
                println!("spin plumbing supervise {resource}: not yet implemented");
            }
            PlumbingCommand::Kill { resource } => {
                println!("spin plumbing kill {resource}: not yet implemented");
            }
        },
    }
}
