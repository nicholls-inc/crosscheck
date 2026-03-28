use anyhow::Result;
use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::db::{
    ContractDb, ContractRecord, ContractRole, ConstraintType, NodeKind, NodeRecord,
    VerificationLevel,
};
use crate::defaults::FieldDefaults;

/// Extracted model field information.
#[derive(Debug)]
pub struct ModelField {
    pub model_name: String,
    pub field_name: String,
    pub field_type: String,
    pub max_digits: Option<i64>,
    pub decimal_places: Option<i64>,
    pub max_length: Option<i64>,
    pub null: Option<bool>,
    pub null_is_explicit: bool,
    pub choices: Option<String>,
    pub source_file: String,
    pub source_line: u32,
}

/// Extract Django model definitions from a parsed Python module.
pub fn extract_models(
    stmts: &[Stmt],
    source_file: &str,
    defaults: &std::collections::HashMap<String, FieldDefaults>,
) -> Vec<ModelField> {
    let mut fields = Vec::new();

    for stmt in stmts {
        if let Stmt::ClassDef(class_def) = stmt {
            // Check if this class inherits from models.Model
            if is_django_model(class_def) {
                let model_name = class_def.name.to_string();
                for body_stmt in &class_def.body {
                    if let Some(field) =
                        extract_field_assignment(body_stmt, &model_name, source_file, defaults)
                    {
                        fields.push(field);
                    }
                }
            }
        }
    }

    fields
}

/// Check if a class definition inherits from models.Model.
fn is_django_model(class_def: &ast::StmtClassDef) -> bool {
    class_def
        .arguments
        .as_ref()
        .map(|args| {
            args.args.iter().any(|arg| {
                matches_dotted_name(arg, &["models", "Model"])
                    || matches_name(arg, "Model")
            })
        })
        .unwrap_or(false)
}

/// Check if an expression is a dotted name like `models.Model`.
fn matches_dotted_name(expr: &Expr, parts: &[&str]) -> bool {
    match parts {
        [] => false,
        [name] => matches_name(expr, name),
        _ => {
            if let Expr::Attribute(attr) = expr {
                let remaining = &parts[..parts.len() - 1];
                attr.attr.as_str() == parts[parts.len() - 1]
                    && matches_dotted_name(&attr.value, remaining)
            } else {
                false
            }
        }
    }
}

/// Check if an expression is a simple name.
fn matches_name(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Name(n) if n.id.as_str() == name)
}

/// Extract a field assignment from a class body statement.
fn extract_field_assignment(
    stmt: &Stmt,
    model_name: &str,
    source_file: &str,
    defaults: &std::collections::HashMap<String, FieldDefaults>,
) -> Option<ModelField> {
    if let Stmt::Assign(assign) = stmt {
        if assign.targets.len() == 1 {
            if let Expr::Name(name) = &assign.targets[0] {
                if let Expr::Call(call) = assign.value.as_ref() {
                    return extract_field_from_call(
                        &name.id,
                        call,
                        model_name,
                        source_file,
                        defaults,
                    );
                }
            }
        }
    }
    // Also handle annotated assignments
    if let Stmt::AnnAssign(ann_assign) = stmt {
        if let Expr::Name(name) = ann_assign.target.as_ref() {
            if let Some(value) = &ann_assign.value {
                if let Expr::Call(call) = value.as_ref() {
                    return extract_field_from_call(
                        &name.id,
                        call,
                        model_name,
                        source_file,
                        defaults,
                    );
                }
            }
        }
    }
    None
}

/// Extract field info from a models.XxxField(...) call.
fn extract_field_from_call(
    field_name: &str,
    call: &ast::ExprCall,
    model_name: &str,
    source_file: &str,
    defaults: &std::collections::HashMap<String, FieldDefaults>,
) -> Option<ModelField> {
    let field_type = get_field_type_name(&call.func)?;

    // Get defaults for this field type
    let field_defaults = defaults.get(&field_type);

    let mut field = ModelField {
        model_name: model_name.to_string(),
        field_name: field_name.to_string(),
        field_type: field_type.clone(),
        max_digits: None,
        decimal_places: None,
        max_length: field_defaults.and_then(|d| d.max_length),
        null: field_defaults.and_then(|d| d.null),
        null_is_explicit: false,
        choices: None,
        source_file: source_file.to_string(),
        source_line: call.range.start().to_u32(),
    };

    // Extract keyword arguments
    for keyword in &call.arguments.keywords {
        if let Some(arg_name) = &keyword.arg {
            match arg_name.as_str() {
                "max_digits" => {
                    field.max_digits = extract_int_value(&keyword.value);
                }
                "decimal_places" => {
                    field.decimal_places = extract_int_value(&keyword.value);
                }
                "max_length" => {
                    field.max_length = extract_int_value(&keyword.value);
                }
                "null" => {
                    field.null = extract_bool_value(&keyword.value);
                    field.null_is_explicit = true;
                }
                "choices" => {
                    field.choices = extract_choices_value(&keyword.value);
                }
                _ => {}
            }
        }
    }

    Some(field)
}

