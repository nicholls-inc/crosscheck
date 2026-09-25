use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::dataclass_extractor::{annotation_facts, VALUE_TYPES};
use crate::db::{ConstraintType, ContractRecord, ContractRole, VerificationLevel};
use crate::docstring_parser::DocstringContract;
use crate::resolve::qualify;
use crate::value_analysis::{facts_rows, ValueFacts};

/// How a parameter binds arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    PositionalOnly,
    Normal,
    KeywordOnly,
    VarArgs,
    VarKeywords,
}

/// One declared parameter.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub kind: ParamKind,
    pub annotation: Option<Expr>,
    /// Value type from the annotation (`Optional[T]` → `T`), if it is one of `VALUE_TYPES`.
    pub type_name: Option<String>,
    /// From the annotation (`Optional` → true, other types → false, `Any`/none → unknown);
    /// a `= None` default makes it true.
    pub nullable: Option<bool>,
    /// The default is `None`.
    pub default_none: bool,
}

/// Whether and how a function is bound to a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Function,
    Instance,
    ClassMethod,
    Static,
}

/// Extracted function information.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Short name: `f` for module-level functions, `Class.m` for methods.
    pub name: String,
    /// `module.f` / `module.Class.m`.
    pub qualified_name: String,
    pub module: String,
    /// Short name of the enclosing class, for methods.
    pub class_name: Option<String>,
    pub method_kind: MethodKind,
    /// Every declared parameter, including `self` / `cls`.
    pub params: Vec<ParamInfo>,
    pub return_annotation: Option<Expr>,
    pub return_type: Option<String>,
    pub is_return_optional: bool,
    /// For `tuple[T, T, ...]` returns with uniform element types, the element type.
    pub tuple_element_type: Option<String>,
    pub source_file: String,
    pub source_line: u32,
    pub body: Vec<Stmt>,
    pub docstring: Option<String>,
    /// A `typing.overload` stub (replaced by the implementation that follows).
    pub is_overload: bool,
    /// The implementation of overloads whose return annotations disagree on
    /// `None` (`-> T` for some arguments, `-> T | None` for others): which
    /// one applies at a call is not decided, so the result's nullability is unknown.
    pub overloads_differ_on_none: bool,
    /// What the return annotation says about `None`: `Some(false)` excludes it
    /// (a contract on the function's return values, see `return_contract`),
    /// `Some(true)` allows it, `None` unknown (no annotation, `Any`, `object`,
    /// a type variable).
    pub return_nullable: Option<bool>,
    /// Decorators as dotted names (`background_task`, `app.task`; a call
    /// `@d(...)` gives `d`).
    pub decorators: Vec<Vec<String>>,
    /// The body yields: calling the function returns a generator.
    pub is_generator: bool,
    /// Byte offset of the return annotation (a line after `Project::build`).
    pub return_line: u32,
}

/// Decorators that neither inject arguments nor change what a parameter
/// receives (matched on the last dotted segment).
const PLAIN_DECORATORS: [&str; 13] = [
    "staticmethod",
    "classmethod",
    "property",
    "wraps",
    "lru_cache",
    "cache",
    "cached_property",
    "overload",
    "abstractmethod",
    "override",
    "setter",
    "getter",
    "deleter",
];

/// Value types whose return annotation is also a type requirement on the
/// function's return values (`float` accepts `int`, `int` accepts `bool`,
/// so they are not compared by equality).
pub const RETURN_TYPE_CONTRACTS: [&str; 3] = ["Decimal", "str", "bool"];

impl FunctionInfo {
    /// Decorators other than the plain ones (`PLAIN_DECORATORS`); the caller
    /// decides which of them are project-defined.
    pub fn wrapping_decorators(&self) -> impl Iterator<Item = &Vec<String>> {
        self.decorators.iter().filter(|d| {
            d.last().is_none_or(|last| !PLAIN_DECORATORS.contains(&last.as_str()))
        })
    }

    /// Qualified name of the return contract node `f.<return>`.
    pub fn return_node_name(&self) -> String {
        format!("{}.<return>", self.qualified_name)
    }

