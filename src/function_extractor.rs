use anyhow::Result;
use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::db::{
    ContractDb, ContractRecord, ContractRole, ConstraintType, NodeKind, NodeRecord,
    VerificationLevel,
};

/// Extracted function information.
#[derive(Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub return_type: Option<String>,
    pub is_return_optional: bool,
    /// For `tuple[T, T, ...]` returns with uniform element types, the element type.
    pub tuple_element_type: Option<String>,
    pub source_file: String,
    pub source_line: u32,
    pub body: Vec<Stmt>,
    pub docstring: Option<String>,
}

/// Extract function definitions from a parsed Python module.
pub fn extract_functions(stmts: &[Stmt], source_file: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                let info = extract_function_info(func_def, source_file);
                functions.push(info);
            }
            Stmt::ClassDef(class_def) => {
                // Extract methods from classes too
                for body_stmt in &class_def.body {
                    if let Stmt::FunctionDef(func_def) = body_stmt {
                        let info = extract_function_info(func_def, source_file);
                        functions.push(info);
                    }
                }
            }
            _ => {}
        }
    }

    functions
}

/// Extract info from a single function definition.
fn extract_function_info(func_def: &ast::StmtFunctionDef, source_file: &str) -> FunctionInfo {
    // For Optional[T] / T | None, the type postcondition is T; nullability is
    // recorded separately via `is_return_optional`.
    let return_type = func_def
        .returns
        .as_ref()
        .map(|ret| format_type_annotation(strip_optional(ret)));

    let is_return_optional = func_def
        .returns
        .as_ref()
        .map(|ret| is_optional_type(ret))
        .unwrap_or(false);

    let tuple_element_type = func_def
        .returns
        .as_ref()
        .and_then(|ret| uniform_tuple_element_type(ret));

    let docstring = extract_docstring(&func_def.body);

    FunctionInfo {
        name: func_def.name.to_string(),
        return_type,
        is_return_optional,
        tuple_element_type,
        source_file: source_file.to_string(),
        source_line: func_def.range.start().to_u32(),
        body: func_def.body.clone(),
        docstring,
    }
}

/// Format a type annotation expression as a string.
fn format_type_annotation(expr: &Expr) -> String {
    match expr {
        Expr::Name(name) => name.id.to_string(),
        Expr::Attribute(attr) => format!("{}.{}", format_type_annotation(&attr.value), attr.attr),
        Expr::Subscript(sub) => {
            format!(
                "{}[{}]",
                format_type_annotation(&sub.value),
                format_type_annotation(&sub.slice)
            )
        }
        Expr::Tuple(tuple) => {
            let items: Vec<String> = tuple.elts.iter().map(format_type_annotation).collect();
            items.join(", ")
        }
        _ => "unknown".to_string(),
    }
}

/// Check if a type annotation represents an Optional type.
fn is_optional_type(expr: &Expr) -> bool {
    match expr {
        Expr::Subscript(sub) => matches_name_str(&sub.value, "Optional"),
        Expr::BinOp(binop) => {
            // X | None syntax
            matches!(binop.op, ast::Operator::BitOr)
                && (is_none_type(&binop.left) || is_none_type(&binop.right))
        }
        _ => false,
    }
}

/// `Optional[T]`, `T | None` and `None | T` → `T`; anything else unchanged.
fn strip_optional(expr: &Expr) -> &Expr {
    match expr {
        Expr::Subscript(sub) if matches_name_str(&sub.value, "Optional") => &sub.slice,
        Expr::BinOp(binop) if matches!(binop.op, ast::Operator::BitOr) => {
            if is_none_type(&binop.right) {
                &binop.left
            } else if is_none_type(&binop.left) {
                &binop.right
            } else {
                expr
            }
        }
        _ => expr,
    }
}

/// Check if an expression is the None type.
fn is_none_type(expr: &Expr) -> bool {
    matches_name_str(expr, "None") || matches!(expr, Expr::NoneLiteral(_))
}

/// If the return annotation is `tuple[T, T, ...]` with all element types identical,
/// return the element type name. This handles the common pattern where a function
/// returns a tuple of values that get written individually to model fields.
fn uniform_tuple_element_type(expr: &Expr) -> Option<String> {
    if let Expr::Subscript(sub) = expr {
        if matches_name_str(&sub.value, "tuple") {
            if let Expr::Tuple(tuple) = sub.slice.as_ref() {
                if tuple.elts.is_empty() {
                    return None;
                }
                let first = format_type_annotation(&tuple.elts[0]);
                if tuple.elts[1..].iter().all(|e| format_type_annotation(e) == first) {
                    return Some(first);
                }
            }
        }
    }
    None
}

/// Check if an expression is a name matching a string.
fn matches_name_str(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Name(n) if n.id.as_str() == name)
}

/// Extract docstring from function body.
fn extract_docstring(body: &[Stmt]) -> Option<String> {
    if let Some(Stmt::Expr(expr_stmt)) = body.first() {
        if let Expr::StringLiteral(s) = expr_stmt.value.as_ref() {
            return Some(s.value.to_string());
        }
    }
    None
}

