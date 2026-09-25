use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use ruff_python_ast::Stmt;

use crate::dataclass_extractor::{self, DataClass};
use crate::db::{ContractDb, Discovery, EdgeRecord, NodeKind, NodeRecord, Relationship};
use crate::defaults::{self, FieldDefaults};
use crate::docstring_parser::{self, DocstringContract};
use crate::edge_discovery::{self, DiscoveredEdge};
use crate::function_extractor;
use crate::model_extractor::{self, ModelField};
use crate::resolve::{self, ClassInfo, ClassKind, ModuleInfo, ProjectIndex, Symbol};
use crate::source;
use crate::value_analysis::{self, ValueFacts};

/// One parsed Python file.
pub struct SourceModule {
    /// Path relative to the application root (as recorded in `source_file`).
    pub relative_path: String,
    /// Dotted module name.
    pub module: String,
    pub is_package: bool,
    pub source: String,
    pub stmts: Vec<Stmt>,
}

impl SourceModule {
    pub fn parse(relative_path: &str, source: String) -> Self {
        let parsed =
            ruff_python_parser::parse_unchecked(&source, ruff_python_parser::Mode::Module.into());
        if !parsed.errors().is_empty() {
            eprintln!(
                "Warning: parse errors in {}: {}",
                relative_path,
                parsed.errors()[0]
            );
        }
        let stmts = match parsed.into_syntax() {
            ruff_python_ast::Mod::Module(module) => module.body,
            _ => Vec::new(),
        };
        SourceModule {
            relative_path: relative_path.to_string(),
            module: resolve::module_name(relative_path),
            is_package: relative_path.ends_with("__init__.py"),
            source,
            stmts,
        }
    }
}

/// Everything extracted from a project, before it is written to SQLite.
pub struct Project {
    pub modules: Vec<SourceModule>,
    pub index: ProjectIndex,
    pub model_fields: Vec<ModelField>,
    pub data_classes: Vec<DataClass>,
    /// Return-value summaries (EXTRACTED facts) per qualified function name.
    pub summaries: HashMap<String, ValueFacts>,
    /// Docstring clauses per qualified function name.
    pub docstrings: HashMap<String, Vec<DocstringContract>>,
}

impl Project {
    /// Parse and analyse `(relative path, source)` pairs with the Django 4.2 defaults.
    pub fn from_sources(files: Vec<(String, String)>) -> Project {
        let modules = files
            .into_iter()
            .map(|(p, s)| SourceModule::parse(&p, s))
            .collect();
        Project::from_parsed(modules)
    }

    /// Analyse parsed modules with the Django 4.2 defaults.
    pub fn from_parsed(modules: Vec<SourceModule>) -> Project {
        let defaults = defaults::load_defaults("4.2").unwrap_or_default();
        Project::build(modules, &defaults)
    }

