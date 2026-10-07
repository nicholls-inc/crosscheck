//! Django model field extraction.
//!
//! A class is a Django model when a base is `models.Model` (or `Model`,
//! `django.db.models.Model`), or a project class that is one (abstract bases
//! with `class Meta: abstract = True` and concrete multi-table parents alike;
//! the project-level recognition lives in `extractor::Project::build`).
//! Inherited fields are reported under the child (`Child.field`); a child
//! field replaces a parent field of the same name.

use std::collections::HashMap;

use anyhow::Result;
use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::bounds::{self, Dec};
use crate::db::{
    ContractDb, ContractRecord, ContractRole, ConstraintType, NodeKind, NodeRecord,
    VerificationLevel,
};
use crate::defaults::FieldDefaults;

/// Extracted model field information.
#[derive(Debug, Clone)]
pub struct ModelField {
    pub model_name: String,
    /// Dotted module name of the model that has the field (the child, for inherited fields).
    pub module: String,
    pub field_name: String,
    pub field_type: String,
    pub max_digits: Option<i64>,
    pub decimal_places: Option<i64>,
    pub max_length: Option<i64>,
    pub null: Option<bool>,
    pub null_is_explicit: bool,
    /// Allowed values (`choices=`), when they could be read.
    pub choices: Option<Vec<String>>,
    /// The `choices=` expression, for project-level resolution (constants,
    /// `SomeTextChoices.choices`).
    pub choices_expr: Option<Expr>,
    /// Lower / upper bounds (micros) from validators and positive field types.
    pub min_value: Option<Dec>,
    pub max_value: Option<Dec>,
    pub source_file: String,
    pub source_line: u32,
}

/// Extract Django model definitions from a parsed Python module: classes
/// whose bases name `models.Model` directly (no project context; see
/// `Project::build` for inheritance). Each field's `source_line` is the
/// byte offset of its definition.
pub fn extract_models(
    stmts: &[Stmt],
    source_file: &str,
    defaults: &HashMap<String, FieldDefaults>,
) -> Vec<ModelField> {
    let mut fields = Vec::new();
    for class_def in crate::resolve::module_classes(stmts) {
        let direct = class_def
            .arguments
            .as_ref()
            .is_some_and(|args| args.args.iter().any(is_django_root));
        if direct {
            fields.extend(extract_model_class(class_def, source_file, defaults));
        }
    }
    fields
}

/// Whether a base-class expression names Django's `Model` itself.
pub fn is_django_root(expr: &Expr) -> bool {
    matches_dotted_name(expr, &["models", "Model"])
        || matches_dotted_name(expr, &["django", "db", "models", "Model"])
        || matches_name(expr, "Model")
}

/// Fields declared in the body of a model class (not inherited ones).
/// `source_line` holds the byte offset of each field's definition.
pub fn extract_model_class(
    class_def: &ast::StmtClassDef,
    source_file: &str,
    defaults: &HashMap<String, FieldDefaults>,
) -> Vec<ModelField> {
    let model_name = class_def.name.to_string();
    let constants = crate::resolve::module_constants(&class_def.body);
    class_def
        .body
        .iter()
        .filter_map(|stmt| {
            let mut f = extract_field_assignment(stmt, &model_name, source_file, defaults)?;
            // Choices given by a class-body constant (`choices=STATUS`).
            if f.choices.is_none() {
                if let Some(Expr::Name(n)) = &f.choices_expr {
                    f.choices = constants.get(n.id.as_str()).and_then(literal_choices);
                }
            }
            Some(f)
        })
        .collect()
}

/// Whether the class body says `class Meta: abstract = True`.
pub fn is_abstract(class_def: &ast::StmtClassDef) -> bool {
    class_def.body.iter().any(|s| match s {
        Stmt::ClassDef(meta) if meta.name.as_str() == "Meta" => meta.body.iter().any(|m| {
            matches!(m, Stmt::Assign(a)
                if a.targets.len() == 1
                    && matches!(&a.targets[0], Expr::Name(n) if n.id.as_str() == "abstract")
                    && matches!(a.value.as_ref(), Expr::BooleanLiteral(b) if b.value))
        }),
        _ => false,
    })
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
    defaults: &HashMap<String, FieldDefaults>,
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
    defaults: &HashMap<String, FieldDefaults>,
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
        choices_expr: None,
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
                    field.choices = literal_choices(&keyword.value);
                    field.choices_expr = Some(keyword.value.clone());
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
            let arg = v.strip_prefix("MinValueValidator(")?.strip_suffix(')')?;
            bounds::parse_decimal(arg)
        })
        .max();
    let positive = matches!(
        field_type.as_str(),
        "PositiveIntegerField" | "PositiveSmallIntegerField" | "PositiveBigIntegerField"
    );
    for bound in implicit_min.into_iter().chain(positive.then_some(Dec::ZERO)) {
        field.min_value = Some(field.min_value.map_or(bound, |m| m.max(bound)));
    }

    // DecimalValidator rejects more than max_digits - decimal_places whole digits.
    if field_type == "DecimalField" {
        if let Some(limit) = field.max_digits.and_then(|m| bounds::digits_limit(m, field.decimal_places)) {
            bounds::narrow_to(limit, &mut field.min_value, &mut field.max_value);
        }
    }

    Some(field)
}

