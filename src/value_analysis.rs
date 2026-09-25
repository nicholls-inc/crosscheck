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

use ruff_python_ast::{self as ast, BoolOp, Expr, Number, Operator, UnaryOp};

use crate::bounds::{self, Dec};
use crate::dataclass_extractor::VALUE_TYPES;
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
        }
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
                self.facts(&i.body, &ctx.narrow(flow::positive(&i.test))),
                self.facts(&i.orelse, &ctx.narrow(flow::negative(&i.test))),
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
                if class.stored_attrs.contains(name) {
                    return ValueFacts::default();
                }
                return match self.index.class_constant(&class.qualified, name) {
                    Some(q) => self.constant_facts(&q),
                    None => ValueFacts::default(),
                };
            }
        }
        // `obj.f` for a field of the known class of local `obj`.
        if let Expr::Name(n) = attr.value.as_ref() {
            if let Some((class, true)) = self.receiver_class(&attr.value, ctx) {
                let name = attr.attr.as_str();
                if class.fields.iter().any(|f| f == name) {
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
            if class.fields.contains(&parts[1]) || class.stored_attrs.contains(&parts[1]) {
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

    /// A read of field `field` of `obj` (an instance of `class`): the values
    /// assigned to `obj.f` in this function that reach here, or, when the
    /// function does not assign it, the field's declared nullability.
    fn field_read_facts(&self, obj: &str, class: &ClassInfo, field: &str, ctx: &Ctx) -> ValueFacts {
        let name = format!("{obj}.{field}");
        let declared = || match class.field_nullable.get(field) {
            Some(false) => ValueFacts::non_null(),
            Some(true) => ValueFacts::nullable(),
            None => ValueFacts::default(),
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
            None => match self.index.constant(qualified) {
                Some(e) => self.facts(e, &ctx),
                None => ValueFacts::default(),
            },
        }
    }

    fn name_facts(&self, name: &str, ctx: &Ctx) -> ValueFacts {
        if ctx.shadowed.contains(name) {
            return ValueFacts::default();
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
            (Some("str"), Operator::Add, Some(x), Some(y)) => Some(x + y),
            (Some("str"), Operator::Mult, _, _) if l.type_name.as_deref() == Some("str") => {
                repeat(&l, &b.right)
            }
            (Some("str"), Operator::Mult, _, _) => repeat(&r, &b.left),
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
                    Expr::Name(n) if self.is_self(n.id.as_str()) => self
                        .class
                        .is_some_and(|c| c.patterns.contains(attr) && !c.stored_attrs.contains(attr)),
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

    /// `sum(xs)`: never None; for a generator, list or tuple display of
    /// Decimal elements (or of ints), the elements' precision. A start value
    /// joins in.
    fn sum_facts(&self, call: &ast::ExprCall, ctx: &Ctx) -> ValueFacts {
        let args = &call.arguments.args;
        let mut out = ValueFacts::non_null();
        let Some(first) = args.first() else { return out };
        let elements = match first {
            Expr::Generator(g) => {
                let names: Vec<String> = flow::bound_names_in_expr(first).into_iter().collect();
                let c = ctx.shadow(names);
                Some(self.facts(&g.elt, &c))
            }
            Expr::ListComp(g) => {
                let names: Vec<String> = flow::bound_names_in_expr(first).into_iter().collect();
                let c = ctx.shadow(names);
                Some(self.facts(&g.elt, &c))
            }
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
    /// join, never None); `min(xs)`: never None (unless `default=None`).
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
        ValueFacts::non_null()
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
                if let Some(class) = self.cls_class(n.id.as_str(), ctx) {
                    return Callee::Class(class); // `cls(...)` in a classmethod
                }
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
        let _guard = self.enter()?;
        // `v = v.method()`: resolving `v` needs `v` itself; unknown.
        if !self.receiving.borrow_mut().insert(name.to_string()) {
            return None;
        }
        let result = self.receiver_class_of_local(name);
        self.receiving.borrow_mut().remove(name);
        result
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
        match expr {
            Expr::Await(a) => self.producer_of(&a.value, ctx),
            Expr::Call(c) => match self.callee(&c.func, ctx) {
                Callee::Function { qualified, .. } => Some((qualified, c)),
                _ => None,
            },
            Expr::Name(n) => {
                let a = self.single_def(n.id.as_str(), ctx)?;
                match strip_await(a.value) {
                    Expr::Call(c) => match self.callee(&c.func, &Ctx::new(&a.narrowed)) {
                        Callee::Function { qualified, .. } => Some((qualified, c)),
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
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
        match expr {
            // Later entries override earlier ones. A `**spread` of a known dict
            // adds its entries; any other spread, or a non-literal key, may
            // override every earlier entry, so only the later ones are known.
            Expr::Dict(d) => {
                let mut out: Vec<(String, &'a Expr, Ctx)> = Vec::new();
                for item in d.items.iter() {
                    let entries = match &item.key {
                        Some(Expr::StringLiteral(k)) => {
                            Some(vec![(k.value.to_string(), &item.value, ctx.clone())])
                        }
                        Some(_) => None,
                        None => self.dict_items(&item.value, ctx),
                    };
                    match entries {
                        Some(entries) => {
                            for e in entries {
                                out.retain(|(k, _, _)| *k != e.0);
                                out.push(e);
                            }
                        }
                        None => out.clear(),
                    }
                }
                Some(out)
            }
            Expr::Name(n) => {
                let name = n.id.as_str();
                if self.flow.escaping.contains(name) {
                    return None;
                }
                let a = self.single_def(name, ctx)?;
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

/// `await e` → `e` (the awaited call's result stands for the call's).
pub fn strip_await(expr: &Expr) -> &Expr {
    match expr {
        Expr::Await(a) => strip_await(&a.value),
        other => other,
    }
}

/// Result type of a binary operation on value types, if known.
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
    /// in module `code` (evaluated with the narrowing at that point).
    fn facts_of(files: &[(&str, &str)], func: &str) -> ValueFacts {
        let p = project(files);
        let f = p.index.function(func).expect("function");
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
        flow::walk_block(&f.body, &flow.entry, &mut last);
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
            "max(xs)",
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
