use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use crate::body_analyzer;
use crate::dataclass_extractor;
use crate::db::{ContractDb, Discovery, EdgeRecord, Relationship};
use crate::defaults;
use crate::docstring_parser;
use crate::edge_discovery;
use crate::function_extractor;
use crate::model_extractor;
use crate::source;

/// Run the full extraction pipeline on a Python project directory (or a single .py file).
///
/// If `output_db` is `Some(path)`, the SQLite database is written there.
/// If `None`, a default temp path is used.
pub fn extract(
    app_path: &Path,
    overrides_path: Option<&Path>,
    django_version: &str,
    output_db: Option<&Path>,
) -> Result<std::path::PathBuf> {
    if !app_path.exists() {
        anyhow::bail!("Application path does not exist: {}", app_path.display());
    }

    // Determine database path
    let db_path = match output_db {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            path.to_path_buf()
        }
        None => {
            let db_dir = std::env::temp_dir().join("crosscheck");
            std::fs::create_dir_all(&db_dir)?;
            db_dir.join("contracts.sqlite")
        }
    };

    // Remove existing database
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let db = ContractDb::create(&db_path)?;

    // Load defaults
    let field_defaults = defaults::load_defaults(django_version)?;

    // Find all Python files
    let py_files = find_python_files(app_path)?;

    let mut all_model_fields = Vec::new();
    let mut all_functions = Vec::new();
    let mut all_body_contracts = Vec::new();
    let mut all_docstring_contracts = Vec::new();
    let mut all_discovered_edges = Vec::new();
    let mut all_class_candidates = Vec::new();

    // Parse each file
    for py_file in &py_files {
        let source = std::fs::read_to_string(py_file)?;
        let relative_path = py_file
            .strip_prefix(app_path)
            .unwrap_or(py_file)
            .to_string_lossy()
            .to_string();

        let parsed = ruff_python_parser::parse_unchecked(
            &source,
            ruff_python_parser::Mode::Module.into(),
        );
        if !parsed.errors().is_empty() {
            eprintln!(
                "Warning: parse errors in {}: {}",
                relative_path,
                parsed.errors()[0]
            );
        }
        let stmts = match parsed.syntax() {
            ruff_python_ast::Mod::Module(module) => &module.body,
            _ => continue,
        };

        // Extract models
        let mut model_fields =
            model_extractor::extract_models(stmts, &relative_path, &field_defaults);
        for field in &mut model_fields {
            field.source_line = source::line_of(&source, field.source_line);
        }
        all_model_fields.extend(model_fields);

        // Collect class definitions; data classes are resolved project-wide below
        all_class_candidates.extend(dataclass_extractor::collect_classes(
            stmts,
            &source,
            &relative_path,
        ));

        // Extract functions
        let mut functions = function_extractor::extract_functions(stmts, &relative_path);
        for func in &mut functions {
            func.source_line = source::line_of(&source, func.source_line);
        }

        // Analyze function bodies
        for func in &functions {
            let body_contracts = body_analyzer::analyze_body(&func.body);
            if !body_contracts.is_empty() {
                all_body_contracts.push((func.name.clone(), body_contracts));
            }

            // Parse docstrings
            if let Some(ref docstring) = func.docstring {
                let doc_contracts = docstring_parser::parse_docstring(docstring);
                if !doc_contracts.is_empty() {
                    all_docstring_contracts.push((func.name.clone(), doc_contracts));
                }
            }
        }

        all_functions.extend(functions);

        // Discover edges via AST patterns
        let edges = edge_discovery::discover_edges(stmts);
        all_discovered_edges.extend(edges);
    }

    // Write model fields to database
    model_extractor::write_model_fields(&db, &all_model_fields)?;

    // Write functions to database
    let node_ids = function_extractor::write_functions(
        &db,
        &all_functions,
        &all_body_contracts,
        &all_docstring_contracts,
    )?;

    // Build node ID lookup for models
    let mut model_node_ids: HashMap<String, i64> = HashMap::new();
    // We need to query back the node IDs for model fields
    // For simplicity, build the map from what we know
    for field in &all_model_fields {
        let full_name = format!("{}.{}", field.model_name, field.field_name);
        // The node ID was auto-assigned; we need to look it up
        // This is a simplification - in production, write_model_fields should return IDs
        if let Some(id) = lookup_node_id_by_name(&db_path, &full_name)? {
            model_node_ids.insert(full_name, id);
        }
    }

    // Resolve and write plain-Python data classes (dataclass, attrs, pydantic, ...)
    let data_classes = dataclass_extractor::resolve_data_classes(&all_class_candidates);
    let data_class_ids = dataclass_extractor::write_data_classes(&db, &data_classes)?;
    let data_class_field_count = data_class_ids.len();
    model_node_ids.extend(data_class_ids);

    // Map positional constructor arguments (`#<index>`) to declared field names
    resolve_positional_fields(&mut all_discovered_edges, &data_classes);

    // Write AST-discovered edges
    write_discovered_edges(&db, &all_discovered_edges, &node_ids, &model_node_ids)?;

    // Load and write manual overrides
    if let Some(overrides_path) = overrides_path {
        if overrides_path.exists() {
            let override_edges = edge_discovery::load_overrides(overrides_path)?;
            write_discovered_edges(&db, &override_edges, &node_ids, &model_node_ids)?;
        }
    }

    // Count actual edges written to the database
    let edge_count: i64 = {
        let conn = rusqlite::Connection::open(&db_path)?;
        conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?
    };

    eprintln!(
        "Extracted {} model fields, {} data class fields, {} functions, {} edges to {}",
        all_model_fields.len(),
        data_class_field_count,
        all_functions.len(),
        edge_count,
        db_path.display()
    );

    Ok(db_path)
}

