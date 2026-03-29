use std::path::Path;

use crosscheck_contracts::extractor;
use ruff_python_ast::Stmt;
use rusqlite::Connection;
use tempfile::TempDir;

/// Parse a Python source string and return the top-level statements.
pub fn parse_python_stmts(source: &str) -> Vec<Stmt> {
    let parsed =
        ruff_python_parser::parse_unchecked(source, ruff_python_parser::Mode::Module.into());
    match parsed.into_syntax() {
        ruff_python_ast::Mod::Module(module) => module.body,
        _ => panic!("Expected a Module"),
    }
}

/// Run the full extraction pipeline on a fixture directory and return
/// a SQLite connection to the resulting database.
pub fn extract_to_db(fixture_dir: &Path, tmp: &TempDir) -> Connection {
    let db_path = tmp.path().join("contracts.sqlite");
    let result_path =
        extractor::extract(fixture_dir, None, "4.2", Some(&db_path)).expect("extraction failed");
    Connection::open(&result_path).expect("failed to open database")
}

// -- Query result structs --

#[derive(Debug)]
pub struct NodeRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub source_file: String,
    pub source_line: u32,
}

#[derive(Debug)]
pub struct ContractRow {
    pub node_name: String,
    pub constraint_type: String,
    pub param_max_digits: Option<i64>,
    pub param_decimal_places: Option<i64>,
    pub param_max_length: Option<i64>,
    pub param_nullable: Option<i64>,
    pub param_type_name: Option<String>,
    pub param_choices: Option<String>,
    pub is_implicit: bool,
    pub verification_level: String,
    pub contract_role: Option<String>,
    pub dependent_expr: Option<String>,
}

#[derive(Debug)]
pub struct EdgeRow {
    pub source_name: String,
    pub target_name: String,
    pub relationship: String,
    pub discovery: String,
}

// -- Query helpers --

/// Query all nodes of a given kind.
pub fn query_nodes(conn: &Connection, kind: &str) -> Vec<NodeRow> {
    let mut stmt = conn
        .prepare("SELECT id, name, kind, source_file, source_line FROM nodes WHERE kind = ?1")
        .unwrap();
    stmt.query_map(rusqlite::params![kind], |row| {
        Ok(NodeRow {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            source_file: row.get(3)?,
            source_line: row.get(4)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

/// Query all contracts for a given node name.
pub fn query_contracts_for(conn: &Connection, node_name: &str) -> Vec<ContractRow> {
    let mut stmt = conn
        .prepare(
            "SELECT n.name, c.constraint_type, c.param_max_digits, c.param_decimal_places,
                    c.param_max_length, c.param_nullable, c.param_type_name, c.param_choices,
                    c.is_implicit, c.verification_level, c.contract_role, c.dependent_expr
             FROM contracts c JOIN nodes n ON c.node_id = n.id
             WHERE n.name = ?1",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![node_name], |row| {
        Ok(ContractRow {
            node_name: row.get(0)?,
            constraint_type: row.get(1)?,
            param_max_digits: row.get(2)?,
            param_decimal_places: row.get(3)?,
            param_max_length: row.get(4)?,
            param_nullable: row.get(5)?,
            param_type_name: row.get(6)?,
            param_choices: row.get(7)?,
            is_implicit: row.get::<_, i64>(8)? != 0,
            verification_level: row.get(9)?,
            contract_role: row.get(10)?,
            dependent_expr: row.get(11)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

/// Query contracts for a node filtered by constraint type.
pub fn query_contract_by_type(
    conn: &Connection,
    node_name: &str,
    constraint_type: &str,
) -> Vec<ContractRow> {
    query_contracts_for(conn, node_name)
        .into_iter()
        .filter(|c| c.constraint_type == constraint_type)
        .collect()
}

/// Query all edges with resolved node names.
pub fn query_edges(conn: &Connection) -> Vec<EdgeRow> {
    let mut stmt = conn
        .prepare(
            "SELECT src.name, tgt.name, e.relationship, e.discovery
             FROM edges e
             JOIN nodes src ON e.source_node_id = src.id
             JOIN nodes tgt ON e.target_node_id = tgt.id",
        )
        .unwrap();
    stmt.query_map([], |row| {
        Ok(EdgeRow {
            source_name: row.get(0)?,
            target_name: row.get(1)?,
            relationship: row.get(2)?,
            discovery: row.get(3)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

/// Count total rows in a table.
pub fn count_rows(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}
