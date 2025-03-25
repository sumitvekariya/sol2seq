use anyhow::Result;
use clap::Parser;
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
    /// AST JSON file path
    #[clap(value_parser)]
    ast_file: PathBuf,

    /// Output file path (optional, will print to stdout if not provided)
    #[clap(value_parser)]
    output_file: Option<PathBuf>,

    /// Use lighter colors for diagram
    #[clap(long, short, action)]
    light_colors: bool,

    /// Solidity source files to process directly
    #[clap(long, short = 's', value_parser, conflicts_with = "ast_file")]
    source_files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let has_output_file = args.output_file.is_some();

    // Create configuration
    let config = Config { light_colors: args.light_colors, output_file: args.output_file };

    // Generate the diagram
    let diagram = if !args.source_files.is_empty() {
        sol2seq::generate_diagram_from_sources(&args.source_files, config)?
    } else {
        sol2seq::generate_diagram_from_file(&args.ast_file, config)?
    };

    // If no output file specified, print to stdout
    if !has_output_file {
        println!("{}", diagram);
    } else {
        println!("Sequence diagram generated successfully!");
    }

    Ok(())
}
