//! Plain-Python data class extraction.
//!
//! Recognises the common ways of declaring a typed record in Python and turns
//! each annotated field into a contract-graph node with precondition contracts:
//!
//! - `@dataclass` / `@dataclasses.dataclass(...)`
//! - attrs: `@attr.s`, `@attr.define`, `@attrs.define`, `@define`, `@frozen`, `@mutable`
//! - pydantic: subclasses of `BaseModel` / `pydantic.BaseModel`
//! - `typing.NamedTuple` and `typing.TypedDict` subclasses
//! - subclasses of any class above that is defined in the same project
//!
//! Contracts come from the annotation and, for pydantic, from `Field(...)`,
//! `Annotated[T, Field(...)]` and the `con*()` helpers:
//!
//! | Source                                  | Contract                         |
//! |-----------------------------------------|----------------------------------|
//! | `Decimal`, `int`, `str`, `float`, `bool` | type                             |
//! | `Optional[T]`, `T \| None`, `Union[T, None]` | nullability = 1 (else 0)     |
//! | `max_digits` / `decimal_places`          | precision                        |
//! | `max_length`                             | length                           |
//! | `le=N`, or `lt=N` on an `int` field      | range (upper bound N, or N - 1)  |
//!
//! Field nodes use kind `model`, so the checker treats them as path targets
//! exactly like Django model fields. Constructor calls such as
//! `Invoice(total=x)` or `Point(1, 2)` become `writes_to` edges (see
//! `edge_discovery`).

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use ruff_python_ast::{self as ast, Expr, Number, Stmt, UnaryOp};

use crate::db::{
    ContractDb, ContractRecord, ContractRole, ConstraintType, NodeKind, NodeRecord,
    VerificationLevel,
};

/// Value types whose names are comparable with function type postconditions.
const VALUE_TYPES: [&str; 5] = ["Decimal", "int", "str", "float", "bool"];

/// Which Python construct declared the class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataClassKind {
    Dataclass,
    Attrs,
    Pydantic,
    NamedTuple,
    TypedDict,
}

impl DataClassKind {
    /// Whether positional constructor arguments map to fields in declaration order.
    pub fn positional_fields(self) -> bool {
        matches!(
            self,
            DataClassKind::Dataclass | DataClassKind::Attrs | DataClassKind::NamedTuple
        )
    }
}

/// A class definition found in the project, before recognition.
#[derive(Debug, Clone)]
pub struct ClassCandidate {
    pub class_def: ast::StmtClassDef,
    pub source_file: String,
    /// Line number of the class statement (1-based).
    pub source_line: u32,
    /// Line number of each class-body statement, parallel to `class_def.body`.
    pub body_lines: Vec<u32>,
}

/// One annotated field of a recognised data class.
#[derive(Debug, Clone, PartialEq)]
pub struct DataClassField {
    pub class_name: String,
    pub field_name: String,
    pub type_name: Option<String>,
    /// `None` when the annotation says nothing about None (e.g. `Any`).
    pub nullable: Option<bool>,
    pub max_digits: Option<i64>,
    pub decimal_places: Option<i64>,
    pub max_length: Option<i64>,
    pub max_value: Option<i64>,
    pub source_file: String,
    pub source_line: u32,
}

/// A recognised data class with its fields, inherited ones first.
#[derive(Debug, Clone)]
pub struct DataClass {
    pub name: String,
    pub kind: DataClassKind,
    pub fields: Vec<DataClassField>,
}

/// Collect top-level class definitions from a module.
pub fn collect_classes(stmts: &[Stmt], source: &str, source_file: &str) -> Vec<ClassCandidate> {
    stmts
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::ClassDef(class_def) => Some(ClassCandidate {
                class_def: class_def.clone(),
                source_file: source_file.to_string(),
                source_line: crate::source::line_of(source, class_def.range.start().to_u32()),
                body_lines: class_def
                    .body
                    .iter()
                    .map(|s| crate::source::line_of(source, stmt_start(s)))
                    .collect(),
            }),
            _ => None,
        })
        .collect()
}

/// Start offset of a class-body statement. Only annotated assignments become
/// fields, so other statements report offset 0.
fn stmt_start(stmt: &Stmt) -> u32 {
    match stmt {
        Stmt::AnnAssign(ann) => ann.range.start().to_u32(),
        _ => 0,
    }
}