/// Replace `#<index>` placeholder fields on constructor edges with the
/// declared field at that position, for data classes whose constructors take
/// fields positionally. Placeholders that cannot be mapped are left as-is and
/// are dropped at edge resolution.
fn resolve_positional_fields(
    edges: &mut [edge_discovery::DiscoveredEdge],
    classes: &[dataclass_extractor::DataClass],
) {
    for edge in edges.iter_mut() {
        let Some(index) = edge
            .target_field
            .as_deref()
            .and_then(|f| f.strip_prefix('#'))
            .and_then(|i| i.parse::<usize>().ok())
        else {
            continue;
        };
        let field = classes
            .iter()
            .find(|c| c.name == edge.target_name && c.kind.positional_fields())
            .and_then(|c| c.fields.get(index));
        if let Some(field) = field {
            edge.target_field = Some(field.field_name.clone());
        }
    }
}

/// Find all .py files in a directory recursively.
fn find_python_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_file() && dir.extension().is_some_and(|ext| ext == "py") {
        files.push(dir.to_path_buf());
        return Ok(files);
    }
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip __pycache__, .git, etc.
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if !dir_name.starts_with('.')
                    && dir_name != "__pycache__"
                    && dir_name != "node_modules"
                {
                    files.extend(find_python_files(&path)?);
                }
            } else if path.extension().is_some_and(|ext| ext == "py") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// Look up a node ID by name from the database.
fn lookup_node_id_by_name(db_path: &Path, name: &str) -> Result<Option<i64>> {
    let conn = rusqlite::Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT id FROM nodes WHERE name = ?1")?;
    let result = stmt.query_row(rusqlite::params![name], |row| row.get(0));
    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Write discovered edges to the database.
fn write_discovered_edges(
    db: &ContractDb,
    edges: &[edge_discovery::DiscoveredEdge],
    func_node_ids: &HashMap<String, i64>,
    model_node_ids: &HashMap<String, i64>,
) -> Result<()> {
    for edge in edges {
        let source_id = func_node_ids
            .get(&edge.source_function)
            .or_else(|| find_by_suffix(func_node_ids, &edge.source_function));
        let target_id = if let Some(field) = &edge.target_field {
            let key = format!("{}.{}", edge.target_name, field);
            model_node_ids
                .get(&key)
                .or_else(|| find_by_suffix(model_node_ids, &key))
        } else {
            // Try to find by full qualified name
            model_node_ids
                .get(&edge.target_name)
                .or_else(|| find_by_suffix(model_node_ids, &edge.target_name))
                .or_else(|| func_node_ids.get(&edge.target_name))
                .or_else(|| find_by_suffix(func_node_ids, &edge.target_name))
        };

        if let (Some(&src_id), Some(&tgt_id)) = (source_id, target_id) {
            let relationship = match edge.relationship.as_str() {
                "calls" => Relationship::Calls,
                "writes_to" => Relationship::WritesTo,
                "flows_to" => Relationship::FlowsTo,
                _ => continue,
            };
            let discovery = match edge.discovery.as_str() {
                "ast_pattern" => Discovery::AstPattern,
                "manual" => Discovery::Manual,
                "type_inference" => Discovery::TypeInference,
                _ => continue,
            };

            db.insert_edge(&EdgeRecord {
                source_node_id: src_id,
                target_node_id: tgt_id,
                relationship,
                discovery,
            })?;
        }
    }

    Ok(())
}

/// Find a node ID by checking if any key in the map is a suffix of the given name.
/// This handles qualified override names like "pkg.module.func" matching stored
/// short names like "func" or "Model.field".
fn find_by_suffix<'a>(map: &'a HashMap<String, i64>, qualified_name: &str) -> Option<&'a i64> {
    map.iter()
        .find(|(key, _)| {
            qualified_name.ends_with(key.as_str())
                && qualified_name[..qualified_name.len() - key.len()].ends_with('.')
        })
        .map(|(_, id)| id)
}
