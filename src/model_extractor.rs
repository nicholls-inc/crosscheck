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
    /// Dotted module name of the defining file.
    pub module: String,
    pub field_name: String,
    pub field_type: String,
    pub max_digits: Option<i64>,
    pub decimal_places: Option<i64>,
    pub max_length: Option<i64>,
    pub null: Option<bool>,
    pub null_is_explicit: bool,
    pub choices: Option<String>,
    /// Integer lower / upper bounds from validators and positive field types.
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
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
        module: crate::resolve::module_name(source_file),
        field_name: field_name.to_string(),
        field_type: field_type.clone(),
        max_digits: None,
        decimal_places: None,
        max_length: field_defaults.and_then(|d| d.max_length),
        null: field_defaults.and_then(|d| d.null),
        null_is_explicit: false,
        choices: None,
        min_value: None,
        max_value: None,
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
                "validators" => apply_validators(&mut field, &keyword.value),
                _ => {}
            }
        }
    }

    // Positive*Field: Django adds MinValueValidator(0) (see the defaults file).
    let implicit_min = field_defaults
        .and_then(|d| d.implicit_validators.as_ref())
        .into_iter()
        .flatten()
        .filter_map(|v| {
            v.strip_prefix("MinValueValidator(")?
                .strip_suffix(')')?
                .trim()
                .parse::<i64>()
                .ok()
        })
        .max();
    let positive = matches!(
        field_type.as_str(),
        "PositiveIntegerField" | "PositiveSmallIntegerField" | "PositiveBigIntegerField"
    );
    for bound in implicit_min.into_iter().chain(positive.then_some(0)) {
        field.min_value = Some(field.min_value.map_or(bound, |m| m.max(bound)));
    }

    Some(field)
}

/// Read `MinValueValidator(n)` / `MaxValueValidator(n)` (integer `n`) from a
/// `validators=[...]` list. With several, the tightest bound wins (all apply).
fn apply_validators(field: &mut ModelField, expr: &Expr) {
    let items: &[Expr] = match expr {
        Expr::List(l) => &l.elts,
        Expr::Tuple(t) => &t.elts,
        _ => return,
    };
    for item in items {
        let Expr::Call(call) = item else { continue };
        let name = match call.func.as_ref() {
            Expr::Name(n) => n.id.to_string(),
            Expr::Attribute(a) => a.attr.to_string(),
            _ => continue,
        };
        let arg = call.arguments.args.first().or_else(|| {
            call.arguments
                .keywords
                .iter()
                .find(|k| k.arg.as_deref() == Some("limit_value"))
                .map(|k| &k.value)
        });
        let Some(value) = arg.and_then(crate::value_analysis::int_literal) else { continue };
        match name.as_str() {
            "MinValueValidator" => field.min_value = Some(field.min_value.map_or(value, |m| m.max(value))),
            "MaxValueValidator" => field.max_value = Some(field.max_value.map_or(value, |m| m.min(value))),
            _ => {}
        }
    }
}

