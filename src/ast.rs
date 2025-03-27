use crate::{types::*, utils::*};
use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

/// Parse AST JSON and extract contract information
pub fn extract_contract_info(ast: &Value) -> Result<DiagramData> {
    let mut data = DiagramData::default();

    // Handle combined-json format
    if let Some(sources) = ast.get("sources") {
        for (_file_path, source) in sources.as_object().with_context(|| "sources is not an object")? {
            if let Some(source_ast) = source.get("AST") {
                // First pass: collect all contracts, state variables, and events
                collect_contracts_and_variables(source_ast, &mut data)?;

                // Add default participants
                data.participants.insert("User".to_string());
                data.participants.insert("Events".to_string());
                data.participants.insert("TokenContract".to_string());

                // Second pass: analyze function calls and interactions
                process_functions_and_interactions(source_ast, &mut data)?;
            }
        }
    } else {
        // Handle legacy format
        // First pass: collect all contracts, state variables, and events
        collect_contracts_and_variables(ast, &mut data)?;

        // Add default participants
        data.participants.insert("User".to_string());
        data.participants.insert("Events".to_string());
        data.participants.insert("TokenContract".to_string());

        // Second pass: analyze function calls and interactions
        process_functions_and_interactions(ast, &mut data)?;
    }

    Ok(data)
}

