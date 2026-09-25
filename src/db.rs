use anyhow::Result;
use rusqlite::Connection;

/// Node kinds matching the SQL CHECK constraint.
#[derive(Debug, Clone, Copy)]
pub enum NodeKind {
    Model,
    Function,
    Field,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Model => "model",
            NodeKind::Function => "function",
            NodeKind::Field => "field",
        }
    }
}

/// Constraint types matching the SQL CHECK constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintType {
    Precision,
    Nullability,
    Type,
    Range,
    Length,
    Choices,
}

impl ConstraintType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConstraintType::Precision => "precision",
            ConstraintType::Nullability => "nullability",
            ConstraintType::Type => "type",
            ConstraintType::Range => "range",
            ConstraintType::Length => "length",
            ConstraintType::Choices => "choices",
        }
    }
}

/// Verification levels.
#[derive(Debug, Clone)]
pub enum VerificationLevel {
    Proved,
    Tested,
    Extracted,
    Assumed,
}

impl VerificationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationLevel::Proved => "PROVED",
            VerificationLevel::Tested => "TESTED",
            VerificationLevel::Extracted => "EXTRACTED",
            VerificationLevel::Assumed => "ASSUMED",
        }
    }
}

/// Contract role.
#[derive(Debug, Clone)]
pub enum ContractRole {
    Precondition,
    Postcondition,
}

impl ContractRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContractRole::Precondition => "precondition",
            ContractRole::Postcondition => "postcondition",
        }
    }
}

/// Edge relationship types.
#[derive(Debug, Clone)]
pub enum Relationship {
    Calls,
    WritesTo,
    FlowsTo,
}

impl Relationship {
    pub fn as_str(&self) -> &'static str {
        match self {
            Relationship::Calls => "calls",
            Relationship::WritesTo => "writes_to",
            Relationship::FlowsTo => "flows_to",
        }
    }
}

/// Edge discovery methods.
#[derive(Debug, Clone)]
pub enum Discovery {
    AstPattern,
    Manual,
    TypeInference,
}

impl Discovery {
    pub fn as_str(&self) -> &'static str {
        match self {
            Discovery::AstPattern => "ast_pattern",
            Discovery::Manual => "manual",
            Discovery::TypeInference => "type_inference",
        }
    }
}

/// A node to be inserted.
pub struct NodeRecord {
    /// Display name: the short name when unique in the project, else the qualified name.
    pub name: String,
    /// Module-qualified name (`billing.records.Invoice.total`), always set by the extractor.
    pub qualified_name: Option<String>,
    pub kind: NodeKind,
    pub source_file: String,
    pub source_line: u32,
}

/// A contract to be inserted.
#[derive(Debug, Clone)]
pub struct ContractRecord {
    pub node_id: i64,
    pub constraint_type: ConstraintType,
    pub param_max_digits: Option<i64>,
    pub param_decimal_places: Option<i64>,
    pub param_max_length: Option<i64>,
    pub param_nullable: Option<i64>,
    pub param_type_name: Option<String>,
    pub param_min_value: Option<f64>,
    pub param_max_value: Option<f64>,
    pub param_choices: Option<String>,
    pub source_file: String,
    pub source_line: u32,
    pub is_implicit: bool,
    pub verification_level: VerificationLevel,
    pub contract_role: Option<ContractRole>,
    pub dependent_expr: Option<String>,
    /// For preconditions: the parameter the row constrains (`None` = every parameter).
    pub subject: Option<String>,
    /// For edge override rows: the edge whose source postconditions this row replaces.
    pub edge_id: Option<i64>,
}

impl ContractRecord {
    /// A row of the given kind and role with every parameter column empty.
    pub fn new(
        node_id: i64,
        constraint_type: ConstraintType,
        role: ContractRole,
        level: VerificationLevel,
        source_file: &str,
        source_line: u32,
    ) -> Self {
        ContractRecord {
            node_id,
            constraint_type,
            param_max_digits: None,
            param_decimal_places: None,
            param_max_length: None,
            param_nullable: None,
            param_type_name: None,
            param_min_value: None,
            param_max_value: None,
            param_choices: None,
            source_file: source_file.to_string(),
            source_line,
            is_implicit: false,
            verification_level: level,
            contract_role: Some(role),
            dependent_expr: None,
            subject: None,
            edge_id: None,
        }
    }
}

