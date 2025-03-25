/*!
# sol2seq

A library for generating sequence diagrams from Solidity smart contracts.

This crate can be used as:
- A library in your Rust projects
- A command-line tool for generating diagrams

## Library Usage

```rust
use anyhow::Result;
use sol2seq::{generate_diagram_from_file, Config};

fn main() -> Result<()> {
    // Create a configuration
    let config = Config {
        light_colors: false,
        output_file: Some("diagram.md".into()),
    };

    // Generate diagram from AST file
    generate_diagram_from_file("path/to/ast.json", config)?;

    println!("Diagram generated successfully!");
    Ok(())
}
```

## CLI Usage

```bash
# Generate a diagram from an AST JSON file
sol2seq path/to/ast.json output.md

# Use lighter colors
sol2seq --light-colors path/to/ast.json output.md
```
*/

mod ast;
mod diagram;
mod types;
mod utils;

use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

// Re-export types for public API
pub use diagram::generate_sequence_diagram;
pub use types::{
    ContractInfo, ContractRelationship, DiagramData, Interaction, InteractionType, Parameter,
    StateVariable,
};

/// Configuration for diagram generation
#[derive(Debug, Clone)]
pub struct Config {
    /// Use lighter colors for the diagram
    pub light_colors: bool,

    /// Output file path (None for stdout)
    pub output_file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self { light_colors: false, output_file: None }
    }
}

/// Generate a sequence diagram from an AST JSON file
///
/// # Arguments
///
/// * `ast_file` - Path to the AST JSON file
/// * `config` - Configuration for diagram generation
///
/// # Returns
///
/// The generated diagram as a string
///
/// # Example
///
/// ```no_run
/// use sol2seq::{Config, generate_diagram_from_file};
///
/// let config = Config::default();
/// match generate_diagram_from_file("ast.json", config) {
///     Ok(diagram) => println!("Generated diagram: {}", diagram),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn generate_diagram_from_file<P: AsRef<std::path::Path>>(
    ast_file: P,
    config: Config,
) -> Result<String> {
    // Load AST file
    let ast_content = fs::read_to_string(&ast_file)
        .with_context(|| format!("Failed to read AST file: {}", ast_file.as_ref().display()))?;

    // Parse JSON
    let ast_json: serde_json::Value =
        serde_json::from_str(&ast_content).with_context(|| "Failed to parse AST JSON")?;

    // Generate sequence diagram
    let diagram = generate_sequence_diagram(&ast_json, config.light_colors)?;

    // Save to file if specified
    if let Some(output_path) = config.output_file {
        fs::write(&output_path, &diagram)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    }

    Ok(diagram)
}

/// Generate a sequence diagram from Solidity source files
///
/// # Arguments
///
/// * `source_files` - Paths to Solidity source files
/// * `config` - Configuration for diagram generation
///
/// # Returns
///
/// The generated diagram as a string
///
/// # Example
///
/// ```no_run
/// use sol2seq::{Config, generate_diagram_from_sources};
///
/// let config = Config::default();
/// let source_files = vec!["Contract.sol", "Library.sol"];
/// match generate_diagram_from_sources(&source_files, config) {
///     Ok(diagram) => println!("Generated diagram: {}", diagram),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn generate_diagram_from_sources<P: AsRef<std::path::Path>>(
    source_files: &[P],
    config: Config,
) -> Result<String> {
    // Process each Solidity file and combine ASTs
    let mut combined_ast = serde_json::Value::Object(serde_json::Map::new());

    for file_path in source_files {
        let ast = ast::process_solidity_file(file_path.as_ref().to_str().unwrap())?;

        // Merge with combined AST
        utils::merge_ast_json(&mut combined_ast, &ast)?;
    }

    // Generate sequence diagram
    let diagram = generate_sequence_diagram(&combined_ast, config.light_colors)?;

    // Save to file if specified
    if let Some(output_path) = config.output_file {
        fs::write(&output_path, &diagram)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    }

    Ok(diagram)
}