/// Qualified node name of a model field (`module.Model.field`).
pub fn field_qualified(field: &ModelField) -> String {
    crate::resolve::qualify(&field.module, &format!("{}.{}", field.model_name, field.field_name))
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

/// Write extracted model fields to the database. `names` maps a field's
/// qualified name to its display name. Returns qualified name → node ID.
pub fn write_model_fields(
    db: &ContractDb,
    fields: &[ModelField],
    names: &std::collections::HashMap<String, String>,
) -> Result<std::collections::HashMap<String, i64>> {
    let mut ids = std::collections::HashMap::new();
    for field in fields {
        let qualified = field_qualified(field);
        if ids.contains_key(&qualified) {
            continue;
        }
        let full_name = names
            .get(&qualified)
            .cloned()
            .unwrap_or_else(|| format!("{}.{}", field.model_name, field.field_name));

        // Insert node for the model field
        let node_id = db.insert_node(&NodeRecord {
            name: full_name,
            qualified_name: Some(qualified.clone()),
            kind: NodeKind::Model,
            source_file: field.source_file.clone(),
            source_line: field.source_line,
        })?;
        ids.insert(qualified, node_id);

        let base = |constraint_type: ConstraintType| {
            ContractRecord::new(
                node_id,
                constraint_type,
                ContractRole::Precondition,
                VerificationLevel::Extracted,
                &field.source_file,
                field.source_line,
            )
        };

        // Insert precision constraint for DecimalField
        if field.field_type == "DecimalField" {
            if let Some(decimal_places) = field.decimal_places {
                db.insert_contract(&ContractRecord {
                    param_max_digits: field.max_digits,
                    param_decimal_places: Some(decimal_places),
                    ..base(ConstraintType::Precision)
                })?;
            }
        }

        // Insert length constraint for CharField
        if field.field_type == "CharField" || field.field_type == "SlugField" {
            if let Some(max_length) = field.max_length {
                db.insert_contract(&ContractRecord {
                    param_max_length: Some(max_length),
                    ..base(ConstraintType::Length)
                })?;
            }
        }

        // Insert nullability constraint
        if let Some(null) = field.null {
            db.insert_contract(&ContractRecord {
                param_nullable: Some(if null { 1 } else { 0 }),
                is_implicit: !field.null_is_explicit,
                ..base(ConstraintType::Nullability)
            })?;
        }

        // Insert choices constraint
        if let Some(ref choices) = field.choices {
            db.insert_contract(&ContractRecord {
                param_choices: Some(choices.clone()),
                ..base(ConstraintType::Choices)
            })?;
        }

        // Insert range constraint (lower and/or upper bound)
        if field.min_value.is_some() || field.max_value.is_some() {
            db.insert_contract(&ContractRecord {
                param_min_value: field.min_value.map(|v| v as f64),
                param_max_value: field.max_value.map(|v| v as f64),
                ..base(ConstraintType::Range)
            })?;
        }

        // Insert type constraint
        let type_name = match field.field_type.as_str() {
            "DecimalField" => Some("Decimal"),
            "IntegerField" | "PositiveIntegerField" | "PositiveSmallIntegerField"
            | "PositiveBigIntegerField" | "SmallIntegerField" | "BigIntegerField" => Some("int"),
            "CharField" | "TextField" | "SlugField" => Some("str"),
            "BooleanField" => Some("bool"),
            "FloatField" => Some("float"),
            _ => None,
        };
        if let Some(tn) = type_name {
            db.insert_contract(&ContractRecord {
                param_type_name: Some(tn.to_string()),
                ..base(ConstraintType::Type)
            })?;
        }
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(src: &str) -> Vec<ModelField> {
        let stmts = match ruff_python_parser::parse_unchecked(src, ruff_python_parser::Mode::Module.into())
            .into_syntax()
        {
            ruff_python_ast::Mod::Module(m) => m.body,
            _ => unreachable!(),
        };
        let defaults = crate::defaults::load_defaults("4.2").unwrap();
        extract_models(&stmts, "app/models.py", &defaults)
    }

    #[test]
    fn test_validator_and_positive_bounds() {
        let fs = fields(
            "class M(models.Model):\n    \
             a = models.IntegerField(validators=[MinValueValidator(0), validators.MaxValueValidator(100)])\n    \
             b = models.PositiveIntegerField()\n    \
             c = models.PositiveSmallIntegerField(validators=[MinValueValidator(5)])\n    \
             d = models.IntegerField(validators=[MinValueValidator(Decimal('0.5'))])\n",
        );
        let bounds: Vec<(Option<i64>, Option<i64>)> = fs.iter().map(|f| (f.min_value, f.max_value)).collect();
        assert_eq!(bounds, [(Some(0), Some(100)), (Some(0), None), (Some(5), None), (None, None)]);
        assert_eq!(field_qualified(&fs[0]), "app.models.M.a");
    }
}