    pub fn build(
        modules: Vec<SourceModule>,
        field_defaults: &HashMap<String, FieldDefaults>,
    ) -> Project {
        let mut index = ProjectIndex::default();
        let mut model_fields = Vec::new();
        let mut candidates = Vec::new();

        for m in &modules {
            index.modules.insert(
                m.module.clone(),
                ModuleInfo::from_stmts(&m.module, m.is_package, &m.stmts),
            );

            // Functions (a later definition with the same qualified name replaces an earlier one)
            for mut func in
                function_extractor::extract_functions(&m.stmts, &m.relative_path, &m.module)
            {
                func.source_line = source::line_of(&m.source, func.source_line);
                match index.function_ids.get(&func.qualified_name) {
                    Some(&i) => index.functions[i] = func,
                    None => {
                        index
                            .function_ids
                            .insert(func.qualified_name.clone(), index.functions.len());
                        index.functions.push(func);
                    }
                }
            }

            // Classes
            for stmt in &m.stmts {
                if let Stmt::ClassDef(c) = stmt {
                    let qualified = resolve::qualify(&m.module, c.name.as_str());
                    index.classes.insert(
                        qualified.clone(),
                        ClassInfo {
                            qualified,
                            module: m.module.clone(),
                            name: c.name.to_string(),
                            kind: ClassKind::Plain,
                            fields: Vec::new(),
                            positional: None,
                            bases: c
                                .arguments
                                .as_ref()
                                .map(|a| a.args.to_vec())
                                .unwrap_or_default(),
                            methods: HashMap::new(),
                        },
                    );
                }
            }

            // Django model fields
            let mut fields =
                model_extractor::extract_models(&m.stmts, &m.relative_path, field_defaults);
            for field in &mut fields {
                field.source_line = source::line_of(&m.source, field.source_line);
            }
            model_fields.extend(fields);

            candidates.extend(dataclass_extractor::collect_classes(
                &m.stmts,
                &m.source,
                &m.relative_path,
            ));
        }

        for func in &index.functions {
            if let Some(class) = &func.class_name {
                let q = resolve::qualify(&func.module, class);
                if let Some(c) = index.classes.get_mut(&q) {
                    let method = func
                        .name
                        .rsplit('.')
                        .next()
                        .unwrap_or(&func.name)
                        .to_string();
                    c.methods.insert(method, func.qualified_name.clone());
                }
            }
        }

        for field in &model_fields {
            let q = resolve::qualify(&field.module, &field.model_name);
            if let Some(c) = index.classes.get_mut(&q) {
                c.kind = ClassKind::Django;
                if !c.fields.contains(&field.field_name) {
                    c.fields.push(field.field_name.clone());
                }
            }
        }

        let data_classes = {
            let idx = &index;
            dataclass_extractor::resolve_data_classes_with(&candidates, &|c, base| match idx
                .resolve_expr(&c.module, base)
            {
                Some(Symbol::Class(q)) => Some(q),
                _ => None,
            })
        };
        for dc in &data_classes {
            if let Some(c) = index.classes.get_mut(&dc.qualified_name) {
                if c.kind == ClassKind::Plain {
                    c.kind = ClassKind::Data(dc.kind);
                    c.fields = dc.fields.iter().map(|f| f.field_name.clone()).collect();
                    c.positional = dc.positional.clone();
                }
            }
        }

        let docstrings = index
            .functions
            .iter()
            .filter_map(|f| {
                let doc = docstring_parser::parse_docstring(f.docstring.as_deref()?);
                (!doc.is_empty()).then(|| (f.qualified_name.clone(), doc))
            })
            .collect();

        let summaries = value_analysis::compute_summaries(&index);

        Project {
            modules,
            index,
            model_fields,
            data_classes,
            summaries,
            docstrings,
        }
    }

    /// Source text of a module by its relative path.
    pub fn source_of(&self, relative_path: &str) -> &str {
        self.modules
            .iter()
            .find(|m| m.relative_path == relative_path)
            .map(|m| m.source.as_str())
            .unwrap_or("")
    }

    /// `(qualified, short)` names of every node, for display-name selection.
    fn node_names(&self) -> Vec<(String, String)> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        let mut add = |q: String, short: String| {
            if seen.insert(q.clone()) {
                names.push((q, short));
            }
        };
        for f in &self.model_fields {
            add(
                model_extractor::field_qualified(f),
                format!("{}.{}", f.model_name, f.field_name),
            );
        }
        for c in &self.data_classes {
            for f in &c.fields {
                add(
                    dataclass_extractor::field_qualified(f),
                    format!("{}.{}", f.class_name, f.field_name),
                );
            }
        }
        for f in &self.index.functions {
            add(f.qualified_name.clone(), f.name.clone());
        }
        names
    }
}

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

    // Parse all Python files
    let mut py_files = find_python_files(app_path)?;
    py_files.sort();
    let mut modules = Vec::new();
    for py_file in &py_files {
        let source = std::fs::read_to_string(py_file)?;
        let mut relative_path = py_file
            .strip_prefix(app_path)
            .unwrap_or(py_file)
            .to_string_lossy()
            .to_string();
        if relative_path.is_empty() {
            // A single file was given: name the module after the file.
            relative_path = py_file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
        }
        modules.push(SourceModule::parse(&relative_path, source));
    }

    let project = Project::build(modules, &field_defaults);
    let names = resolve::display_names(&project.node_names());

    // Nodes: model fields, data class fields, functions
    let mut model_node_ids =
        model_extractor::write_model_fields(&db, &project.model_fields, &names)?;
    for (q, id) in dataclass_extractor::write_data_classes(&db, &project.data_classes, &names)? {
        model_node_ids.entry(q).or_insert(id);
    }
    let func_node_ids = write_functions(&db, &project, &names)?;

    // Edges
    let edges = dedupe(edge_discovery::discover_project_edges(&project));
    let mut edge_count = write_edges(&db, &edges, &func_node_ids, &model_node_ids, &names)?;

    // Load and write manual overrides
    if let Some(overrides_path) = overrides_path {
        if overrides_path.exists() {
            let override_edges = edge_discovery::load_overrides(overrides_path)?;
            edge_count += write_edges(
                &db,
                &override_edges,
                &func_node_ids,
                &model_node_ids,
                &names,
            )?;
        }
    }

    let data_class_field_count: usize = project.data_classes.iter().map(|c| c.fields.len()).sum();
    eprintln!(
        "Extracted {} model fields, {} data class fields, {} functions, {} edges to {}",
        project.model_fields.len(),
        data_class_field_count,
        project.index.functions.len(),
        edge_count,
        db_path.display()
    );

    Ok(db_path)
}