/// Recognise data classes across the whole project and resolve inheritance.
pub fn resolve_data_classes(candidates: &[ClassCandidate]) -> Vec<DataClass> {
    let by_name: HashMap<&str, &ClassCandidate> = candidates
        .iter()
        .map(|c| (c.class_def.name.as_str(), c))
        .collect();

    let mut kinds: HashMap<String, DataClassKind> = HashMap::new();
    for c in candidates {
        if let Some(kind) = direct_kind(&c.class_def) {
            kinds.insert(c.class_def.name.to_string(), kind);
        }
    }
    // Propagate recognition to project-local subclasses until a fixed point.
    loop {
        let mut changed = false;
        for c in candidates {
            let name = c.class_def.name.to_string();
            if kinds.contains_key(&name) {
                continue;
            }
            let inherited = base_names(&c.class_def)
                .iter()
                .find_map(|b| kinds.get(b.as_str()).copied());
            if let Some(kind) = inherited {
                kinds.insert(name, kind);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut result = Vec::new();
    for c in candidates {
        let name = c.class_def.name.to_string();
        if let Some(&kind) = kinds.get(&name) {
            let mut visiting = HashSet::new();
            let fields = fields_with_inheritance(&name, &by_name, &kinds, &mut visiting);
            result.push(DataClass { name, kind, fields });
        }
    }
    result
}

/// Fields of `name`, parents first; a child field replaces a parent field of the same name.
fn fields_with_inheritance(
    name: &str,
    by_name: &HashMap<&str, &ClassCandidate>,
    kinds: &HashMap<String, DataClassKind>,
    visiting: &mut HashSet<String>,
) -> Vec<DataClassField> {
    let Some(candidate) = by_name.get(name) else {
        return Vec::new();
    };
    if !visiting.insert(name.to_string()) {
        return Vec::new();
    }
    let mut fields: Vec<DataClassField> = Vec::new();
    for base in base_names(&candidate.class_def) {
        if kinds.contains_key(&base) {
            for f in fields_with_inheritance(&base, by_name, kinds, visiting) {
                fields.retain(|existing| existing.field_name != f.field_name);
                fields.push(f);
            }
        }
    }
    for f in own_fields(candidate) {
        if let Some(pos) = fields.iter().position(|e| e.field_name == f.field_name) {
            fields[pos] = f;
        } else {
            fields.push(f);
        }
    }
    // Inherited fields are reported under the child's class name.
    for f in &mut fields {
        f.class_name = name.to_string();
    }
    fields
}

/// Recognise a class from its own decorators and bases (no project lookup).
fn direct_kind(class_def: &ast::StmtClassDef) -> Option<DataClassKind> {
    for dec in &class_def.decorator_list {
        let target = match &dec.expression {
            Expr::Call(call) => call.func.as_ref(),
            other => other,
        };
        match dotted_name(target).as_deref() {
            Some("dataclass") | Some("dataclasses.dataclass") => {
                return Some(DataClassKind::Dataclass)
            }
            Some(
                "attr.s" | "attr.attrs" | "attr.define" | "attr.frozen" | "attr.mutable"
                | "attrs.define" | "attrs.frozen" | "attrs.mutable" | "define" | "frozen"
                | "mutable",
            ) => return Some(DataClassKind::Attrs),
            _ => {}
        }
    }
    for base in base_names(class_def) {
        match base.as_str() {
            "BaseModel" | "pydantic.BaseModel" => return Some(DataClassKind::Pydantic),
            "NamedTuple" | "typing.NamedTuple" => return Some(DataClassKind::NamedTuple),
            "TypedDict" | "typing.TypedDict" | "typing_extensions.TypedDict" => {
                return Some(DataClassKind::TypedDict)
            }
            _ => {}
        }
    }
    None
}

/// Dotted names of a class's positional bases.
fn base_names(class_def: &ast::StmtClassDef) -> Vec<String> {
    class_def
        .arguments
        .as_ref()
        .map(|args| args.args.iter().filter_map(dotted_name).collect())
        .unwrap_or_default()
}

/// `a.b.c` for a Name/Attribute chain.
fn dotted_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(n) => Some(n.id.to_string()),
        Expr::Attribute(attr) => Some(format!("{}.{}", dotted_name(&attr.value)?, attr.attr)),
        _ => None,
    }
}

/// Last segment of a dotted name (`typing.Optional` → `Optional`).
fn last_segment(expr: &Expr) -> Option<String> {
    dotted_name(expr).map(|d| d.rsplit('.').next().unwrap_or(&d).to_string())
}

/// Annotated fields declared directly in the class body.
fn own_fields(candidate: &ClassCandidate) -> Vec<DataClassField> {
    let class_name = candidate.class_def.name.to_string();
    let mut fields = Vec::new();
    for (stmt, &line) in candidate.class_def.body.iter().zip(&candidate.body_lines) {
        let Stmt::AnnAssign(ann) = stmt else { continue };
        let Expr::Name(target) = ann.target.as_ref() else { continue };
        let field_name = target.id.to_string();
        if field_name.starts_with('_') || field_name == "model_config" {
            continue;
        }
        let Some(info) = analyze_annotation(&ann.annotation) else {
            continue; // ClassVar and similar: not an instance field
        };
        let mut field = DataClassField {
            class_name: class_name.clone(),
            field_name,
            type_name: info.type_name,
            nullable: info.nullable,
            max_digits: None,
            decimal_places: None,
            max_length: None,
            max_value: None,
            source_file: candidate.source_file.clone(),
            source_line: line,
        };
        for call in &info.constraint_calls {
            apply_constraint_keywords(&mut field, call);
        }
        if let Some(value) = &ann.value {
            if let Expr::Call(call) = value.as_ref() {
                if is_field_call(&call.func) {
                    apply_constraint_keywords(&mut field, call);
                }
            }
        }
        fields.push(field);
    }
    fields
}

/// What an annotation says about a field.
struct AnnotationInfo<'a> {
    type_name: Option<String>,
    nullable: Option<bool>,
    /// `Field(...)` in `Annotated` metadata, or a `con*()` helper call.
    constraint_calls: Vec<&'a ast::ExprCall>,
}