    /// Whether the return annotation is a contract on the function's own
    /// return values (assume-guarantee): it excludes `None`, and the function
    /// returns values (not a generator, not an overload stub).
    pub fn has_return_contract(&self) -> bool {
        self.return_nullable == Some(false)
            && !self.is_overload
            && !self.is_generator
    }
    /// Whether the method is only a declaration that a subclass implements:
    /// `@abstractmethod`, or a body that is `...`, `pass` or a docstring alone
    /// under a return annotation other than `None` / `Optional` (so it does
    /// not mean "returns None"), or that only raises `NotImplementedError`.
    /// Such a method is no dispatch target (see `ProjectIndex::dispatch_targets`).
    pub fn is_stub(&self) -> bool {
        if self
            .decorators
            .iter()
            .any(|d| d.last().is_some_and(|l| l == "abstractmethod"))
        {
            return true;
        }
        let body: Vec<&Stmt> = self
            .body
            .iter()
            .filter(|s| !matches!(s, Stmt::Expr(e) if matches!(e.value.as_ref(), Expr::StringLiteral(_))))
            .collect();
        let raises_not_implemented = |s: &Stmt| match s {
            Stmt::Raise(r) => r.exc.as_deref().is_some_and(|e| {
                let target = match e {
                    Expr::Call(c) => c.func.as_ref(),
                    other => other,
                };
                matches!(target, Expr::Name(n) if n.id.as_str() == "NotImplementedError")
            }),
            _ => false,
        };
        match body.as_slice() {
            [only] if raises_not_implemented(only) => true,
            [] => self.return_nullable == Some(false),
            [only] => {
                let empty = matches!(only, Stmt::Pass(_))
                    || matches!(only, Stmt::Expr(e) if matches!(e.value.as_ref(), Expr::EllipsisLiteral(_)));
                empty && self.return_nullable == Some(false)
            }
            _ => false,
        }
    }

    /// Number of leading parameters bound implicitly when called through an
    /// instance or class (`self` / `cls`).
    pub fn implicit_params(&self) -> usize {
        match self.method_kind {
            MethodKind::Instance | MethodKind::ClassMethod => {
                usize::from(self.params.first().is_some_and(|p| {
                    matches!(p.kind, ParamKind::PositionalOnly | ParamKind::Normal)
                }))
            }
            _ => 0,
        }
    }

    /// The `self` / `cls` parameter name, if any.
    pub fn self_name(&self) -> Option<&str> {
        if self.implicit_params() == 1 {
            Some(self.params[0].name.as_str())
        } else {
            None
        }
    }

    /// Parameters other than `self` / `cls`.
    pub fn value_params(&self) -> &[ParamInfo] {
        &self.params[self.implicit_params()..]
    }

    /// Named parameters (not `*args` / `**kwargs`) other than `self` / `cls`.
    pub fn named_value_params(&self) -> impl Iterator<Item = &ParamInfo> {
        self.value_params()
            .iter()
            .filter(|p| !matches!(p.kind, ParamKind::VarArgs | ParamKind::VarKeywords))
    }

    /// The only parameter, when there is exactly one (ignoring `self` / `cls`)
    /// and no `*args` / `**kwargs`: values derived from it may get bounds that
    /// depend on the function's input.
    pub fn single_param(&self) -> Option<&str> {
        match self.value_params() {
            [p] if matches!(
                p.kind,
                ParamKind::PositionalOnly | ParamKind::Normal | ParamKind::KeywordOnly
            ) =>
            {
                Some(p.name.as_str())
            }
            _ => None,
        }
    }
}