/// Process source units to collect contracts and variables
fn collect_contracts_and_variables(ast: &Value, data: &mut DiagramData) -> Result<()> {
    let nodes = ast["nodes"].as_array().with_context(|| "nodes is not an array")?;

    for node in nodes {
        if node["nodeType"].as_str() == Some("ContractDefinition") {
            let contract_name = node["name"].as_str().unwrap_or("Unknown").to_string();

            data.participants.insert(contract_name.clone());

            // Create contract info
            let mut contract_info = ContractInfo {
                name: contract_name.clone(),
                contract_type: node["contractKind"].as_str().unwrap_or("contract").to_string(),
                source_file: ast["absolutePath"].as_str().unwrap_or("unknown").to_string(),
                ..Default::default()
            };

            // Check inheritance
            if let Some(base_contracts) = node["baseContracts"].as_array() {
                for base in base_contracts {
                    if let Some(base_name) = base
                        .get("baseName")
                        .and_then(|bn| bn.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        contract_info.inherits_from.push(base_name.to_string());
                        data.contract_relationships.push(ContractRelationship {
                            source: contract_name.clone(),
                            target: base_name.to_string(),
                            relation_type: "inherits".to_string(),
                        });
                    }
                }
            }

            // Collect events and state variables
            if let Some(contract_nodes) = node["nodes"].as_array() {
                for contract_node in contract_nodes {
                    let node_type = contract_node["nodeType"].as_str().unwrap_or("");

                    match node_type {
                        "EventDefinition" => {
                            let event_name = contract_node["name"]
                                .as_str()
                                .unwrap_or("UnknownEvent")
                                .to_string();
                            data.events.push((contract_name.clone(), event_name.clone()));
                            contract_info.events.push(event_name);
                        }
                        "VariableDeclaration" => {
                            let var_name =
                                contract_node["name"].as_str().unwrap_or("unknown").to_string();
                            let var_type = extract_type_name(&contract_node["typeName"]);

                            contract_info.variables.push((var_name.clone(), var_type.clone()));

                            // Check if this creates a relationship with another contract
                            if data.participants.contains(&var_type)
                                || var_type.to_lowercase().contains("address")
                            {
                                data.contract_relationships.push(ContractRelationship {
                                    source: contract_name.clone(),
                                    target: var_type.clone(),
                                    relation_type: "references".to_string(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Store the contract info
            data.contracts.insert(contract_name, contract_info);
        }
    }

    Ok(())
}

/// Process functions and extract interactions
fn process_functions_and_interactions(ast: &Value, data: &mut DiagramData) -> Result<()> {
    let nodes = ast["nodes"].as_array().with_context(|| "nodes is not an array")?;

    for node in nodes {
        if node["nodeType"].as_str() == Some("ContractDefinition") {
            let contract_name = node["name"].as_str().unwrap_or("Unknown").to_string();

            // Process functions
            if let Some(contract_nodes) = node["nodes"].as_array() {
                for contract_node in contract_nodes {
                    if contract_node["nodeType"].as_str() == Some("FunctionDefinition") {
                        let function_name = if let Some(name) = contract_node["name"].as_str() {
                            if name.is_empty()
                                && contract_node["kind"].as_str() == Some("constructor")
                            {
                                "constructor".to_string()
                            } else {
                                name.to_string()
                            }
                        } else {
                            continue;
                        };

                        // Store function info
                        if let Some(contract_info) = data.contracts.get_mut(&contract_name) {
                            contract_info.functions.push(function_name.clone());
                        }

                        // Add interaction from user to public/external functions
                        let visibility = contract_node["visibility"].as_str().unwrap_or("");
                        if visibility == "public" || visibility == "external" {
                            // Extract function parameters
                            let mut params = Vec::new();
                            let mut param_types = Vec::new();

                            if let Some(parameters) = contract_node
                                .get("parameters")
                                .and_then(|p| p.get("parameters"))
                                .and_then(|p| p.as_array())
                            {
                                for param in parameters {
                                    let param_name =
                                        param["name"].as_str().unwrap_or("").to_string();

                                    // Extract parameter type
                                    let mut param_type = "unknown".to_string();
                                    if param.get("typeName").is_some() {
                                        param_type = extract_type_name(&param["typeName"]);
                                    }

                                    // Try to get type from typeDescriptions if still unknown
                                    if param_type == "unknown" {
                                        if let Some(type_desc) = param.get("typeDescriptions") {
                                            if let Some(type_str) = type_desc
                                                .get("typeString")
                                                .and_then(|ts| ts.as_str())
                                            {
                                                param_type = type_str.to_string();
                                            }
                                        }
                                    }

                                    if !param_name.is_empty() {
                                        params.push(param_name);
                                        param_types.push(param_type);
                                    }
                                }
                            }

                            // Create message with parameter types
                            let message = if params.is_empty() {
                                format!("{}()", function_name)
                            } else {
                                let param_type_str: Vec<String> = params
                                    .iter()
                                    .zip(param_types.iter())
                                    .map(|(name, typ)| format!("{}: {}", name, typ))
                                    .collect();
                                format!("{}({})", function_name, param_type_str.join(", "))
                            };

                            // Add note about function purpose
                            let function_purpose = get_function_purpose(&function_name);
                            if let Some(purpose) = function_purpose {
                                data.user_interactions.push(format!(
                                    "Note over User,{}: {}",
                                    contract_name, purpose
                                ));
                            }

                            // Add user interaction
                            data.user_interactions
                                .push(format!("User->>+{}: {}", contract_name, message));

                            // Process function body for internal interactions
                            if let Some(body) = contract_node.get("body") {
                                if let Some(statements) =
                                    body.get("statements").and_then(|s| s.as_array())
                                {
                                    let function_key =
                                        format!("{}.{}", contract_name, function_name);
                                    let body_interactions = process_function_body(
                                        &contract_name,
                                        &function_name,
                                        statements,
                                    );
                                    data.contract_interactions
                                        .insert(function_key, body_interactions);
                                }
                            }

                            // Add return value
                            let return_type = extract_return_type(contract_node);
                            if let Some(ret_type) = return_type {
                                data.user_interactions.push(format!(
                                    "{}-->>-User: return {}",
                                    contract_name, ret_type
                                ));
                            } else {
                                // Check for view/pure functions
                                let state_mutability =
                                    contract_node["stateMutability"].as_str().unwrap_or("");
                                if state_mutability == "view" || state_mutability == "pure" {
                                    data.user_interactions.push(format!(
                                        "{}-->>-User: return (view function)",
                                        contract_name
                                    ));
                                } else {
                                    data.user_interactions
                                        .push(format!("{}-->>-User: return", contract_name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process a function body and extract interactions
fn process_function_body(
    contract_name: &str,
    function_name: &str,
    statements: &[Value],
) -> Vec<String> {
    let mut interactions = Vec::new();

    for statement in statements {
        let node_type = statement["nodeType"].as_str().unwrap_or("");

        match node_type {
            "ForStatement" => {
                // Handle for loops
                let mut loop_description = "For each item".to_string();

                if let Some(init_expr) = statement.get("initializationExpression") {
                    if let Some(declarations) =
                        init_expr.get("declarations").and_then(|d| d.as_array())
                    {
                        for decl in declarations {
                            if let Some(var_name) = decl.get("name").and_then(|n| n.as_str()) {
                                loop_description = format!("For each {}", var_name);

                                // Try to get type information
                                if let Some(type_name) = decl.get("typeName") {
                                    let loop_var_type = extract_type_name(type_name);
                                    if loop_var_type != "unknown" {
                                        loop_description =
                                            format!("For each {}: {}", var_name, loop_var_type);
                                    }
                                }
                            }
                        }
                    }
                }

                // Start loop block
                interactions.push(format!("loop {}", loop_description));

                // Process loop body
                if let Some(body) = statement.get("body") {
                    if let Some(body_statements) = body.get("statements").and_then(|s| s.as_array())
                    {
                        let loop_body =
                            process_function_body(contract_name, function_name, body_statements);
                        for line in loop_body {
                            interactions.push(format!("    {}", line));
                        }
                    } else if body.get("nodeType").is_some() {
                        // Handle single statement body
                        let loop_body =
                            process_function_body(contract_name, function_name, &[body.clone()]);
                        for line in loop_body {
                            interactions.push(format!("    {}", line));
                        }
                    }
                }

                // End loop block
                interactions.push("end".to_string());
            }
            "IfStatement" => {
                // Handle if statements
                let mut condition_description = "if condition".to_string();

                if let Some(condition) = statement.get("condition") {
                    if condition["nodeType"].as_str() == Some("BinaryOperation") {
                        if let (Some(left), Some(right), Some(op)) = (
                            condition.get("leftExpression"),
                            condition.get("rightExpression"),
                            condition.get("operator").and_then(|o| o.as_str()),
                        ) {
                            if let Some(left_name) = left.get("name").and_then(|n| n.as_str()) {
                                // Fix the temporary reference issue
                                if let Some(right_val_str) =
                                    right.get("value").and_then(|v| v.as_str())
                                {
                                    condition_description =
                                        format!("if {} {} {}", left_name, op, right_val_str);
                                } else if let Some(right_val_num) =
                                    right.get("value").and_then(|v| v.as_u64())
                                {
                                    condition_description =
                                        format!("if {} {} {}", left_name, op, right_val_num);
                                }
                            }
                        }
                    }
                }

                interactions.push(format!("alt {}", condition_description));

                // Process true body
                if let Some(true_body) = statement.get("trueBody") {
                    if let Some(true_statements) =
                        true_body.get("statements").and_then(|s| s.as_array())
                    {
                        let body =
                            process_function_body(contract_name, function_name, true_statements);
                        for line in body {
                            interactions.push(format!("    {}", line));
                        }
                    } else if true_body.get("nodeType").is_some() {
                        let body = process_function_body(contract_name, function_name, &[
                            true_body.clone(),
                        ]);
                        for line in body {
                            interactions.push(format!("    {}", line));
                        }
                    }
                }

                // Process false body
                if let Some(false_body) = statement.get("falseBody") {
                    if false_body.is_object() {
                        interactions.push("else".to_string());

                        if let Some(false_statements) =
                            false_body.get("statements").and_then(|s| s.as_array())
                        {
                            let body = process_function_body(
                                contract_name,
                                function_name,
                                false_statements,
                            );
                            for line in body {
                                interactions.push(format!("    {}", line));
                            }
                        } else if false_body.get("nodeType").is_some() {
                            let body = process_function_body(contract_name, function_name, &[
                                false_body.clone(),
                            ]);
                            for line in body {
                                interactions.push(format!("    {}", line));
                            }
                        }
                    }
                }

                interactions.push("end".to_string());
            }
            "EmitStatement" => {
                // Handle event emissions
                if let Some(event_call) = statement.get("eventCall") {
                    if let Some(expression) = event_call.get("expression") {
                        if let Some(event_name) = expression.get("name").and_then(|n| n.as_str()) {
                            let mut args = Vec::new();
                            let mut args_with_types = Vec::new();

                            if let Some(arguments) =
                                event_call.get("arguments").and_then(|a| a.as_array())
                            {
                                for arg in arguments {
                                    if arg["nodeType"].as_str() == Some("Identifier") {
                                        if let Some(arg_name) =
                                            arg.get("name").and_then(|n| n.as_str())
                                        {
                                            args.push(arg_name.to_string());
                                            let arg_type = guess_type_from_name(arg_name);
                                            args_with_types
                                                .push(format!("{}: {}", arg_name, arg_type));
                                        }
                                    } else if arg["nodeType"].as_str() == Some("Literal") {
                                        if let Some(value) = arg.get("value").map(|v| v.to_string())
                                        {
                                            args.push(value.clone());
                                            let literal_type = get_literal_type(arg);
                                            args_with_types
                                                .push(format!("{}: {}", value, literal_type));
                                        }
                                    }
                                }
                            }

                            let arg_str = if !args_with_types.is_empty() {
                                args_with_types.join(", ")
                            } else if !args.is_empty() {
                                args.join(", ")
                            } else {
                                String::new()
                            };

                            interactions.push(format!(
                                "{}->>Events: emit {}({})",
                                contract_name, event_name, arg_str
                            ));
                        }
                    }
                }
            }
            "ExpressionStatement" => {
                // Handle function calls
                if let Some(expression) = statement.get("expression") {
                    if expression["nodeType"].as_str() == Some("FunctionCall") {
                        if let Some(call_expr) = expression.get("expression") {
                            if call_expr["nodeType"].as_str() == Some("MemberAccess") {
                                let member_name =
                                    call_expr["memberName"].as_str().unwrap_or("unknown");

                                if let Some(base_expr) = call_expr.get("expression") {
                                    if base_expr["nodeType"].as_str() == Some("Identifier") {
                                        let target_name =
                                            base_expr["name"].as_str().unwrap_or("Unknown");

                                        // Extract arguments
                                        let mut args = Vec::new();
                                        let mut args_with_types = Vec::new();

                                        if let Some(arguments) =
                                            expression.get("arguments").and_then(|a| a.as_array())
                                        {
                                            for arg in arguments {
                                                if arg["nodeType"].as_str() == Some("Identifier") {
                                                    if let Some(arg_name) =
                                                        arg.get("name").and_then(|n| n.as_str())
                                                    {
                                                        args.push(arg_name.to_string());
                                                        let arg_type =
                                                            guess_type_from_name(arg_name);
                                                        args_with_types.push(format!(
                                                            "{}: {}",
                                                            arg_name, arg_type
                                                        ));
                                                    }
                                                } else if arg["nodeType"].as_str()
                                                    == Some("Literal")
                                                {
                                                    if let Some(value) =
                                                        arg.get("value").map(|v| v.to_string())
                                                    {
                                                        args.push(value.clone());
                                                        let literal_type = get_literal_type(arg);
                                                        args_with_types.push(format!(
                                                            "{}: {}",
                                                            value, literal_type
                                                        ));
                                                    }
                                                }
                                            }
                                        }

                                        let arg_str = if !args_with_types.is_empty() {
                                            args_with_types.join(", ")
                                        } else if !args.is_empty() {
                                            args.join(", ")
                                        } else {
                                            String::new()
                                        };

                                        // Get function purpose
                                        let func_purpose = get_function_purpose(member_name);

                                        // Process based on function type
                                        if member_name == "transfer" || member_name == "send" {
                                            if let Some(purpose) = func_purpose {
                                                interactions.push(format!(
                                                    "Note right of {}: {}",
                                                    contract_name, purpose
                                                ));
                                            }
                                            interactions.push(format!(
                                                "{}->>+{}: {}({})",
                                                contract_name, target_name, member_name, arg_str
                                            ));
                                            interactions.push(format!(
                                                "{}-->>-{}: return (success)",
                                                target_name, contract_name
                                            ));
                                        } else if (member_name == "transferFrom"
                                            || member_name == "transfer")
                                            && target_name.to_lowercase().contains("token")
                                        {
                                            if let Some(purpose) = func_purpose {
                                                interactions.push(format!(
                                                    "Note right of {}: {}",
                                                    contract_name, purpose
                                                ));
                                            }
                                            interactions.push(format!(
                                                "{}->>+TokenContract: {}({})",
                                                contract_name, member_name, arg_str
                                            ));
                                            interactions.push(format!(
                                                "TokenContract-->>-{}: return (success)",
                                                contract_name
                                            ));
                                        } else {
                                            if let Some(purpose) = func_purpose {
                                                interactions.push(format!(
                                                    "Note right of {}: {}",
                                                    contract_name, purpose
                                                ));
                                            }
                                            interactions.push(format!(
                                                "{}->>+{}: {}({})",
                                                contract_name, target_name, member_name, arg_str
                                            ));
                                            interactions.push(format!(
                                                "{}-->>-{}: return",
                                                target_name, contract_name
                                            ));
                                        }
                                    } else if base_expr["nodeType"].as_str() == Some("FunctionCall")
                                        && base_expr.get("kind").and_then(|k| k.as_str())
                                            == Some("typeConversion")
                                    {
                                        // Handle special cases like address(this).balance
                                        let mut args = Vec::new();
                                        let mut args_with_types = Vec::new();

                                        if let Some(arguments) =
                                            expression.get("arguments").and_then(|a| a.as_array())
                                        {
                                            for arg in arguments {
                                                if arg["nodeType"].as_str() == Some("Identifier") {
                                                    if let Some(arg_name) =
                                                        arg.get("name").and_then(|n| n.as_str())
                                                    {
                                                        args.push(arg_name.to_string());
                                                        let arg_type =
                                                            guess_type_from_name(arg_name);
                                                        args_with_types.push(format!(
                                                            "{}: {}",
                                                            arg_name, arg_type
                                                        ));
                                                    }
                                                } else if arg["nodeType"].as_str()
                                                    == Some("Literal")
                                                {
                                                    if let Some(value) =
                                                        arg.get("value").map(|v| v.to_string())
                                                    {
                                                        args.push(value.clone());
                                                        let literal_type = get_literal_type(arg);
                                                        args_with_types.push(format!(
                                                            "{}: {}",
                                                            value, literal_type
                                                        ));
                                                    }
                                                }
                                            }
                                        }

                                        let special_arg_str = if !args_with_types.is_empty() {
                                            args_with_types.join(", ")
                                        } else if !args.is_empty() {
                                            args.join(", ")
                                        } else {
                                            String::new()
                                        };

                                        if member_name == "transfer"
                                            || member_name == "send"
                                            || member_name == "call"
                                        {
                                            interactions.push(format!(
                                                "{}->>+Recipient: ETH {}({})",
                                                contract_name, member_name, special_arg_str
                                            ));
                                            interactions.push(format!(
                                                "Recipient-->>-{}: return (success)",
                                                contract_name
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "VariableDeclarationStatement" => {
                // Handle variable declarations with function calls
                if let Some(init_value) = statement.get("initialValue") {
                    if init_value["nodeType"].as_str() == Some("FunctionCall") {
                        if let Some(call_expr) = init_value.get("expression") {
                            if call_expr["nodeType"].as_str() == Some("MemberAccess") {
                                let member_name =
                                    call_expr["memberName"].as_str().unwrap_or("unknown");

                                if let Some(base_expr) = call_expr.get("expression") {
                                    if base_expr["nodeType"].as_str() == Some("Identifier") {
                                        let target_name =
                                            base_expr["name"].as_str().unwrap_or("Unknown");

                                        // Extract arguments
                                        let mut args = Vec::new();
                                        let mut args_with_types = Vec::new();

                                        if let Some(arguments) =
                                            init_value.get("arguments").and_then(|a| a.as_array())
                                        {
                                            for arg in arguments {
                                                if arg["nodeType"].as_str() == Some("Identifier") {
                                                    if let Some(arg_name) =
                                                        arg.get("name").and_then(|n| n.as_str())
                                                    {
                                                        args.push(arg_name.to_string());
                                                        let arg_type =
                                                            guess_type_from_name(arg_name);
                                                        args_with_types.push(format!(
                                                            "{}: {}",
                                                            arg_name, arg_type
                                                        ));
                                                    }
                                                } else if arg["nodeType"].as_str()
                                                    == Some("Literal")
                                                {
                                                    if let Some(value) =
                                                        arg.get("value").map(|v| v.to_string())
                                                    {
                                                        args.push(value.clone());
                                                        let literal_type = get_literal_type(arg);
                                                        args_with_types.push(format!(
                                                            "{}: {}",
                                                            value, literal_type
                                                        ));
                                                    }
                                                }
                                            }
                                        }

                                        let arg_str = if !args_with_types.is_empty() {
                                            args_with_types.join(", ")
                                        } else if !args.is_empty() {
                                            args.join(", ")
                                        } else {
                                            String::new()
                                        };

                                        // Extract variable names being assigned
                                        let mut var_names = Vec::new();
                                        if let Some(declarations) =
                                            statement.get("declarations").and_then(|d| d.as_array())
                                        {
                                            for decl in declarations {
                                                if let Some(name) =
                                                    decl.get("name").and_then(|n| n.as_str())
                                                {
                                                    var_names.push(name.to_string());
                                                }
                                            }
                                        }

                                        let var_str = if !var_names.is_empty() {
                                            var_names.join(", ")
                                        } else {
                                            "result".to_string()
                                        };

                                        interactions.push(format!(
                                            "{}->>+{}: {}({})",
                                            contract_name, target_name, member_name, arg_str
                                        ));
                                        interactions.push(format!(
                                            "{}-->>-{}: return → {}",
                                            target_name, contract_name, var_str
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    interactions
}

/// Process a Solidity file and generate AST JSON
///
/// # Arguments
///
/// * `file_path` - Path to the Solidity file
///
/// # Returns
///
/// The AST JSON representation of the Solidity file
pub fn process_solidity_file(file_path: &str) -> Result<Value> {
    // Run solc to generate AST
    let output = Command::new("solc")
        .args([
            "--combined-json",
            "ast",
            file_path,
        ])
        .output()
        .with_context(|| format!("Failed to execute solc on {}", file_path))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("solc failed: {}", stderr));
    }

    // Parse the JSON output
    let ast_content = String::from_utf8_lossy(&output.stdout);
    let ast_json: Value = serde_json::from_str(&ast_content)?;

    // The AST is already in the correct format, just return it
    Ok(ast_json)
}
