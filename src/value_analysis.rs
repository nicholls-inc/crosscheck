//! Contracts of a value computed inside a function.
//!
//! `Scope::facts` computes what is known about an expression in the context of
//! its enclosing function: decimal places (static, or dependent on the
//! function's only parameter), nullability, value type, string length and
//! integer range. Anything not derivable is left unknown (`None`), never
//! guessed: an unknown fact produces no contract row, which the checker
//! reports as a vacuous pass rather than a proof.
//!
//! Local variables are traced flow-insensitively (a local's facts are the
//! join over every assignment to it); names rebound in other ways (loops,
//! `with`, augmented assignment, unpacking from a non-tuple, ...) are unknown.
//! None-narrowing (`flow`) refines nullability at the point of use.
//!
//! Function results use project-wide summaries computed by
//! `compute_summaries`, a bounded fixpoint in which each pass uses the
//! previous pass's summaries of callees; the first pass treats every call as
//! unknown, so every pass is sound on its own.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use ruff_python_ast::{self as ast, BoolOp, Expr, Number, Operator, UnaryOp};

use crate::dataclass_extractor::VALUE_TYPES;
use crate::db::{ConstraintType, ContractRecord, ContractRole, VerificationLevel};
use crate::flow::{self, FunctionFlow, Narrowed};
use crate::function_extractor::{strip_optional, FunctionInfo, MethodKind, ParamInfo, ParamKind};
use crate::resolve::{dotted_parts, qualify, ClassInfo, ClassKind, ProjectIndex, Symbol};

/// A bound in the checker's DepExpr grammar, over the function's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dep {
    Lit(i64),
    /// `input_precision`: the bound of whatever flows into the function.
    Input,
    Max(Box<Dep>, Box<Dep>),
    Add(Box<Dep>, Box<Dep>),
}

impl Dep {
    pub fn max(a: Dep, b: Dep) -> Dep {
        match (a, b) {
            (Dep::Lit(x), Dep::Lit(y)) => Dep::Lit(x.max(y)),
            (a, b) if a == b => a,
            (a, b) => Dep::Max(Box::new(a), Box::new(b)),
        }
    }

    pub fn sum(a: Dep, b: Dep) -> Dep {
        match (a, b) {
            (Dep::Lit(x), Dep::Lit(y)) => Dep::Lit(x + y),
            (a, Dep::Lit(0)) | (Dep::Lit(0), a) => a,
            (a, b) => Dep::Add(Box::new(a), Box::new(b)),
        }
    }

    pub fn as_static(&self) -> Option<i64> {
        match self {
            Dep::Lit(n) => Some(*n),
            _ => None,
        }
    }

    /// DepExpr source text (`max(3, input_precision)`).
    pub fn render(&self) -> String {
        match self {
            Dep::Lit(n) => n.to_string(),
            Dep::Input => "input_precision".to_string(),
            Dep::Max(a, b) => format!("max({}, {})", a.render(), b.render()),
            Dep::Add(a, b) => format!("add({}, {})", a.render(), b.render()),
        }
    }
}

/// What is known about a value. `None` fields are unknown.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueFacts {
    /// The value is always `None`.
    pub always_none: bool,
    /// `Some(true)`: may be None; `Some(false)`: never None.
    pub nullable: Option<bool>,
    pub type_name: Option<String>,
    /// The type is only known from a numeric literal, which Django and
    /// pydantic accept for any numeric field; no type contract is emitted.
    pub weak_type: bool,
    /// Upper bound on decimal places.
    pub precision: Option<Dep>,
    pub max_length: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

impl ValueFacts {
    pub fn none_value() -> Self {
        ValueFacts {
            always_none: true,
            nullable: Some(true),
            ..Default::default()
        }
    }

    pub fn non_null() -> Self {
        ValueFacts {
            nullable: Some(false),
            ..Default::default()
        }
    }

    pub fn typed(t: &str) -> Self {
        ValueFacts {
            type_name: Some(t.to_string()),
            ..Self::non_null()
        }
    }

    pub fn nullable() -> Self {
        ValueFacts {
            nullable: Some(true),
            ..Default::default()
        }
    }

    /// Facts that hold for either of two possible values.
    pub fn join(a: ValueFacts, b: ValueFacts) -> ValueFacts {
        match (a.always_none, b.always_none) {
            (true, true) => return a,
            (true, false) => {
                return ValueFacts {
                    nullable: Some(true),
                    ..b
                }
            }
            (false, true) => {
                return ValueFacts {
                    nullable: Some(true),
                    ..a
                }
            }
            _ => {}
        }
        let both = |x: Option<i64>, y: Option<i64>, f: fn(i64, i64) -> i64| match (x, y) {
            (Some(x), Some(y)) => Some(f(x, y)),
            _ => None,
        };
        ValueFacts {
            always_none: false,
            nullable: match (a.nullable, b.nullable) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            },
            weak_type: a.type_name == b.type_name && a.weak_type && b.weak_type,
            type_name: if a.type_name == b.type_name {
                a.type_name
            } else {
                None
            },
            precision: match (a.precision, b.precision) {
                (Some(x), Some(y)) => Some(Dep::max(x, y)),
                _ => None,
            },
            max_length: both(a.max_length, b.max_length, i64::max),
            min_value: both(a.min_value, b.min_value, i64::min),
            max_value: both(a.max_value, b.max_value, i64::max),
        }
    }

    pub fn join_all(items: impl IntoIterator<Item = ValueFacts>) -> Option<ValueFacts> {
        items.into_iter().reduce(ValueFacts::join)
    }

    /// The facts restricted to the non-None case.
    pub fn non_none_part(self) -> ValueFacts {
        if self.always_none {
            return ValueFacts::non_null();
        }
        ValueFacts {
            nullable: Some(false),
            ..self
        }
    }

    fn numeric_type(&self) -> Option<&str> {
        self.type_name
            .as_deref()
            .filter(|t| matches!(*t, "Decimal" | "int" | "float"))
    }
}