/// Write extracted functions to the database.
pub fn write_functions(
    db: &ContractDb,
    functions: &[FunctionInfo],
    body_contracts: &[(String, Vec<crate::body_analyzer::BodyContract>)],
    docstring_contracts: &[(String, Vec<crate::docstring_parser::DocstringContract>)],
) -> Result<std::collections::HashMap<String, i64>> {
    let mut node_ids = std::collections::HashMap::new();

    for func in functions {
        let node_id = db.insert_node(&NodeRecord {
            name: func.name.clone(),
            kind: NodeKind::Function,
            source_file: func.source_file.clone(),
            source_line: func.source_line,
        })?;

        node_ids.insert(func.name.clone(), node_id);

        // Type annotation contracts
        if let Some(ref return_type) = func.return_type {
            // For tuple[T, T, ...] with uniform element types, use the element
            // type as the postcondition — individual elements flow through
            // writes_to edges, not the tuple itself.
            let effective_type = if let Some(ref elem_type) = func.tuple_element_type {
                elem_type.as_str()
            } else {
                return_type
                    .split('[')
                    .next()
                    .unwrap_or(return_type)
                    .trim()
            };
            // Only emit type postconditions for value types that are
            // meaningful to compare against model field types. Model class
            // names (e.g. EnergyRecord) aren't comparable to field value
            // types (e.g. Decimal) — for writes_to edges, the field values
            // are the ORM call arguments, not the returned model instance.
            let value_types = ["Decimal", "int", "str", "float", "bool"];
            if effective_type != "unknown" && value_types.contains(&effective_type) {
                db.insert_contract(&ContractRecord {
                    node_id,
                    constraint_type: ConstraintType::Type,
                    param_max_digits: None,
                    param_decimal_places: None,
                    param_max_length: None,
                    param_nullable: None,
                    param_type_name: Some(effective_type.to_string()),
                    param_min_value: None,
                    param_max_value: None,
                    param_choices: None,
                    source_file: func.source_file.clone(),
                    source_line: func.source_line,
                    is_implicit: false,
                    verification_level: VerificationLevel::Extracted,
                    contract_role: Some(ContractRole::Postcondition),
                    dependent_expr: None,
                })?;
            }
        }

        // Nullability from Optional type
        if func.is_return_optional {
            db.insert_contract(&ContractRecord {
                node_id,
                constraint_type: ConstraintType::Nullability,
                param_max_digits: None,
                param_decimal_places: None,
                param_max_length: None,
                param_nullable: Some(1),
                param_type_name: None,
                param_min_value: None,
                param_max_value: None,
                param_choices: None,
                source_file: func.source_file.clone(),
                source_line: func.source_line,
                is_implicit: false,
                verification_level: VerificationLevel::Extracted,
                contract_role: Some(ContractRole::Postcondition),
                dependent_expr: None,
            })?;
        }

        // Docstring contracts (highest priority)
        if let Some((_, doc_contracts)) = docstring_contracts.iter().find(|(n, _)| n == &func.name)
        {
            for dc in doc_contracts {
                // Put the clause's bound in the column the checker reads for its kind.
                let value = |kind: ConstraintType| {
                    if dc.constraint_type.as_str() == kind.as_str() {
                        dc.param_value
                    } else {
                        None
                    }
                };
                let range_bound = value(ConstraintType::Range).map(|v| v as f64);
                db.insert_contract(&ContractRecord {
                    node_id,
                    constraint_type: dc.constraint_type.clone(),
                    param_max_digits: None,
                    param_decimal_places: value(ConstraintType::Precision),
                    param_max_length: value(ConstraintType::Length),
                    param_nullable: value(ConstraintType::Nullability),
                    param_type_name: None,
                    param_min_value: range_bound.filter(|_| dc.is_lower_bound),
                    param_max_value: range_bound.filter(|_| !dc.is_lower_bound),
                    param_choices: None,
                    source_file: func.source_file.clone(),
                    source_line: func.source_line,
                    is_implicit: false,
                    verification_level: VerificationLevel::Assumed,
                    contract_role: Some(dc.role.clone()),
                    dependent_expr: dc.dependent_expr.clone(),
                })?;
            }
        }

        // Body analysis contracts (lower priority than docstring)
        if let Some((_, body_cs)) = body_contracts.iter().find(|(n, _)| n == &func.name) {
            for bc in body_cs {
                // Skip if docstring already provides this constraint type + role
                let dominated = docstring_contracts
                    .iter()
                    .find(|(n, _)| n == &func.name)
                    .map(|(_, dcs)| {
                        dcs.iter().any(|dc| {
                            dc.constraint_type.as_str() == bc.constraint_type.as_str()
                                && dc.role.as_str() == bc.role.as_str()
                        })
                    })
                    .unwrap_or(false);
                // An Optional return annotation already gave a nullability postcondition.
                let duplicate = func.is_return_optional
                    && bc.constraint_type.as_str() == ConstraintType::Nullability.as_str();

                if !dominated && !duplicate {
                    db.insert_contract(&ContractRecord {
                        node_id,
                        constraint_type: bc.constraint_type.clone(),
                        param_max_digits: None,
                        param_decimal_places: bc.param_value,
                        param_max_length: None,
                        param_nullable: bc.param_nullable,
                        param_type_name: None,
                        param_min_value: None,
                        param_max_value: None,
                        param_choices: None,
                        source_file: func.source_file.clone(),
                        source_line: func.source_line,
                        is_implicit: false,
                        verification_level: VerificationLevel::Extracted,
                        contract_role: Some(bc.role.clone()),
                        dependent_expr: None,
                    })?;
                }
            }
        }
    }

    Ok(node_ids)
}