/// Read `MinValueValidator(n)` / `MaxValueValidator(n)` (`n` an int, float
/// or `Decimal` literal) from a `validators=[...]` list. With several, the
/// tightest bound wins (all apply).
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
        let Some(arg) = arg else { continue };
        match name.as_str() {
            "MinValueValidator" => {
                if let Some(v) = bounds::literal_bound(arg) {
                    field.min_value = Some(field.min_value.map_or(v, |m| m.max(v)));
                }
            }
            "MaxValueValidator" => {
                if let Some(v) = bounds::literal_bound(arg) {
                    field.max_value = Some(field.max_value.map_or(v, |m| m.min(v)));
                }
            }
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

/// Values of a literal `choices=` list or tuple: `[("a", "Active"), ...]`,
/// `["a", "b"]`, and named groups `[("Group", [("a", "A"), ...]), ...]`.
/// `None` unless every entry is readable.
pub fn literal_choices(expr: &Expr) -> Option<Vec<String>> {
    let items: &[Expr] = match expr {
        Expr::List(l) => &l.elts,
        Expr::Tuple(t) => &t.elts,
        _ => return None,
    };
    let mut out = Vec::new();
    for item in items {
        let pair: Option<&[Expr]> = match item {
            Expr::Tuple(t) => Some(&t.elts[..]),
            Expr::List(l) => Some(&l.elts[..]),
            _ => None,
        };
        match pair {
            Some([first, second]) => {
                // A named group `("Group", [(value, label), ...])`: its entries.
                let group = matches!(second, Expr::List(_) | Expr::Tuple(_))
                    .then(|| literal_choices(second))
                    .flatten();
                match group {
                    Some(values) => out.extend(values),
                    None => out.push(crate::resolve::literal_choice(first)?),
                }
            }
            Some(_) => return None,
            None => out.push(crate::resolve::literal_choice(item)?),
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Value type a Django field stores, for the type contract. Numeric fields
/// accept `int`, `float` and `Decimal` alike (Django converts), so they have
/// no type contract.
fn type_contract(field_type: &str) -> Option<&'static str> {
    match field_type {
        "CharField" | "TextField" | "SlugField" => Some("str"),
        "BooleanField" => Some("bool"),
        _ => None,
    }
}

/// The field's requirements as facts (what `write_model_fields` states as
/// preconditions): what a read of the field yields, since every write to it
/// is checked against them.
pub fn requirement_facts(field: &ModelField) -> crate::value_analysis::ValueFacts {
    use crate::value_analysis::{Dep, ValueFacts};
    // A FloatField read is a binary float: arithmetic on it rounds, so it
    // gets no exact interval bounds. Weak: the type carries no contract.
    let float = field.field_type == "FloatField";
    ValueFacts {
        nullable: field.null,
        type_name: type_contract(&field.field_type)
            .or(float.then_some("float"))
            .map(str::to_string),
        weak_type: float,
        precision: (field.field_type == "DecimalField")
            .then_some(field.decimal_places)
            .flatten()
            .map(Dep::Lit),
        max_length: matches!(field.field_type.as_str(), "CharField" | "SlugField")
            .then_some(field.max_length)
            .flatten(),
        min_value: field.min_value,
        max_value: field.max_value,
        choices: field.choices.clone(),
        ..ValueFacts::default()
    }
}

/// Write extracted model fields to the database. `names` maps a field's
/// qualified name to its display name. Returns qualified name → node ID.
pub fn write_model_fields(
    db: &ContractDb,
    fields: &[ModelField],
    names: &HashMap<String, String>,
) -> Result<HashMap<String, i64>> {
    let mut ids = HashMap::new();
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
            is_call_site: false,
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
        if let Some(choices) = &field.choices {
            db.insert_contract(&base(ConstraintType::Choices).with_choices(choices))?;
        }

        // Insert range constraint (lower and/or upper bound)
        if field.min_value.is_some() || field.max_value.is_some() {
            db.insert_contract(&base(ConstraintType::Range).with_range(
                field.min_value,
                field.max_value,
                true,
            ))?;
        }

        if let Some(tn) = type_contract(&field.field_type) {
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
    #[allow(unused_imports)]
    use crate::bounds::mu;

    fn parse(src: &str) -> Vec<Stmt> {
        match ruff_python_parser::parse_unchecked(src, ruff_python_parser::Mode::Module.into())
            .into_syntax()
        {
            ruff_python_ast::Mod::Module(m) => m.body.to_vec(),
            _ => unreachable!(),
        }
    }

    fn fields(src: &str) -> Vec<ModelField> {
        let defaults = crate::defaults::load_defaults("4.2").unwrap();
        extract_models(&parse(src), "app/models.py", &defaults)
    }

    #[test]
    fn test_validator_and_positive_bounds() {
        let fs = fields(
            "class M(models.Model):\n    \
             a = models.IntegerField(validators=[MinValueValidator(0), validators.MaxValueValidator(100)])\n    \
             b = models.PositiveIntegerField()\n    \
             c = models.PositiveSmallIntegerField(validators=[MinValueValidator(5)])\n    \
             d = models.DecimalField(validators=[MinValueValidator(Decimal('0.5')), MaxValueValidator(9.99)])\n    \
             e = models.IntegerField(validators=[MinValueValidator(limit)])\n",
        );
        let bounds: Vec<(Option<Dec>, Option<Dec>)> =
            fs.iter().map(|f| (f.min_value, f.max_value)).collect();
        assert_eq!(
            bounds,
            [
                (mu(0), mu(100_000_000)),
                (mu(0), None),
                (mu(5_000_000), None),
                (mu(500_000), mu(9_990_000)),
                (None, None),
            ]
        );
        assert_eq!(field_qualified(&fs[0]), "app.models.M.a");
    }

    #[test]
    fn test_max_digits_bounds() {
        let fs = fields(
            "class M(models.Model):\n    \
             a = models.DecimalField(max_digits=5, decimal_places=2)\n    \
             b = models.DecimalField(max_digits=5, decimal_places=2, validators=[MinValueValidator(0), MaxValueValidator(5000)])\n    \
             c = models.DecimalField(max_digits=5, decimal_places=places)\n    \
             d = models.DecimalField(max_digits=width, decimal_places=2)\n    \
             e = models.IntegerField(max_digits=5)\n",
        );
        let bounds: Vec<(Option<Dec>, Option<Dec>)> =
            fs.iter().map(|f| (f.min_value, f.max_value)).collect();
        assert_eq!(
            bounds,
            [
                (mu(-999_990_000), mu(999_990_000)),
                (mu(0), mu(999_990_000)),
                (mu(-99_999_000_000), mu(99_999_000_000)),
                (None, None),
                (None, None),
            ]
        );
    }

    #[test]
    fn test_choices_forms() {
        let fs = fields(
            "class M(models.Model):\n    \
             STATUS = [('a', _('Active')), ('x', 'Expired')]\n    \
             a = models.CharField(max_length=5, choices=[('a', 'Active'), ('x', 'Expired')])\n    \
             b = models.CharField(max_length=5, choices=(('p', 'P'),))\n    \
             c = models.CharField(max_length=5, choices=STATUS)\n    \
             d = models.IntegerField(choices=[(1, 'One'), (2, 'Two')])\n    \
             e = models.CharField(max_length=5, choices=[('Group', [('g1', 'G1'), ('g2', 'G2')]), ('o', 'O')])\n    \
             f = models.CharField(max_length=5, choices=Status.choices)\n",
        );
        let choices: Vec<Option<Vec<String>>> = fs.iter().map(|f| f.choices.clone()).collect();
        let v = |xs: &[&str]| Some(xs.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(
            choices,
            [
                v(&["a", "x"]),
                v(&["p"]),
                v(&["a", "x"]),
                v(&["1", "2"]),
                v(&["g1", "g2", "o"]),
                None, // resolved with the project index
            ]
        );
        assert!(fs[5].choices_expr.is_some());
    }

    #[test]
    fn test_abstract_meta() {
        let stmts = parse(
            "class A(models.Model):\n    class Meta:\n        abstract = True\nclass B(models.Model):\n    class Meta:\n        ordering = ['x']\n",
        );
        let abstracts: Vec<bool> = stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::ClassDef(c) => Some(is_abstract(c)),
                _ => None,
            })
            .collect();
        assert_eq!(abstracts, [true, false]);
    }
}