/// An edge to be inserted.
pub struct EdgeRecord {
    pub source_node_id: i64,
    pub target_node_id: i64,
    pub relationship: Relationship,
    pub discovery: Discovery,
    /// For `flows_to`: the callee parameter the value binds (`None` = all preconditions).
    pub target_param: Option<String>,
    /// When true, the source's postconditions on this edge are the rows with `edge_id` = this edge.
    pub source_override: bool,
}

/// Database wrapper for the contract graph.
pub struct ContractDb {
    conn: Connection,
}

impl ContractDb {
    /// Create a new database at the given path with the schema.
    pub fn create(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(ContractDb { conn })
    }

    /// Insert a node and return its ID.
    pub fn insert_node(&self, node: &NodeRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO nodes (name, qualified_name, kind, source_file, source_line)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                node.name,
                node.qualified_name,
                node.kind.as_str(),
                node.source_file,
                node.source_line,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Insert a contract.
    pub fn insert_contract(&self, contract: &ContractRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO contracts (
                node_id, constraint_type, param_max_digits, param_decimal_places,
                param_max_length, param_nullable, param_type_name,
                param_min_value, param_max_value, param_choices,
                source_file, source_line, is_implicit, verification_level,
                contract_role, dependent_expr, subject, edge_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            rusqlite::params![
                contract.node_id,
                contract.constraint_type.as_str(),
                contract.param_max_digits,
                contract.param_decimal_places,
                contract.param_max_length,
                contract.param_nullable,
                contract.param_type_name,
                contract.param_min_value,
                contract.param_max_value,
                contract.param_choices,
                contract.source_file,
                contract.source_line,
                contract.is_implicit as i64,
                contract.verification_level.as_str(),
                contract.contract_role.as_ref().map(|r| r.as_str()),
                contract.dependent_expr,
                contract.subject,
                contract.edge_id,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Insert an edge.
    pub fn insert_edge(&self, edge: &EdgeRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO edges (source_node_id, target_node_id, relationship, discovery,
                                target_param, source_override)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                edge.source_node_id,
                edge.target_node_id,
                edge.relationship.as_str(),
                edge.discovery.as_str(),
                edge.target_param,
                edge.source_override as i64,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
}

const SCHEMA: &str = r#"
CREATE TABLE nodes (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    kind           TEXT NOT NULL CHECK (kind IN ('model', 'function', 'field')),
    source_file    TEXT NOT NULL,
    source_line    INTEGER NOT NULL,
    qualified_name TEXT
);

CREATE TABLE edges (
    id              INTEGER PRIMARY KEY,
    source_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    target_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    relationship    TEXT NOT NULL CHECK (relationship IN (
                        'calls', 'writes_to', 'flows_to'
                    )),
    discovery       TEXT NOT NULL CHECK (discovery IN ('ast_pattern', 'manual', 'type_inference')),
    target_param    TEXT,
    source_override INTEGER NOT NULL DEFAULT 0 CHECK (source_override IN (0, 1))
);

CREATE TABLE contracts (
    id                  INTEGER PRIMARY KEY,
    node_id             INTEGER NOT NULL REFERENCES nodes(id),
    constraint_type     TEXT NOT NULL CHECK (constraint_type IN (
                            'precision', 'nullability', 'type', 'range', 'length', 'choices'
                        )),
    param_max_digits    INTEGER,
    param_decimal_places INTEGER,
    param_max_length    INTEGER,
    param_nullable      INTEGER,
    param_type_name     TEXT,
    param_min_value     REAL,
    param_max_value     REAL,
    param_choices       TEXT,
    source_file         TEXT NOT NULL,
    source_line         INTEGER NOT NULL,
    is_implicit         INTEGER NOT NULL DEFAULT 0,
    verification_level  TEXT NOT NULL DEFAULT 'EXTRACTED'
                        CHECK (verification_level IN ('PROVED', 'TESTED', 'EXTRACTED', 'ASSUMED')),
    contract_role       TEXT CHECK (contract_role IN ('precondition', 'postcondition', NULL)),
    dependent_expr      TEXT,
    subject             TEXT,
    edge_id             INTEGER REFERENCES edges(id)
);

CREATE INDEX idx_contracts_node ON contracts(node_id);
CREATE INDEX idx_contracts_edge ON contracts(edge_id);
CREATE INDEX idx_edges_source ON edges(source_node_id);
CREATE INDEX idx_edges_target ON edges(target_node_id);
"#;
