mod cli;

use std::path::PathBuf;

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
        Command::Check { file } => {
            // Get SPIN_PATH from env, defaulting to current directory
            let spin_path_str = std::env::var("SPIN_PATH").unwrap_or_else(|_| ".".to_string());
            let spin_path_dirs: Vec<PathBuf> =
                spin_path_str.split(':').map(PathBuf::from).collect();

            // Phase 1: Module resolution
            let resolve_result =
                spin_up::analysis::resolve::resolve_modules(&file, &spin_path_dirs);

            // Phase 2: Type unification
            let mut diagnostics = resolve_result.diagnostics;
            let unify_diags = spin_up::analysis::unify::unify(&resolve_result.registry);
            diagnostics.merge(unify_diags);

            // Phase 3: Dependency graph
            let graph = spin_up::analysis::graph::build_dependency_graph(&resolve_result.registry);
            diagnostics.merge(graph.diagnostics);

            // Report results
            if diagnostics.is_ok() {
                println!("\u{2713} No errors found");
            } else {
                let reports = diagnostics.into_reports(&resolve_result.sources);
                for report in &reports {
                    eprintln!("{:?}", miette::Report::new(report.clone()));
                }
                std::process::exit(1);
            }
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
