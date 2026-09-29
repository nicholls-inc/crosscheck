//! Shared helpers for the integration tests (each test crate uses a subset).
#![allow(dead_code)]

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
        ruff_python_ast::Mod::Module(module) => module.body.to_vec(),
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
    pub qualified_name: Option<String>,
    pub kind: String,
    pub source_file: String,
    pub source_line: u32,
}

#[derive(Debug, Clone)]
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
    pub param_min_value: Option<f64>,
    pub param_max_value: Option<f64>,
    pub subject: Option<String>,
    pub edge_id: Option<i64>,
}

#[derive(Debug)]
pub struct EdgeRow {
    pub id: i64,
    pub source_name: String,
    pub target_name: String,
    pub relationship: String,
    pub discovery: String,
    pub target_param: Option<String>,
    pub source_override: bool,
    /// The source is a call-site node (`qualified_name` = `f@file:line`).
    pub source_is_site: bool,
    /// The target is a call-site node.
    pub target_is_site: bool,
    pub site_line: Option<u32>,
}

// -- Query helpers --

/// Query all nodes of a given kind.
pub fn query_nodes(conn: &Connection, kind: &str) -> Vec<NodeRow> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, kind, source_file, source_line, qualified_name FROM nodes WHERE kind = ?1",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![kind], |row| {
        Ok(NodeRow {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            source_file: row.get(3)?,
            source_line: row.get(4)?,
            qualified_name: row.get(5)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

const CONTRACT_COLUMNS: &str =
    "n.name, c.constraint_type, c.param_max_digits, c.param_decimal_places,
    c.param_max_length, c.param_nullable, c.param_type_name, c.param_choices,
    c.is_implicit, c.verification_level, c.contract_role, c.dependent_expr,
    c.param_min_value, c.param_max_value, c.subject, c.edge_id";

fn contract_rows(
    conn: &Connection,
    where_clause: &str,
    param: &dyn rusqlite::ToSql,
) -> Vec<ContractRow> {
    let sql = format!(
        "SELECT {CONTRACT_COLUMNS} FROM contracts c JOIN nodes n ON c.node_id = n.id WHERE {where_clause} ORDER BY c.id"
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    stmt.query_map([param], |row| {
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
            param_min_value: row.get(12)?,
            param_max_value: row.get(13)?,
            subject: row.get(14)?,
            edge_id: row.get(15)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect()
}

/// Query the node contracts (not edge override rows) of a node, by display
/// name. Call-site nodes (which copy their callee's rows) are left out.
pub fn query_contracts_for(conn: &Connection, node_name: &str) -> Vec<ContractRow> {
    contract_rows(
        conn,
        "n.name = ?1 AND c.edge_id IS NULL AND COALESCE(n.qualified_name, '') NOT LIKE '%@%'",
        &node_name,
    )
}

/// Query the override rows of an edge.
pub fn query_edge_contracts(conn: &Connection, edge_id: i64) -> Vec<ContractRow> {
    contract_rows(conn, "c.edge_id = ?1", &edge_id)
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
            "SELECT e.id, src.name, tgt.name, e.relationship, e.discovery, e.target_param, e.source_override,
                    COALESCE(src.qualified_name, '') LIKE '%@%', COALESCE(tgt.qualified_name, '') LIKE '%@%',
                    e.site_line
             FROM edges e
             JOIN nodes src ON e.source_node_id = src.id
             JOIN nodes tgt ON e.target_node_id = tgt.id
             ORDER BY e.id",
        )
        .unwrap();
    stmt.query_map([], |row| {
        Ok(EdgeRow {
            id: row.get(0)?,
            source_name: row.get(1)?,
            target_name: row.get(2)?,
            relationship: row.get(3)?,
            discovery: row.get(4)?,
            target_param: row.get(5)?,
            source_override: row.get::<_, i64>(6)? != 0,
            source_is_site: row.get::<_, i64>(7)? != 0,
            target_is_site: row.get::<_, i64>(8)? != 0,
            site_line: row.get(9)?,
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

/// The single edge `source -> target` with `relationship` (and, if given,
/// `target_param`) into the target's own node (argument edges into
/// call-site nodes are ignored); panics unless exactly one matches.
pub fn the_edge<'a>(
    edges: &'a [EdgeRow],
    source: &str,
    target: &str,
    relationship: &str,
    target_param: Option<&str>,
) -> &'a EdgeRow {
    let matching: Vec<&EdgeRow> = edges
        .iter()
        .filter(|e| {
            e.source_name == source
                && e.target_name == target
                && !e.target_is_site
                && e.relationship == relationship
                && (target_param.is_none() || e.target_param.as_deref() == target_param)
        })
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "expected one {source} -{relationship}-> {target} ({target_param:?}) edge, got {matching:?} in {edges:#?}"
    );
    matching[0]
}

/// Compact rendering of contract rows, sorted: `kind=value` with the value
/// column the checker reads for that kind (`range` shows `min..max`),
/// prefixed by the subject and suffixed by `[ASSUMED]` where applicable.
pub fn render_rows(rows: &[ContractRow]) -> Vec<String> {
    let mut out: Vec<String> = rows
        .iter()
        .map(|r| {
            let value = match r.constraint_type.as_str() {
                "precision" => r
                    .param_decimal_places
                    .map(|p| p.to_string())
                    .or_else(|| r.dependent_expr.clone())
                    .unwrap_or_default(),
                "nullability" => r.param_nullable.map(|n| n.to_string()).unwrap_or_default(),
                "type" => r.param_type_name.clone().unwrap_or_default(),
                "length" => r
                    .param_max_length
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
                "range" => format!(
                    "{}..{}",
                    r.param_min_value.map(|v| v.to_string()).unwrap_or_default(),
                    r.param_max_value.map(|v| v.to_string()).unwrap_or_default()
                ),
                "choices" => r.param_choices.clone().unwrap_or_default(),
                _ => String::new(),
            };
            let mut s = format!("{}={value}", r.constraint_type);
            if r.verification_level == "ASSUMED" {
                s.push_str(" [ASSUMED]");
            }
            if let Some(subject) = &r.subject {
                s = format!("{subject}: {s}");
            }
            s
        })
        .collect();
    out.sort();
    out
}

/// Rendered override rows of the single matching edge (see `the_edge`);
/// panics if the edge has no override.
pub fn override_rows(
    conn: &Connection,
    edges: &[EdgeRow],
    source: &str,
    target: &str,
    relationship: &str,
    target_param: Option<&str>,
) -> Vec<String> {
    let e = the_edge(edges, source, target, relationship, target_param);
    assert!(e.source_override, "edge {e:?} has no override");
    render_rows(&query_edge_contracts(conn, e.id))
}

/// Rendered node contracts (pre- and postconditions) of a node, with the role.
pub fn node_rows(conn: &Connection, node_name: &str, role: &str) -> Vec<String> {
    let rows: Vec<ContractRow> = query_contracts_for(conn, node_name)
        .into_iter()
        .filter(|c| c.contract_role.as_deref() == Some(role))
        .collect();
    render_rows(&rows)
}
