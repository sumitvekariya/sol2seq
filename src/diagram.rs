use crate::{ast::extract_contract_info, types::*, utils::*};
use anyhow::{Context, Result};
use itertools::Itertools;
use serde_json::Value;
use std::collections::HashSet;

/// Generate a Mermaid sequence diagram from AST JSON
pub fn generate_sequence_diagram(ast: &Value, light_colors: bool) -> Result<String> {
    // Extract contract information
    let data = extract_contract_info(ast)?;

    // Generate diagram content
    let mut diagram = Vec::new();

    // Start diagram
    diagram.push("```mermaid".to_string());
    diagram.push("sequenceDiagram".to_string());
    diagram.push("title Smart Contract Interaction Sequence Diagram".to_string());
    diagram.push("autonumber".to_string());
    diagram.push("".to_string());

    // Add visual styling with theme
    add_theme_config(&mut diagram, light_colors);

    // Format participants for the diagram - ensure User is first
    let ordered_participants = order_participants(&data.participants);

    // Create the participant declarations with descriptions
    add_participants(&mut diagram, &ordered_participants, &data.contracts);

    // Add a blank line
    diagram.push("".to_string());

    // Add title and section separators
    add_section_title(&mut diagram, "User Interactions", light_colors);

    // Add user interactions
    diagram.extend(data.user_interactions);

    // Add contract interactions
    if !data.contract_interactions.is_empty() {
        diagram.push("".to_string());
        add_section_title(&mut diagram, "Contract-to-Contract Interactions", light_colors);

        // Add contract interactions grouped by function
        for (function_key, interactions_list) in data.contract_interactions.iter() {
            if !interactions_list.is_empty() {
                let parts: Vec<&str> = function_key.split('.').collect();
                if parts.len() == 2 {
                    let (contract, function) = (parts[0], parts[1]);
                    diagram.push(format!("Note right of {}: Processing {}", contract, function));
                    diagram.extend(interactions_list.clone());
                    diagram.push("".to_string()); // Add spacing
                }
            }
        }
    }

    // Add event notes
    if !data.events.is_empty() {
        diagram.push("".to_string());
        add_section_title(&mut diagram, "Event Definitions", light_colors);

        for (contract, event) in &data.events {
            diagram.push(format!("Note over {},{}: Event: {}", contract, contract, event));
        }
    }

    // Add contract overview/relationships
    if !data.contracts.is_empty() {
        diagram.push("".to_string());
        add_section_title(&mut diagram, "Contract Relationships", light_colors);

        // Add function summaries
        for (contract_name, info) in &data.contracts {
            if !info.functions.is_empty() {
                let functions_str = info.functions.join(", ");
                diagram.push(format!("Note over {}: Functions: {}", contract_name, functions_str));
            }
        }

        diagram.push("".to_string());

        // Add inheritance relationships
        for (contract_name, info) in &data.contracts {
            if !info.inherits_from.is_empty() {
                let bases_str = info.inherits_from.join(", ");
                diagram
                    .push(format!("Note right of {}: Inherits from: {}", contract_name, bases_str));
            }
        }

        // Add contract type information
        diagram.push("".to_string());
        for (contract_name, info) in &data.contracts {
            if info.contract_type != "contract" {
                diagram
                    .push(format!("Note right of {}: Type: {}", contract_name, info.contract_type));
            }
        }

        // Add contract dependencies/interactions
        if !data.contract_relationships.is_empty() {
            diagram.push("".to_string());
            let mut seen_relationships = HashSet::new();

            for rel in &data.contract_relationships {
                if rel.relation_type == "calls"
                    && data.participants.contains(&rel.source)
                    && data.participants.contains(&rel.target)
                {
                    let rel_key = format!("{}->{}", rel.source, rel.target);
                    if !seen_relationships.contains(&rel_key) {
                        diagram.push(format!(
                            "Note right of {}: Interacts with {}",
                            rel.source, rel.target
                        ));
                        seen_relationships.insert(rel_key);
                    }
                }
            }
        }
    }

    // Add a legend at the end
    add_legend(&mut diagram, light_colors);

    // Close the diagram
    diagram.push("```".to_string());

    Ok(diagram.join("\n"))
}

/// Add theme configuration to the diagram
fn add_theme_config(diagram: &mut Vec<String>, light_colors: bool) {
    diagram.push("%%{init: {".to_string());
    diagram.push("  'theme': 'base',".to_string());
    diagram.push("  'themeVariables': {".to_string());

    if light_colors {
        // Lighter theme
        diagram.push("    'primaryColor': '#fafbfc',".to_string());
        diagram.push("    'primaryTextColor': '#444',".to_string());
        diagram.push("    'primaryBorderColor': '#e1e4e8',".to_string());
        diagram.push("    'lineColor': '#a0aec0',".to_string());
        diagram.push("    'secondaryColor': '#f5fbff',".to_string());
        diagram.push("    'tertiaryColor': '#fff8f8'".to_string());
    } else {
        // Default theme
        diagram.push("    'primaryColor': '#f5f5f5',".to_string());
        diagram.push("    'primaryTextColor': '#333',".to_string());
        diagram.push("    'primaryBorderColor': '#999',".to_string());
        diagram.push("    'lineColor': '#666',".to_string());
        diagram.push("    'secondaryColor': '#f0f8ff',".to_string());
        diagram.push("    'tertiaryColor': '#fff5f5'".to_string());
    }

    diagram.push("  }".to_string());
    diagram.push("}}%%".to_string());
    diagram.push("".to_string());
}