/// Analyse an annotation. Returns `None` for `ClassVar[...]`.
fn analyze_annotation(expr: &Expr) -> Option<AnnotationInfo<'_>> {
    let mut info = AnnotationInfo {
        type_name: None,
        nullable: Some(false),
        constraint_calls: Vec::new(),
    };
    if walk_annotation(expr, &mut info) {
        Some(info)
    } else {
        None
    }
}

/// Returns false if the annotation marks a non-instance attribute.
fn walk_annotation<'a>(expr: &'a Expr, info: &mut AnnotationInfo<'a>) -> bool {
    match expr {
        Expr::Subscript(sub) => {
            let head = last_segment(&sub.value).unwrap_or_default();
            match head.as_str() {
                "ClassVar" => false,
                "Optional" => {
                    info.nullable = Some(true);
                    walk_annotation(&sub.slice, info)
                }
                "Annotated" => {
                    if let Expr::Tuple(t) = sub.slice.as_ref() {
                        let mut elts = t.elts.iter();
                        let keep = elts.next().map_or(true, |base| walk_annotation(base, info));
                        for meta in elts {
                            if let Expr::Call(call) = meta {
                                if is_field_call(&call.func) {
                                    info.constraint_calls.push(call);
                                }
                            }
                        }
                        keep
                    } else {
                        walk_annotation(&sub.slice, info)
                    }
                }
                "Union" => {
                    let members: Vec<&Expr> = match sub.slice.as_ref() {
                        Expr::Tuple(t) => t.elts.iter().collect(),
                        other => vec![other],
                    };
                    walk_union(&members, info)
                }
                "Final" | "Required" | "NotRequired" | "ReadOnly" => {
                    walk_annotation(&sub.slice, info)
                }
                // Generic containers: record the container name only.
                _ => {
                    info.type_name = Some(head);
                    true
                }
            }
        }
        Expr::BinOp(binop) if matches!(binop.op, ast::Operator::BitOr) => {
            let mut members = Vec::new();
            flatten_bitor(expr, &mut members);
            walk_union(&members, info)
        }
        Expr::Call(call) => {
            // pydantic v1 style: condecimal(...), constr(...), conint(...)
            let name = last_segment(&call.func).unwrap_or_default();
            let type_name = match name.as_str() {
                "condecimal" => Some("Decimal"),
                "constr" => Some("str"),
                "conint" => Some("int"),
                "confloat" => Some("float"),
                _ => None,
            };
            if let Some(t) = type_name {
                info.type_name = Some(t.to_string());
                info.constraint_calls.push(call);
            }
            true
        }
        Expr::StringLiteral(s) => {
            // Forward reference: only simple names are interpreted.
            let text = s.value.to_string();
            let trimmed = text.trim();
            if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                info.type_name = Some(trimmed.rsplit('.').next().unwrap_or(trimmed).to_string());
            }
            true
        }
        Expr::NoneLiteral(_) => {
            info.nullable = Some(true);
            true
        }
        _ => {
            let name = last_segment(expr);
            if name.as_deref() == Some("Any") {
                info.nullable = None;
            } else {
                info.type_name = name;
            }
            true
        }
    }
}

