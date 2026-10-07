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

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use ruff_python_ast::visitor;
use ruff_python_ast::{self as ast, BoolOp, Expr, Number, Operator, UnaryOp};

use crate::bounds::{self, Dec};
use crate::db::{ConstraintType, ContractRecord, ContractRole, VerificationLevel};
use crate::flow::{self, Def, FunctionFlow, Narrowed};
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
            // Saturating: a larger bound on places is still an upper bound.
            (Dep::Lit(x), Dep::Lit(y)) => Dep::Lit(x.saturating_add(y)),
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
    /// Lower / upper bound on the numeric value, exact (see `bounds`).
    pub min_value: Option<Dec>,
    pub max_value: Option<Dec>,
    /// The possible values, as choice strings (string and integer literals).
    pub choices: Option<Vec<String>>,
}

impl ValueFacts {
    /// `None`: nullable, and trivially within any precision, length or
    /// choices bound (there is no value to exceed it).
    pub fn none_value() -> Self {
        ValueFacts {
            always_none: true,
            nullable: Some(true),
            precision: Some(Dep::Lit(0)),
            max_length: Some(0),
            choices: Some(Vec::new()),
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
            min_value: match (a.min_value, b.min_value) {
                (Some(x), Some(y)) => Some(x.min(y)),
                _ => None,
            },
            max_value: match (a.max_value, b.max_value) {
                (Some(x), Some(y)) => Some(x.max(y)),
                _ => None,
            },
            choices: match (a.choices, b.choices) {
                (Some(mut x), Some(y)) => {
                    for v in y {
                        if !x.contains(&v) {
                            x.push(v);
                        }
                    }
                    Some(x)
                }
                _ => None,
            },
        }
    }

    pub fn join_all(items: impl IntoIterator<Item = ValueFacts>) -> Option<ValueFacts> {
        items.into_iter().reduce(ValueFacts::join)
    }

    /// The facts restricted to the non-None case.
    pub fn non_none_part(self) -> ValueFacts {
        if self.always_none {
            // Unreachable as a value; nothing more is known.
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
    /// Comprehension variables (shadowed) known to be elements of a
    /// collection of a project class: name -> qualified class name.
    pub elements: HashMap<String, String>,
}

impl Ctx {
    pub fn new(narrowed: &Narrowed) -> Self {
        Ctx {
            narrowed: narrowed.clone(),
            shadowed: HashSet::new(),
            elements: HashMap::new(),
        }
    }

    pub fn narrow(&self, names: Vec<String>) -> Ctx {
        let mut c = self.clone();
        c.narrowed
            .extend(names.into_iter().filter(|n| !self.shadowed.contains(n)));
        c
    }

    /// Where the operand after `v` of a boolean operator `op` is evaluated:
    /// `v` was truthy (`and`) or falsy (`or`), and its effects took place.
    pub fn after_operand(&self, v: &Expr, op: BoolOp) -> Ctx {
        let mut c = match op {
            BoolOp::And => self.narrow(flow::positive(v)),
            BoolOp::Or => self.narrow(flow::negative(v)),
        };
        c.narrowed.apply(&flow::effects(v));
        c
    }

    /// Where a branch of `body if test else orelse` is evaluated.
    pub fn branch(&self, test: &Expr, taken: bool) -> Ctx {
        let mut c = self.clone();
        c.narrowed.apply(&flow::effects(test));
        c.narrow(if taken { flow::positive(test) } else { flow::negative(test) })
    }

    pub fn shadow(&self, names: impl IntoIterator<Item = String>) -> Ctx {
        let mut c = self.clone();
        for n in names {
            c.narrowed.remove(&n);
            c.elements.remove(&n);
            c.shadowed.insert(n);
        }
        c
    }
}

/// Most alternatives a value is split into at a write or argument site.
pub const MAX_ALTERNATIVES: usize = 4;

/// One alternative of a value (see `Scope::alternatives`).
#[derive(Debug, Clone)]
pub enum Alt<'e> {
    /// An expression evaluated with this context; `true`: known non-None
    /// where the value is used.
    Expr(&'e Expr, Ctx, bool),
    /// A value with these facts (a parameter's argument, an augmented assignment).
    Facts(ValueFacts),
}

/// The string-keyed entries of a dict, with each value's context.
pub type DictEntries<'e> = Vec<(String, &'e Expr, Ctx)>;

/// What a `**d` splat (or a dict argument) supplies (see `Scope::splat`).
#[derive(Debug, Clone)]
pub struct Splat<'e> {
    pub entries: Vec<(String, SplatValue<'e>)>,
    /// No keys other than `entries` (else unknown ones may be present).
    pub complete: bool,
}

/// The value of one known key of a splatted dict.
#[derive(Debug, Clone)]
pub enum SplatValue<'e> {
    /// An expression of the enclosing function, with its context.
    Expr(&'e Expr, Ctx),
    /// Facts of a value computed in another function (a returned dict),
    /// located at `file` / byte offset.
    Facts(ValueFacts, String, u32),
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

/// Queryset methods that return one model instance (awaited, for the `a…` forms).
const INSTANCE_METHODS: [&str; 12] = [
    "get", "create", "first", "last", "latest", "earliest", "aget", "acreate", "afirst",
    "alast", "alatest", "aearliest",
];

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

/// Per function whose callers the project shows (see
/// `edge_discovery::caller_guards`), per parameter, the fields `p.f` that
/// every call narrows on its argument (`if h.name: label_of(h)`).
pub type CallerGuards = HashMap<String, HashMap<String, HashSet<String>>>;

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
    /// Facts of single assignments (`name`, index), for reaching definitions.
    def_memo: RefCell<HashMap<(String, usize), ValueFacts>>,
    def_visiting: RefCell<HashSet<(String, usize)>>,
    /// Locals whose receiver class is being resolved (`v = v.m()` cycles).
    receiving: RefCell<HashSet<String>>,
    /// Nesting depth of `facts` / `receiver_class`; past `MAX_EVAL_DEPTH`
    /// the value is unknown.
    depth: Cell<usize>,
    /// Parameters with a `None` default have unknown nullability: a
    /// project-defined decorator may inject their argument.
    injected: bool,
    /// Dict forwarder parameters (`edge_discovery::dict_param_key`), whose
    /// argument dicts are only splatted by the callee.
    pub dict_forwarders: Option<&'a HashSet<String>>,
    caller_guards: Option<&'a CallerGuards>,
}

/// Recursion bound of the value analysis: deeper values are unknown.
const MAX_EVAL_DEPTH: usize = 200;

/// Decrements the scope's evaluation depth on drop.
struct DepthGuard<'s>(&'s Cell<usize>);

impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
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
            def_memo: RefCell::new(HashMap::new()),
            def_visiting: RefCell::new(HashSet::new()),
            receiving: RefCell::new(HashSet::new()),
            depth: Cell::new(0),
            injected: func.is_some_and(|f| injecting_decorator(index, f)),
            dict_forwarders: None,
            caller_guards: None,
        }
    }

    /// The same scope, knowing the project's dict forwarder parameters.
    pub fn with_dict_forwarders(mut self, forwarders: &'a HashSet<String>) -> Self {
        self.dict_forwarders = Some(forwarders);
        self
    }

    /// The same scope, knowing the project's caller guards.
    pub fn with_caller_guards(mut self, guards: &'a CallerGuards) -> Self {
        self.caller_guards = Some(guards);
        self
    }

    /// Enter one level of recursive evaluation; `None` past the bound.
    fn enter(&self) -> Option<DepthGuard<'_>> {
        if self.depth.get() >= MAX_EVAL_DEPTH {
            return None;
        }
        self.depth.set(self.depth.get() + 1);
        Some(DepthGuard(&self.depth))
    }

    fn param(&self, name: &str) -> Option<&'a ParamInfo> {
        self.func?.value_params().iter().find(|p| p.name == name)
    }

    fn is_self(&self, name: &str) -> bool {
        self.func.and_then(|f| f.self_name()) == Some(name)
    }

    /// Whether `name` refers to something outside the project (a builtin or
    /// a third-party module or name), rather than a local or project symbol.
    pub fn is_external(&self, name: &str, ctx: &Ctx) -> bool {
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
        let Some(_guard) = self.enter() else {
            return ValueFacts::default();
        };
        match expr {
            Expr::NoneLiteral(_) => ValueFacts::none_value(),
            Expr::StringLiteral(s) => ValueFacts {
                max_length: Some(s.value.chars().count() as i64),
                choices: Some(vec![s.value.to_string()]),
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
                        min_value: v.map(bounds::from_int),
                        max_value: v.map(bounds::from_int),
                        choices: v.map(|v| vec![v.to_string()]),
                        ..ValueFacts::non_null()
                    }
                }
                Number::Float(f) => ValueFacts {
                    type_name: Some("float".to_string()),
                    weak_type: true,
                    min_value: bounds::from_f64(*f),
                    max_value: bounds::from_f64(*f),
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
                self.facts(&i.body, &ctx.branch(&i.test, true)),
                self.facts(&i.orelse, &ctx.branch(&i.test, false)),
            ),
            Expr::Subscript(s) => self.subscript_facts(s, ctx),
            Expr::Named(n) => self.facts(&n.value, ctx),
            // The awaited result of a coroutine call has the callee's return facts.
            Expr::Await(a) => self.facts(&a.value, ctx),
            Expr::Attribute(_) => self.attribute_facts(expr, ctx),
            _ => ValueFacts::default(),
        }
    }

    /// `module.CONST`, `Class.CONST`, `self.CONST`, `Enum.MEMBER`.
    fn attribute_facts(&self, expr: &Expr, ctx: &Ctx) -> ValueFacts {
        let Expr::Attribute(attr) = expr else {
            return ValueFacts::default();
        };
        if let Expr::Name(n) = attr.value.as_ref() {
            if self.is_self(n.id.as_str()) && !ctx.shadowed.contains(n.id.as_str()) {
                let Some(class) = self.class else {
                    return ValueFacts::default();
                };
                let name = attr.attr.as_str();
                if class.fields.iter().any(|f| f == name) {
                    let instance = self.func.is_some_and(|f| f.method_kind == MethodKind::Instance);
                    return if instance {
                        self.field_read_facts(n.id.as_str(), class, name, ctx)
                    } else {
                        ValueFacts::default()
                    };
                }
                // Every value the class constant has in `class` or in a
                // project subclass overriding it: their join.
                return match self.self_constant_values(class, name) {
                    Some(values) => ValueFacts::join_all(values).unwrap_or_default(),
                    None => ValueFacts::default(),
                };
            }
        }
        // `obj.f` for a field of the known class of local `obj`.
        if let Expr::Name(n) = attr.value.as_ref() {
            if let Some((class, true)) = self.receiver_class(&attr.value, ctx) {
                let name = attr.attr.as_str();
                if class.fields.iter().any(|f| f == name) {
                    // A comprehension variable: an element of the collection,
                    // not the function's own `obj`.
                    if ctx.elements.contains_key(n.id.as_str()) {
                        return declared_field_facts(class, name);
                    }
                    return self.field_read_facts(n.id.as_str(), class, name, ctx);
                }
            }
        }
        let Some(parts) = dotted_parts(expr) else {
            return ValueFacts::default();
        };
        // `self.Status.PAID`: a nested class (or class constant) of the class.
        if parts.len() >= 3 && self.is_self(&parts[0]) && !ctx.shadowed.contains(&parts[0]) {
            let Some(class) = self.class else {
                return ValueFacts::default();
            };
            // A subclass may bind its own `Status` (or assign it on instances).
            if class.fields.contains(&parts[1])
                || class.stored_attrs.contains(&parts[1])
                || self.index.overridden_in_subclass(&class.qualified, &parts[1])
            {
                return ValueFacts::default();
            }
            let sym = Symbol::Class(class.qualified.clone());
            return match self.index.resolve_attrs(self.module, sym, &parts[1..]) {
                Some(Symbol::Constant(q)) => self.constant_facts(&q),
                _ => ValueFacts::default(),
            };
        }
        if self.is_local(&parts[0], ctx) {
            return ValueFacts::default();
        }
        match self.index.resolve_dotted(self.module, &parts) {
            Some(Symbol::Constant(q)) => self.constant_facts(&q),
            _ => ValueFacts::default(),
        }
    }

    /// The facts of each value class constant `name` may have when read as
    /// `self.name` / `cls.name` in a method of `class` (the receiver may be
    /// an instance of a project subclass that overrides it): at most
    /// `MAX_ALTERNATIVES`, else unknown (`None`).
    fn self_constant_values(&self, class: &ClassInfo, name: &str) -> Option<Vec<ValueFacts>> {
        let values = self
            .index
            .class_constant_values(&class.qualified, name, MAX_ALTERNATIVES)?;
        Some(values.iter().map(|q| self.constant_facts(q)).collect())
    }

    /// `self.name` / `cls.name` for a class constant with several values
    /// (overridden in project subclasses): the facts of each.
    fn self_constant_alternatives(&self, expr: &Expr, ctx: &Ctx) -> Option<Vec<ValueFacts>> {
        let Expr::Attribute(attr) = expr else { return None };
        let Expr::Name(n) = attr.value.as_ref() else { return None };
        if !self.is_self(n.id.as_str()) || ctx.shadowed.contains(n.id.as_str()) {
            return None;
        }
        let class = self.class?;
        let name = attr.attr.as_str();
        if class.fields.iter().any(|f| f == name) {
            return None;
        }
        self.self_constant_values(class, name).filter(|v| v.len() > 1)
    }

    /// A read of field `field` of `obj` (an instance of `class`): the values
    /// assigned to `obj.f` in this function that reach here, or, when the
    /// function does not assign it, the field's declared nullability.
    fn field_read_facts(&self, obj: &str, class: &ClassInfo, field: &str, ctx: &Ctx) -> ValueFacts {
        let name = format!("{obj}.{field}");
        // Every caller narrows the argument's field: unknown, not nullable,
        // since a caller the project does not show may still pass None.
        let guarded = self.func.is_some_and(|f| {
            f.params.iter().any(|p| p.name == obj)
                && !self.flow.assignments.contains_key(obj)
                && !self.flow.opaque.contains(obj)
                // No call can have written the field since the entry.
                && ctx.narrowed.untouched(obj)
                && self
                    .caller_guards
                    .and_then(|g| g.get(&f.qualified_name)?.get(obj))
                    .is_some_and(|fields| fields.contains(field))
        });
        let declared = || {
            let mut facts = declared_field_facts(class, field);
            if guarded && facts.nullable == Some(true) {
                facts.nullable = None;
            }
            facts
        };
        let facts = if self.flow.opaque.contains(&name) {
            ValueFacts::default()
        } else if let Some(assigns) = self.flow.attr_assignments.get(&name) {
            match ctx.narrowed.defs(&name) {
                Some(defs) if !defs.is_empty() => ValueFacts::join_all(defs.iter().map(|&(d, nn)| {
                    let f = match d {
                        Def::Assign(i) => assigns
                            .get(i)
                            .map(|a| self.attr_assignment_facts(&name, i, a))
                            .unwrap_or_default(),
                        Def::Param => declared(),
                        Def::Augmented => ValueFacts::non_null(),
                    };
                    if nn {
                        f.non_none_part()
                    } else {
                        f
                    }
                }))
                .unwrap_or_default(),
                // Assigned on some paths only, or in a loop: unknown.
                _ => ValueFacts::default(),
            }
        } else {
            declared()
        };
        if ctx.narrowed.contains(&name) {
            facts.non_none_part()
        } else {
            facts
        }
    }

    /// Facts of the `i`-th assignment `obj.f = v` (flow name `name`).
    fn attr_assignment_facts(&self, name: &str, i: usize, a: &flow::Assignment<'a>) -> ValueFacts {
        let key = (name.to_string(), i);
        if let Some(f) = self.def_memo.borrow().get(&key) {
            return f.clone();
        }
        if !self.def_visiting.borrow_mut().insert(key.clone()) {
            return ValueFacts::default(); // cyclic: unknown
        }
        let facts = self.facts(a.value, &Ctx::new(&a.narrowed));
        self.def_visiting.borrow_mut().remove(&key);
        self.def_memo.borrow_mut().insert(key, facts.clone());
        facts
    }

    /// Facts of a module or class constant (an enum member's value).
    fn constant_facts(&self, qualified: &str) -> ValueFacts {
        let enum_class = qualified
            .rsplit_once('.')
            .and_then(|(c, _)| self.index.class(c))
            .filter(|c| c.enum_kind.is_some());
        let ctx = Ctx::default();
        match enum_class {
            Some(class) => {
                let member = qualified.rsplit('.').next().unwrap_or_default();
                let Some(value) = class.member_value(member) else {
                    return ValueFacts::default();
                };
                let f = self.facts(value, &ctx);
                match class.enum_kind {
                    // A TextChoices / IntegerChoices member is its value.
                    Some(crate::resolve::EnumKind::DjangoChoices) => f,
                    // An Enum member is not its value; its choice is.
                    _ => ValueFacts {
                        choices: f.choices,
                        ..ValueFacts::non_null()
                    },
                }
            }
            None => {
                // A class-body name that code elsewhere assigns on an object
                // (`plugin.configuration = ...`) is not a constant.
                let (owner, name) = qualified.rsplit_once('.').unwrap_or(("", qualified));
                if self.index.class(owner).is_some() && self.index.stored_attributes.contains(name) {
                    return ValueFacts::default();
                }
                match self.index.constant(qualified) {
                    Some(e) => self.facts(e, &ctx),
                    None => ValueFacts::default(),
                }
            }
        }
    }

    fn name_facts(&self, name: &str, ctx: &Ctx) -> ValueFacts {
        if ctx.shadowed.contains(name) {
            return ValueFacts::default();
        }
        // A loop variable over instances of a known class (a queryset, a
        // typed collection): an instance, not None.
        if self.flow.loop_vars.contains_key(name) && self.loop_var_class(name, ctx).is_some() {
            return ValueFacts::non_null();
        }
        // Reaching definitions are exact for names bound only by simple and
        // augmented assignments.
        let reaching = ctx.narrowed.defs(name).filter(|_| {
            (!self.flow.opaque.contains(name) || self.flow.augmented_only.contains(name))
                && !self.flow.unstable.contains(name)
        });
        let facts = if self.is_self(name) {
            ValueFacts::non_null()
        } else if let Some(defs) = reaching {
            // Flow-sensitive: the definitions that reach this point.
            ValueFacts::join_all(defs.iter().map(|&(d, non_none)| {
                let f = match d {
                    Def::Param => self
                        .param(name)
                        .map(|p| self.param_facts(p))
                        .unwrap_or_default(),
                    Def::Assign(i) => self.assignment_facts(name, i),
                    Def::Augmented => ValueFacts::non_null(),
                };
                if non_none {
                    f.non_none_part()
                } else {
                    f
                }
            }))
            .unwrap_or_default()
        } else if !self.is_local(name, ctx) {
            match self.index.lookup(self.module, name) {
                Some(Symbol::Constant(q)) => self.constant_facts(&q),
                _ => ValueFacts::default(),
            }
        } else {
            let param = self.param(name).map(|p| self.param_facts(p));
            let local = if self.flow.opaque.contains(name) {
                if self.flow.augmented_only.contains(name) {
                    // `x op= v` never leaves None: only nullability survives.
                    let simple = self
                        .flow
                        .assignments
                        .contains_key(name)
                        .then(|| self.local_facts(name));
                    Some(
                        simple
                            .into_iter()
                            .fold(ValueFacts::non_null(), ValueFacts::join),
                    )
                } else {
                    Some(ValueFacts::default())
                }
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

    /// Facts of the `i`-th simple assignment to `name`.
    fn assignment_facts(&self, name: &str, i: usize) -> ValueFacts {
        let key = (name.to_string(), i);
        if let Some(f) = self.def_memo.borrow().get(&key) {
            return f.clone();
        }
        let Some(a) = self.flow.assignments.get(name).and_then(|v| v.get(i)) else {
            return ValueFacts::default();
        };
        if !self.def_visiting.borrow_mut().insert(key.clone()) {
            return ValueFacts::default(); // cyclic: unknown
        }
        let facts = self.facts(a.value, &Ctx::new(&a.narrowed));
        self.def_visiting.borrow_mut().remove(&key);
        self.def_memo.borrow_mut().insert(key, facts.clone());
        facts
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
        // A decorator may inject the argument of a `None`-default parameter:
        // whether it is None inside the function is not known.
        let nullable = if self.injected && p.default_none { None } else { p.nullable };
        ValueFacts {
            nullable,
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
                let neg = matches!(u.op, UnaryOp::USub);
                let (min_value, max_value) = if neg {
                    (f.max_value.map(|v| -v), f.min_value.map(|v| -v))
                } else {
                    (f.min_value, f.max_value)
                };
                let choices = match (neg, &f.choices) {
                    (false, c) => c.clone(),
                    (true, Some(c)) if t == "int" => Some(
                        c.iter()
                            .map(|v| match v.strip_prefix('-') {
                                Some(p) => p.to_string(),
                                None if v == "0" => v.clone(),
                                None => format!("-{v}"),
                            })
                            .collect(),
                    ),
                    _ => None,
                };
                ValueFacts {
                    nullable: f.nullable.filter(|n| !n),
                    type_name: Some(t),
                    weak_type: f.weak_type,
                    precision: f.precision,
                    min_value,
                    max_value,
                    choices,
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
            c = c.after_operand(v, b.op);
        }
        ValueFacts::join_all(parts).unwrap_or_default()
    }

    fn subscript_facts(&self, s: &ast::ExprSubscript, ctx: &Ctx) -> ValueFacts {
        let Expr::Slice(slice) = s.slice.as_ref() else {
            return ValueFacts::default();
        };
        // Any slice `s[a:b]` (a string, list, tuple, ...) is a part of `s`: at
        // most `s`'s length, and at most `b` when `b >= 0` whatever `a` is
        // (it lies within `s[:b]`); with a positive step or none. A slice is
        // a new sequence, never None (slicing None raises).
        let base = self.facts(&s.value, ctx);
        let step_ok = match slice.step.as_deref() {
            None => true,
            Some(step) => int_literal(step).is_some_and(|v| v > 0),
        };
        // An exact non-negative integer (a literal or a constant).
        let exact = |e: &Option<Box<Expr>>| -> Option<i64> {
            let e = e.as_deref()?;
            if let Some(v) = int_literal(e) {
                return (v >= 0).then_some(v);
            }
            let f = self.facts(e, ctx);
            if f.type_name.as_deref() != Some("int") {
                return None;
            }
            match (f.min_value, f.max_value) {
                (Some(lo), Some(hi)) if lo >= Dec::ZERO => hi.to_i64(),
                _ => None,
            }
        };
        let upper = exact(&slice.upper).filter(|_| step_ok);
        let bound = match (upper, int_literal_opt(&slice.lower)) {
            (Some(u), Some(l)) if l >= 0 => Some((u - l).max(0)),
            (Some(u), _) => Some(u),
            _ => None,
        };
        let max_length = match (bound, base.max_length) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let type_name = base
            .type_name
            .filter(|t| matches!(t.as_str(), "str"));
        ValueFacts {
            max_length,
            type_name,
            ..ValueFacts::non_null()
        }
    }

    fn binop_facts(&self, b: &ast::ExprBinOp, ctx: &Ctx) -> ValueFacts {
        let l = self.facts(&b.left, ctx);
        let r = self.facts(&b.right, ctx);
        let result_type = binop_type(b.op, l.type_name.as_deref(), r.type_name.as_deref());
        // An operator's result is a value (None operands raise instead).
        let nullable = Some(false);
        let precision = match result_type {
            Some("int") => Some(Dep::Lit(0)),
            // Binary floating point: a sum of 2-place values need not have 2 places.
            Some("str" | "float") => None,
            _ => match (b.op, l.precision.clone(), r.precision.clone()) {
                // Decimal sum / difference: the exponent is the smaller one,
                // so the places are those of the wider operand.
                (Operator::Add | Operator::Sub, Some(x), Some(y)) => Some(Dep::max(x, y)),
                (Operator::Mult, Some(x), Some(y)) => Some(Dep::sum(x, y)),
                // Division (and anything else) has no decimal-place bound.
                _ => None,
            },
        };
        // `"ab" * n` with a literal count.
        let repeat = |s: &ValueFacts, count: &Expr| {
            let n = int_literal(count)?.max(0);
            s.max_length?.checked_mul(n)
        };
        let max_length = match (result_type, b.op, l.max_length, r.max_length) {
            (Some("str"), Operator::Add, Some(x), Some(y)) => Some(x.saturating_add(y)),
            (Some("str"), Operator::Mult, _, _) if l.type_name.as_deref() == Some("str") => {
                repeat(&l, &b.right)
            }
            (Some("str"), Operator::Mult, _, _) => repeat(&r, &b.left),
            _ => None,
        };
        // Interval arithmetic on exact (int, Decimal) sums and differences:
        // `len(x) - 1 >= -1`, `a + b >= 0` for non-negative `a`, `b`.
        // Also for operands of unknown type with known bounds (a numeric
        // field read): numbers, not strings (a string has no range).
        let exact = matches!(result_type, Some("int" | "Decimal"))
            || (result_type.is_none()
                && [&l, &r].iter().all(|f| {
                    f.type_name.as_deref().is_none_or(|t| matches!(t, "int" | "Decimal"))
                        && (f.min_value.is_some() || f.max_value.is_some())
                }));
        let add = |a: Option<Dec>, b: Option<Dec>| a?.checked_add(b?);
        let (min_value, max_value) = match b.op {
            Operator::Add if exact => (add(l.min_value, r.min_value), add(l.max_value, r.max_value)),
            Operator::Sub if exact => (
                add(l.min_value, r.max_value.map(|v| -v)),
                add(l.max_value, r.min_value.map(|v| -v)),
            ),
            _ => (None, None),
        };
        ValueFacts {
            nullable,
            weak_type: result_type.is_some() && l.weak_type && r.weak_type,
            type_name: result_type.map(str::to_string),
            precision,
            max_length,
            min_value,
            max_value,
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
                            min_value: (name == "len").then_some(Dec::ZERO),
                            ..ValueFacts::typed("int")
                        }
                    }
                    "abs" if args.len() == 1 => {
                        let f = self.facts(&args[0], ctx);
                        return ValueFacts {
                            type_name: f.numeric_type().map(str::to_string),
                            weak_type: f.weak_type,
                            precision: f.precision,
                            min_value: Some(Dec::ZERO),
                            nullable: f.nullable.filter(|n| !n),
                            ..Default::default()
                        };
                    }
                    "float" => return ValueFacts::typed("float"),
                    "bool" => return ValueFacts::typed("bool"),
                    "sum" => return self.sum_facts(call, ctx),
                    "min" | "max" => return self.min_max_facts(call, ctx),
                    "list" | "tuple" | "dict" | "set" | "frozenset" | "sorted" | "reversed"
                    | "enumerate" | "zip" | "range" | "map" | "filter" | "iter" | "F"
                    | "Q" | "Value" | "Sum" | "Count" => return ValueFacts::non_null(),
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
        // `Model.objects.<...>.<method>(...)`: an instance (`get`, `create`,
        // `latest`: they raise rather than return None), a queryset, a
        // tuple, a count; `first()` / `last()` below are nullable.
        if let Some((_, method)) = self.objects_call(call, ctx) {
            let m = method.strip_prefix('a').filter(|m| {
                matches!(*m, "get" | "create" | "get_or_create" | "update_or_create" | "latest" | "earliest"
                    | "update" | "count" | "exists" | "bulk_create" | "bulk_update" | "first" | "last")
            }).unwrap_or(method);
            match m {
                "get" | "create" | "latest" | "earliest" | "get_or_create" | "update_or_create"
                | "bulk_create" | "in_bulk" | "values" | "values_list" | "iterator" | "aggregate" => {
                    return ValueFacts::non_null()
                }
                "update" | "count" | "bulk_update" => {
                    return ValueFacts {
                        precision: Some(Dep::Lit(0)),
                        min_value: Some(Dec::ZERO),
                        ..ValueFacts::typed("int")
                    }
                }
                "exists" => return ValueFacts::typed("bool"),
                m if QUERYSET_METHODS.contains(&m) => return ValueFacts::non_null(),
                _ => {}
            }
        }
        // pydantic constructors and copies: `Cls.model_validate(...)`,
        // `obj.model_copy(...)` return an instance (or raise).
        if let Expr::Attribute(a) = func {
            let pydantic = |c: &ClassInfo| {
                c.kind == ClassKind::Data(crate::dataclass_extractor::DataClassKind::Pydantic)
            };
            let constructor = matches!(
                a.attr.as_str(),
                "model_validate" | "model_validate_json" | "model_validate_strings" | "model_construct"
                    | "parse_obj" | "parse_raw" | "construct"
            );
            let copy = matches!(a.attr.as_str(), "model_copy" | "copy");
            match self.receiver_class(&a.value, ctx) {
                Some((c, true)) if copy && pydantic(c) => return ValueFacts::non_null(),
                Some((c, false)) if constructor && pydantic(c) => return ValueFacts::non_null(),
                _ => {}
            }
            if constructor {
                let class = dotted_parts(&a.value)
                    .filter(|p| !self.is_local(&p[0], ctx))
                    .and_then(|p| self.index.resolve_dotted(self.module, &p));
                if let Some(Symbol::Class(q)) = class {
                    if self.index.class(&q).is_some_and(pydantic) {
                        return ValueFacts::non_null();
                    }
                }
            }
        }
        let callees = self.callees(func, ctx);
        match callees.as_slice() {
            [Callee::Class(_)] => return ValueFacts::non_null(),
            [Callee::Unknown] => {}
            targets => {
                // The join over every target the call may dispatch to.
                let facts = targets.iter().map(|t| match t {
                    Callee::Function { qualified, .. } => {
                        let mut f = self.summaries.get(qualified).cloned().unwrap_or_default();
                        // A bound over the callee's own input means nothing here.
                        if f.precision
                            .as_ref()
                            .is_some_and(|p| p.as_static().is_none())
                        {
                            f.precision = None;
                        }
                        f
                    }
                    _ => ValueFacts::default(),
                });
                if let Some(f) = ValueFacts::join_all(facts) {
                    return f;
                }
            }
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
                // Django query expressions (`models.F("n") + 1`).
                [.., "F" | "Q" | "Value"] if external(&parts[0]) => return ValueFacts::non_null(),
                _ => {}
            }
        }
        let kw_is_none = |name: &str| {
            call.arguments
                .keywords
                .iter()
                .any(|k| k.arg.as_deref() == Some(name) && matches!(k.value, Expr::NoneLiteral(_)))
        };
        // `k in d` holds here and `d` is a dict: `d.get(k)` is `d[k]`, never
        // the default. Another object's `.get` may do anything.
        let checked = method == "get"
            && args
                .first()
                .and_then(|k| flow::member_key_of(k, &attr.value))
                .is_some_and(|m| ctx.narrowed.contains(&m));
        let present = checked && self.known_dict(&attr.value, ctx);
        // `.get(k, default)` on a module-level dict of non-None literals: one
        // of its values or the default (`None` when not given).
        if method == "get" && matches!(args.len(), 1 | 2) && call.arguments.keywords.is_empty() {
            let dict = dotted_parts(&attr.value)
                .filter(|p| !self.is_local(&p[0], ctx))
                .and_then(|p| self.index.const_dict(self.module, &p));
            if let Some(Expr::Dict(d)) = dict {
                let values = d.items.iter().map(|item| self.facts(&item.value, &Ctx::default()));
                // A module-level dict display: a dict.
                let default = (!checked)
                    .then(|| args.get(1).map_or_else(ValueFacts::none_value, |d| self.facts(d, ctx)));
                return ValueFacts::join_all(values.chain(default)).unwrap_or_default();
            }
        }
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
                nullable_unless(present)
            }
            "get" if args.len() == 2 && matches!(args[1], Expr::NoneLiteral(_)) => {
                nullable_unless(present)
            }
            "get" if args.len() == 1 && kw_is_none("default") => nullable_unless(present),
            "pop" if args.len() == 2 && matches!(args[1], Expr::NoneLiteral(_)) => {
                ValueFacts::nullable()
            }
            "match" | "search" | "fullmatch" if self.is_pattern(&attr.value, ctx) => {
                ValueFacts::nullable()
            }
            "first" | "last" | "afirst" | "alast"
                if args.is_empty() && call.arguments.keywords.is_empty() =>
            {
                ValueFacts::nullable()
            }
            m if STR_METHODS.contains(&m) => {
                let base = self.facts(&attr.value, ctx);
                if base.type_name.as_deref() != Some("str") {
                    // A string method on a value of unknown type: a string in
                    // practice, so not None; nothing else is known.
                    return ValueFacts::non_null();
                }
                // Stripping never lengthens a string. Case mapping of a literal
                // keeps its length and maps its possible values (`"ok".upper()`);
                // of other strings it can lengthen them (`"ß".upper()` is `"SS"`).
                let keeps_length = matches!(m, "strip" | "lstrip" | "rstrip");
                let case_map = matches!(m, "upper" | "lower" | "title" | "capitalize" | "swapcase")
                    && args.is_empty()
                    && call.arguments.keywords.is_empty();
                let mapped = case_map
                    .then(|| base.choices.as_ref().and_then(|c| case_mapped(m, c)))
                    .flatten();
                match mapped {
                    Some(values) => ValueFacts {
                        max_length: values.iter().map(|v| v.chars().count() as i64).max(),
                        choices: Some(values),
                        ..ValueFacts::typed("str")
                    },
                    None => ValueFacts {
                        max_length: base.max_length.filter(|_| keeps_length && args.len() <= 1),
                        ..ValueFacts::typed("str")
                    },
                }
            }
            "readline" | "read" | "decode" | "encode" | "hexdigest" | "isoformat" => {
                ValueFacts::non_null()
            }
            _ => ValueFacts::default(),
        }
    }

    /// Whether `expr` is a compiled regular expression: `re.compile(...)`, a
    /// local bound to one, a module-level or class-body name bound once to one.
    fn is_pattern(&self, expr: &Expr, ctx: &Ctx) -> bool {
        if crate::resolve::is_re_compile(expr) {
            return true;
        }
        match expr {
            Expr::Name(n) => {
                let name = n.id.as_str();
                if self.is_local(name, ctx) {
                    return self
                        .single_def(name, ctx)
                        .is_some_and(|a| crate::resolve::is_re_compile(a.value));
                }
                self.index.is_pattern(self.module, name)
            }
            Expr::Attribute(a) => {
                let attr = a.attr.as_str();
                match a.value.as_ref() {
                    // (Every subclass overriding it binds a pattern too.)
                    Expr::Name(n) if self.is_self(n.id.as_str()) => self.class.is_some_and(|c| {
                        c.patterns.contains(attr)
                            && !c.stored_attrs.contains(attr)
                            && self.index.all_subclasses(&c.qualified).is_some_and(|subs| {
                                subs.iter().all(|s| {
                                    self.index.class(s).is_none_or(|sc| {
                                        !sc.stored_attrs.contains(attr)
                                            && (!sc.body_names.contains(attr) || sc.patterns.contains(attr))
                                    })
                                })
                            })
                    }),
                    other => match dotted_parts(other)
                        .filter(|p| !self.is_local(&p[0], ctx))
                        .and_then(|p| self.index.resolve_dotted(self.module, &p))
                    {
                        Some(Symbol::Class(q)) => {
                            self.index.class(&q).is_some_and(|c| c.patterns.contains(attr))
                        }
                        Some(Symbol::Module(m)) => self
                            .index
                            .find_module(&m, self.module)
                            .is_some_and(|found| self.index.patterns.contains(&qualify(found, attr))),
                        _ => false,
                    },
                }
            }
            _ => false,
        }
    }

    /// `sum(xs)`: never None; for a generator, list or set comprehension, or
    /// a list or tuple display of
    /// Decimal elements (or of ints), the elements' precision. A start value
    /// joins in.
    fn sum_facts(&self, call: &ast::ExprCall, ctx: &Ctx) -> ValueFacts {
        let args = &call.arguments.args;
        let mut out = ValueFacts::non_null();
        let Some(first) = args.first() else { return out };
        let elements = match first {
            Expr::Generator(g) => Some(self.facts(&g.elt, &self.comprehension_ctx(first, ctx))),
            Expr::ListComp(g) => Some(self.facts(&g.elt, &self.comprehension_ctx(first, ctx))),
            Expr::SetComp(g) => Some(self.facts(&g.elt, &self.comprehension_ctx(first, ctx))),
            Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })
                if !elts.is_empty() && !elts.iter().any(|e| matches!(e, Expr::Starred(_))) =>
            {
                ValueFacts::join_all(elts.iter().map(|e| self.facts(e, ctx)))
            }
            _ => None,
        };
        let start = match (args.get(1), call.arguments.keywords.iter().find(|k| k.arg.as_deref() == Some("start"))) {
            (Some(s), _) => Some(self.facts(s, ctx)),
            (None, Some(k)) => Some(self.facts(&k.value, ctx)),
            // The implicit start is the int 0.
            (None, None) => None,
        };
        if let Some(e) = elements {
            let t = e.numeric_type().map(str::to_string);
            let start_ok = start.as_ref().is_none_or(|s| {
                matches!(s.numeric_type(), Some("int" | "Decimal")) && s.precision.is_some()
            });
            if matches!(t.as_deref(), Some("Decimal" | "int")) && start_ok {
                out.type_name = t;
                out.precision = match (e.precision, start.and_then(|s| s.precision)) {
                    (Some(p), Some(q)) => Some(Dep::max(p, q)),
                    (p, _) => p,
                }
                .filter(|p| p.as_static().is_some());
            }
        }
        out
    }

    /// `min(a, b, ...)` / `max(a, b, ...)`: one of the arguments (their
    /// join, never None); `min(xs)`: an element of `xs`.
    fn min_max_facts(&self, call: &ast::ExprCall, ctx: &Ctx) -> ValueFacts {
        let args = &call.arguments.args;
        let default_none = call.arguments.keywords.iter().any(|k| {
            k.arg.as_deref() == Some("default") && matches!(k.value, Expr::NoneLiteral(_))
        });
        if default_none {
            return ValueFacts::nullable();
        }
        let has_key = call.arguments.keywords.iter().any(|k| k.arg.as_deref() == Some("key"));
        if args.len() >= 2 && !has_key && !args.iter().any(|a| matches!(a, Expr::Starred(_))) {
            let facts: Vec<ValueFacts> = args.iter().map(|a| self.facts(a, ctx)).collect();
            let is_max = matches!(call.func.as_ref(), Expr::Name(n) if n.id.as_str() == "max");
            // `max` is at least every argument's lower bound, `min` at most
            // every argument's upper bound (known ones suffice).
            let tight = |pick: fn(&ValueFacts) -> Option<Dec>, larger: bool| {
                facts.iter().filter_map(pick).reduce(|a, b| if (a > b) == larger { a } else { b })
            };
            let (min_value, max_value) = if is_max {
                (tight(|f| f.min_value, true), None)
            } else {
                (None, tight(|f| f.max_value, false))
            };
            let joined = ValueFacts::join_all(facts.iter().cloned()).unwrap_or_default();
            // A None argument would raise (it does not compare).
            return ValueFacts {
                nullable: Some(false),
                min_value: min_value.or(joined.min_value),
                max_value: max_value.or(joined.max_value),
                ..joined
            };
        }
        // `max(<generator>)` / `max([a, b])`: one of the elements (an empty
        // iterable raises, unless a `default=` is given, which joins in).
        if let [only] = &args[..] {
            // Two or more literal elements are compared with each other, so
            // a None among them raises; one element (or a comprehension or
            // other iterable, which may yield just one) is returned as is:
            // `max([None])` is None.
            let compared = matches!(only,
                Expr::List(ast::ExprList { elts, .. })
                | Expr::Tuple(ast::ExprTuple { elts, .. })
                | Expr::Set(ast::ExprSet { elts, .. }) if elts.len() >= 2);
            let elements = match only {
                Expr::Generator(ast::ExprGenerator { elt, .. })
                | Expr::ListComp(ast::ExprListComp { elt, .. })
                | Expr::SetComp(ast::ExprSetComp { elt, .. }) => {
                    Some(self.facts(elt, &self.comprehension_ctx(only, ctx)))
                }
                Expr::List(ast::ExprList { elts, .. })
                | Expr::Tuple(ast::ExprTuple { elts, .. })
                | Expr::Set(ast::ExprSet { elts, .. })
                    if !elts.iter().any(|e| matches!(e, Expr::Starred(_))) =>
                {
                    ValueFacts::join_all(elts.iter().map(|e| self.facts(e, ctx)))
                }
                _ => None,
            };
            if let Some(e) = elements {
                let default = call
                    .arguments
                    .keywords
                    .iter()
                    .find(|k| k.arg.as_deref() == Some("default"))
                    .map(|k| self.facts(&k.value, ctx));
                let mut f = match default {
                    Some(d) => ValueFacts::join(e, d),
                    None => e,
                };
                if f.nullable.is_none() && compared {
                    f.nullable = Some(false);
                }
                return f;
            }
        }
        // `max(xs)`: an element of `xs`, which may be None when `xs` has
        // exactly one element, unless its elements cannot be None.
        match args.first().and_then(|xs| self.element_nullable(xs, ctx)) {
            Some(false) => ValueFacts::non_null(),
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
        // A rounded float is still a binary float (`round(0.1234, 2)` is
        // 0.11999...), so it has no decimal-place bound.
        let is_float = value_type.as_deref() == Some("float");
        ValueFacts {
            precision: int_literal(&args[1])
                .filter(|_| !is_float)
                .map(|n| Dep::Lit(n.max(0))),
            type_name: value_type,
            ..ValueFacts::non_null()
        }
    }

    /// Resolve the target of a call.
    pub fn callee(&self, func: &Expr, ctx: &Ctx) -> Callee<'a> {
        match func {
            Expr::Name(n) => {
                if let Some(class) = self.cls_class(n.id.as_str(), ctx) {
                    return Callee::Class(class); // `cls(...)` in a classmethod
                }
                if self.is_local(n.id.as_str(), ctx) {
                    return Callee::Unknown;
                }
                self.symbol_callee(self.index.lookup(self.module, n.id.as_str()))
            }
            Expr::Attribute(attr) => {
                if self.is_super_call(&attr.value, ctx) {
                    let q = self
                        .class
                        .and_then(|c| self.index.super_method(&c.qualified, attr.attr.as_str()));
                    let instance = self.func.is_some_and(|f| f.method_kind == MethodKind::Instance);
                    return q.map_or(Callee::Unknown, |q| self.method_callee(q, instance));
                }
                if let Some((class, instance)) = self.receiver_class(&attr.value, ctx) {
                    return match self.index.method(&class.qualified, attr.attr.as_str()) {
                        Some(q) => self.method_callee(q, instance),
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

    /// Every target a call may run: for a method call on an instance (or
    /// class) of a known class, the method as resolved from that class and
    /// from each project subclass that overrides it (class hierarchy
    /// analysis; `self.m()` in an inherited method included); on a freshly
    /// constructed object (`Cls().m()`) only `Cls`'s method; `super().m()`
    /// the parent's method. Otherwise the one target of `callee`.
    /// `[Callee::Unknown]` when the targets are not known.
    pub fn callees(&self, func: &Expr, ctx: &Ctx) -> Vec<Callee<'a>> {
        if let Expr::Attribute(attr) = func {
            let name = attr.attr.as_str();
            if self.is_super_call(&attr.value, ctx) {
                let Some(class) = self.class else { return vec![Callee::Unknown] };
                let Some(q) = self.index.super_method(&class.qualified, name) else {
                    return vec![Callee::Unknown];
                };
                let instance = self.func.is_some_and(|f| f.method_kind == MethodKind::Instance);
                return vec![self.method_callee(q, instance)];
            }
            if let Some((class, instance)) = self.receiver_class(&attr.value, ctx) {
                // A freshly constructed object (`Cls(...)`, `Model.objects.create(...)`)
                // is exactly of its class; a function's result may be a subclass.
                let exact = match strip_await(&attr.value) {
                    Expr::Call(c) => {
                        self.objects_call(c, ctx).is_some()
                            || matches!(self.callee(&c.func, ctx), Callee::Class(_))
                    }
                    _ => false,
                };
                let targets = if exact {
                    Some(self.index.method(&class.qualified, name).into_iter().collect())
                } else {
                    self.index.dispatch_targets(&class.qualified, name)
                };
                return match targets {
                    Some(ts) if !ts.is_empty() => {
                        ts.into_iter().map(|q| self.method_callee(q, instance)).collect()
                    }
                    _ => vec![Callee::Unknown],
                };
            }
        }
        vec![self.callee(func, ctx)]
    }

    /// `method` called through an instance (`instance`) or through the class.
    fn method_callee(&self, qualified: String, instance: bool) -> Callee<'a> {
        let implicit = self.index.function(&qualified).map_or(0, |f| {
            if instance || f.method_kind == MethodKind::ClassMethod {
                f.implicit_params()
            } else {
                0
            }
        });
        Callee::Function {
            qualified,
            implicit,
        }
    }

    /// `super()` / `super(Cls, self)` inside a method.
    fn is_super_call(&self, expr: &Expr, ctx: &Ctx) -> bool {
        matches!(expr, Expr::Call(c)
            if matches!(c.func.as_ref(), Expr::Name(n) if n.id.as_str() == "super" && self.is_external("super", ctx)))
    }

    /// The enclosing class, when `name` is the `cls` parameter of a classmethod.
    fn cls_class(&self, name: &str, ctx: &Ctx) -> Option<&'a ClassInfo> {
        (self.is_self(name)
            && !ctx.shadowed.contains(name)
            && self.func?.method_kind == MethodKind::ClassMethod)
            .then_some(self.class)
            .flatten()
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
        if let Expr::Call(_) = strip_await(expr) {
            // `Cls(...).m()`, `Model.objects.get(...).m()`, `make().m()`
            let _guard = self.enter()?;
            return self.class_of_value(expr, ctx).map(|c| (c, true));
        }
        let Expr::Name(n) = expr else { return None };
        let name = n.id.as_str();
        if let Some(q) = ctx.elements.get(name) {
            return self.index.class(q).map(|c| (c, true));
        }
        if ctx.shadowed.contains(name) {
            return None;
        }
        if self.is_self(name) {
            let instance = self.func?.method_kind == MethodKind::Instance;
            return self.class.map(|c| (c, instance));
        }
        if self.flow.loop_vars.contains_key(name) {
            return self.loop_var_class(name, ctx).map(|c| (c, true));
        }
        if self.flow.opaque.contains(name) {
            return None;
        }
        let _guard = self.enter()?;
        // `v = v.method()`: resolving `v` needs `v` itself; unknown.
        if !self.receiving.borrow_mut().insert(name.to_string()) {
            return None;
        }
        let result = self.receiver_class_of_local(name);
        self.receiving.borrow_mut().remove(name);
        result
    }

    /// The class of loop variable `name` (bound only by `for name in ...`),
    /// when every loop iterates over instances of one known class.
    fn loop_var_class(&self, name: &str, ctx: &Ctx) -> Option<&'a ClassInfo> {
        let iters = self.flow.loop_vars.get(name)?;
        if ctx.shadowed.contains(name) || self.param(name).is_some() || self.flow.unstable.contains(name) {
            return None;
        }
        let _guard = self.enter()?;
        let first = self.element_class(iters.first()?, ctx)?;
        iters
            .iter()
            .all(|it| self.element_class(it, ctx).is_some_and(|c| c.qualified == first.qualified))
            .then_some(first)
    }

    /// The class of the elements an iterable yields, when known: a queryset
    /// (`Model.objects.filter(...)`, `.all()`, `.iterator()`, or a local bound
    /// to one), a parameter annotated as a collection of a class
    /// (`list[Model]`, `QuerySet[Model]`, `Iterable[Model]`), or a local the
    /// function passes to `Model.objects.bulk_update(xs, ...)` /
    /// `bulk_create(xs)` (its elements are instances of the model).
    fn element_class(&self, iter: &Expr, ctx: &Ctx) -> Option<&'a ClassInfo> {
        match iter {
            Expr::Call(c) => {
                let (class, method) = self.objects_call(c, ctx)?;
                (QUERYSET_METHODS.contains(&method) || method == "iterator").then_some(class)
            }
            Expr::Name(n) => {
                let name = n.id.as_str();
                if let Some(p) = self.param(name) {
                    if let Some(c) = p.annotation.as_ref().and_then(|a| self.collection_class(a)) {
                        return Some(c);
                    }
                } else if let Some(a) = self.single_def(name, ctx) {
                    if let Expr::Call(c) = a.value {
                        if let Some((class, method)) = self.objects_call(c, &Ctx::new(&a.narrowed)) {
                            if QUERYSET_METHODS.contains(&method) {
                                return Some(class);
                            }
                        }
                    }
                }
                self.bulk_written_class(name)
            }
            _ => None,
        }
    }

    /// Whether `expr` is known to hold a dict: a parameter annotated `dict`,
    /// `Dict`, `Mapping` (or a subclass of dict: `defaultdict`, `OrderedDict`)
    /// that the body does not rebind, or a local every assignment of which is
    /// a dict display, a dict comprehension or a `dict()` / `defaultdict()` /
    /// `OrderedDict()` construction.
    fn known_dict(&self, expr: &Expr, ctx: &Ctx) -> bool {
        const DICTS: [&str; 7] = ["dict", "Dict", "Mapping", "MutableMapping", "defaultdict", "DefaultDict", "OrderedDict"];
        let Expr::Name(n) = expr else { return false };
        let name = n.id.as_str();
        if ctx.shadowed.contains(name) || self.flow.unstable.contains(name) || self.flow.opaque.contains(name) {
            return false;
        }
        let head_is_dict = |e: &Expr| {
            let e = match e {
                Expr::Subscript(s) => s.value.as_ref(),
                other => other,
            };
            dotted_parts(e).and_then(|p| p.last().cloned()).is_some_and(|h| DICTS.contains(&h.as_str()))
        };
        if let Some(p) = self.param(name) {
            return !self.flow.binds(name) && p.annotation.as_ref().is_some_and(|a| head_is_dict(strip_optional(a)));
        }
        let constructed = |e: &Expr| match e {
            Expr::Dict(_) | Expr::DictComp(_) => true,
            Expr::Call(c) => {
                head_is_dict(&c.func)
                    && dotted_parts(&c.func)
                        .and_then(|p| p.first().cloned())
                        .is_some_and(|root| self.is_external(&root, ctx))
            }
            _ => false,
        };
        self.flow
            .assignments
            .get(name)
            .is_some_and(|assigns| !assigns.is_empty() && assigns.iter().all(|a| constructed(a.value)))
    }

    /// Where the element expression of comprehension `comp` is evaluated:
    /// its variables shadow the function's names; a variable bound by
    /// `for x in xs` over a collection of a project class (see
    /// `element_class`: `xs: list[Line]`, a queryset) is an element of it.
    pub fn comprehension_ctx(&self, comp: &Expr, ctx: &Ctx) -> Ctx {
        let names = flow::bound_names_in_expr(comp);
        let mut c = ctx.shadow(names.iter().cloned());
        // Every part may run before the element: a filter in this
        // iteration, and every part in an earlier one.
        c.narrowed.apply(&flow::effects(comp));
        let generators = match comp {
            Expr::ListComp(g) => &g.generators[..],
            Expr::SetComp(g) => &g.generators[..],
            Expr::Generator(g) => &g.generators[..],
            Expr::DictComp(g) => &g.generators[..],
            _ => return c,
        };
        for g in generators {
            let Expr::Name(target) = &g.target else { continue };
            // An iterable naming a comprehension variable is not the function's.
            let mentions_bound = flow::names_in_expr(&g.iter).iter().any(|n| names.contains(n));
            if mentions_bound {
                continue;
            }
            if let Some(class) = self.element_class(&g.iter, ctx) {
                c.elements.insert(target.id.to_string(), class.qualified.clone());
            }
        }
        c
    }

    /// `list[C]`, `QuerySet[C]`, `Iterable[C]`, `tuple[C, ...]` ... (also
    /// `Optional[...]`): `C`.
    fn collection_class(&self, annotation: &Expr) -> Option<&'a ClassInfo> {
        self.annotation_class(self.module, collection_element(annotation)?)
    }

    /// Whether the elements iterable `iter` yields may be None, when known.
    /// A parameter with a collection annotation says so by its element
    /// annotation (`list[Decimal]`: no; `list[Optional[Line]]`: yes), and only
    /// that is consulted, since `element_class` strips `Optional` from the
    /// element type. The annotation describes the parameter only while the
    /// body does not rebind it (`xs = [None]`): a rebound parameter is unknown.
    /// Otherwise the elements of a queryset or of a bulk-written name
    /// (`element_class`) are model instances, never None.
    fn element_nullable(&self, iter: &Expr, ctx: &Ctx) -> Option<bool> {
        if let Expr::Name(n) = iter {
            let name = n.id.as_str();
            if let Some(p) = self.param(name) {
                if ctx.shadowed.contains(name) || self.flow.binds(name) || self.flow.unstable.contains(name) {
                    return None;
                }
                if let Some(element) = p.annotation.as_ref().and_then(|a| collection_element(a)) {
                    return crate::function_extractor::annotation_nullability(element, &[]);
                }
            }
        }
        self.element_class(iter, ctx).map(|_| false)
    }

    /// The model whose `objects.bulk_update(name, ...)` / `bulk_create(name)`
    /// (or async form) this function calls with local `name`.
    fn bulk_written_class(&self, name: &str) -> Option<&'a ClassInfo> {
        struct Bulk<'n, 'b>(&'n str, Vec<&'b ast::ExprCall>);
        impl<'b> visitor::Visitor<'b> for Bulk<'_, 'b> {
            fn visit_expr(&mut self, expr: &'b Expr) {
                if let Expr::Call(c) = expr {
                    let bulk = matches!(c.func.as_ref(), Expr::Attribute(a)
                        if matches!(a.attr.as_str(), "bulk_update" | "abulk_update" | "bulk_create" | "abulk_create"));
                    if bulk && matches!(c.arguments.args.first(), Some(Expr::Name(n)) if n.id.as_str() == self.0) {
                        self.1.push(c);
                    }
                }
                visitor::walk_expr(self, expr);
            }
            fn visit_stmt(&mut self, stmt: &'b ruff_python_ast::Stmt) {
                if !matches!(stmt, ruff_python_ast::Stmt::FunctionDef(_) | ruff_python_ast::Stmt::ClassDef(_)) {
                    visitor::walk_stmt(self, stmt);
                }
            }
        }
        let func = self.func?;
        let mut b = Bulk(name, Vec::new());
        for s in &func.body {
            visitor::Visitor::visit_stmt(&mut b, s);
        }
        let classes: Vec<&'a ClassInfo> = b
            .1
            .iter()
            .filter_map(|c| self.objects_call(c, &Ctx::default()).map(|(class, _)| class))
            .collect();
        let first = *classes.first()?;
        classes.iter().all(|c| c.qualified == first.qualified).then_some(first)
    }

    fn receiver_class_of_local(&self, name: &str) -> Option<(&'a ClassInfo, bool)> {
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
        let Expr::Call(call) = strip_await(expr) else { return None };
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
        if let [name] = parts.as_slice() {
            if let Some(class) = self.cls_class(name, ctx) {
                // `cls.objects.create(...)` in a classmethod
                return (class.kind == ClassKind::Django).then_some((class, attr.attr.as_str()));
            }
        }
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

    /// The extracted function whose result `expr` is, with the call that
    /// produces it: a direct call, or a local whose one reaching definition
    /// is such a call.
    pub fn producer_of<'e>(&self, expr: &'e Expr, ctx: &Ctx) -> Option<(String, &'e ast::ExprCall)>
    where
        'a: 'e,
    {
        self.producers_of(expr, ctx).into_iter().next()
    }

    /// The extracted functions whose result `expr` is (every dispatch target
    /// of the call), with the call: a direct call, or a local whose one
    /// reaching definition is such a call. Empty when `expr` is not such a
    /// result (or a target is unknown).
    pub fn producers_of<'e>(&self, expr: &'e Expr, ctx: &Ctx) -> Vec<(String, &'e ast::ExprCall)>
    where
        'a: 'e,
    {
        let (call, c) = match expr {
            Expr::Await(a) => return self.producers_of(&a.value, ctx),
            Expr::Call(c) => (c, ctx.clone()),
            Expr::Name(n) => {
                let Some(a) = self.single_def(n.id.as_str(), ctx) else { return Vec::new() };
                match strip_await(a.value) {
                    Expr::Call(c) => (c, Ctx::new(&a.narrowed)),
                    _ => return Vec::new(),
                }
            }
            _ => return Vec::new(),
        };
        let targets = self.callees(&call.func, &c);
        if !targets.iter().all(|t| matches!(t, Callee::Function { .. })) {
            return Vec::new();
        }
        targets
            .into_iter()
            .filter_map(|t| match t {
                Callee::Function { qualified, .. } => Some((qualified, call)),
                _ => None,
            })
            .collect()
    }

    /// The possible values of `expr` as separate alternatives (their union is
    /// every value `expr` can have): the branches of `a if c else b` and the
    /// reaching definitions of a local, expanded recursively. More than
    /// `MAX_ALTERNATIVES` give the single alternative `expr` itself (a join).
    pub fn alternatives<'e>(&self, expr: &'e Expr, ctx: &Ctx) -> Vec<Alt<'e>>
    where
        'a: 'e,
    {
        let mut out = Vec::new();
        if self.expand(expr, ctx, false, 0, &mut out) && out.len() <= MAX_ALTERNATIVES {
            out
        } else {
            vec![Alt::Expr(expr, ctx.clone(), false)]
        }
    }

    /// Append the alternatives of `expr`; false when there are too many.
    fn expand<'e>(&self, expr: &'e Expr, ctx: &Ctx, non_none: bool, depth: usize, out: &mut Vec<Alt<'e>>) -> bool
    where
        'a: 'e,
    {
        if out.len() > MAX_ALTERNATIVES {
            return false;
        }
        if depth > 4 {
            out.push(Alt::Expr(expr, ctx.clone(), non_none));
            return true;
        }
        match expr {
            Expr::If(i) => {
                self.expand(&i.body, &ctx.branch(&i.test, true), non_none, depth + 1, out)
                    && self.expand(&i.orelse, &ctx.branch(&i.test, false), non_none, depth + 1, out)
            }
            Expr::Name(n) => {
                let name = n.id.as_str();
                let reaching = ctx.narrowed.defs(name).filter(|d| {
                    !d.is_empty()
                        && !ctx.shadowed.contains(name)
                        && !self.is_self(name)
                        && !self.flow.opaque.contains(name)
                        && !self.flow.unstable.contains(name)
                });
                let Some(defs) = reaching else {
                    out.push(Alt::Expr(expr, ctx.clone(), non_none));
                    return true;
                };
                let here = non_none || (ctx.narrowed.contains(name) && !self.flow.unstable.contains(name));
                for &(d, nn) in defs {
                    let nn = here || nn;
                    match d {
                        Def::Assign(i) => {
                            let Some(a) = self.flow.assignments.get(name).and_then(|v| v.get(i)) else {
                                out.push(Alt::Facts(ValueFacts::default()));
                                continue;
                            };
                            if !self.expand(a.value, &Ctx::new(&a.narrowed), nn, depth + 1, out) {
                                return false;
                            }
                        }
                        Def::Param => {
                            let f = self.param(name).map(|p| self.param_facts(p)).unwrap_or_default();
                            out.push(Alt::Facts(if nn { f.non_none_part() } else { f }));
                        }
                        Def::Augmented => out.push(Alt::Facts(ValueFacts::non_null())),
                    }
                }
                true
            }
            // `self.CODE` overridden in project subclasses: one alternative per value.
            Expr::Attribute(_) if self.self_constant_alternatives(expr, ctx).is_some() => {
                for f in self.self_constant_alternatives(expr, ctx).unwrap_or_default() {
                    out.push(Alt::Facts(if non_none { f.non_none_part() } else { f }));
                }
                true
            }
            _ => {
                out.push(Alt::Expr(expr, ctx.clone(), non_none));
                true
            }
        }
    }

    /// The one assignment whose value a local holds here: the only reaching
    /// definition, or (definitions unknown) the local's only assignment.
    fn single_def(&self, name: &str, ctx: &Ctx) -> Option<&flow::Assignment<'a>> {
        if ctx.shadowed.contains(name)
            || self.is_self(name)
            || self.flow.opaque.contains(name)
            || self.flow.unstable.contains(name)
        {
            return None;
        }
        let assigns = self.flow.assignments.get(name)?;
        match ctx.narrowed.defs(name) {
            Some([(Def::Assign(i), _)]) => assigns.get(*i),
            Some(_) => None,
            None if self.param(name).is_some() => None,
            None => match assigns.as_slice() {
                [a] => Some(a),
                _ => None,
            },
        }
    }

    /// The string-keyed entries of a dict literal, or of a local bound once to
    /// one and never mutated or passed on, with the context of each value.
    pub fn dict_items(&self, expr: &'a Expr, ctx: &Ctx) -> Option<Vec<(String, &'a Expr, Ctx)>> {
        self.dict_entries(expr, ctx).map(|(entries, _)| entries)
    }

    /// `dict_items`, and whether the entries are all the dict's keys (no
    /// unknown `**spread` or computed key could add others).
    pub fn dict_entries(&self, expr: &'a Expr, ctx: &Ctx) -> Option<(DictEntries<'a>, bool)> {
        let _guard = self.enter()?;
        let add = |out: &mut Vec<(String, &'a Expr, Ctx)>, e: (String, &'a Expr, Ctx)| {
            out.retain(|(k, _, _)| *k != e.0);
            out.push(e);
        };
        match expr {
            // Later entries override earlier ones. A `**spread` of a known dict
            // adds its entries; any other spread, or a non-literal key, may
            // override every earlier entry, so only the later ones are known.
            Expr::Dict(d) => {
                let mut out: Vec<(String, &'a Expr, Ctx)> = Vec::new();
                let mut complete = true;
                for item in d.items.iter() {
                    let entries = match &item.key {
                        Some(Expr::StringLiteral(k)) => {
                            Some((vec![(k.value.to_string(), &item.value, ctx.clone())], true))
                        }
                        Some(_) => None,
                        None => self.dict_entries(&item.value, ctx),
                    };
                    match entries {
                        Some((entries, c)) => {
                            complete &= c;
                            for e in entries {
                                add(&mut out, e);
                            }
                        }
                        None => {
                            out.clear();
                            complete = false;
                        }
                    }
                }
                Some((out, complete))
            }
            // `dict(k=v, ...)`, `dict(other, k=v)`, `dict(**other, k=v)`
            Expr::Call(c)
                if matches!(c.func.as_ref(), Expr::Name(n) if n.id.as_str() == "dict" && self.is_external("dict", ctx))
                    && c.arguments.args.len() <= 1 =>
            {
                let (mut out, mut complete) = match c.arguments.args.first() {
                    Some(base) => self.dict_entries(base, ctx).unwrap_or_default(),
                    None => (Vec::new(), true),
                };
                for kw in c.arguments.keywords.iter() {
                    match &kw.arg {
                        Some(k) => add(&mut out, (k.to_string(), &kw.value, ctx.clone())),
                        None => match self.dict_entries(&kw.value, ctx) {
                            Some((entries, c)) => {
                                complete &= c;
                                for e in entries {
                                    add(&mut out, e);
                                }
                            }
                            None => {
                                out.clear();
                                complete = false;
                            }
                        },
                    }
                }
                Some((out, complete))
            }
            Expr::Name(n) => {
                let name = n.id.as_str();
                if self.flow.escaping.contains(name) && !self.passed_to_forwarders(name) {
                    return None;
                }
                let a = self.single_def(name, ctx)?;
                match a.value {
                    Expr::Dict(_) | Expr::Call(_) => self.dict_entries(a.value, &Ctx::new(&a.narrowed)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Whether every escaping use of local `name` passes it directly to a
    /// dict forwarder parameter (a parameter the callee only splats), so the
    /// dict is not mutated.
    fn passed_to_forwarders(&self, name: &str) -> bool {
        let (Some(forwarders), Some(passes)) = (self.dict_forwarders, self.flow.passed.get(name)) else {
            return false;
        };
        passes.iter().all(|(call, pos)| {
            let targets = self.callees(&call.func, &Ctx::default());
            !targets.is_empty()
                && targets.iter().all(|t| {
                    let Callee::Function { qualified, implicit } = t else { return false };
                    let Some(f) = self.index.function(qualified) else { return false };
                    let param = match pos {
                        flow::ArgPos::Positional(i) => f
                            .params
                            .get(implicit + i)
                            .filter(|p| matches!(p.kind, ParamKind::PositionalOnly | ParamKind::Normal)),
                        flow::ArgPos::Keyword(k) => f
                            .params
                            .iter()
                            .find(|p| &p.name == k && matches!(p.kind, ParamKind::Normal | ParamKind::KeywordOnly)),
                    };
                    param.is_some_and(|p| forwarders.contains(&format!("{qualified}({})", p.name)))
                })
        })
    }

    /// What `**expr` supplies: known entries (expressions here, or facts of a
    /// dict a called function returns) and whether they are all its keys.
    pub fn splat(&self, expr: &'a Expr, ctx: &Ctx) -> Splat<'a> {
        if let Some((entries, complete)) = self.dict_entries(expr, ctx) {
            return Splat {
                entries: entries
                    .into_iter()
                    .map(|(k, e, c)| (k, SplatValue::Expr(e, c)))
                    .collect(),
                complete,
            };
        }
        if let Expr::Call(call) = strip_await(expr) {
            if let Some(s) = self.returned_dict(call, ctx) {
                return s;
            }
        }
        // `data = payload(x); Cls(**data)`
        if let Expr::Name(n) = expr {
            let name = n.id.as_str();
            if !self.flow.escaping.contains(name) || self.passed_to_forwarders(name) {
                if let Some(a) = self.single_def(name, ctx) {
                    if let Expr::Call(call) = strip_await(a.value) {
                        if let Some(s) = self.returned_dict(call, &Ctx::new(&a.narrowed)) {
                            return s;
                        }
                    }
                }
            }
        }
        Splat {
            entries: Vec::new(),
            complete: false,
        }
    }

    /// The dict a call returns, when every target returns a dict display
    /// (or `dict(...)`) on every `return`: each key's facts, computed in the
    /// callee (a bound over the callee's own input is dropped).
    fn returned_dict(&self, call: &ast::ExprCall, ctx: &Ctx) -> Option<Splat<'a>> {
        let targets = self.callees(&call.func, ctx);
        let mut by_key: Vec<(String, ValueFacts, String, u32)> = Vec::new();
        let mut complete = true;
        for t in &targets {
            let Callee::Function { qualified, .. } = t else { return None };
            let g = self.index.function(qualified)?;
            if g.is_generator {
                return None;
            }
            let flow = FunctionFlow::of_info(g);
            if flow.returns.is_empty() {
                return None;
            }
            let scope = Scope::new(self.index, self.summaries, Some(g), &flow);
            for (ret, narrowed) in &flow.returns {
                // A bare `return` / `return None`: `**None` raises, nothing is written.
                let Some(ret) = ret.filter(|e| !matches!(e, Expr::NoneLiteral(_))) else { continue };
                let (entries, c) = scope.dict_entries(ret, &Ctx::new(narrowed))?;
                complete &= c;
                for (k, e, ectx) in entries {
                    use ruff_text_size::Ranged;
                    let mut f = scope.facts(e, &ectx);
                    if f.precision.as_ref().is_some_and(|p| p.as_static().is_none()) {
                        f.precision = None;
                    }
                    let offset = e.range().start().to_u32();
                    match by_key.iter_mut().find(|(key, ..)| *key == k) {
                        Some(entry) => entry.1 = ValueFacts::join(entry.1.clone(), f),
                        None => by_key.push((k, f, g.source_file.clone(), offset)),
                    }
                }
            }
        }
        Some(Splat {
            entries: by_key
                .into_iter()
                .map(|(k, f, file, offset)| (k, SplatValue::Facts(f, file, offset)))
                .collect(),
            complete,
        })
    }

    /// Facts about the function's return value: body `return` expressions
    /// (plus the implicit `None` when the end is reachable), refined by the
    /// return annotation.
    pub fn return_facts(&self) -> ValueFacts {
        let mut facts = self.return_facts_of_body();
        if self.func.is_some_and(|f| f.overloads_differ_on_none) {
            facts.nullable = None;
        }
        facts
    }

    fn return_facts_of_body(&self) -> ValueFacts {
        if self.flow.is_generator {
            // A generator object; `return` inside only ends the iteration.
            return ValueFacts::non_null();
        }
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
        match func.return_nullable {
            Some(true) => facts.nullable = Some(true),
            // The annotation excludes None: a contract on the function's own
            // return values (checked at every `return`, see
            // `edge_discovery`), so callers rely on it (assume-guarantee).
            Some(false) if func.has_return_contract() => facts.nullable = Some(false),
            _ => {}
        }
        let annotated = func.tuple_element_type.clone().or_else(|| {
            func.return_type
                .as_deref()
                .map(|t| t.split('[').next().unwrap_or(t).trim().to_string())
        });
        facts.type_name = annotated.filter(|t| self.index.is_contract_type(t));
        facts.weak_type = false;
        facts
    }
}

/// A value that may be `None`, unless `present`: then the value of a dict
/// entry, whose facts are unknown.
fn nullable_unless(present: bool) -> ValueFacts {
    if present {
        ValueFacts::default()
    } else {
        ValueFacts::nullable()
    }
}

/// What a read of field `field` of an instance of `class` gives when nothing
/// else is known: the field's declared contracts (every write to it is
/// checked against them), else its declared nullability.
fn declared_field_facts(class: &ClassInfo, field: &str) -> ValueFacts {
    match class.field_facts.get(field) {
        Some(f) => f.clone(),
        None => match class.field_nullable.get(field) {
            Some(false) => ValueFacts::non_null(),
            Some(true) => ValueFacts::nullable(),
            None => ValueFacts::default(),
        },
    }
}

/// The possible values of a string literal after a case-mapping method,
/// when the mapping keeps every value's length (ASCII).
fn case_mapped(method: &str, values: &[String]) -> Option<Vec<String>> {
    if !values.iter().all(|v| v.is_ascii()) {
        return None;
    }
    Some(
        values
            .iter()
            .map(|v| match method {
                "upper" => v.to_ascii_uppercase(),
                "lower" => v.to_ascii_lowercase(),
                "swapcase" => v
                    .chars()
                    .map(|c| {
                        if c.is_ascii_uppercase() {
                            c.to_ascii_lowercase()
                        } else {
                            c.to_ascii_uppercase()
                        }
                    })
                    .collect(),
                "capitalize" => {
                    let lower = v.to_ascii_lowercase();
                    let mut cs = lower.chars();
                    match cs.next() {
                        Some(f) => f.to_ascii_uppercase().to_string() + cs.as_str(),
                        None => String::new(),
                    }
                }
                // title: upper after a non-letter, lower after a letter
                _ => {
                    let mut prev_letter = false;
                    v.chars()
                        .map(|c| {
                            let out = if prev_letter {
                                c.to_ascii_lowercase()
                            } else {
                                c.to_ascii_uppercase()
                            };
                            prev_letter = c.is_ascii_alphabetic();
                            out
                        })
                        .collect()
                }
            })
            .collect(),
    )
}

/// Whether `func` has a project-defined decorator (a function or class of
/// the project, other than the plain ones in `function_extractor`), which
/// may supply arguments the caller leaves out (`@background_task` passing
/// `db_session`).
pub fn injecting_decorator(index: &ProjectIndex, func: &FunctionInfo) -> bool {
    func.wrapping_decorators().any(|parts| {
        matches!(
            index.resolve_dotted(&func.module, parts),
            Some(Symbol::Function(_) | Symbol::Class(_))
        )
    })
}

/// `await e` → `e` (the awaited call's result stands for the call's).
pub fn strip_await(expr: &Expr) -> &Expr {
    match expr {
        Expr::Await(a) => strip_await(&a.value),
        other => other,
    }
}

/// Result type of a binary operation on value types, if known.
/// The element annotation of a collection annotation: `C` of `list[C]`,
/// `QuerySet[C]`, `Iterable[C]`, `tuple[C, ...]` ... (also `Optional[...]`).
fn collection_element(annotation: &Expr) -> Option<&Expr> {
    let Expr::Subscript(s) = strip_optional(annotation) else { return None };
    let head = dotted_parts(&s.value)?;
    if matches!(head.last().map(String::as_str), Some("tuple" | "Tuple")) {
        return match s.slice.as_ref() {
            Expr::Tuple(t) => match &t.elts[..] {
                [elem, Expr::EllipsisLiteral(_)] => Some(elem),
                _ => None,
            },
            _ => None,
        };
    }
    let collection = matches!(
        head.last().map(String::as_str),
        Some(
            "list" | "List" | "Sequence" | "Iterable" | "Iterator" | "Collection" | "QuerySet"
                | "set" | "Set" | "frozenset" | "FrozenSet" | "MutableSequence" | "AbstractSet"
        )
    );
    collection.then_some(s.slice.as_ref())
}

fn binop_type(op: Operator, l: Option<&str>, r: Option<&str>) -> Option<&'static str> {
    use Operator::*;
    let arith = matches!(op, Add | Sub | Mult | Div | FloorDiv | Mod);
    match (l?, r?) {
        ("str", "str") if matches!(op, Add) => Some("str"),
        ("str", _) if matches!(op, Mod) => Some("str"),
        ("str", "int") | ("int", "str") if matches!(op, Mult) => Some("str"),
        ("Decimal", "Decimal" | "int") | ("int", "Decimal") if arith => Some("Decimal"),
        ("int", "int") if matches!(op, Add | Sub | Mult | FloorDiv | Mod) => Some("int"),
        ("int", "int") if matches!(op, Div) => Some("float"),
        ("float", "float" | "int") | ("int", "float") if arith => Some("float"),
        _ => None,
    }
}

/// `Decimal('1.25')` → 2 places, value 1.25; `Decimal(3)` → 0 places;
/// other arguments unknown.
fn decimal_ctor_facts(call: &ast::ExprCall, scope: &Scope, ctx: &Ctx) -> ValueFacts {
    let mut facts = ValueFacts::typed("Decimal");
    if !call.arguments.keywords.is_empty() || call.arguments.args.len() != 1 {
        return facts;
    }
    match &call.arguments.args[0] {
        Expr::StringLiteral(s) => {
            let text = s.value.to_str();
            facts.precision = decimal_literal_places(text).map(Dep::Lit);
            facts.min_value = bounds::parse_decimal(text);
            facts.max_value = bounds::parse_decimal(text);
        }
        arg if float_literal(arg).is_some() => {
            // `Decimal(0.1)` is the float's exact binary value: 55 places.
            if let Some((places, value)) = float_literal(arg).and_then(float_exact) {
                facts.precision = Some(Dep::Lit(places));
                facts.min_value = value;
                facts.max_value = value;
            }
        }
        arg => {
            let f = scope.facts(arg, ctx);
            if f.type_name.as_deref() == Some("int") {
                facts.precision = Some(Dep::Lit(0));
                facts.min_value = f.min_value;
                facts.max_value = f.max_value;
            }
        }
    }
    facts
}

/// Decimal places of a decimal literal string (`"-12.340"` → 3,
/// `"1e-4"` → 4, `"1.5E2"` → 0). `NaN`, `Infinity` and anything else give `None`.
pub fn decimal_literal_places(text: &str) -> Option<i64> {
    let t = text.trim();
    let t = t.strip_prefix(['-', '+']).unwrap_or(t);
    let (mantissa, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i64>().ok()?),
        None => (t, 0),
    };
    let (int_part, frac) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit() || c == '_');
    if (int_part.is_empty() && frac.is_empty()) || !digits(int_part) || !digits(frac) {
        return None;
    }
    let places = frac.chars().filter(|c| c.is_ascii_digit()).count() as i64;
    Some(places.saturating_sub(exp).max(0))
}

/// A float literal, including a negated one.
fn float_literal(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::NumberLiteral(n) => match &n.value {
            Number::Float(f) => Some(*f),
            _ => None,
        },
        Expr::UnaryOp(u) if matches!(u.op, UnaryOp::USub) => float_literal(&u.operand).map(|v| -v),
        _ => None,
    }
}

/// `int_literal` of an optional expression.
fn int_literal_opt(expr: &Option<Box<Expr>>) -> Option<i64> {
    expr.as_deref().and_then(int_literal)
}

/// Exact decimal places of a binary float (`0.1` is
/// `0.1000000000000000055511151231257827021181583404541015625`: 55 places)
/// and, when it has at most 30 places, its exact value.
pub fn float_exact(v: f64) -> Option<(i64, Option<Dec>)> {
    if !v.is_finite() {
        return None;
    }
    if v == 0.0 {
        return Some((0, Some(Dec::ZERO)));
    }
    let bits = v.to_bits();
    let negative = bits >> 63 == 1;
    let exp_bits = ((bits >> 52) & 0x7ff) as i64;
    let frac = bits & ((1u64 << 52) - 1);
    // v = m × 2^e
    let (mut m, mut e) = if exp_bits == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), exp_bits - 1075)
    };
    while m % 2 == 0 && e < 0 {
        m /= 2;
        e += 1;
    }
    let sign: i128 = if negative { -1 } else { 1 };
    if e >= 0 {
        // An integer.
        let value = (e < 64)
            .then(|| (m as i128).checked_mul(1i128.checked_shl(e as u32)?))
            .flatten()
            .and_then(|x| Dec::from_scaled(sign * x, 0));
        return Some((0, value));
    }
    // m / 2^k = m × 5^k / 10^k with m odd: exactly k places.
    let k = -e;
    let value = u32::try_from(k)
        .ok()
        .filter(|k| *k <= 30)
        .and_then(|k| 5i128.checked_pow(k).and_then(|p| (m as i128).checked_mul(p)).map(|x| (x, k)))
        .and_then(|(x, k)| Dec::from_scaled(sign * x, k));
    Some((k, value))
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
    index: &ProjectIndex,
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
        .filter(|t| index.is_contract_type(t))
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
    if let Some(p) = &facts.precision {
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
        let row = base(ConstraintType::Range).with_range(facts.min_value, facts.max_value, false);
        if row.param_min_decimal.is_some() || row.param_max_decimal.is_some() {
            rows.push(row);
        }
    }
    if let Some(choices) = &facts.choices {
        rows.push(base(ConstraintType::Choices).with_choices(choices));
    }
    rows
}