/// Where an expression is evaluated: narrowed names, and names bound by an
/// enclosing lambda or comprehension (unknown).
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub narrowed: Narrowed,
    pub shadowed: HashSet<String>,
}

impl Ctx {
    pub fn new(narrowed: &Narrowed) -> Self {
        Ctx {
            narrowed: narrowed.clone(),
            shadowed: HashSet::new(),
        }
    }

    pub fn narrow(&self, names: Vec<String>) -> Ctx {
        let mut c = self.clone();
        c.narrowed
            .extend(names.into_iter().filter(|n| !self.shadowed.contains(n)));
        c
    }

    pub fn shadow(&self, names: impl IntoIterator<Item = String>) -> Ctx {
        let mut c = self.clone();
        for n in names {
            c.narrowed.remove(&n);
            c.shadowed.insert(n);
        }
        c
    }
}

/// A resolved call target.
#[derive(Debug, Clone)]
pub enum Callee<'a> {
    /// An extracted function; `implicit` leading parameters are bound by the
    /// call itself (`self` / `cls`).
    Function {
        qualified: String,
        implicit: usize,
    },
    Class(&'a ClassInfo),
    Unknown,
}

/// Queryset methods that return a queryset of the same model.
const QUERYSET_METHODS: [&str; 14] = [
    "filter",
    "exclude",
    "order_by",
    "all",
    "select_related",
    "prefetch_related",
    "using",
    "select_for_update",
    "distinct",
    "annotate",
    "only",
    "defer",
    "reverse",
    "none",
];

/// Queryset methods that return one model instance.
const INSTANCE_METHODS: [&str; 6] = ["get", "create", "first", "last", "latest", "earliest"];

/// `str` methods whose result is a `str`.
const STR_METHODS: [&str; 16] = [
    "upper",
    "lower",
    "strip",
    "lstrip",
    "rstrip",
    "title",
    "capitalize",
    "casefold",
    "replace",
    "format",
    "join",
    "zfill",
    "center",
    "ljust",
    "rjust",
    "swapcase",
];

/// The analysis context of one function (or of a bare body, with `func` None).
pub struct Scope<'a> {
    pub index: &'a ProjectIndex,
    pub summaries: &'a HashMap<String, ValueFacts>,
    pub func: Option<&'a FunctionInfo>,
    pub module: &'a str,
    pub flow: &'a FunctionFlow<'a>,
    pub class: Option<&'a ClassInfo>,
    memo: RefCell<HashMap<String, ValueFacts>>,
    visiting: RefCell<HashSet<String>>,
}

impl<'a> Scope<'a> {
    pub fn new(
        index: &'a ProjectIndex,
        summaries: &'a HashMap<String, ValueFacts>,
        func: Option<&'a FunctionInfo>,
        flow: &'a FunctionFlow<'a>,
    ) -> Self {
        let module = func.map(|f| f.module.as_str()).unwrap_or("");
        let class = func
            .and_then(|f| f.class_name.as_deref())
            .and_then(|c| index.class(&qualify(module, c)));
        Scope {
            index,
            summaries,
            func,
            module,
            flow,
            class,
            memo: RefCell::new(HashMap::new()),
            visiting: RefCell::new(HashSet::new()),
        }
    }