/// Union members: `None` makes the field nullable; a single remaining member gives the type.
fn walk_union<'a>(members: &[&'a Expr], info: &mut AnnotationInfo<'a>) -> bool {
    let non_none: Vec<&&Expr> = members
        .iter()
        .filter(|m| !matches!(m, Expr::NoneLiteral(_)) && last_segment(m).as_deref() != Some("None"))
        .collect();
    if non_none.len() < members.len() {
        info.nullable = Some(true);
    }
    if non_none.len() == 1 {
        let nullable = info.nullable;
        let keep = walk_annotation(non_none[0], info);
        if nullable == Some(true) {
            info.nullable = Some(true);
        }
        keep
    } else {
        true // a union of several value types has no single type contract
    }
}

fn flatten_bitor<'a>(expr: &'a Expr, out: &mut Vec<&'a Expr>) {
    match expr {
        Expr::BinOp(binop) if matches!(binop.op, ast::Operator::BitOr) => {
            flatten_bitor(&binop.left, out);
            flatten_bitor(&binop.right, out);
        }
        other => out.push(other),
    }
}

/// `Field(...)`, `pydantic.Field(...)`, `attr.ib(...)`, `attrs.field(...)`, `dataclasses.field(...)`.
fn is_field_call(func: &Expr) -> bool {
    matches!(
        dotted_name(func).as_deref(),
        Some("Field" | "pydantic.Field" | "field" | "dataclasses.field" | "attr.ib" | "attrs.field")
    )
}

/// Read pydantic-style constraint keywords from a call.
fn apply_constraint_keywords(field: &mut DataClassField, call: &ast::ExprCall) {
    for kw in &call.arguments.keywords {
        let Some(arg) = kw.arg.as_ref() else { continue };
        let value = int_literal(&kw.value);
        match arg.as_str() {
            "max_digits" => field.max_digits = value.or(field.max_digits),
            "decimal_places" => field.decimal_places = value.or(field.decimal_places),
            "max_length" => field.max_length = value.or(field.max_length),
            "le" => field.max_value = value.or(field.max_value),
            "lt" if field.type_name.as_deref() == Some("int") => {
                field.max_value = value.map(|v| v - 1).or(field.max_value)
            }
            _ => {}
        }
    }
}

/// An integer literal, including a negated one.
fn int_literal(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::NumberLiteral(n) => match &n.value {
            Number::Int(i) => i.as_i64(),
            _ => None,
        },
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::USub) => int_literal(&u.operand).map(|v| -v),
        _ => None,
    }
}

