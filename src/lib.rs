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
use std::{fs, path::{Path, PathBuf}};

/// Recursively find all Solidity files in a directory
fn find_solidity_files(dir_path: &Path) -> Result<Vec<PathBuf>> {
    let mut sol_files = Vec::new();
    
    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory: {}", dir_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recursively search subdirectories
                let mut sub_files = find_solidity_files(&path)?;
                sol_files.append(&mut sub_files);
            } else if let Some(ext) = path.extension() {
                // Check if file has .sol extension
                if ext == "sol" {
                    sol_files.push(path);
                }
            }
        }
    }
    
    Ok(sol_files)
}

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
/// * `source_paths` - Paths to Solidity source files or directories
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
/// let source_paths = vec!["Contract.sol", "Library.sol"];
/// match generate_diagram_from_sources(&source_paths, config) {
///     Ok(diagram) => println!("Generated diagram: {}", diagram),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn generate_diagram_from_sources<P: AsRef<std::path::Path>>(
    source_paths: &[P],
    config: Config,
) -> Result<String> {
    // Process each Solidity file and combine ASTs
    let mut combined_ast = serde_json::Value::Object(serde_json::Map::new());
    let mut all_source_files = Vec::new();

    // First, collect all Solidity files from provided paths (could be files or directories)
    for path in source_paths {
        let path = path.as_ref();
        if path.is_dir() {
            // If it's a directory, find all Solidity files inside it
            let mut sol_files = find_solidity_files(path)?;
            all_source_files.append(&mut sol_files);
        } else {
            // If it's a file, add it directly (assuming it's a Solidity file)
            all_source_files.push(path.to_path_buf());
        }
    }

    if all_source_files.is_empty() {
        return Err(anyhow::anyhow!("No Solidity files found in the provided paths"));
    }

    // Process each Solidity file and combine ASTs
    for file_path in &all_source_files {
        let file_str = file_path.to_str().ok_or_else(|| {
            anyhow::anyhow!("Failed to convert path to string: {}", file_path.display())
        })?;
        
        let ast = ast::process_solidity_file(file_str)?;

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

// Re-export types for public API
pub use diagram::generate_sequence_diagram;
pub use types::{
    ContractInfo, ContractRelationship, DiagramData, Interaction, InteractionType, Parameter,
    StateVariable,
};
