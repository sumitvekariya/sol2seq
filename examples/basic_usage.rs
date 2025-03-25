use anyhow::Result;
use sol2seq::{generate_diagram_from_file, Config};
use std::path::PathBuf;

fn main() -> Result<()> {
    // Create a configuration
    let config = Config { light_colors: true, output_file: Some(PathBuf::from("diagram.md")) };

    // Generate diagram from AST file
    // Replace "path/to/ast.json" with an actual file path to test
    generate_diagram_from_file("path/to/ast.json", config)?;

    println!("Diagram generated successfully!");
    Ok(())
}
