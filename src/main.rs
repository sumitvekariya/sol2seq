use anyhow::Result;
use clap::{Parser, Subcommand};
use sol2seq::Config;
use std::path::PathBuf;

/// Solidity Sequence Diagram Generator
///
/// Generate sequence diagrams from Solidity smart contracts
#[derive(Parser, Debug)]
#[clap(
    name = "sol2seq",
    about = "Generate sequence diagrams from Solidity smart contracts",
    version,
    author = "Cyfrin"
)]
struct Args {
    #[clap(subcommand)]
    command: Commands,

    /// Use lighter colors for diagram
    #[clap(long, short, action)]
    light_colors: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate diagram from AST JSON file
    Ast {
        /// AST JSON file path
        ast_file: PathBuf,
        /// Output file path (optional, will print to stdout if not provided)
        output_file: Option<PathBuf>,
    },
    /// Generate diagram from Solidity source files
    Source {
        /// Solidity source files to process
        #[clap(required = true)]
        source_files: Vec<PathBuf>,
        /// Output file path (optional, will print to stdout if not provided)
        output_file: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let has_output_file = match &args.command {
        Commands::Ast { output_file, .. } => output_file.is_some(),
        Commands::Source { output_file, .. } => output_file.is_some(),
    };

    // Create configuration
    let config = Config {
        light_colors: args.light_colors,
        output_file: match &args.command {
            Commands::Ast { output_file, .. } => output_file.clone(),
            Commands::Source { output_file, .. } => output_file.clone(),
        },
    };

    // Generate the diagram
    let diagram = match args.command {
        Commands::Ast { ast_file, .. } => {
            sol2seq::generate_diagram_from_file(ast_file, config)?
        }
        Commands::Source { source_files, .. } => {
            sol2seq::generate_diagram_from_sources(&source_files, config)?
        }
    };

    // If no output file specified, print to stdout
    if !has_output_file {
        println!("{}", diagram);
    } else {
        println!("Sequence diagram generated successfully!");
    }

    Ok(())
}
