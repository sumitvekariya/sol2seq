use anyhow::Result;
use serde_json::Value;

/// Extract a type name from an AST type node
pub fn extract_type_name(type_node: &Value) -> String {
    if type_node.is_null() || !type_node.is_object() {
        return "unknown".to_string();
    }

    let node_type = type_node["nodeType"].as_str().unwrap_or("");

    match node_type {
        "ElementaryTypeName" => type_node["name"].as_str().unwrap_or("unknown").to_string(),
        "UserDefinedTypeName" => {
            // Check if it has a path representation for contract types
            if type_node.get("pathNode").is_some() {
                type_node["pathNode"]["name"]
                    .as_str()
                    .or_else(|| type_node["name"].as_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                type_node["name"].as_str().unwrap_or("unknown").to_string()
            }
        }
        "ArrayTypeName" => {
            let base_type = extract_type_name(&type_node["baseType"]);

            // Check if it's a fixed size array
            if type_node.get("length").is_some() {
                // Fix the temporary value issue
                if let Some(length_str) = type_node["length"]["value"].as_str() {
                    return format!("{}[{}]", base_type, length_str);
                } else if let Some(length_num) = type_node["length"]["value"].as_u64() {
                    return format!("{}[{}]", base_type, length_num);
                }
            }

            format!("{}[]", base_type)
        }
        "Mapping" => {
            let key_type = extract_type_name(&type_node["keyType"]);
            let value_type = extract_type_name(&type_node["valueType"]);
            format!("mapping({}=>{})", key_type, value_type)
        }
        "TupleType" => {
            if let Some(components) = type_node["components"].as_array() {
                let component_types: Vec<String> =
                    components.iter().map(extract_type_name).collect();
                format!("({})", component_types.join(", "))
            } else {
                "tuple".to_string()
            }
        }
        "FunctionTypeName" => "function".to_string(),
        "AddressType" => {
            if type_node["stateMutability"].as_str() == Some("payable") {
                "address payable".to_string()
            } else {
                "address".to_string()
            }
        }
        _ => {
            // Try to extract from typeDescriptions if available
            if let Some(type_descriptions) = type_node.get("typeDescriptions") {
                if let Some(type_string) =
                    type_descriptions.get("typeString").and_then(|ts| ts.as_str())
                {
                    return type_string.to_string();
                }
            }

            "unknown".to_string()
        }
    }
}

/// Extract return type information from a function definition
pub fn extract_return_type(function_node: &Value) -> Option<String> {
    if let Some(return_parameters) = function_node.get("returnParameters") {
        if let Some(parameters) = return_parameters.get("parameters").and_then(|p| p.as_array()) {
            if parameters.is_empty() {
                return None;
            }

            let mut return_types = Vec::new();
            let mut return_names = Vec::new();

            for param in parameters {
                // Get type information
                let mut param_type = "unknown".to_string();

                // First try from typeName
                if param.get("typeName").is_some() {
                    param_type = extract_type_name(&param["typeName"]);
                }

                // If still unknown, try from typeDescriptions
                if param_type == "unknown" {
                    if let Some(type_desc) = param.get("typeDescriptions") {
                        if let Some(type_str) =
                            type_desc.get("typeString").and_then(|ts| ts.as_str())
                        {
                            param_type = type_str.to_string();
                        }
                    }
                }

                return_types.push(param_type);

                // Get return parameter name if available
                if let Some(name) = param.get("name").and_then(|n| n.as_str()) {
                    if !name.is_empty() {
                        return_names.push(name.to_string());
                    }
                }
            }

            if !return_types.is_empty() {
                // If we have return names, include them
                if !return_names.is_empty() && return_names.len() == return_types.len() {
                    let combined: Vec<String> = return_names
                        .iter()
                        .zip(return_types.iter())
                        .map(|(name, typ)| format!("{}: {}", name, typ))
                        .collect();
                    return Some(combined.join(", "));
                } else {
                    return Some(return_types.join(", "));
                }
            }
        }
    }

    None
}

/// Get a description of a function based on its name
pub fn get_function_purpose(function_name: &str) -> Option<String> {
    let common_functions = [
        ("constructor", "Contract initialization"),
        ("transfer", "Transfer tokens or ETH"),
        ("approve", "Approve token spending"),
        ("mint", "Create new tokens"),
        ("burn", "Destroy tokens"),
        ("deposit", "Deposit funds"),
        ("withdraw", "Withdraw funds"),
        ("claim", "Claim rewards or tokens"),
        ("stake", "Stake tokens"),
        ("unstake", "Unstake tokens"),
        ("vote", "Cast vote"),
        ("execute", "Execute operation"),
        ("deploy", "Deploy new contract instance"),
        ("predictAddress", "Calculate deterministic address"),
        ("airdrop", "Distribute tokens to addresses"),
        ("airdropToAddresses", "Send ETH to multiple addresses"),
        ("airdropToKeyIds", "Send ETH to wallets identified by public keys"),
    ];

    for (key, description) in common_functions.iter() {
        if function_name.to_lowercase().contains(&key.to_lowercase()) {
            return Some(description.to_string());
        }
    }

    None
}

/// Determine if a variable is important enough to include in the contract description
pub fn is_important_variable(var_name: &str) -> bool {
    let important_prefixes =
        ["owner", "admin", "token", "deployer", "implementation", "registry", "factory"];

    for prefix in important_prefixes.iter() {
        if var_name.to_lowercase().contains(prefix) {
            return true;
        }
    }

    false
}

/// Guess the type of a variable based on its name
pub fn guess_type_from_name(name: &str) -> String {
    if name.starts_with("is") || name.starts_with("has") {
        "bool".to_string()
    } else if name.to_lowercase().contains("amount")
        || name.to_lowercase().contains("value")
        || name.to_lowercase().contains("balance")
    {
        "uint256".to_string()
    } else if name.to_lowercase().contains("address") || name.ends_with("Addr") {
        "address".to_string()
    } else if name.to_lowercase().contains("id") {
        "bytes32".to_string()
    } else if name.to_lowercase().contains("key") {
        "bytes".to_string()
    } else {
        "any".to_string()
    }
}

/// Get the type of a literal value
pub fn get_literal_type(literal: &Value) -> String {
    if let Some(kind) = literal.get("kind").and_then(|k| k.as_str()) {
        match kind {
            "number" => "uint256".to_string(),
            "string" => "string".to_string(),
            "bool" => "bool".to_string(),
            _ => "any".to_string(),
        }
    } else {
        "any".to_string()
    }
}

/// Merge two AST JSON objects
///
/// This function combines two AST JSON objects into one, merging arrays and objects.
///
/// # Arguments
///
/// * `target` - The target AST JSON that will be modified
/// * `source` - The source AST JSON that will be merged into the target
///
/// # Returns
///
/// Result indicating success or failure
pub fn merge_ast_json(target: &mut Value, source: &Value) -> Result<()> {
    if let (Value::Object(target_obj), Value::Object(source_obj)) = (target, source) {
        for (key, value) in source_obj {
            if !target_obj.contains_key(key) {
                // If the key doesn't exist in target, simply insert the value
                target_obj.insert(key.clone(), value.clone());
            } else {
                match (target_obj.get_mut(key).unwrap(), value) {
                    (Value::Array(target_arr), Value::Array(source_arr)) => {
                        // If both are arrays, append source array items to target
                        target_arr.extend_from_slice(source_arr);
                    }
                    (Value::Object(target_inner), Value::Object(source_inner)) => {
                        // If both are objects, recursively merge
                        for (inner_key, inner_value) in source_inner {
                            if !target_inner.contains_key(inner_key) {
                                target_inner.insert(inner_key.clone(), inner_value.clone());
                            } else {
                                let mut temp_value = target_inner.get(inner_key).unwrap().clone();
                                if let (Value::Array(temp_arr), Value::Array(source_inner_arr)) =
                                    (&mut temp_value, inner_value)
                                {
                                    temp_arr.extend_from_slice(source_inner_arr);
                                    target_inner.insert(inner_key.clone(), temp_value);
                                } else {
                                    // For conflicts, prefer the source value
                                    target_inner.insert(inner_key.clone(), inner_value.clone());
                                }
                            }
                        }
                    }
                    (_, _) => {
                        // For other types, prefer the source value
                        target_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    Ok(())
}
