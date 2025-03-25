use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};

/// Represents a contract's state variable
#[derive(Debug, Clone)]
pub struct StateVariable {
    pub name: String,
    pub var_type: String,
    pub visibility: String,
}

/// Represents a function parameter or return value
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
}

/// Represents a sequence diagram interaction
#[derive(Debug, Clone)]
pub enum InteractionType {
    Call,
    Return,
    Event,
}

/// Represents a diagram interaction between participants
#[derive(Debug, Clone)]
pub struct Interaction {
    pub interaction_type: InteractionType,
    pub from: String,
    pub to: String,
    pub message: String,
    pub inside_loop: bool,
    pub origin_function: Option<String>,
}

/// Represents contract information
#[derive(Debug, Clone, Default)]
pub struct ContractInfo {
    pub name: String,
    pub events: Vec<String>,
    pub functions: Vec<String>,
    pub variables: Vec<(String, String)>,
    pub inherits_from: Vec<String>,
    pub contract_type: String,
    pub source_file: String,
}

/// Relationship between contracts
#[derive(Debug, Clone)]
pub struct ContractRelationship {
    pub source: String,
    pub target: String,
    pub relation_type: String,
}

/// Container for all extracted contract information
#[derive(Debug, Clone, Default)]
pub struct DiagramData {
    pub participants: HashSet<String>,
    pub contracts: HashMap<String, ContractInfo>,
    pub user_interactions: Vec<String>,
    pub contract_interactions: IndexMap<String, Vec<String>>, // Grouped by function
    pub events: Vec<(String, String)>,
    pub contract_relationships: Vec<ContractRelationship>,
}