/// Write function nodes with their pre- and postconditions.
/// Returns qualified name → node ID.
fn write_functions(
    db: &ContractDb,
    project: &Project,
    names: &HashMap<String, String>,
) -> Result<HashMap<String, i64>> {
    let mut ids = HashMap::new();
    for func in &project.index.functions {
        let node_id = db.insert_node(&NodeRecord {
            name: names
                .get(&func.qualified_name)
                .cloned()
                .unwrap_or_else(|| func.name.clone()),
            qualified_name: Some(func.qualified_name.clone()),
            kind: NodeKind::Function,
            source_file: func.source_file.clone(),
            source_line: func.source_line,
        })?;
        ids.insert(func.qualified_name.clone(), node_id);

        let doc = project
            .docstrings
            .get(&func.qualified_name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let summary = project
            .summaries
            .get(&func.qualified_name)
            .cloned()
            .unwrap_or_default();
        for row in function_extractor::precondition_rows(func, doc, node_id)
            .into_iter()
            .chain(function_extractor::postcondition_rows(
                func, &summary, doc, node_id,
            ))
        {
            db.insert_contract(&row)?;
        }
    }
    Ok(ids)
}

/// Drop repeated edges that carry no override rows (e.g. the same producer
/// written to the same field at two call sites).
fn dedupe(edges: Vec<DiscoveredEdge>) -> Vec<DiscoveredEdge> {
    let mut seen = HashSet::new();
    edges
        .into_iter()
        .filter(|e| {
            e.override_rows.is_some()
                || seen.insert((
                    e.source_function.clone(),
                    e.target_name.clone(),
                    e.target_field.clone(),
                    e.relationship.clone(),
                    e.target_param.clone(),
                ))
        })
        .collect()
}

/// Write edges (and each edge's override rows, after the edge itself).
/// Edges whose endpoints are not extracted nodes are dropped.
fn write_edges(
    db: &ContractDb,
    edges: &[DiscoveredEdge],
    func_node_ids: &HashMap<String, i64>,
    model_node_ids: &HashMap<String, i64>,
    names: &HashMap<String, String>,
) -> Result<usize> {
    let mut written = 0;
    for edge in edges {
        // Discovered edges carry qualified names; only manual overrides are
        // matched loosely (display name or dotted suffix).
        let lookup = |ids: &HashMap<String, i64>, name: &str| {
            if edge.discovery == "manual" {
                lookup(ids, names, name)
            } else {
                ids.get(name).copied()
            }
        };
        let source_id = lookup(func_node_ids, &edge.source_function);
        let target_id = match &edge.target_field {
            Some(field) => lookup(model_node_ids, &format!("{}.{}", edge.target_name, field)),
            None => lookup(model_node_ids, &edge.target_name)
                .or_else(|| lookup(func_node_ids, &edge.target_name)),
        };
        let (Some(src_id), Some(tgt_id)) = (source_id, target_id) else {
            continue;
        };
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

        let edge_id = db.insert_edge(&EdgeRecord {
            source_node_id: src_id,
            target_node_id: tgt_id,
            relationship,
            discovery,
            target_param: edge.target_param.clone(),
            source_override: edge.override_rows.is_some(),
        })?;
        written += 1;
        for row in edge.override_rows.iter().flatten() {
            let mut row = row.clone();
            row.node_id = src_id;
            row.edge_id = Some(edge_id);
            db.insert_contract(&row)?;
        }
    }
    Ok(written)
}

/// A node ID by qualified name, else by display name, else by a unique
/// dotted-suffix match (for manual overrides naming `pkg.module.func`
/// or a short name).
fn lookup(ids: &HashMap<String, i64>, names: &HashMap<String, String>, name: &str) -> Option<i64> {
    if let Some(&id) = ids.get(name) {
        return Some(id);
    }
    let by_display: Vec<i64> = ids
        .iter()
        .filter(|(q, _)| names.get(*q).map(String::as_str) == Some(name))
        .map(|(_, &id)| id)
        .collect();
    if let [id] = by_display.as_slice() {
        return Some(*id);
    }
    let suffix = |a: &str, b: &str| {
        a.len() > b.len() && a.ends_with(b) && a[..a.len() - b.len()].ends_with('.')
    };
    let by_suffix: Vec<i64> = ids
        .iter()
        .filter(|(q, _)| suffix(q, name) || suffix(name, q))
        .map(|(_, &id)| id)
        .collect();
    match by_suffix.as_slice() {
        [id] => Some(*id),
        _ => None,
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