/// Write data class fields as `model` nodes with precondition contracts.
/// Returns a map from `Class.field` to node ID.
pub fn write_data_classes(db: &ContractDb, classes: &[DataClass]) -> Result<HashMap<String, i64>> {
    let mut ids = HashMap::new();
    for class in classes {
        for field in &class.fields {
            let full_name = format!("{}.{}", field.class_name, field.field_name);
            let node_id = db.insert_node(&NodeRecord {
                name: full_name.clone(),
                kind: NodeKind::Model,
                source_file: field.source_file.clone(),
                source_line: field.source_line,
            })?;
            ids.insert(full_name, node_id);

            let base = |constraint_type: ConstraintType| ContractRecord {
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
                source_file: field.source_file.clone(),
                source_line: field.source_line,
                is_implicit: false,
                verification_level: VerificationLevel::Extracted,
                contract_role: Some(ContractRole::Precondition),
                dependent_expr: None,
            };

            if let Some(t) = &field.type_name {
                if VALUE_TYPES.contains(&t.as_str()) {
                    db.insert_contract(&ContractRecord {
                        param_type_name: Some(t.clone()),
                        ..base(ConstraintType::Type)
                    })?;
                }
            }
            if let Some(nullable) = field.nullable {
                db.insert_contract(&ContractRecord {
                    param_nullable: Some(nullable as i64),
                    ..base(ConstraintType::Nullability)
                })?;
            }
            if let Some(dp) = field.decimal_places {
                db.insert_contract(&ContractRecord {
                    param_max_digits: field.max_digits,
                    param_decimal_places: Some(dp),
                    ..base(ConstraintType::Precision)
                })?;
            }
            if let Some(len) = field.max_length {
                db.insert_contract(&ContractRecord {
                    param_max_length: Some(len),
                    ..base(ConstraintType::Length)
                })?;
            }
            if let Some(max) = field.max_value {
                db.insert_contract(&ContractRecord {
                    param_max_value: Some(max as f64),
                    ..base(ConstraintType::Range)
                })?;
            }
        }
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(src: &str) -> Vec<DataClass> {
        let parsed =
            ruff_python_parser::parse_unchecked(src, ruff_python_parser::Mode::Module.into());
        let stmts = match parsed.into_syntax() {
            ruff_python_ast::Mod::Module(m) => m.body,
            _ => unreachable!(),
        };
        resolve_data_classes(&collect_classes(&stmts, src, "t.py"))
    }

    fn field<'a>(cs: &'a [DataClass], class: &str, name: &str) -> &'a DataClassField {
        cs.iter()
            .find(|c| c.name == class)
            .and_then(|c| c.fields.iter().find(|f| f.field_name == name))
            .unwrap_or_else(|| panic!("missing {class}.{name}"))
    }

    #[test]
    fn test_dataclass_types_and_nullability() {
        let cs = classes(
            "from dataclasses import dataclass\n\
             @dataclass\n\
             class P:\n    a: Decimal\n    b: Optional[int]\n    c: str | None = None\n    d: ClassVar[int] = 0\n    e: Any\n",
        );
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].kind, DataClassKind::Dataclass);
        let a = field(&cs, "P", "a");
        assert_eq!((a.type_name.as_deref(), a.nullable), (Some("Decimal"), Some(false)));
        let b = field(&cs, "P", "b");
        assert_eq!((b.type_name.as_deref(), b.nullable), (Some("int"), Some(true)));
        let c = field(&cs, "P", "c");
        assert_eq!((c.type_name.as_deref(), c.nullable), (Some("str"), Some(true)));
        assert!(cs[0].fields.iter().all(|f| f.field_name != "d"), "ClassVar is not a field");
        assert_eq!(field(&cs, "P", "e").nullable, None);
        assert_eq!(a.source_line, 4);
    }

    #[test]
    fn test_pydantic_field_constraints() {
        let cs = classes(
            "class M(BaseModel):\n    \
             amount: Decimal = Field(max_digits=10, decimal_places=2)\n    \
             name: Annotated[str, Field(max_length=32)]\n    \
             qty: int = Field(lt=10)\n    \
             pct: condecimal(decimal_places=3, le=100)\n",
        );
        assert_eq!(cs[0].kind, DataClassKind::Pydantic);
        let amount = field(&cs, "M", "amount");
        assert_eq!((amount.max_digits, amount.decimal_places), (Some(10), Some(2)));
        assert_eq!(field(&cs, "M", "name").max_length, Some(32));
        assert_eq!(field(&cs, "M", "qty").max_value, Some(9));
        let pct = field(&cs, "M", "pct");
        assert_eq!((pct.type_name.as_deref(), pct.decimal_places, pct.max_value), (Some("Decimal"), Some(3), Some(100)));
    }

    #[test]
    fn test_inheritance_and_override() {
        let cs = classes(
            "class Base(BaseModel):\n    id: int\n    note: str\n\
             class Child(Base):\n    note: Optional[str]\n    extra: bool\n",
        );
        let child = cs.iter().find(|c| c.name == "Child").unwrap();
        let names: Vec<&str> = child.fields.iter().map(|f| f.field_name.as_str()).collect();
        assert_eq!(names, ["id", "note", "extra"]);
        let id = field(&cs, "Child", "id");
        assert_eq!(id.source_line, 2, "inherited field keeps the parent's location");
        assert_eq!(field(&cs, "Child", "note").nullable, Some(true));
    }

    #[test]
    fn test_unrecognised_classes_ignored() {
        let cs = classes("class Plain:\n    x: int\nclass R(models.Model):\n    x = models.IntegerField()\n");
        assert!(cs.is_empty());
    }

    #[test]
    fn test_namedtuple_typeddict_attrs() {
        let cs = classes(
            "class T(NamedTuple):\n    x: int\n\
             class D(TypedDict):\n    y: NotRequired[str]\n\
             @attrs.define\nclass A:\n    z: float\n",
        );
        let kinds: Vec<DataClassKind> = cs.iter().map(|c| c.kind).collect();
        assert_eq!(kinds, [DataClassKind::NamedTuple, DataClassKind::TypedDict, DataClassKind::Attrs]);
        assert_eq!(field(&cs, "D", "y").type_name.as_deref(), Some("str"));
    }
}