/// Order participants in a logical sequence
fn order_participants(participants: &HashSet<String>) -> Vec<String> {
    let mut ordered = Vec::new();

    // User always first
    if participants.contains("User") {
        ordered.push("User".to_string());
    }

    // Then add other participants in sorted order (except Events which comes last)
    for participant in participants.iter().sorted() {
        if participant != "User" && participant != "Events" {
            ordered.push(participant.clone());
        }
    }

    // Add Events last
    if participants.contains("Events") {
        ordered.push("Events".to_string());
    }

    ordered
}

/// Add participants to the diagram
fn add_participants(
    diagram: &mut Vec<String>,
    ordered_participants: &[String],
    contracts: &std::collections::HashMap<String, ContractInfo>,
) {
    for participant in ordered_participants {
        if participant == "User" {
            diagram.push("participant User as \"External User\"".to_string());
        } else if participant == "Events" {
            diagram.push("participant Events as \"Blockchain Events\"".to_string());
        } else if participant == "TokenContract" {
            diagram.push("participant TokenContract as \"ERC20/ERC721 Tokens\"".to_string());
        } else {
            // Add contract description if available
            if let Some(contract_info) = contracts.get(participant) {
                // Extract key state variables for description
                let key_vars: Vec<&(String, String)> = contract_info
                    .variables
                    .iter()
                    .filter(|(name, _)| is_important_variable(name))
                    .collect();

                let mut description_parts = Vec::new();

                // Add contract name (always)
                description_parts.push(participant.clone());

                // Add contract type if it's not a standard contract
                if contract_info.contract_type != "contract" {
                    description_parts[0] =
                        format!("{} ({})", participant, contract_info.contract_type);
                }

                // Add key variables if available
                if !key_vars.is_empty() {
                    let var_list: Vec<String> = key_vars
                        .iter()
                        .take(2)
                        .map(|(name, typ)| format!("{}: {}", name, typ))
                        .collect();
                    description_parts.push(format!("({})", var_list.join(", ")));
                }

                // Add source file if available
                if !contract_info.source_file.is_empty() {
                    description_parts.push(format!("from {}", contract_info.source_file));
                }

                // Combine the parts with line breaks
                let title = description_parts.join("<br/>");
                diagram.push(format!("participant {} as \"{}\"", participant, title));
            } else {
                diagram.push(format!("participant {}", participant));
            }
        }
    }
}

/// Add a section title to the diagram
fn add_section_title(diagram: &mut Vec<String>, title: &str, light_colors: bool) {
    let color = if light_colors {
        match title {
            "User Interactions" => "rgb(252, 252, 255)",
            "Contract-to-Contract Interactions" => "rgb(248, 252, 255)",
            "Event Definitions" => "rgb(255, 252, 252)",
            "Contract Relationships" => "rgb(252, 255, 252)",
            _ => "rgb(250, 250, 250)",
        }
    } else {
        match title {
            "User Interactions" => "rgb(245, 245, 245)",
            "Contract-to-Contract Interactions" => "rgb(240, 248, 255)",
            "Event Definitions" => "rgb(255, 245, 245)",
            "Contract Relationships" => "rgb(245, 255, 245)",
            _ => "rgb(240, 240, 240)",
        }
    };

    diagram.push(format!("rect {}", color));
    diagram.push(format!("Note over User: {}", title));
    diagram.push("end".to_string());
    diagram.push("".to_string());
}

/// Add a legend to the diagram
fn add_legend(diagram: &mut Vec<String>, light_colors: bool) {
    diagram.push("".to_string());
    diagram.push("%%{init: { 'sequence': { 'showSequenceNumbers': true } }}%%".to_string());
    diagram.push("".to_string());

    let legend_color = if light_colors { "rgb(248, 252, 255)" } else { "rgb(240, 240, 255)" };

    diagram.push(format!("rect {}", legend_color));
    diagram.push("Note over User: Diagram Legend".to_string());
    diagram.push("end".to_string());
    diagram.push("".to_string());

    diagram.push("Note left of User: User→Contract: Public/External function calls".to_string());
    diagram.push("Note left of User: User←Contract: Function returns".to_string());
    diagram.push("Note left of User: Contract→Contract: Internal interactions".to_string());
    diagram.push("Note left of User: Contract→Events: Emitted events".to_string());
    diagram.push(
        "Note left of User: Colored sections indicate different interaction types".to_string(),
    );
}