/// Bound on the summary fixpoint iterations.
const MAX_SUMMARY_PASSES: usize = 64;

/// Return-value summaries of every extracted function (EXTRACTED facts only:
/// annotations and body; docstrings are not included).
pub fn compute_summaries(index: &ProjectIndex) -> HashMap<String, ValueFacts> {
    let flows: Vec<FunctionFlow> = index.functions.iter().map(FunctionFlow::of_info).collect();
    let mut summaries: HashMap<String, ValueFacts> = HashMap::new();
    // Every pass is sound on its own, so a cap only costs precision.
    for _ in 0..(index.functions.len() + 2).min(MAX_SUMMARY_PASSES) {
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
    #[allow(unused_imports)]
    use crate::bounds::mu;
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
    /// in module `code` (evaluated with the narrowing at that point, and the
    /// project's caller guards as edge discovery uses them).
    fn facts_of(files: &[(&str, &str)], func: &str) -> ValueFacts {
        let p = project(files);
        let f = p.index.function(func).expect("function");
        let flow = FunctionFlow::of_info(f);
        let guards = crate::edge_discovery::caller_guards(&p);
        let scope = Scope::new(&p.index, &p.summaries, Some(f), &flow).with_caller_guards(&guards);
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
        flow::walk_block(&f.body, &flow.entry, &flow.exits, &mut last);
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
        assert_eq!(decimal_literal_places("1E-3"), Some(3));
        assert_eq!(decimal_literal_places("1e-4"), Some(4));
        assert_eq!(decimal_literal_places("1.5E2"), Some(0));
        assert_eq!(decimal_literal_places("1.25e-1"), Some(3));
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
            (mu(-5_000_000), mu(-5_000_000), Some(false))
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
        // Any slice is bounded by its upper index, whatever the type.
        let f = one("def f(a):\n    a[:5]\n");
        assert_eq!((f.max_length, f.nullable, f.type_name), (Some(5), Some(false), None));
        assert_eq!(one("def f(a):\n    a[-3:4]\n").max_length, Some(4));
        assert_eq!(one("def f(a):\n    a[:4:-1]\n").max_length, None, "a negative step");
        assert_eq!(one("def f(a):\n    a[:-1]\n").max_length, None);
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

    /// Round 3 D1: rebinding a name to a method call on itself used to
    /// recurse without bound (receiver class of `v` needs the class of
    /// `v.m()`, which needs the receiver of `v`).
    #[test]
    fn test_self_referential_rebinding_terminates() {
        for body in [
            "def f(value):\n    value = value.strip()\n    return value\n",
            "def f(x):\n    x = x.strip()\n",
            "def f(x):\n    x = x.m(1)\n",
            "def f(x):\n    y = 1\n    y = y.m()\n",
            "def f(x):\n    x = x.strip()\n    return 1\n",
            "def f(a, b):\n    a = b.m()\n    b = a.m()\n    return a.n()\n",
            "def f(x):\n    x = f(x).m()\n    return x\n",
            "y = 1\ny = y.m()\n",
        ] {
            let p = project(&[("code.py", body)]);
            let _ = crate::edge_discovery::discover_project_edges(&p);
            assert!(!p.summaries.is_empty(), "{body}");
        }
    }

    /// Deeply nested expressions stay within the evaluation bound (and the
    /// extractor's large stack); the value is then unknown.
    #[test]
    fn test_deep_expressions_are_bounded() {
        let handle = std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(|| {
                let sum = vec!["x"; 3000].join(" + ");
                let calls = format!("{}x{}", "g(".repeat(400), ")".repeat(400));
                let src = format!(
                    "def g(x):\n    return x\ndef f(x):\n    return {sum}\ndef h(x):\n    return {calls}\n"
                );
                let p = project(&[("code.py", &src)]);
                let _ = crate::edge_discovery::discover_project_edges(&p);
                p.summaries["code.f"].precision.clone()
            })
            .unwrap();
        assert_eq!(handle.join().unwrap(), None);
    }

    #[test]
    fn test_flow_sensitive_reassignment() {
        // `v = d.get(k); if v is None: v = "x"`: never None afterwards.
        let f = one("def f(d):\n    v = d.get('k')\n    if v is None:\n        v = 'unset'\n    v\n");
        assert_eq!((f.nullable, f.max_length), (Some(false), None));
        // The same through a parameter's `None` default.
        let f = one("def f(p=None):\n    if p is None:\n        p = 'x'\n    p\n");
        assert_eq!(f.nullable, Some(false));
        // if / else rebinding both branches.
        let f = one("def f(c):\n    if c:\n        v = 'ab'\n    else:\n        v = 'abcd'\n    v\n");
        assert_eq!((f.nullable, f.max_length), (Some(false), Some(4)));
        // A branch that does not rebind keeps the earlier value (nullable).
        let f = one("def f(d, c):\n    v = d.get('k')\n    if c:\n        v = 'x'\n    v\n");
        assert_eq!(f.nullable, Some(true));
        // Straight-line rebinding: only the last value reaches.
        let f = one("def f(d):\n    v = d.get('k')\n    v = 'abc'\n    v\n");
        assert_eq!((f.nullable, f.max_length), (Some(false), Some(3)));
        // Rebinding in a loop falls back to every assignment.
        let f = one("def f(d, xs):\n    v = 'a'\n    for x in xs:\n        v = d.get(x)\n    v\n");
        assert_eq!(f.nullable, Some(true));
    }

    #[test]
    fn test_decimal_sum_keeps_places() {
        let f = one("def f(a: Decimal, b: Decimal):\n    a.quantize(Decimal('0.01')) + b.quantize(Decimal('0.001'))\n");
        assert_eq!(f.precision, Some(Dep::Lit(3)));
        let f = one("def f(a: Decimal):\n    a.quantize(Decimal('0.01')) - Decimal('1.5')\n");
        assert_eq!(f.precision, Some(Dep::Lit(2)));
        // A float sum has no decimal-place bound.
        assert_eq!(one("def f():\n    round(0.1, 1) + 0.25\n").precision, None);
    }

    #[test]
    fn test_decidable_literals_and_constants() {
        let f = one("def f():\n    'x' * 30\n");
        assert_eq!((f.max_length, f.nullable, f.type_name.as_deref()), (Some(30), Some(false), Some("str")));
        assert_eq!(one("def f():\n    3 * 'ab'\n").max_length, Some(6));
        let f = one("def f():\n    Decimal('-1.00')\n");
        assert_eq!(
            (f.precision, f.min_value, f.max_value),
            (Some(Dep::Lit(2)), mu(-1_000_000), mu(-1_000_000))
        );
        assert_eq!(one("def f(x: Decimal):\n    x.quantize(Decimal('1e-4'))\n").precision, Some(Dep::Lit(4)));
        let f = one("def f():\n    None\n");
        assert_eq!((f.precision, f.max_length, f.choices), (Some(Dep::Lit(0)), Some(0), Some(vec![])));
        let f = one("def f():\n    0.7\n");
        assert_eq!((f.min_value, f.max_value), (mu(700_000), mu(700_000)));
        // Module constants (bound once to a literal) resolve, also imported.
        let files = [
            ("consts.py", "Q4 = Decimal('0.0001')\nNAME = 'frozen'\n"),
            (
                "code.py",
                "from consts import Q4, NAME\nimport consts\nQ2 = Decimal('0.01')\nLATER = 'a'\nLATER = 'bb'\ndef f(x: Decimal):\n    x.quantize(Q4)\ndef g(x: Decimal):\n    x.quantize(Q2)\ndef h():\n    NAME\ndef i():\n    consts.NAME\ndef j():\n    LATER\n",
            ),
        ];
        assert_eq!(facts_of(&files, "code.f").precision, Some(Dep::Lit(4)));
        assert_eq!(facts_of(&files, "code.g").precision, Some(Dep::Lit(2)));
        let h = facts_of(&files, "code.h");
        assert_eq!((h.max_length, h.choices.clone()), (Some(6), Some(vec!["frozen".to_string()])));
        assert_eq!(facts_of(&files, "code.i").max_length, Some(6));
        assert_eq!(facts_of(&files, "code.j").max_length, None, "bound twice: not a constant");
    }

    #[test]
    fn test_class_constants_and_enum_members() {
        let files = [(
            "code.py",
            "from django.db import models\n\
             class Status(models.TextChoices):\n    ACTIVE = 'active', 'Active'\n\
             class Colour(Enum):\n    RED = 'red'\n\
             class Job:\n    FROZEN = 'frozen'\n    RATE = Decimal('0.001')\n    def a(self):\n        self.FROZEN\n    def b(self):\n        Job.RATE\n\
             class Mutable:\n    STATE = 'x'\n    def set(self):\n        self.STATE = 'longer'\n    def get(self):\n        self.STATE\n\
             def s():\n    Status.ACTIVE\n\
             def c():\n    Colour.RED\n",
        )];
        let a = facts_of(&files, "code.Job.a");
        assert_eq!((a.max_length, a.choices), (Some(6), Some(vec!["frozen".to_string()])));
        assert_eq!(facts_of(&files, "code.Job.b").precision, Some(Dep::Lit(3)));
        assert_eq!(facts_of(&files, "code.Mutable.get").max_length, None, "instance state shadows");
        let s = facts_of(&files, "code.s");
        assert_eq!((s.max_length, s.choices), (Some(6), Some(vec!["active".to_string()])));
        let c = facts_of(&files, "code.c");
        assert_eq!((c.max_length, c.choices), (None, Some(vec!["red".to_string()])));
    }

    #[test]
    fn test_choices_join() {
        let f = one("def f(c):\n    'a' if c else 'b'\n");
        assert_eq!(f.choices, Some(vec!["a".to_string(), "b".to_string()]));
        let f = one("def f(c, d: str):\n    'a' if c else d\n");
        assert_eq!(f.choices, None);
        assert_eq!(one("def f():\n    -3\n").choices, Some(vec!["-3".to_string()]));
    }

    /// Round 5 N10: values that cannot be None.
    #[test]
    fn test_non_null_by_construction() {
        for expr in [
            "a + b",
            "a % b",
            "'%s' % a",
            "sum(xs)",
            "max([a, b])",
            "min(a, b)",
            "len(xs)",
            "list(xs)",
            "sorted(xs)",
            "dict(a=1)",
            "a.strip()",
            "F('n') + 1",
            "models.F('n')",
        ] {
            let src = format!("from django.db import models\nfrom django.db.models import F\ndef f(a, b, xs):\n    {expr}\n");
            assert_eq!(facts_of(&[("code.py", &src)], "code.f").nullable, Some(false), "{expr}");
        }
        assert_eq!(one("def f(xs):\n    min(xs, default=None)\n").nullable, Some(true));
        // One element is returned without a comparison: `max([None])` is None.
        for expr in ["max(xs)", "max([a])", "min(x for x in xs)"] {
            let src = format!("def f(a, xs):\n    {expr}\n");
            assert_eq!(facts_of(&[("code.py", &src)], "code.f").nullable, None, "{expr}");
        }
        // Unless the element annotation excludes None; a project class element
        // that may be None stays unknown (Optional is not stripped).
        let typed = |ann: &str| {
            let src = format!(
                "from dataclasses import dataclass\nfrom decimal import Decimal\nfrom typing import List, Optional\n\n@dataclass\nclass Line:\n    n: int\n\ndef f(xs: {ann}):\n    max(xs, key=lambda l: l.n)\n"
            );
            facts_of(&[("code.py", &src)], "code.f").nullable
        };
        assert_eq!(typed("List[Decimal]"), Some(false));
        assert_eq!(typed("List[Line]"), Some(false));
        assert_eq!(typed("List[Optional[Line]]"), None);
        assert_eq!(typed("Optional[List[Optional[Decimal]]]"), None);
        assert_eq!(typed("List[\"Optional[Line]\"]"), None);
        assert_eq!(typed("List[Any]"), None);
        assert_eq!(typed("List[T]"), None);
        let body = |sig: &str, body: &str| {
            let src = format!(
                "from django.db import models\nfrom decimal import Decimal\nfrom typing import List\n\nclass Line(models.Model):\n    n = models.IntegerField()\n\ndef f({sig}):\n{body}    max(xs, key=lambda l: l.n)\n"
            );
            facts_of(&[("code.py", &src)], "code.f").nullable
        };
        // A rebound parameter is no longer described by its annotation.
        assert_eq!(body("xs: List[Decimal]", "    xs = [None]\n"), None);
        assert_eq!(body("xs: List[Decimal]", "    xs += [None]\n"), None);
        for rebind in [
            "    for xs in [[None]]:\n        pass\n",
            "    with open('f') as xs:\n        pass\n",
            "    xs, y = [None], 1\n",
            "    if (xs := [None]):\n        pass\n",
            "    try:\n        pass\n    except Exception as xs:\n        pass\n",
            "    match 1:\n        case xs:\n            pass\n",
            "    del xs\n    xs = [None]\n",
            "    def xs():\n        pass\n",
        ] {
            assert_eq!(body("xs: List[Decimal]", rebind), None, "{rebind}");
        }
        // An unannotated parameter bulk-written as a model's instances.
        assert_eq!(body("xs", "    Line.objects.bulk_create(xs)\n"), Some(false));
        // Augmented assignment: never None afterwards (flow-insensitively).
        let f = one("def f(xs):\n    total = 0\n    for x in xs:\n        total += x\n    total\n");
        assert_eq!((f.nullable, f.precision), (Some(false), None));
        let f = one("def f(d, xs):\n    t = d.get('k')\n    for x in xs:\n        t += x\n    t\n");
        assert_eq!(f.nullable, Some(true), "a nullable simple assignment still reaches");
    }

    #[test]
    fn test_sum_and_min_max_bounds() {
        let files = [(
            "code.py",
            "def four(x: Decimal) -> Decimal:\n    return x.quantize(Decimal('0.0001'))\n\
             def f(xs):\n    sum((four(x) for x in xs), Decimal('0'))\n\
             def g(a: Decimal, b: Decimal):\n    sum([a.quantize(Decimal('0.01')), b.quantize(Decimal('0.001'))])\n\
             def h(xs):\n    sum(x for x in xs)\n\
             def m(n: int):\n    max(1, n)\n\
             def k(n: int):\n    min(5, len(n))\n",
        )];
        let f = facts_of(&files, "code.f");
        assert_eq!((f.precision, f.type_name.as_deref()), (Some(Dep::Lit(4)), Some("Decimal")));
        assert_eq!(facts_of(&files, "code.g").precision, Some(Dep::Lit(3)));
        let h = facts_of(&files, "code.h");
        assert_eq!((h.precision, h.nullable), (None, Some(false)));
        // max(1, n) >= 1 even though n is unbounded; min(5, len(n)) <= 5.
        assert_eq!(facts_of(&files, "code.m").min_value, mu(1_000_000));
        let k = facts_of(&files, "code.k");
        assert_eq!((k.min_value, k.max_value), (mu(0), mu(5_000_000)));
    }

    #[test]
    fn test_case_mapping_of_literals() {
        let f = one("def f():\n    'ok'.upper()\n");
        assert_eq!((f.max_length, f.choices), (Some(2), Some(vec!["OK".to_string()])));
        let f = one("def f(c):\n    ('ab' if c else 'xyz').title()\n");
        assert_eq!(
            (f.max_length, f.choices),
            (Some(3), Some(vec!["Ab".to_string(), "Xyz".to_string()]))
        );
        assert_eq!(one("def f():\n    \"they're 1st\".title()\n").choices, Some(vec!["They'Re 1St".to_string()]));
        assert_eq!(one("def f():\n    'aB'.swapcase()\n").choices, Some(vec!["Ab".to_string()]));
        assert_eq!(one("def f():\n    'hELLO'.capitalize()\n").choices, Some(vec!["Hello".to_string()]));
        // Non-ASCII case mapping may change the length: unknown.
        assert_eq!(one("def f():\n    'straße'.upper()\n").max_length, None);
        assert_eq!(one("def f(s: str):\n    s.upper()\n").max_length, None);
    }

    #[test]
    fn test_compiled_patterns_and_await() {
        let files = [(
            "code.py",
            "import re\nCODE = re.compile('x')\n\
             class P:\n    PAT = re.compile('y')\n    def m(self, t):\n        self.PAT.match(t)\n\
             def f(t):\n    CODE.match(t)\n\
             def g(t):\n    p = re.compile('z')\n    p.fullmatch(t)\n\
             def h(t):\n    (m := CODE.search(t)) and m.group(1)\n\
             def other(t, obj):\n    obj.match(t)\n\
             async def fetch() -> Decimal:\n    return Decimal('1.25')\n\
             async def a():\n    await fetch()\n",
        )];
        for func in ["code.P.m", "code.f", "code.g", "code.h"] {
            assert_eq!(facts_of(&files, func).nullable, Some(true), "{func}");
        }
        assert_eq!(facts_of(&files, "code.other").nullable, None);
        assert_eq!(facts_of(&files, "code.a").precision, Some(Dep::Lit(2)));
    }

    #[test]
    fn test_generators_and_match_returns() {
        let p = project(&[(
            "code.py",
            "def gen(xs):\n    if not xs:\n        return\n    yield 1\n\
             def nested(xs):\n    def inner():\n        yield 1\n    return None\n\
             def rate(k: str) -> Decimal:\n    match k:\n        case 'a':\n            return Decimal('1')\n        case _:\n            raise ValueError(k)\n",
        )]);
        assert_eq!(p.summaries["code.gen"], ValueFacts::non_null());
        assert_eq!(p.summaries["code.nested"].nullable, Some(true), "a nested generator does not count");
        assert_eq!(p.summaries["code.rate"].nullable, Some(false));
    }

    /// Reads of a field of a known class: the values assigned in the function
    /// that reach, else the declared nullability.
    #[test]
    fn test_field_reads() {
        let files = [(
            "code.py",
            "from dataclasses import dataclass\nfrom typing import Optional\n\
             @dataclass\nclass R:\n    a: str\n    b: Optional[str]\n\
             def f(r: R):\n    r.a\n\
             def g(r: R):\n    r.b\n\
             def h(r: R):\n    if r.b is not None:\n        r.b\n\
             def i(r: R, d):\n    r.a = d.get('x')\n    r.a\n\
             def j(r: R):\n    r.a = 'abc'\n    r.a\n",
        )];
        assert_eq!(facts_of(&files, "code.f").nullable, Some(false));
        assert_eq!(facts_of(&files, "code.g").nullable, Some(true));
        assert_eq!(facts_of(&files, "code.h").nullable, Some(false));
        assert_eq!(facts_of(&files, "code.i").nullable, Some(true));
        assert_eq!(facts_of(&files, "code.j").max_length, Some(3));
    }

    /// CG-1.9: a `try` keeps what its fall-through paths agree on.
    #[test]
    fn test_try_reaching_definitions() {
        let f = |handler: &str| {
            one(&format!(
                "def f(raw):\n    v = raw.get('n')\n    if not v:\n        return\n    try:\n        v = int(v)\n    except ValueError:\n        {handler}\n    v\n"
            ))
            .nullable
        };
        assert_eq!(f("return"), Some(false));
        assert_eq!(f("v = 0"), Some(false));
        assert_eq!(f("pass"), Some(true));
    }

    /// CG-1.9: under `k in d`, `d.get(k)` is `d[k]` (unknown), not the default.
    #[test]
    fn test_get_under_membership() {
        assert_eq!(one("def f(d: dict, k):\n    if k in d:\n        d.get(k)\n").nullable, None);
        assert_eq!(one("def f(d: dict, k):\n    if k in d:\n        d.get(k, None)\n").nullable, None);
        assert_eq!(one("def f(d: dict, k):\n    if k in d:\n        d.get(k, default=None)\n").nullable, None);
        assert_eq!(one("def f(d, k):\n    d.get(k)\n").nullable, Some(true));
        assert_eq!(one("def f(d: dict, e, k):\n    if k in d:\n        e.get(k)\n").nullable, Some(true));
        assert_eq!(one("def f(d: dict, k):\n    if k in d:\n        d.pop(k)\n        d.get(k)\n").nullable, Some(true));
        assert_eq!(one("def f(d: dict, k):\n    if k in d and g():\n        d.get(k)\n").nullable, Some(true));
        // Only a receiver known to be a dict has a `.get` that returns `d[k]`.
        let get = |sig: &str, pre: &str| {
            one(&format!("def f({sig}, k):\n    {pre}if k in d:\n        d.get(k)\n")).nullable
        };
        for (sig, pre) in [
            ("d: Dict[str, str]", ""),
            ("d: Mapping[str, str]", ""),
            ("d: Optional[dict]", ""),
            ("d: typing.MutableMapping", ""),
            ("x", "d = {}\n    "),
            ("x", "d = dict(a=x)\n    "),
            ("x", "d = {y: y for y in x}\n    "),
            ("x", "d = defaultdict(list)\n    "),
        ] {
            assert_eq!(get(sig, pre), None, "{sig} {pre}");
        }
        for (sig, pre) in [
            ("d", ""),
            ("d: Cache", ""),
            ("d: dict", "d = load()\n    "),
            ("x", "d = load()\n    "),
            ("x", "d = {}\n    for d in x:\n        pass\n    "),
        ] {
            assert_eq!(get(sig, pre), Some(true), "{sig} {pre}");
        }
        assert_eq!(one("def f(o):\n    if 'a' in o.m:\n        o.m.get('a')\n").nullable, Some(true));
        // A mutating call inside the expression, after the test.
        for code in [
            "str(d.pop(k)) and d.get(k)",
            "k in d and str(d.pop(k)) and d.get(k)",
            "d.get(k) if d.pop(k) else 'x'",
            "'x' if not d.pop(k) else d.get(k)",
        ] {
            let narrowed = one(&format!("def f(d: dict, k):\n    if k in d:\n        {code}\n")).nullable;
            assert_eq!(narrowed, Some(true), "{code}");
        }
        assert_eq!(one("def f(d: dict, k):\n    k in d and d.get(k) and d.get(k)\n").nullable, None);
        let dict = "R = {'a': 'x'}\n";
        let g = |body: &str| facts_of(&[("code.py", &format!("{dict}{body}"))], "code.f").nullable;
        assert_eq!(g("def f(k):\n    if k in R:\n        R.get(k)\n"), Some(false));
        assert_eq!(g("def f(k):\n    R.get(k)\n"), Some(true));
    }

    /// CG-1.9: a field every caller narrows has unknown nullability, when
    /// the project shows every caller.
    #[test]
    fn test_caller_guards() {
        let records = "from dataclasses import dataclass\nfrom typing import Optional\n\
                       @dataclass\nclass H:\n    name: Optional[str]\n";
        let f = |code: &str| {
            let src = format!("{records}def label_of(h: H):\n    h.name\n{code}");
            facts_of(&[("code.py", &src)], "code.label_of").nullable
        };
        let guarded = "def a(h: H):\n    if h.name:\n        label_of(h)\n\
                       def b(h: H):\n    if h.name is None:\n        return\n    label_of(h=h)\n";
        assert_eq!(f(guarded), None);
        assert_eq!(f(&format!("{guarded}def c(h: H):\n    label_of(h)\n")), Some(true));
        assert_eq!(f(&format!("{guarded}HANDLERS = [label_of]\n")), Some(true));
        assert_eq!(f(&format!("{guarded}def c(h: H, o):\n    if h.name:\n        o.label_of(h)\n")), Some(true));
        assert_eq!(f(&format!("{guarded}def c(h: H, kw):\n    if h.name:\n        label_of(**kw)\n")), Some(true));
        assert_eq!(f(""), Some(true));
        // A lookup by string can reach `label_of` by a call the project does not show.
        for (code, want) in [
            ("def c(o, n):\n    getattr(o, n)(H(name=None))\n", Some(true)),
            ("def c(o):\n    getattr(o, 'label_of')(H(name=None))\n", Some(true)),
            ("def c(o):\n    getattr(o, 'other')\n", None),
            ("def c(o):\n    vars(o)\n", Some(true)),
            ("def c(n):\n    globals()[n](H(name=None))\n", Some(true)),
            ("def c(n):\n    locals()[n]\n", Some(true)),
            ("import operator\nG = operator.attrgetter('m.label_of')\n", Some(true)),
            ("import operator\nG = operator.methodcaller(NAME)\n", Some(true)),
        ] {
            assert_eq!(f(&format!("{guarded}{code}")), want, "{code}");
        }
        let src = format!("{records}def label_of(h: H):\n    h.name\n{guarded}");
        for (other, want) in [
            ("from code import label_of\ndef c(n):\n    globals()[n]\n", Some(true)),
            ("from code import *\ndef c(n):\n    globals()[n]\n", Some(true)),
            ("from code import a\ndef c(n):\n    globals()[n]\n", None),
        ] {
            let files = [("code.py", src.as_str()), ("other.py", other)];
            assert_eq!(facts_of(&files, "code.label_of").nullable, want, "{other}");
        }
        let src = format!("{records}def label_of(h: H):\n    h.name\n{guarded}");
        let aliased = [("code.py", src.as_str()), ("other.py", "from code import label_of as lo\nHANDLERS = [lo]\n")];
        assert_eq!(facts_of(&aliased, "code.label_of").nullable, Some(true));
        let decorated = format!("{records}@trace\ndef label_of(h: H):\n    h.name\n{guarded}");
        assert_eq!(facts_of(&[("code.py", &decorated)], "code.label_of").nullable, Some(true));
        // The body of an async function or a generator runs after the call.
        // The generator reads `h.name` before its first `yield`, so only its
        // own exclusion, not the loss of the guard at a suspension, keeps the error.
        for def in ["async def", "def"] {
            let body = if def == "def" { "h.name\n    _ = yield" } else { "h.name" };
            let lazy = format!("{records}{def} label_of(h: H):\n    {body}\n{guarded}");
            assert_eq!(facts_of(&[("code.py", &lazy)], "code.label_of").nullable, Some(true), "{def}");
        }
        let rebound = format!("{records}def label_of(h: H):\n    h = H(name=None)\n    h.name\n{guarded}");
        assert_eq!(facts_of(&[("code.py", &rebound)], "code.label_of").nullable, Some(true));
        let method = format!(
            "{records}class S:\n    def label_of(self, h: H):\n        h.name\n\
             def a(s: S, h: H):\n    if h.name:\n        s.label_of(h)\n"
        );
        assert_eq!(facts_of(&[("code.py", &method)], "code.S.label_of").nullable, None);
        let framework = method.replace("class S:", "from ext import Base\nclass S(Base):");
        assert_eq!(facts_of(&[("code.py", &framework)], "code.S.label_of").nullable, Some(true));
        let dunder = method.replace("label_of", "__label__");
        assert_eq!(facts_of(&[("code.py", &dunder)], "code.S.__label__").nullable, Some(true));
        // `*xs` may bind `h`, so the guarded `h` may bind `x`.
        let starred = format!(
            "{records}def label_of(x: H, h: H):\n    h.name\n\
             def a(h: H, xs):\n    if h.name:\n        label_of(*xs, h)\n"
        );
        assert_eq!(facts_of(&[("code.py", &starred)], "code.label_of").nullable, Some(true));
        // Guards through a conditional expression and boolean operators.
        for (code, want) in [
            ("label_of(h) if h.name else None", None),
            ("label_of(h) if not h.name else None", Some(true)),
            ("None if h.name else label_of(h)", Some(true)),
            ("None if h.name is None else label_of(h)", None),
            ("h.name and label_of(h)", None),
            ("not h.name or label_of(h)", None),
            ("h.name or label_of(h)", Some(true)),
        ] {
            assert_eq!(f(&format!("def c(h: H):\n    {code}\n")), want, "{code}");
        }
        // A positional argument that falls into `*a` does not bind the
        // keyword-only `h`, and neither does a third positional one.
        let kwonly = format!(
            "{records}def label_of(x, *a, h: H):\n    h.name\n\
             def c(g, h: H):\n    if h.name:\n        label_of(g, h)\n"
        );
        assert_eq!(facts_of(&[("code.py", &kwonly)], "code.label_of").nullable, Some(true));
        let third = kwonly.replace("label_of(g, h)", "label_of(g, g, h)");
        assert_eq!(facts_of(&[("code.py", &third)], "code.label_of").nullable, Some(true));
        let bare = format!(
            "{records}def label_of(x, *, h: H):\n    h.name\n\
             def c(g, h: H):\n    if h.name:\n        label_of(g, h)\n"
        );
        assert_eq!(facts_of(&[("code.py", &bare)], "code.label_of").nullable, Some(true));
        let kwonly_ok = kwonly.replace("label_of(g, h)", "label_of(g, h=h)");
        assert_eq!(facts_of(&[("code.py", &kwonly_ok)], "code.label_of").nullable, None);
        let posonly = format!(
            "{records}def label_of(h: H, /, x):\n    h.name\n\
             def c(h: H):\n    if h.name:\n        label_of(h, 1)\n"
        );
        assert_eq!(facts_of(&[("code.py", &posonly)], "code.label_of").nullable, None);
        // A narrowed field of the receiver binds `self`.
        let recv = format!(
            "{records}@dataclass\nclass K:\n    name: Optional[str]\n    def label_of(self):\n        self.name\n\
             def a(k: K):\n    if k.name:\n        k.label_of()\n"
        );
        assert_eq!(facts_of(&[("code.py", &recv)], "code.K.label_of").nullable, None);
        let recv_bad = recv.replace("if k.name:", "if k:");
        assert_eq!(facts_of(&[("code.py", &recv_bad)], "code.K.label_of").nullable, Some(true));
        // A method reference is a use as a value.
        let attr = format!("{method}def c(s: S):\n    cb = s.label_of\n");
        assert_eq!(facts_of(&[("code.py", &attr)], "code.S.label_of").nullable, Some(true));
        // Only a function's own recursion, or a cycle nothing else enters,
        // shows no caller.
        let own = format!(
            // The read comes before the recursive call, which may write `h.name`.
            "{records}def label_of(h: H, n: int):\n    h.name\n    if n:\n        if h.name:\n            return label_of(h, n - 1)\n"
        );
        assert_eq!(facts_of(&[("code.py", &own)], "code.label_of").nullable, Some(true));
        let cycle = format!(
            "{records}def label_of(h: H):\n    if h.name:\n        other(h)\n    h.name\n\
             def other(h: H):\n    if h.name:\n        label_of(h)\n"
        );
        assert_eq!(facts_of(&[("code.py", &cycle)], "code.label_of").nullable, Some(true));
        // A chain of calls that starts at a function nothing calls.
        let chain = format!(
            "{records}def label_of(h: H):\n    h.name\n\
             def mid(h: H):\n    if h.name:\n        label_of(h)\n\
             def start(h: H):\n    if h.name:\n        mid(h)\n"
        );
        assert_eq!(facts_of(&[("code.py", &chain)], "code.label_of").nullable, None);
        // A call that can reach `h` may write `h.name` before the read.
        for (body, want) in [
            ("clear(h)", Some(true)),
            ("clear(x=h)", Some(true)),
            ("h.save()", Some(true)),
            ("h.parts.append(1)", Some(true)),
            ("q = h\n    reset(q)", Some(true)),
            ("cb = lambda: h\n    cb()", Some(true)),
            ("def g():\n        return h\n    g()", Some(true)),
            ("if h:\n        pass", Some(true)),
            // A write on one path only still ends the guard after the join.
            ("if x:\n        clear(h)", Some(true)),
            ("if x:\n        pass\n    else:\n        clear(h)", Some(true)),
            ("if clear(h) is None:\n        pass", Some(true)),
            ("if x:\n        pass", None),
            ("setattr(h, 'name', None)", Some(true)),
            ("await other", Some(true)),
            ("log(h.name)", None),
            ("if h is None:\n        return", None),
            ("reset()", None),
        ] {
            let src = format!("{records}def label_of(h: H, x: H):\n    {body}\n    h.name\n\
                               def a(h: H):\n    if h.name:\n        label_of(h, h)\n");
            assert_eq!(facts_of(&[("code.py", &src)], "code.label_of").nullable, want, "{body}");
        }
        // An outside entry into the same recursion shows a caller.
        let entered = format!("{own}def start(h: H):\n    if h.name:\n        label_of(h, 3)\n");
        assert_eq!(facts_of(&[("code.py", &entered)], "code.label_of").nullable, None);
    }

    /// Round 7: class constants read through `self` / `cls` take every
    /// override in project subclasses.
    #[test]
    fn test_round7_class_constant_overrides() {
        let base = "class B:\n    C = 'ab'\n    def f(self):\n        self.C\n";
        let f = |subs: &str| facts_of(&[("code.py", &format!("{base}{subs}"))], "code.B.f");
        assert_eq!(f("").max_length, Some(2));
        assert_eq!(f("class S(B):\n    C = 'abcde'\nclass T(S):\n    pass\n").max_length, Some(5));
        assert_eq!(
            f("class S(B):\n    C = 'abcde'\n").choices,
            Some(vec!["ab".to_string(), "abcde".to_string()])
        );
        // A computed override, an instance store in a subclass, or more than
        // four values: unknown.
        assert_eq!(f("class S(B):\n    C = str(1)\n").max_length, None);
        assert_eq!(f("class S(B):\n    def g(self):\n        self.C = 'x'\n").max_length, None);
        let five: String = (0..5).map(|i| format!("class S{i}(B):\n    C = '{}'\n", "x".repeat(i + 3))).collect();
        assert_eq!(f(&five).max_length, None);
        // A method of the subclass named like the constant: unknown.
        assert_eq!(f("class S(B):\n    def C(self):\n        return 1\n").max_length, None);
    }

    /// Round 7: stubs and abstract methods are no dispatch targets; module
    /// constant dicts; annotations naming external types.
    #[test]
    fn test_round7_stubs_dicts_external() {
        let p = project(&[(
            "code.py",
            "from abc import ABC, abstractmethod\nfrom typing import Protocol\n\
             class A(ABC):\n    @abstractmethod\n    def m(self) -> int: ...\n\
             class B(A):\n    def m(self) -> int:\n        return 1\n\
             class C:\n    def m(self) -> int:\n        raise NotImplementedError()\n    def n(self) -> None:\n        pass\n\
             class D(C):\n    def m(self) -> int:\n        return 2\n    def n(self) -> None:\n        print(1)\n\
             class P(Protocol):\n    def m(self) -> int: ...\n\
             class Q(P):\n    def m(self) -> int:\n        return 3\n",
        )]);
        assert_eq!(p.index.dispatch_targets("code.A", "m"), Some(vec!["code.B.m".to_string()]));
        assert_eq!(p.index.dispatch_targets("code.C", "m"), Some(vec!["code.D.m".to_string()]));
        // `pass` under `-> None` is a real no-op, kept.
        assert_eq!(p.index.dispatch_targets("code.C", "n").map(|t| t.len()), Some(2));
        // A Protocol receiver keeps its declaration (structural implementations).
        assert_eq!(p.index.dispatch_targets("code.P", "m").map(|t| t.len()), Some(2));
        assert_eq!(p.index.dispatch_targets("code.Q", "m"), Some(vec!["code.Q.m".to_string()]));

        let dict = "from decimal import Decimal\nR = {'a': Decimal('1.5'), 'b': Decimal('2.25')}\nM = {'a': 1}\n";
        let g = |body: &str| facts_of(&[("code.py", &format!("{dict}{body}"))], "code.f");
        let f = g("def f(k):\n    R.get(k, Decimal('1'))\n");
        assert_eq!((f.nullable, f.precision), (Some(false), Some(Dep::Lit(2))));
        assert_eq!(g("def f(k):\n    R.get(k)\n").nullable, Some(true));
        assert_eq!(g("def f(k):\n    R.get(k, None)\n").nullable, Some(true));
        // Mutated somewhere: not a constant.
        assert_eq!(g("def f(k):\n    M.get(k, 0)\ndef h():\n    M['b'] = None\n").nullable, None);
        // A local shadowing the name.
        assert_eq!(g("def f(k, R):\n    R.get(k, 1)\n").nullable, None);

        let p = project(&[(
            "code.py",
            "from typing import Optional\nfrom ext.types import Attributes\nimport ext.mod as em\nfrom decimal import Decimal\n\
             def f(a: Attributes, b: em.Thing, c: Decimal, d: Optional[Attributes], e: 'Attributes', g: Attributes | int) -> Attributes:\n    return a\n\
             def h() -> Decimal:\n    return Decimal(1)\n",
        )]);
        let f = p.index.function("code.f").unwrap();
        let nullable: Vec<Option<bool>> = f.params.iter().map(|p| p.nullable).collect();
        assert_eq!(nullable, vec![None, None, Some(false), Some(true), None, None]);
        assert_eq!(f.return_nullable, None);
        assert_eq!(p.index.function("code.h").unwrap().return_nullable, Some(false));
    }

    /// Round 7: elements of collection-annotated parameters in comprehensions.
    #[test]
    fn test_round7_element_classes() {
        let m = "from decimal import Decimal\nfrom typing import Sequence\nfrom pydantic import BaseModel, Field\n\
                 class Line(BaseModel):\n    net: Decimal = Field(decimal_places=2)\n";
        let src = |body: &str| format!("{m}{body}");
        let f = |body: &str| facts_of(&[("code.py", &src(body))], "code.f");
        let p = |body: &str| f(body).precision.and_then(|d| d.as_static());
        assert_eq!(p("def f(lines: list[Line]):\n    sum(ln.net for ln in lines)\n"), Some(2));
        assert_eq!(
            p("def f(lines: Sequence[Line]):\n    sum((ln.net for ln in lines), Decimal('0.00'))\n"),
            Some(2)
        );
        assert_eq!(p("def f(lines: tuple[Line, ...]):\n    max([ln.net for ln in lines])\n"), Some(2));
        assert_eq!(p("def f(lines: set[Line]):\n    sum(ln.net * 2 for ln in lines)\n"), Some(2));
        assert_eq!(p("def f(lines: list[Line]):\n    sum(ln.net * Decimal('0.1') for ln in lines)\n"), Some(3));
        // A comprehension variable shadows the function's own local.
        assert_eq!(
            p("def f(lines: list[Line], ln: Line):\n    ln.net = Decimal('0.001')\n    sum(ln.net for ln in lines)\n"),
            Some(2)
        );
        // Unknown collections give nothing.
        assert_eq!(p("def f(lines):\n    sum(ln.net for ln in lines)\n"), None);
    }

    /// Round 6 value facts: exact float places, interval arithmetic, `max`
    /// over a generator, field reads with declared contracts.
    #[test]
    fn test_round6_value_facts() {
        let (p, v) = float_exact(0.1).unwrap();
        assert_eq!((p, v), (55, None), "0.1 has 55 exact places");
        assert_eq!(float_exact(0.5), Some((1, Dec::from_scaled(5, 1))));
        assert_eq!(float_exact(-2.0), Some((0, Some(Dec::from_int(-2)))));
        assert_eq!(float_exact(0.375).unwrap().0, 3);
        let f = one("def f():\n    Decimal(0.25)\n");
        assert_eq!((f.precision, f.min_value), (Some(Dep::Lit(2)), Dec::from_scaled(25, 2)));
        // len(x) - 1 >= -1; a + b for non-negative bounds; float: none.
        assert_eq!(one("def f(x):\n    len(x) - 1\n").min_value, Some(Dec::from_int(-1)));
        let f = one("def f(x):\n    len(x) + 2\n");
        assert_eq!((f.min_value, f.max_value), (Some(Dec::from_int(2)), None));
        assert_eq!(one("def f(x: float):\n    x + 1\n").min_value, None);
        // max / min over a generator: the elements' facts.
        let f = one("def f(xs):\n    max(x.quantize(Decimal('0.01')) for x in xs)\n");
        assert_eq!((f.precision, f.nullable), (Some(Dep::Lit(2)), Some(false)));
        let f = one("def f(xs):\n    min((len(x) for x in xs), default=0)\n");
        assert_eq!(f.min_value, Some(Dec::ZERO));
        // Field reads carry the declared contracts.
        let files = [(
            "code.py",
            "from django.db import models\n\
             class Inv(models.Model):\n    number = models.CharField(max_length=12)\n    total = models.DecimalField(max_digits=9, decimal_places=2)\n    n = models.PositiveIntegerField()\n\
             def f(i: Inv):\n    i.number\n\
             def g(i: Inv):\n    i.total\n\
             def h(i: Inv):\n    i.n - 1\n",
        )];
        let f = facts_of(&files, "code.f");
        assert_eq!((f.max_length, f.type_name.as_deref(), f.nullable), (Some(12), Some("str"), Some(false)));
        assert_eq!(facts_of(&files, "code.g").precision, Some(Dep::Lit(2)));
        assert_eq!(facts_of(&files, "code.h").min_value, Some(Dec::from_int(-1)));
    }

    /// Round 6: alternatives of a value at a use.
    #[test]
    fn test_alternatives() {
        let alts = |src: &str| {
            let p = project(&[("code.py", src)]);
            let f = p.index.function("code.f").unwrap();
            let flow = FunctionFlow::of_info(f);
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
            flow::walk_block(&f.body, &flow.entry, &flow.exits, &mut last);
            let (expr, n) = last.0.unwrap();
            let out: Vec<Option<i64>> = scope
                .alternatives(expr, &Ctx::new(&n))
                .into_iter()
                .map(|a| match a {
                    Alt::Expr(e, c, nn) => {
                        let f = scope.facts(e, &c);
                        let f = if nn { f.non_none_part() } else { f };
                        f.max_length
                    }
                    Alt::Facts(f) => f.max_length,
                })
                .collect();
            let mut out = out;
            out.sort();
            out
        };
        assert_eq!(alts("def f(c):\n    'ab' if c else 'abcd'\n"), [Some(2), Some(4)]);
        assert_eq!(
            alts("def f(c, d):\n    v = 'a'\n    if c:\n        v = 'abc'\n    if d:\n        v = d\n    v\n"),
            [None, Some(1), Some(3)]
        );
        // More than four: one joined alternative.
        assert_eq!(
            alts("def f(c):\n    ('a' if c else 'bb') if c else ('ccc' if c else ('dddd' if c else 'eeeee'))\n"),
            [Some(5)]
        );
    }

    /// Round 6: class hierarchy analysis and `super()`.
    #[test]
    fn test_dispatch_callees() {
        let files = [(
            "code.py",
            "class A:\n    def m(self):\n        return 1\n    def run(self):\n        return self.m()\n\
             class B(A):\n    def m(self):\n        return 2\n    def up(self):\n        return super().m()\n\
             class C(B):\n    pass\n\
             def f(a: A):\n    a.m()\n\
             def g():\n    B().m()\n",
        )];
        let p = project(&files);
        let targets = |func: &str| {
            let f = p.index.function(func).unwrap();
            let flow = FunctionFlow::of_info(f);
            let scope = Scope::new(&p.index, &p.summaries, Some(f), &flow);
            let call = match &f.body[0] {
                ruff_python_ast::Stmt::Expr(e) => e.value.clone(),
                ruff_python_ast::Stmt::Return(r) => r.value.clone().unwrap(),
                _ => unreachable!(),
            };
            let Expr::Call(c) = call.as_ref() else { unreachable!() };
            let mut qs: Vec<String> = scope
                .callees(&c.func, &Ctx::default())
                .into_iter()
                .filter_map(|t| match t {
                    Callee::Function { qualified, .. } => Some(qualified),
                    _ => None,
                })
                .collect();
            qs.sort();
            qs
        };
        assert_eq!(targets("code.f"), ["code.A.m", "code.B.m"]);
        assert_eq!(targets("code.A.run"), ["code.A.m", "code.B.m"], "self.m() in an inherited method");
        assert_eq!(targets("code.B.up"), ["code.A.m"], "super().m()");
        assert_eq!(targets("code.g"), ["code.B.m"], "B().m() on B only");
    }

    /// Overloads that disagree on `None`: the result's nullability is unknown.
    #[test]
    fn test_overloads() {
        let p = project(&[(
            "code.py",
            "from typing import overload\n\
             @overload\ndef get(x: str) -> int: ...\n\
             @overload\ndef get(x: None) -> int | None: ...\n\
             def get(x):\n    return None if x is None else 1\n\
             @overload\ndef same(x: str) -> int: ...\n\
             def same(x):\n    return None\n",
        )]);
        assert_eq!(p.summaries["code.get"].nullable, None);
        assert_eq!(p.summaries["code.same"].nullable, Some(true));
    }
}