/// Get the field type name from a call expression (e.g., "DecimalField" from models.DecimalField).
fn get_field_type_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Attribute(attr) => {
            let name = attr.attr.to_string();
            if name.ends_with("Field") {
                Some(name)
            } else {
                None
            }
        }
        Expr::Name(name) => {
            if name.id.ends_with("Field") {
                Some(name.id.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract an integer value from an expression.
fn extract_int_value(expr: &Expr) -> Option<i64> {
    if let Expr::NumberLiteral(num) = expr {
        match &num.value {
            ast::Number::Int(i) => i.as_i64(),
            _ => None,
        }
    } else {
        None
    }
}

/// Extract a boolean value from an expression.
fn extract_bool_value(expr: &Expr) -> Option<bool> {
    if let Expr::BooleanLiteral(b) = expr {
        Some(b.value)
    } else {
        None
    }
}

/// Extract choices as comma-separated string.
fn extract_choices_value(expr: &Expr) -> Option<String> {
    if let Expr::List(list) = expr {
        let choices: Vec<String> = list
            .elts
            .iter()
            .filter_map(|elt| {
                if let Expr::Tuple(tuple) = elt {
                    tuple.elts.first().and_then(|e| {
                        if let Expr::StringLiteral(s) = e {
                            Some(s.value.to_string())
                        } else {
                            None
                        }
                    })
                } else if let Expr::StringLiteral(s) = elt {
                    Some(s.value.to_string())
                } else {
                    None
                }
            })
            .collect();
        if choices.is_empty() {
            None
        } else {
            Some(choices.join(","))
        }
    } else {
        None
    }
}

/// Write extracted model fields to the database.
pub fn write_model_fields(db: &ContractDb, fields: &[ModelField]) -> Result<()> {
    for field in fields {
        let full_name = format!("{}.{}", field.model_name, field.field_name);

        // Insert node for the model field
        let node_id = db.insert_node(&NodeRecord {
            name: full_name,
            kind: NodeKind::Model,
            source_file: field.source_file.clone(),
            source_line: field.source_line,
        })?;

        // Insert precision constraint for DecimalField
        if field.field_type == "DecimalField" {
            if let Some(decimal_places) = field.decimal_places {
                db.insert_contract(&ContractRecord {
                    node_id,
                    constraint_type: ConstraintType::Precision,
                    param_max_digits: field.max_digits,
                    param_decimal_places: Some(decimal_places),
                    param_max_length: None,
                    param_nullable: None,
                    param_type_name: None,
                    param_min_value: None,
                    param_max_value: None,
                    param_choices: None,
                    source_file: field.source_file.clone(),
                    source_line: field.source_line,
                    is_implicit: false,
                    verification_level: VerificationLevel::Extracted,
                    contract_role: Some(ContractRole::Precondition),
                    dependent_expr: None,
                })?;
            }
        }

        // Insert length constraint for CharField
        if field.field_type == "CharField" || field.field_type == "SlugField" {
            if let Some(max_length) = field.max_length {
                db.insert_contract(&ContractRecord {
                    node_id,
                    constraint_type: ConstraintType::Length,
                    param_max_digits: None,
                    param_decimal_places: None,
                    param_max_length: Some(max_length),
                    param_nullable: None,
                    param_type_name: None,
                    param_min_value: None,
                    param_max_value: None,
                    param_choices: None,
                    source_file: field.source_file.clone(),
                    source_line: field.source_line,
                    is_implicit: false,
                    verification_level: VerificationLevel::Extracted,
                    contract_role: Some(ContractRole::Precondition),
                    dependent_expr: None,
                })?;
            }
        }

        // Insert nullability constraint
        if let Some(null) = field.null {
            db.insert_contract(&ContractRecord {
                node_id,
                constraint_type: ConstraintType::Nullability,
                param_max_digits: None,
                param_decimal_places: None,
                param_max_length: None,
                param_nullable: Some(if null { 1 } else { 0 }),
                param_type_name: None,
                param_min_value: None,
                param_max_value: None,
                param_choices: None,
                source_file: field.source_file.clone(),
                source_line: field.source_line,
                is_implicit: !field.null_is_explicit,
                verification_level: VerificationLevel::Extracted,
                contract_role: Some(ContractRole::Precondition),
                dependent_expr: None,
            })?;
        }

        // Insert choices constraint
        if let Some(ref choices) = field.choices {
            db.insert_contract(&ContractRecord {
                node_id,
                constraint_type: ConstraintType::Choices,
                param_max_digits: None,
                param_decimal_places: None,
                param_max_length: None,
                param_nullable: None,
                param_type_name: None,
                param_min_value: None,
                param_max_value: None,
                param_choices: Some(choices.clone()),
                source_file: field.source_file.clone(),
                source_line: field.source_line,
                is_implicit: false,
                verification_level: VerificationLevel::Extracted,
                contract_role: Some(ContractRole::Precondition),
                dependent_expr: None,
            })?;
        }

        // Insert type constraint
        let type_name = match field.field_type.as_str() {
            "DecimalField" => Some("Decimal"),
            "IntegerField" | "PositiveIntegerField" | "SmallIntegerField"
            | "BigIntegerField" => Some("int"),
            "CharField" | "TextField" | "SlugField" => Some("str"),
            "BooleanField" => Some("bool"),
            "FloatField" => Some("float"),
            _ => None,
        };
        if let Some(tn) = type_name {
            db.insert_contract(&ContractRecord {
                node_id,
                constraint_type: ConstraintType::Type,
                param_max_digits: None,
                param_decimal_places: None,
                param_max_length: None,
                param_nullable: None,
                param_type_name: Some(tn.to_string()),
                param_min_value: None,
                param_max_value: None,
                param_choices: None,
                source_file: field.source_file.clone(),
                source_line: field.source_line,
                is_implicit: false,
                verification_level: VerificationLevel::Extracted,
                contract_role: Some(ContractRole::Precondition),
                dependent_expr: None,
            })?;
        }
    }

    Ok(())
}