/// Extract function definitions (module level and methods of module-level
/// classes, including those inside module-level `if` / `try` / `with`
/// blocks) from a parsed Python module.
pub fn extract_functions(stmts: &[Stmt], source_file: &str, module: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    for stmt in crate::resolve::module_defs(stmts) {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                functions.push(extract_function_info(func_def, source_file, module, None));
            }
            Stmt::ClassDef(class_def) => {
                for body_stmt in &class_def.body {
                    if let Stmt::FunctionDef(func_def) = body_stmt {
                        functions.push(extract_function_info(
                            func_def,
                            source_file,
                            module,
                            Some(class_def.name.as_str()),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    // Overload stubs describe the implementation that follows them.
    let mut stubs: std::collections::HashMap<String, (bool, bool)> = std::collections::HashMap::new();
    for f in &mut functions {
        if f.is_overload {
            let e = stubs.entry(f.qualified_name.clone()).or_default();
            if f.is_return_optional {
                e.0 = true;
            } else {
                e.1 = true;
            }
        } else if let Some((optional, plain)) = stubs.remove(&f.qualified_name) {
            f.overloads_differ_on_none = optional && plain;
        }
    }
    functions
}

/// Display (and qualified) name of the pseudo function holding a module's
/// top-level code.
pub fn module_function_name(module: &str) -> String {
    format!("<module {module}>")
}

/// The top-level statements of a module (other than imports and `def` /
/// `class` definitions) as the pseudo function `<module m>`, so writes and
/// calls at module level are seen. `None` when there is no such code
/// (only a docstring, `pass`, `__all__`).
pub fn module_function(stmts: &[Stmt], source_file: &str, module: &str) -> Option<FunctionInfo> {
    let body: Vec<Stmt> = stmts
        .iter()
        .filter(|s| {
            !matches!(
                s,
                Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::Import(_) | Stmt::ImportFrom(_)
            )
        })
        .cloned()
        .collect();
    let evaluates = |s: &Stmt| match s {
        Stmt::Pass(_) => false,
        Stmt::Expr(e) => !matches!(e.value.as_ref(), Expr::StringLiteral(_)),
        _ => true,
    };
    if !body.iter().any(evaluates) {
        return None;
    }
    let name = module_function_name(module);
    Some(FunctionInfo {
        qualified_name: name.clone(),
        name,
        module: module.to_string(),
        class_name: None,
        method_kind: MethodKind::Function,
        params: Vec::new(),
        return_annotation: None,
        return_type: None,
        is_return_optional: false,
        tuple_element_type: None,
        source_file: source_file.to_string(),
        source_line: 0,
        body,
        docstring: None,
        is_overload: false,
        overloads_differ_on_none: false,
        return_nullable: None,
        decorators: Vec::new(),
        return_line: 0,
        is_generator: false,
    })
}

/// Whether a name in an annotation looks like a type variable (`T`,
/// `_T`, `TModel`, `T_co`, `KT_contra`): it may stand for a type that
/// includes `None`.
fn looks_like_type_var(name: &str) -> bool {
    let n = name.trim_start_matches('_');
    let mut chars = n.chars();
    let first = chars.next();
    let second = chars.next();
    n.ends_with("_co")
        || n.ends_with("_contra")
        || (first.is_some_and(|c| c.is_ascii_uppercase()) && second.is_none())
        || (first == Some('T') && second.is_some_and(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()))
}

/// What a return annotation says about `None` (see `FunctionInfo::return_nullable`).
/// `type_params` are the function's own (PEP 695) type parameters.
pub fn annotation_nullability(annotation: &Expr, type_params: &[String]) -> Option<bool> {
    let head = match annotation {
        Expr::Name(n) => Some(n.id.as_str()),
        Expr::Attribute(a) => Some(a.attr.as_str()),
        _ => None,
    };
    if let Some(h) = head {
        if matches!(h, "object" | "NoReturn" | "Never" | "Any")
            || type_params.iter().any(|p| p == h)
            || looks_like_type_var(h)
        {
            return None;
        }
    }
    if let Expr::StringLiteral(s) = annotation {
        let text = s.value.to_str().trim().to_string();
        let parsed = ruff_python_parser::parse_expression(&text).ok()?;
        return annotation_nullability(parsed.expr(), type_params);
    }
    let (_, nullable) = annotation_facts(annotation)?;
    nullable
}

/// Extract info from a single function definition.
fn extract_function_info(
    func_def: &ast::StmtFunctionDef,
    source_file: &str,
    module: &str,
    class_name: Option<&str>,
) -> FunctionInfo {
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

    let decorated = |name: &str| {
        func_def
            .decorator_list
            .iter()
            .any(|d| matches!(&d.expression, Expr::Name(n) if n.id.as_str() == name))
    };
    let method_kind = match class_name {
        None => MethodKind::Function,
        Some(_) if decorated("staticmethod") => MethodKind::Static,
        Some(_) if decorated("classmethod") => MethodKind::ClassMethod,
        Some(_) => MethodKind::Instance,
    };

    let name = match class_name {
        Some(class) => format!("{class}.{}", func_def.name),
        None => func_def.name.to_string(),
    };

    FunctionInfo {
        qualified_name: qualify(module, &name),
        name,
        module: module.to_string(),
        class_name: class_name.map(str::to_string),
        method_kind,
        params: extract_params(&func_def.parameters),
        return_annotation: func_def.returns.as_deref().cloned(),
        return_type,
        is_return_optional,
        tuple_element_type,
        source_file: source_file.to_string(),
        source_line: func_def.range.start().to_u32(),
        body: func_def.body.to_vec(),
        docstring,
        is_overload: func_def.decorator_list.iter().any(|d| {
            matches!(&d.expression, Expr::Name(n) if n.id.as_str() == "overload")
                || matches!(&d.expression, Expr::Attribute(a) if a.attr.as_str() == "overload")
        }),
        overloads_differ_on_none: false,
        return_nullable: func_def
            .returns
            .as_deref()
            .and_then(|r| annotation_nullability(r, &type_param_names(func_def))),
        decorators: func_def
            .decorator_list
            .iter()
            .filter_map(|d| {
                let target = match &d.expression {
                    Expr::Call(c) => c.func.as_ref(),
                    other => other,
                };
                crate::resolve::dotted_parts(target)
            })
            .collect(),
        is_generator: crate::flow::is_generator_body(&func_def.body),
        return_line: func_def
            .returns
            .as_deref()
            .map_or(func_def.range.start().to_u32(), |r| {
                ruff_text_size::Ranged::range(r).start().to_u32()
            }),
    }
}

/// PEP 695 type parameter names of a function (`def f[T](x: T) -> T`).
fn type_param_names(func_def: &ast::StmtFunctionDef) -> Vec<String> {
    func_def
        .type_params
        .as_deref()
        .map(|tp| {
            tp.type_params
                .iter()
                .filter_map(|p| match p {
                    ast::TypeParam::TypeVar(t) => Some(t.name.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_params(params: &ast::Parameters) -> Vec<ParamInfo> {
    let mut out = Vec::new();
    let with_default = |p: &ast::ParameterWithDefault, kind: ParamKind| {
        let mut info = param_info(&p.parameter, kind);
        if matches!(p.default.as_deref(), Some(Expr::NoneLiteral(_))) {
            info.nullable = Some(true);
            info.default_none = true;
        }
        info
    };
    for p in &params.posonlyargs {
        out.push(with_default(p, ParamKind::PositionalOnly));
    }
    for p in &params.args {
        out.push(with_default(p, ParamKind::Normal));
    }
    if let Some(p) = &params.vararg {
        out.push(param_info(p, ParamKind::VarArgs));
    }
    for p in &params.kwonlyargs {
        out.push(with_default(p, ParamKind::KeywordOnly));
    }
    if let Some(p) = &params.kwarg {
        out.push(param_info(p, ParamKind::VarKeywords));
    }
    out
}

/// Re-read parameter annotations that name a module-level alias or type
/// variable (see `resolve::annotation_aliases`).
pub fn apply_aliases(func: &mut FunctionInfo, aliases: &std::collections::HashMap<String, Expr>) {
    if let Some(Expr::Name(n)) = &func.return_annotation {
        if let Some(alias) = aliases.get(n.id.as_str()) {
            func.return_nullable = annotation_nullability(alias, &[]);
            func.is_return_optional = func.return_nullable == Some(true);
            func.return_type = Some(format_type_annotation(strip_optional(alias)));
        }
    }
    for p in &mut func.params {
        let Some(Expr::Name(n)) = &p.annotation else { continue };
        let Some(alias) = aliases.get(n.id.as_str()) else { continue };
        let (type_name, nullable) = match annotation_facts(alias) {
            Some((t, n)) => (t.filter(|t| VALUE_TYPES.contains(&t.as_str())), n),
            None => (None, None),
        };
        p.type_name = type_name;
        // A `= None` default keeps the parameter nullable.
        p.nullable = if p.nullable == Some(true) { Some(true) } else { nullable };
    }
}

/// Forget what annotations naming a type from an unmodelled package (see
/// `resolve::external_type_names`) say about `None`: such a type may be an
/// alias of an `Optional[...]` type. Parameters and the return get unknown
/// nullability (no contract) unless the annotation itself allows `None`.
pub fn apply_external_types(func: &mut FunctionInfo, external: &std::collections::HashSet<String>) {
    if external.is_empty() {
        return;
    }
    if let Some(ret) = &func.return_annotation {
        if func.return_nullable == Some(false) && crate::resolve::annotation_is_external(ret, external) {
            func.return_nullable = None;
            func.return_type = None;
        }
    }
    for p in &mut func.params {
        let Some(a) = &p.annotation else { continue };
        if p.nullable == Some(false) && crate::resolve::annotation_is_external(a, external) {
            p.nullable = None;
            p.type_name = None;
        }
    }
}

fn param_info(p: &ast::Parameter, kind: ParamKind) -> ParamInfo {
    let (type_name, nullable) = match p.annotation.as_deref().and_then(annotation_facts) {
        Some((t, n)) => (t.filter(|t| VALUE_TYPES.contains(&t.as_str())), n),
        None => (None, None),
    };
    ParamInfo {
        name: p.name.to_string(),
        kind,
        annotation: p.annotation.as_deref().cloned(),
        type_name,
        nullable,
        default_none: false,
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
        Expr::NoneLiteral(_) => "None".to_string(),
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
pub fn strip_optional(expr: &Expr) -> &Expr {
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
                if tuple.elts[1..]
                    .iter()
                    .all(|e| format_type_annotation(e) == first)
                {
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

/// A docstring clause as a contract row (ASSUMED) on node `node_id`.
fn docstring_row(func: &FunctionInfo, dc: &DocstringContract, node_id: i64) -> ContractRecord {
    // Put the clause's bound in the column the checker reads for its kind.
    let value = |kind: ConstraintType| {
        if dc.constraint_type == kind {
            dc.param_value
        } else {
            None
        }
    };
    let range = (dc.constraint_type == ConstraintType::Range)
        .then_some(dc.range_bound)
        .flatten();
    let required = matches!(dc.role, ContractRole::Precondition);
    // A precondition clause constrains the parameter it names; a name that is
    // not a parameter (e.g. `result`) applies to every parameter.
    let subject = match dc.role {
        ContractRole::Precondition => dc
            .subject
            .as_ref()
            .filter(|s| func.named_value_params().any(|p| &p.name == *s))
            .cloned(),
        ContractRole::Postcondition => None,
    };
    ContractRecord {
        param_decimal_places: value(ConstraintType::Precision),
        param_max_length: value(ConstraintType::Length),
        param_nullable: value(ConstraintType::Nullability),
        dependent_expr: dc.dependent_expr.clone(),
        subject,
        ..ContractRecord::new(
            node_id,
            dc.constraint_type.clone(),
            dc.role.clone(),
            VerificationLevel::Assumed,
            &func.source_file,
            func.source_line,
        )
        .with_range(
            range.filter(|_| dc.is_lower_bound),
            range.filter(|_| !dc.is_lower_bound),
            required,
        )
    }
}

/// The function's docstring `ensures:` clauses as postcondition rows.
pub fn docstring_postcondition_rows(
    func: &FunctionInfo,
    doc: &[DocstringContract],
    node_id: i64,
) -> Vec<ContractRecord> {
    doc.iter()
        .filter(|dc| matches!(dc.role, ContractRole::Postcondition))
        .map(|dc| docstring_row(func, dc, node_id))
        .collect()
}

/// Postconditions of a function node: they describe its return value.
/// Docstring `ensures:` clauses (ASSUMED) take precedence over extracted
/// facts (`summary`, from the return annotation and body) of the same kind.
pub fn postcondition_rows(
    func: &FunctionInfo,
    summary: &ValueFacts,
    doc: &[DocstringContract],
    node_id: i64,
) -> Vec<ContractRecord> {
    let mut rows = docstring_postcondition_rows(func, doc, node_id);
    let documented: Vec<ConstraintType> = rows.iter().map(|r| r.constraint_type.clone()).collect();
    for row in facts_rows(
        summary,
        node_id,
        &func.source_file,
        func.source_line,
        VerificationLevel::Extracted,
    ) {
        if !documented.contains(&row.constraint_type) {
            rows.push(row);
        }
    }
    rows
}

/// Preconditions of a function node, one set per parameter (`subject`):
/// the parameter's annotation (type; nullability 0 unless `Optional` or a
/// `None` default) and docstring `requires:` clauses.
pub fn precondition_rows(
    func: &FunctionInfo,
    doc: &[DocstringContract],
    node_id: i64,
) -> Vec<ContractRecord> {
    let mut rows = Vec::new();
    for param in func.named_value_params() {
        let base = |kind: ConstraintType| ContractRecord {
            subject: Some(param.name.clone()),
            ..ContractRecord::new(
                node_id,
                kind,
                ContractRole::Precondition,
                VerificationLevel::Extracted,
                &func.source_file,
                func.source_line,
            )
        };
        if let Some(t) = &param.type_name {
            rows.push(ContractRecord {
                param_type_name: Some(t.clone()),
                ..base(ConstraintType::Type)
            });
        }
        if let Some(nullable) = param.nullable {
            rows.push(ContractRecord {
                param_nullable: Some(nullable as i64),
                ..base(ConstraintType::Nullability)
            });
        }
    }
    rows.extend(
        doc.iter()
            .filter(|dc| matches!(dc.role, ContractRole::Precondition))
            .map(|dc| docstring_row(func, dc, node_id)),
    );
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn funcs(src: &str) -> Vec<FunctionInfo> {
        let stmts =
            match ruff_python_parser::parse_unchecked(src, ruff_python_parser::Mode::Module.into())
                .into_syntax()
            {
                ruff_python_ast::Mod::Module(m) => m.body.to_vec(),
                _ => unreachable!(),
            };
        extract_functions(&stmts, "m.py", "pkg.m")
    }

    #[test]
    fn test_names_and_params() {
        let fs = funcs(
            "def f(a: Decimal, b: Optional[str], c=None, *args, d: int, **kw): pass\n\
             class P:\n    def m(self, x: int): pass\n    @staticmethod\n    def s(y): pass\n    @classmethod\n    def c(cls, z): pass\n",
        );
        let f = &fs[0];
        assert_eq!(
            (f.name.as_str(), f.qualified_name.as_str()),
            ("f", "pkg.m.f")
        );
        let kinds: Vec<(&str, ParamKind, Option<&str>, Option<bool>)> = f
            .params
            .iter()
            .map(|p| (p.name.as_str(), p.kind, p.type_name.as_deref(), p.nullable))
            .collect();
        assert_eq!(
            kinds,
            [
                ("a", ParamKind::Normal, Some("Decimal"), Some(false)),
                ("b", ParamKind::Normal, Some("str"), Some(true)),
                ("c", ParamKind::Normal, None, Some(true)),
                ("args", ParamKind::VarArgs, None, None),
                ("d", ParamKind::KeywordOnly, Some("int"), Some(false)),
                ("kw", ParamKind::VarKeywords, None, None),
            ]
        );
        assert_eq!(f.single_param(), None);
        let m = &fs[1];
        assert_eq!(
            (m.name.as_str(), m.method_kind),
            ("P.m", MethodKind::Instance)
        );
        assert_eq!(m.self_name(), Some("self"));
        assert_eq!(m.single_param(), Some("x"));
        assert_eq!(
            (fs[2].method_kind, fs[2].implicit_params()),
            (MethodKind::Static, 0)
        );
        assert_eq!(
            (fs[3].method_kind, fs[3].self_name()),
            (MethodKind::ClassMethod, Some("cls"))
        );
    }

    /// Round 6: which return annotations are contracts (exclude None).
    #[test]
    fn test_return_nullability() {
        let fs = funcs(
            "def a() -> int: ...\n\
             def b() -> Optional[int]: ...\n\
             def c() -> 'Optional[Foo]': ...\n\
             def d() -> Union[int, None]: ...\n\
             def e() -> Any: ...\n\
             def f() -> object: ...\n\
             def g() -> T: ...\n\
             def h[X](x: X) -> X: ...\n\
             def i() -> TModel: ...\n\
             def j() -> 'Invoice': ...\n\
             def k(): ...\n\
             def l() -> Iterator[int]:\n    yield 1\n\
             def m() -> NoReturn: ...\n",
        );
        let n: Vec<(&str, Option<bool>)> = fs
            .iter()
            .map(|f| (f.name.as_str(), f.return_nullable))
            .collect();
        assert_eq!(
            n,
            [
                ("a", Some(false)),
                ("b", Some(true)),
                ("c", Some(true)),
                ("d", Some(true)),
                ("e", None),
                ("f", None),
                ("g", None),
                ("h", None),
                ("i", None),
                ("j", Some(false)),
                ("k", None),
                ("l", Some(false)),
                ("m", None),
            ]
        );
        let contract: Vec<&str> = fs
            .iter()
            .filter(|f| f.has_return_contract())
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(contract, ["a", "j"], "a generator has no return contract");
    }

    #[test]
    fn test_precondition_subjects() {
        let fs = funcs(
            "def combine(amount: Decimal, rate: Decimal):\n    \"\"\"requires: precision(amount) <= 2\n    requires: precision(result) <= 9\n    \"\"\"\n",
        );
        let doc = crate::docstring_parser::parse_docstring(fs[0].docstring.as_deref().unwrap());
        let rows = precondition_rows(&fs[0], &doc, 1);
        let summary: Vec<(&str, Option<&str>)> = rows
            .iter()
            .map(|r| (r.constraint_type.as_str(), r.subject.as_deref()))
            .collect();
        assert_eq!(
            summary,
            [
                ("type", Some("amount")),
                ("nullability", Some("amount")),
                ("type", Some("rate")),
                ("nullability", Some("rate")),
                ("precision", Some("amount")),
                ("precision", None),
            ]
        );
    }
}