    fn param(&self, name: &str) -> Option<&'a ParamInfo> {
        self.func?.value_params().iter().find(|p| p.name == name)
    }

    fn is_self(&self, name: &str) -> bool {
        self.func.and_then(|f| f.self_name()) == Some(name)
    }

    /// Whether `name` refers to something outside the project (a builtin or
    /// a third-party module or name), rather than a local or project symbol.
    fn is_external(&self, name: &str, ctx: &Ctx) -> bool {
        if self.is_local(name, ctx) {
            return false;
        }
        match self.index.lookup(self.module, name) {
            None => true,
            Some(Symbol::Module(path)) => self.index.find_module(&path, self.module).is_none(),
            Some(_) => false,
        }
    }

    /// Whether `name` is a local of the function (so not a global symbol).
    pub fn is_local(&self, name: &str, ctx: &Ctx) -> bool {
        ctx.shadowed.contains(name)
            || self.is_self(name)
            || self.param(name).is_some()
            || self.flow.binds(name)
    }

    /// Facts about `expr` evaluated at a point with context `ctx`.
    pub fn facts(&self, expr: &Expr, ctx: &Ctx) -> ValueFacts {
        match expr {
            Expr::NoneLiteral(_) => ValueFacts::none_value(),
            Expr::StringLiteral(s) => ValueFacts {
                max_length: Some(s.value.chars().count() as i64),
                ..ValueFacts::typed("str")
            },
            Expr::FString(_) => ValueFacts::typed("str"),
            Expr::BooleanLiteral(_) => ValueFacts::typed("bool"),
            Expr::NumberLiteral(n) => match &n.value {
                Number::Int(i) => {
                    let v = i.as_i64();
                    ValueFacts {
                        type_name: Some("int".to_string()),
                        weak_type: true,
                        precision: Some(Dep::Lit(0)),
                        min_value: v,
                        max_value: v,
                        ..ValueFacts::non_null()
                    }
                }
                Number::Float(_) => ValueFacts {
                    type_name: Some("float".to_string()),
                    weak_type: true,
                    ..ValueFacts::non_null()
                },
                Number::Complex { .. } => ValueFacts::non_null(),
            },
            Expr::Tuple(t) => {
                // Element precision, for tuples whose elements are written individually.
                let elems: Vec<Option<Dep>> = t
                    .elts
                    .iter()
                    .map(|e| self.facts(e, ctx).precision)
                    .collect();
                let precision = if !elems.is_empty() && elems.iter().all(|p| p.is_some()) {
                    elems.into_iter().flatten().reduce(Dep::max)
                } else {
                    None
                };
                ValueFacts {
                    precision,
                    ..ValueFacts::non_null()
                }
            }
            Expr::BytesLiteral(_)
            | Expr::List(_)
            | Expr::Dict(_)
            | Expr::Set(_)
            | Expr::ListComp(_)
            | Expr::SetComp(_)
            | Expr::DictComp(_)
            | Expr::Generator(_)
            | Expr::Lambda(_)
            | Expr::EllipsisLiteral(_) => ValueFacts::non_null(),
            Expr::Compare(_) => ValueFacts::typed("bool"),
            Expr::Name(n) => self.name_facts(n.id.as_str(), ctx),
            Expr::Call(c) => self.call_facts(c, ctx),
            Expr::BinOp(b) => self.binop_facts(b, ctx),
            Expr::UnaryOp(u) => self.unary_facts(u, ctx),
            Expr::BoolOp(b) => self.boolop_facts(b, ctx),
            Expr::If(i) => ValueFacts::join(
                self.facts(&i.body, &ctx.narrow(flow::positive(&i.test))),
                self.facts(&i.orelse, &ctx.narrow(flow::negative(&i.test))),
            ),
            Expr::Subscript(s) => self.subscript_facts(s, ctx),
            Expr::Named(n) => self.facts(&n.value, ctx),
            _ => ValueFacts::default(),
        }
    }

    fn name_facts(&self, name: &str, ctx: &Ctx) -> ValueFacts {
        if ctx.shadowed.contains(name) {
            return ValueFacts::default();
        }
        let facts = if self.is_self(name) {
            ValueFacts::non_null()
        } else {
            let param = self.param(name).map(|p| self.param_facts(p));
            let local = if self.flow.opaque.contains(name) {
                Some(ValueFacts::default())
            } else if self.flow.assignments.contains_key(name) {
                Some(self.local_facts(name))
            } else {
                None
            };
            match (param, local) {
                (Some(p), Some(l)) => ValueFacts::join(p, l),
                (Some(p), None) => p,
                (None, Some(l)) => l,
                (None, None) => ValueFacts::default(),
            }
        };
        if ctx.narrowed.contains(name) && !self.flow.unstable.contains(name) {
            facts.non_none_part()
        } else {
            facts
        }
    }

    fn local_facts(&self, name: &str) -> ValueFacts {
        if let Some(f) = self.memo.borrow().get(name) {
            return f.clone();
        }
        if !self.visiting.borrow_mut().insert(name.to_string()) {
            return ValueFacts::default(); // cyclic: unknown
        }
        let facts = ValueFacts::join_all(
            self.flow.assignments[name]
                .iter()
                .map(|a| self.facts(a.value, &Ctx::new(&a.narrowed))),
        )
        .unwrap_or_default();
        self.visiting.borrow_mut().remove(name);
        self.memo
            .borrow_mut()
            .insert(name.to_string(), facts.clone());
        facts
    }

    /// A parameter's value: nullability and type from its annotation;
    /// precision 0 for `int`, else dependent on the input when it is the
    /// function's only parameter.
    pub fn param_facts(&self, p: &ParamInfo) -> ValueFacts {
        if matches!(p.kind, ParamKind::VarArgs | ParamKind::VarKeywords) {
            return ValueFacts::non_null();
        }
        let precision = match p.type_name.as_deref() {
            Some("int") => Some(Dep::Lit(0)),
            Some("str" | "bool") => None,
            _ if self.func.and_then(|f| f.single_param()) == Some(p.name.as_str()) => {
                Some(Dep::Input)
            }
            _ => None,
        };
        ValueFacts {
            nullable: p.nullable,
            type_name: p.type_name.clone(),
            precision,
            ..Default::default()
        }
    }

    fn unary_facts(&self, u: &ast::ExprUnaryOp, ctx: &Ctx) -> ValueFacts {
        match u.op {
            UnaryOp::Not => ValueFacts::typed("bool"),
            UnaryOp::Invert => ValueFacts::default(),
            UnaryOp::USub | UnaryOp::UAdd => {
                let f = self.facts(&u.operand, ctx);
                let Some(t) = f.numeric_type().map(str::to_string) else {
                    return ValueFacts::default();
                };
                let (min_value, max_value) = if matches!(u.op, UnaryOp::USub) {
                    (
                        f.max_value.and_then(i64::checked_neg),
                        f.min_value.and_then(i64::checked_neg),
                    )
                } else {
                    (f.min_value, f.max_value)
                };
                ValueFacts {
                    nullable: f.nullable.filter(|n| !n),
                    type_name: Some(t),
                    weak_type: f.weak_type,
                    precision: f.precision,
                    min_value,
                    max_value,
                    ..Default::default()
                }
            }
        }
    }

    fn boolop_facts(&self, b: &ast::ExprBoolOp, ctx: &Ctx) -> ValueFacts {
        let mut parts = Vec::new();
        let mut c = ctx.clone();
        let last = b.values.len().saturating_sub(1);
        for (i, v) in b.values.iter().enumerate() {
            let f = self.facts(v, &c);
            match b.op {
                // `a or b` is `a` only when `a` is truthy, so not None.
                BoolOp::Or if i < last => parts.push(f.non_none_part()),
                _ => parts.push(f),
            }
            c = match b.op {
                BoolOp::And => c.narrow(flow::positive(v)),
                BoolOp::Or => c.narrow(flow::negative(v)),
            };
        }
        ValueFacts::join_all(parts).unwrap_or_default()
    }

    fn subscript_facts(&self, s: &ast::ExprSubscript, ctx: &Ctx) -> ValueFacts {
        let Expr::Slice(slice) = s.slice.as_ref() else {
            return ValueFacts::default();
        };
        let base = self.facts(&s.value, ctx);
        if base.type_name.as_deref() != Some("str") || slice.step.is_some() {
            return ValueFacts::default();
        }
        let nonneg = |e: &Option<Box<Expr>>| e.as_deref().and_then(int_literal).filter(|v| *v >= 0);
        let bound = match (&slice.lower, nonneg(&slice.upper)) {
            (None, Some(u)) => Some(u),
            (Some(_), Some(u)) => nonneg(&slice.lower).map(|l| (u - l).max(0)),
            _ => None,
        };
        let max_length = match (bound, base.max_length) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        ValueFacts {
            max_length,
            ..ValueFacts::typed("str")
        }
    }

    fn binop_facts(&self, b: &ast::ExprBinOp, ctx: &Ctx) -> ValueFacts {
        let l = self.facts(&b.left, ctx);
        let r = self.facts(&b.right, ctx);
        let result_type = binop_type(b.op, l.type_name.as_deref(), r.type_name.as_deref());
        let nullable =
            (l.nullable == Some(false) && r.nullable == Some(false) && result_type.is_some())
                .then_some(false);
        let precision = match result_type {
            Some("int") => Some(Dep::Lit(0)),
            Some("str") => None,
            _ => match (b.op, l.precision.clone(), r.precision.clone()) {
                // Conservative: one more place than the wider operand.
                (Operator::Add | Operator::Sub, Some(x), Some(y)) => {
                    Some(Dep::sum(Dep::max(x, y), Dep::Lit(1)))
                }
                (Operator::Mult, Some(x), Some(y)) => Some(Dep::sum(x, y)),
                // Division (and anything else) has no decimal-place bound.
                _ => None,
            },
        };
        let max_length = match (result_type, b.op, l.max_length, r.max_length) {
            (Some("str"), Operator::Add, Some(x), Some(y)) => Some(x + y),
            _ => None,
        };
        ValueFacts {
            nullable,
            weak_type: result_type.is_some() && l.weak_type && r.weak_type,
            type_name: result_type.map(str::to_string),
            precision,
            max_length,
            ..Default::default()
        }
    }

    fn call_facts(&self, call: &ast::ExprCall, ctx: &Ctx) -> ValueFacts {
        let args = &call.arguments.args;
        let func = call.func.as_ref();
        // Builtins (names that are neither locals nor project symbols).
        if let Expr::Name(n) = func {
            let name = n.id.as_str();
            if self.is_external(name, ctx) {
                match name {
                    "Decimal" => return decimal_ctor_facts(call, self, ctx),
                    "round" => return self.round_facts(call, ctx),
                    "str" | "repr" => return ValueFacts::typed("str"),
                    "int" | "len" => {
                        return ValueFacts {
                            precision: Some(Dep::Lit(0)),
                            min_value: (name == "len").then_some(0),
                            ..ValueFacts::typed("int")
                        }
                    }
                    "float" => return ValueFacts::typed("float"),
                    "bool" => return ValueFacts::typed("bool"),
                    "getattr" if args.len() == 3 && matches!(args[2], Expr::NoneLiteral(_)) => {
                        return ValueFacts::nullable()
                    }
                    "next" if args.len() == 2 && matches!(args[1], Expr::NoneLiteral(_)) => {
                        return ValueFacts::nullable()
                    }
                    _ => return ValueFacts::default(),
                }
            }
        }
        match self.callee(func, ctx) {
            Callee::Function { qualified, .. } => {
                let mut f = self.summaries.get(&qualified).cloned().unwrap_or_default();
                // A bound over the callee's own input means nothing here.
                if f.precision
                    .as_ref()
                    .is_some_and(|p| p.as_static().is_none())
                {
                    f.precision = None;
                }
                return f;
            }
            Callee::Class(_) => return ValueFacts::non_null(),
            Callee::Unknown => {}
        }
        let Expr::Attribute(attr) = func else {
            return ValueFacts::default();
        };
        let method = attr.attr.as_str();
        if let Some(parts) = dotted_parts(func) {
            let external = |head: &str| self.is_external(head, ctx);
            match parts
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice()
            {
                ["decimal", "Decimal"] if external("decimal") => {
                    return decimal_ctor_facts(call, self, ctx)
                }
                ["re", "match" | "search" | "fullmatch"] if external("re") => {
                    return ValueFacts::nullable()
                }
                _ => {}
            }
        }
        let kw_is_none = |name: &str| {
            call.arguments
                .keywords
                .iter()
                .any(|k| k.arg.as_deref() == Some(name) && matches!(k.value, Expr::NoneLiteral(_)))
        };
        match method {
            "quantize" => {
                let precision = args
                    .first()
                    .and_then(|step| self.facts(step, ctx).precision);
                ValueFacts {
                    precision,
                    ..ValueFacts::typed("Decimal")
                }
            }
            "get" if call.arguments.keywords.is_empty() && args.len() == 1 => {
                ValueFacts::nullable()
            }
            "get" if args.len() == 2 && matches!(args[1], Expr::NoneLiteral(_)) => {
                ValueFacts::nullable()
            }
            "get" if args.len() == 1 && kw_is_none("default") => ValueFacts::nullable(),
            "pop" if args.len() == 2 && matches!(args[1], Expr::NoneLiteral(_)) => {
                ValueFacts::nullable()
            }
            "first" | "last" if args.is_empty() && call.arguments.keywords.is_empty() => {
                ValueFacts::nullable()
            }
            m if STR_METHODS.contains(&m)
                && self.facts(&attr.value, ctx).type_name.as_deref() == Some("str") =>
            {
                ValueFacts::typed("str")
            }
            _ => ValueFacts::default(),
        }
    }

    fn round_facts(&self, call: &ast::ExprCall, ctx: &Ctx) -> ValueFacts {
        let args = &call.arguments.args;
        if !call.arguments.keywords.is_empty() || args.is_empty() {
            return ValueFacts::default();
        }
        if args.len() == 1 {
            return ValueFacts {
                precision: Some(Dep::Lit(0)),
                ..ValueFacts::typed("int")
            };
        }
        let value_type = self.facts(&args[0], ctx).numeric_type().map(str::to_string);
        ValueFacts {
            precision: int_literal(&args[1]).map(|n| Dep::Lit(n.max(0))),
            type_name: value_type,
            ..ValueFacts::non_null()
        }
    }

    /// Resolve the target of a call.
    pub fn callee(&self, func: &Expr, ctx: &Ctx) -> Callee<'a> {
        match func {
            Expr::Name(n) => {
                if self.is_local(n.id.as_str(), ctx) {
                    return Callee::Unknown;
                }
                self.symbol_callee(self.index.lookup(self.module, n.id.as_str()))
            }
            Expr::Attribute(attr) => {
                if let Some((class, instance)) = self.receiver_class(&attr.value, ctx) {
                    return match self.index.method(&class.qualified, attr.attr.as_str()) {
                        Some(q) => {
                            let implicit = self.index.function(&q).map_or(0, |f| {
                                if instance || f.method_kind == MethodKind::ClassMethod {
                                    f.implicit_params()
                                } else {
                                    0
                                }
                            });
                            Callee::Function {
                                qualified: q,
                                implicit,
                            }
                        }
                        None => Callee::Unknown,
                    };
                }
                let Some(parts) = dotted_parts(func) else {
                    return Callee::Unknown;
                };
                if self.is_local(&parts[0], ctx) {
                    return Callee::Unknown;
                }
                self.symbol_callee(self.index.resolve_dotted(self.module, &parts))
            }
            _ => Callee::Unknown,
        }
    }

    fn symbol_callee(&self, sym: Option<Symbol>) -> Callee<'a> {
        match sym {
            Some(Symbol::Function(q)) => match self.index.function(&q) {
                Some(f) => Callee::Function {
                    implicit: usize::from(f.method_kind == MethodKind::ClassMethod)
                        * f.implicit_params(),
                    qualified: q,
                },
                None => Callee::Unknown,
            },
            Some(Symbol::Class(q)) => self.index.class(&q).map_or(Callee::Unknown, Callee::Class),
            _ => Callee::Unknown,
        }
    }

    /// The class of the object `expr` names, and whether it is an instance
    /// (rather than the class itself, as `cls` in a classmethod).
    pub fn receiver_class(&self, expr: &Expr, ctx: &Ctx) -> Option<(&'a ClassInfo, bool)> {
        let Expr::Name(n) = expr else { return None };
        let name = n.id.as_str();
        if ctx.shadowed.contains(name) {
            return None;
        }
        if self.is_self(name) {
            let instance = self.func?.method_kind == MethodKind::Instance;
            return self.class.map(|c| (c, instance));
        }
        if self.flow.opaque.contains(name) {
            return None;
        }
        let mut candidates: Vec<Option<&'a ClassInfo>> = Vec::new();
        if let Some(p) = self.param(name) {
            candidates.push(
                p.annotation
                    .as_ref()
                    .and_then(|a| self.annotation_class(self.module, a)),
            );
        }
        if let Some(assigns) = self.flow.assignments.get(name) {
            for a in assigns {
                candidates.push(self.class_of_value(a.value, &Ctx::new(&a.narrowed)));
            }
        }
        let first = (*candidates.first()?)?;
        candidates
            .iter()
            .all(|c| c.is_some_and(|c| c.qualified == first.qualified))
            .then_some((first, true))
    }

    /// The class named by a type annotation (`Invoice`, `Optional[Invoice]`,
    /// `"Invoice"`), resolved in `module`.
    fn annotation_class(&self, module: &str, annotation: &Expr) -> Option<&'a ClassInfo> {
        let inner = strip_optional(annotation);
        let sym = match inner {
            Expr::StringLiteral(s) => {
                let text = s.value.to_str().trim().to_string();
                let parts: Vec<String> = text.split('.').map(str::to_string).collect();
                if parts
                    .iter()
                    .any(|p| p.is_empty() || !p.chars().all(|c| c.is_alphanumeric() || c == '_'))
                {
                    return None;
                }
                self.index.resolve_dotted(module, &parts)?
            }
            other => self.index.resolve_expr(module, other)?,
        };
        match sym {
            Symbol::Class(q) => self.index.class(&q),
            _ => None,
        }
    }

    /// The class of the instance an expression evaluates to, if known:
    /// `Cls(...)`, `Cls.objects.get/create(...)`, `Cls.objects.filter(...).first()`,
    /// or a call to a function whose return annotation names the class.
    fn class_of_value(&self, expr: &Expr, ctx: &Ctx) -> Option<&'a ClassInfo> {
        let Expr::Call(call) = expr else { return None };
        if let Some((class, method)) = self.objects_call(call, ctx) {
            return INSTANCE_METHODS.contains(&method).then_some(class);
        }
        match self.callee(&call.func, ctx) {
            Callee::Class(c) => Some(c),
            Callee::Function { qualified, .. } => {
                let f = self.index.function(&qualified)?;
                self.annotation_class(&f.module, f.return_annotation.as_ref()?)
            }
            Callee::Unknown => None,
        }
    }

    /// `Model.objects.<qs methods>.<method>(...)` on a Django model: the model and `method`.
    pub fn objects_call<'c>(
        &self,
        call: &'c ast::ExprCall,
        ctx: &Ctx,
    ) -> Option<(&'a ClassInfo, &'c str)> {
        let Expr::Attribute(attr) = call.func.as_ref() else {
            return None;
        };
        let mut value = attr.value.as_ref();
        while let Expr::Call(inner) = value {
            let Expr::Attribute(inner_attr) = inner.func.as_ref() else {
                return None;
            };
            if !QUERYSET_METHODS.contains(&inner_attr.attr.as_str()) {
                return None;
            }
            value = inner_attr.value.as_ref();
        }
        let Expr::Attribute(objects) = value else {
            return None;
        };
        if objects.attr.as_str() != "objects" {
            return None;
        }
        let parts = dotted_parts(&objects.value)?;
        if self.is_local(&parts[0], ctx) {
            return None;
        }
        match self.index.resolve_dotted(self.module, &parts)? {
            Symbol::Class(q) => {
                let class = self.index.class(&q)?;
                (class.kind == ClassKind::Django).then_some((class, attr.attr.as_str()))
            }
            _ => None,
        }
    }

    /// The extracted function whose result `expr` is: a direct call, or a
    /// local assigned exactly once, from such a call.
    pub fn producer_of(&self, expr: &Expr, ctx: &Ctx) -> Option<String> {
        match expr {
            Expr::Call(c) => match self.callee(&c.func, ctx) {
                Callee::Function { qualified, .. } => Some(qualified),
                _ => None,
            },
            Expr::Name(n) => {
                let name = n.id.as_str();
                if ctx.shadowed.contains(name)
                    || self.param(name).is_some()
                    || self.is_self(name)
                    || self.flow.opaque.contains(name)
                {
                    return None;
                }
                let [a] = self.flow.assignments.get(name)?.as_slice() else {
                    return None;
                };
                match a.value {
                    Expr::Call(c) => match self.callee(&c.func, &Ctx::new(&a.narrowed)) {
                        Callee::Function { qualified, .. } => Some(qualified),
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The string-keyed entries of a dict literal, or of a local bound once to
    /// one and never mutated or passed on, with the context of each value.
    pub fn dict_items(&self, expr: &'a Expr, ctx: &Ctx) -> Option<Vec<(String, &'a Expr, Ctx)>> {
        match expr {
            Expr::Dict(d) => d
                .items
                .iter()
                .map(|item| match &item.key {
                    Some(Expr::StringLiteral(k)) => {
                        Some((k.value.to_string(), &item.value, ctx.clone()))
                    }
                    _ => None,
                })
                .collect(),
            Expr::Name(n) => {
                let name = n.id.as_str();
                if ctx.shadowed.contains(name)
                    || self.param(name).is_some()
                    || self.flow.opaque.contains(name)
                    || self.flow.escaping.contains(name)
                {
                    return None;
                }
                let [a] = self.flow.assignments.get(name)?.as_slice() else {
                    return None;
                };
                match a.value {
                    Expr::Dict(_) => self.dict_items(a.value, &Ctx::new(&a.narrowed)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Facts about the function's return value: body `return` expressions
    /// (plus the implicit `None` when the end is reachable), refined by the
    /// return annotation.
    pub fn return_facts(&self) -> ValueFacts {
        let mut value_return = false;
        let mut joined: Option<ValueFacts> = None;
        for (expr, narrowed) in &self.flow.returns {
            let f = match expr {
                Some(e) => {
                    value_return |= !matches!(e, Expr::NoneLiteral(_));
                    self.facts(e, &Ctx::new(narrowed))
                }
                None => ValueFacts::none_value(),
            };
            joined = Some(match joined {
                Some(j) => ValueFacts::join(j, f),
                None => f,
            });
        }
        if self.flow.falls_through && value_return {
            joined = joined.map(|j| ValueFacts::join(j, ValueFacts::none_value()));
        }
        let mut facts = joined.unwrap_or_default();
        let Some(func) = self.func else { return facts };
        let Some(annotation) = &func.return_annotation else {
            return facts;
        };
        if is_named(annotation, "Any") || is_none_annotation(annotation) {
            return facts;
        }
        if func.is_return_optional {
            facts.nullable = Some(true);
        } else if facts.nullable != Some(true) {
            // The annotation excludes None and the body shows no None.
            facts.nullable = Some(false);
        }
        let annotated = func.tuple_element_type.clone().or_else(|| {
            func.return_type
                .as_deref()
                .map(|t| t.split('[').next().unwrap_or(t).trim().to_string())
        });
        facts.type_name = annotated.filter(|t| VALUE_TYPES.contains(&t.as_str()));
        facts.weak_type = false;
        facts
    }
}

/// Result type of a binary operation on value types, if known.
fn binop_type(op: Operator, l: Option<&str>, r: Option<&str>) -> Option<&'static str> {
    use Operator::*;
    let arith = matches!(op, Add | Sub | Mult | Div | FloorDiv | Mod);
    match (l?, r?) {
        ("str", "str") if matches!(op, Add) => Some("str"),
        ("str", _) if matches!(op, Mod) => Some("str"),
        ("Decimal", "Decimal" | "int") | ("int", "Decimal") if arith => Some("Decimal"),
        ("int", "int") if matches!(op, Add | Sub | Mult | FloorDiv | Mod) => Some("int"),
        ("int", "int") if matches!(op, Div) => Some("float"),
        ("float", "float" | "int") | ("int", "float") if arith => Some("float"),
        _ => None,
    }
}

/// `Decimal('1.25')` → 2 places; `Decimal(3)` → 0; other arguments unknown.
fn decimal_ctor_facts(call: &ast::ExprCall, scope: &Scope, ctx: &Ctx) -> ValueFacts {
    let precision = if call.arguments.keywords.is_empty() && call.arguments.args.len() == 1 {
        match &call.arguments.args[0] {
            Expr::StringLiteral(s) => decimal_literal_places(s.value.to_str()),
            arg => {
                let f = scope.facts(arg, ctx);
                (f.type_name.as_deref() == Some("int")).then_some(0)
            }
        }
    } else {
        None
    };
    ValueFacts {
        precision: precision.map(Dep::Lit),
        ..ValueFacts::typed("Decimal")
    }
}

/// Decimal places of a plain decimal literal string (`"-12.340"` → 3).
/// Exponent notation, `NaN`, `Infinity` and anything else give `None`.
pub fn decimal_literal_places(text: &str) -> Option<i64> {
    let t = text.trim();
    let t = t.strip_prefix(['-', '+']).unwrap_or(t);
    let (int_part, frac) = match t.split_once('.') {
        Some((i, f)) => (i, f),
        None => (t, ""),
    };
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit() || c == '_');
    if (int_part.is_empty() && frac.is_empty()) || !digits(int_part) || !digits(frac) {
        return None;
    }
    Some(frac.chars().filter(|c| c.is_ascii_digit()).count() as i64)
}

/// An integer literal, including a negated one.
pub fn int_literal(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::NumberLiteral(n) => match &n.value {
            Number::Int(i) => i.as_i64(),
            _ => None,
        },
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::USub) => {
            int_literal(&u.operand).and_then(i64::checked_neg)
        }
        _ => None,
    }
}

fn is_named(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Name(n) => n.id.as_str() == name,
        Expr::Attribute(a) => a.attr.as_str() == name,
        _ => false,
    }
}

fn is_none_annotation(expr: &Expr) -> bool {
    matches!(expr, Expr::NoneLiteral(_)) || is_named(expr, "None")
}

/// Postcondition rows stating `facts`.
pub fn facts_rows(
    facts: &ValueFacts,
    node_id: i64,
    source_file: &str,
    source_line: u32,
    level: VerificationLevel,
) -> Vec<ContractRecord> {
    let base = |kind: ConstraintType| {
        ContractRecord::new(
            node_id,
            kind,
            ContractRole::Postcondition,
            level.clone(),
            source_file,
            source_line,
        )
    };
    let mut rows = Vec::new();
    if let Some(t) = facts
        .type_name
        .as_ref()
        .filter(|t| VALUE_TYPES.contains(&t.as_str()))
    {
        if !facts.weak_type {
            rows.push(ContractRecord {
                param_type_name: Some(t.clone()),
                ..base(ConstraintType::Type)
            });
        }
    }
    if let Some(n) = facts.nullable {
        rows.push(ContractRecord {
            param_nullable: Some(n as i64),
            ..base(ConstraintType::Nullability)
        });
    }
    if let (false, Some(p)) = (facts.always_none, &facts.precision) {
        rows.push(match p.as_static() {
            Some(v) => ContractRecord {
                param_decimal_places: Some(v.max(0)),
                ..base(ConstraintType::Precision)
            },
            None => ContractRecord {
                dependent_expr: Some(p.render()),
                ..base(ConstraintType::Precision)
            },
        });
    }
    if let Some(len) = facts.max_length {
        rows.push(ContractRecord {
            param_max_length: Some(len),
            ..base(ConstraintType::Length)
        });
    }
    if facts.min_value.is_some() || facts.max_value.is_some() {
        rows.push(ContractRecord {
            param_min_value: facts.min_value.map(|v| v as f64),
            param_max_value: facts.max_value.map(|v| v as f64),
            ..base(ConstraintType::Range)
        });
    }
    rows
}

/// Return-value summaries of every extracted function (EXTRACTED facts only:
/// annotations and body; docstrings are not included).
pub fn compute_summaries(index: &ProjectIndex) -> HashMap<String, ValueFacts> {
    let flows: Vec<FunctionFlow> = index
        .functions
        .iter()
        .map(|f| FunctionFlow::of(&f.body))
        .collect();
    let mut summaries: HashMap<String, ValueFacts> = HashMap::new();
    for _ in 0..index.functions.len() + 2 {
        let next: HashMap<String, ValueFacts> = index
            .functions
            .iter()
            .zip(&flows)
            .map(|(f, flow)| {
                let scope = Scope::new(index, &summaries, Some(f), flow);
                (f.qualified_name.clone(), scope.return_facts())
            })
            .collect();
        if next == summaries {
            break;
        }
        summaries = next;
    }
    summaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::Project;

    fn project(files: &[(&str, &str)]) -> Project {
        Project::from_sources(
            files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        )
    }

    /// Facts of the value of the last expression statement of function `f`
    /// in module `code` (evaluated with the narrowing at that point).
    fn facts_of(files: &[(&str, &str)], func: &str) -> ValueFacts {
        let p = project(files);
        let f = p.index.function(func).expect("function");
        let flow = FunctionFlow::of(&f.body);
        let scope = Scope::new(&p.index, &p.summaries, Some(f), &flow);
        struct Last<'a>(Option<(&'a Expr, Narrowed)>);
        impl<'a> flow::FlowVisitor<'a> for Last<'a> {
            fn simple(&mut self, stmt: &'a ruff_python_ast::Stmt, n: &Narrowed) {
                if let ruff_python_ast::Stmt::Expr(e) = stmt {
                    self.0 = Some((&e.value, n.clone()));
                }
            }
            fn header(&mut self, _: &'a Expr, _: &Narrowed) {}
        }
        let mut last = Last(None);
        flow::walk_block(&f.body, &Narrowed::new(), &mut last);
        let (expr, n) = last.0.expect("an expression statement");
        scope.facts(expr, &Ctx::new(&n))
    }

    fn one(src: &str) -> ValueFacts {
        facts_of(&[("code.py", src)], "code.f")
    }

    #[test]
    fn test_dep_rendering() {
        assert_eq!(
            Dep::max(Dep::Lit(3), Dep::Input).render(),
            "max(3, input_precision)"
        );
        assert_eq!(Dep::max(Dep::Lit(3), Dep::Lit(5)), Dep::Lit(5));
        assert_eq!(
            Dep::sum(Dep::Input, Dep::Lit(1)).render(),
            "add(input_precision, 1)"
        );
        assert_eq!(Dep::sum(Dep::Input, Dep::Lit(0)), Dep::Input);
    }

    #[test]
    fn test_decimal_literal_places() {
        assert_eq!(decimal_literal_places("0.001"), Some(3));
        assert_eq!(decimal_literal_places("-12.340"), Some(3));
        assert_eq!(decimal_literal_places("7"), Some(0));
        assert_eq!(decimal_literal_places("1E-3"), None);
        assert_eq!(decimal_literal_places("NaN"), None);
    }

    #[test]
    fn test_literals() {
        let f = one("def f():\n    'abc'\n");
        assert_eq!(
            (f.type_name.as_deref(), f.nullable, f.max_length),
            (Some("str"), Some(false), Some(3))
        );
        let f = one("def f():\n    None\n");
        assert!(f.always_none && f.nullable == Some(true));
        let f = one("def f():\n    -5\n");
        assert_eq!(
            (f.min_value, f.max_value, f.nullable),
            (Some(-5), Some(-5), Some(false))
        );
        assert!(f.weak_type, "numeric literals carry no type contract");
        let f = one("def f():\n    Decimal('0.25')\n");
        assert_eq!(
            (f.precision, f.type_name.as_deref()),
            (Some(Dep::Lit(2)), Some("Decimal"))
        );
        assert_eq!(one("def f():\n    [1, 2]\n").nullable, Some(false));
    }

    #[test]
    fn test_precision_static_and_dependent() {
        // One parameter: a parameter-derived bound depends on the input.
        let f = one(
            "def f(p: Decimal):\n    x = p.quantize(Decimal('0.001')) if p < 1 else p\n    x\n",
        );
        assert_eq!(
            f.precision.map(|p| p.render()).as_deref(),
            Some("max(3, input_precision)")
        );
        // Two parameters: unknown.
        let f =
            one("def f(p: Decimal, c):\n    x = p.quantize(Decimal('0.001')) if c else p\n    x\n");
        assert_eq!(f.precision, None);
        // int parameters have no decimal places.
        let f = one("def f(q: int, r: int):\n    q - r\n");
        assert_eq!(
            (f.precision, f.type_name.as_deref()),
            (Some(Dep::Lit(0)), Some("int"))
        );
        // An unknown operand makes arithmetic unknown (no skipping).
        let f = one("def f(a: Decimal, rate: Decimal):\n    a.quantize(Decimal('0.01')) * rate\n");
        assert_eq!(
            (f.precision, f.type_name.as_deref(), f.nullable),
            (None, Some("Decimal"), Some(false))
        );
        // Division has no bound.
        assert_eq!(
            one("def f():\n    Decimal('1') / Decimal('3')\n").precision,
            None
        );
    }

    #[test]
    fn test_none_producers() {
        for expr in [
            "d.get(k)",
            "d.get(k, None)",
            "d.get(k, default=None)",
            "getattr(o, 'x', None)",
            "next(it, None)",
            "d.pop(k, None)",
            "re.match(p, s)",
            "qs.filter(a=1).first()",
            "qs.last()",
            "a if c else None",
        ] {
            let src = format!("import re\ndef f(d, k, o, it, p, s, qs, a, c):\n    {expr}\n");
            assert_eq!(
                facts_of(&[("code.py", &src)], "code.f").nullable,
                Some(true),
                "{expr}"
            );
        }
        // A non-None default is not a None producer.
        assert_eq!(one("def f(d):\n    d.get('k', 0)\n").nullable, None);
        assert_eq!(
            one("def f(o: Optional[str]):\n    o\n").nullable,
            Some(true)
        );
        assert_eq!(one("def f(o: str):\n    o\n").nullable, Some(false));
        assert_eq!(one("def f(o):\n    o\n").nullable, None);
        assert_eq!(one("def f(o, d: str):\n    o or d\n").nullable, Some(false));
    }

    #[test]
    fn test_narrowing_refines_nullability() {
        let f = one("def f(code: Optional[str]):\n    if code is None:\n        raise ValueError()\n    code\n");
        assert_eq!(f.nullable, Some(false));
        let f = one("def f(o):\n    label = getattr(o, 'x', None)\n    assert label is not None\n    label\n");
        assert_eq!(f.nullable, Some(false));
        let f = one("def f(o):\n    label = getattr(o, 'x', None)\n    label\n");
        assert_eq!(f.nullable, Some(true));
        // An assignment evaluated under narrowing keeps it.
        let f = one("def f(code: Optional[str]):\n    if code is not None:\n        name = code\n        name\n");
        assert_eq!(f.nullable, Some(false));
    }

    #[test]
    fn test_string_slices_and_methods() {
        let f = one("def f(a: str, b: str):\n    (a + ' ' + b)[:64]\n");
        assert_eq!(
            (f.type_name.as_deref(), f.max_length),
            (Some("str"), Some(64))
        );
        assert_eq!(one("def f(a: str):\n    a[2:10]\n").max_length, Some(8));
        assert_eq!(
            one("def f(a: str):\n    a.upper()\n").type_name.as_deref(),
            Some("str")
        );
        assert_eq!(
            one("def f(a):\n    a[:5]\n").max_length,
            None,
            "unknown type"
        );
    }

    #[test]
    fn test_interprocedural_nullability() {
        let files = [(
            "code.py",
            "P = {}\ndef find(s):\n    return P.get(s)\ndef price(s):\n    return find(s)\ndef ok(s) -> str:\n    return 'x'\ndef f(s):\n    price(s)\n",
        )];
        let p = project(&files);
        assert_eq!(p.summaries["code.find"].nullable, Some(true));
        assert_eq!(p.summaries["code.price"].nullable, Some(true));
        assert_eq!(p.summaries["code.ok"].nullable, Some(false));
        assert_eq!(facts_of(&files, "code.f").nullable, Some(true));
    }

    #[test]
    fn test_fall_through_is_a_none_return() {
        let p = project(&[("code.py", "def f(x):\n    if x:\n        return 'a'\n")]);
        assert_eq!(p.summaries["code.f"].nullable, Some(true));
        let p = project(&[("code.py", "def f(x) -> Optional[int]:\n    return 3\n")]);
        let s = &p.summaries["code.f"];
        assert_eq!(
            (s.nullable, s.type_name.as_deref()),
            (Some(true), Some("int"))
        );
    }
}
