//! Which statements of a function body never complete, decided once the
//! project index is complete (`function_exits`). `flow` reads the result
//! (`flow::Exits`) and does no resolution itself.
//!
//! - A call that never returns: an expression statement calling
//!   `typing.assert_never`, `sys.exit`, `os._exit`, `os.abort`, the builtins
//!   `exit` / `quit`, or a project function annotated `NoReturn` / `Never`.
//! - A `match` over every member of an enum: the subject is a parameter
//!   annotated with a project enum, and unguarded value patterns name every member.
//!
//! The rules trust the type checker as parameter annotations already do: a
//! `NoReturn` function never returns, and a parameter annotated `E` holds a
//! member of `E`. Anything the rules do not recognise is assumed to complete.

use std::collections::HashSet;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt};
use ruff_text_size::Ranged;

use crate::flow::{bound_names, Exits};
use crate::function_extractor::{FunctionInfo, ParamKind};
use crate::resolve::{dotted_parts, ClassInfo, Import, ModuleInfo, ProjectIndex, Symbol};

/// Standard library functions that never return, as `(module, name)`.
const NO_RETURN_FUNCTIONS: [(&str, &str); 5] = [
    ("typing", "assert_never"),
    ("typing_extensions", "assert_never"),
    ("sys", "exit"),
    ("os", "_exit"),
    ("os", "abort"),
];

/// Bases an enum may list for a `match` over its members to be exhaustive
/// (last dotted segment). `Flag` / `IntFlag` are absent: a combined flag
/// value matches no member. A mixin other than `str` / `int` may define an
/// equality under which a member does not equal itself.
const ENUM_BASES: [&str; 8] = [
    "Enum",
    "IntEnum",
    "StrEnum",
    "TextChoices",
    "IntegerChoices",
    "Choices",
    "str",
    "int",
];

/// The statements of `f`'s body (at every nesting level, not inside nested
/// `def` / `class`) that never complete.
pub fn function_exits(index: &ProjectIndex, f: &FunctionInfo) -> Exits {
    let Some(module) = index.modules.get(&f.module) else {
        return Exits::default();
    };
    let bound = bound_names(&f.body);
    let mut local = bound.clone();
    local.extend(f.params.iter().map(|p| p.name.clone()));
    let mut v = ExitFinder {
        cx: Cx {
            index,
            module,
            local,
        },
        f,
        bound,
        exits: Exits::default(),
    };
    for stmt in &f.body {
        v.visit_stmt(stmt);
    }
    v.exits
}

/// What a name in the function means.
struct Cx<'i> {
    index: &'i ProjectIndex,
    /// The function's module.
    module: &'i ModuleInfo,
    /// Names the function binds itself (parameters included).
    local: HashSet<String>,
}

impl Cx<'_> {
    /// Whether the global `name`, read in the function, has the value of
    /// the module's one import or definition of it.
    fn is_stable_global(&self, name: &str) -> bool {
        !self.local.contains(name) && chain_is_stable(self.index, &self.module.name, name, 0)
    }

    /// Whether the dotted `parts`, read in the function, reach what their
    /// imports name: the head is a stable global, and (`mod.name` through
    /// `import mod`) so is `name` in `mod`.
    fn path_is_stable(&self, parts: &[String]) -> bool {
        if !self.is_stable_global(&parts[0]) {
            return false;
        }
        match (self.module.imports.get(&parts[0]), parts) {
            (Some(Import::Module(path)), [_, attr, ..]) => self
                .index
                .find_module(path, &self.module.name)
                .is_none_or(|m| chain_is_stable(self.index, m, attr, 1)),
            _ => true,
        }
    }

    /// Whether the expression statement `expr` is a call that never returns.
    fn is_exit_call(&self, expr: &Expr) -> bool {
        let (call, awaited) = match expr {
            Expr::Call(c) => (c, false),
            Expr::Await(a) => match a.value.as_ref() {
                Expr::Call(c) => (c, true),
                _ => return false,
            },
            _ => return false,
        };
        let Some(parts) = dotted_parts(&call.func) else {
            return false;
        };
        if !self.path_is_stable(&parts) {
            return false;
        }
        if !awaited && self.is_process_exit(&parts) {
            return true;
        }
        match self.index.resolve_expr(&self.module.name, &call.func) {
            Some(Symbol::Function(q)) => self
                .index
                .function(&q)
                .is_some_and(|g| g.is_async == awaited && never_returns(self.index, g)),
            _ => false,
        }
    }

    /// `parts` names a standard library function that never returns
    /// (imported from a module that is not a project module), or the
    /// builtin `exit` / `quit`.
    fn is_process_exit(&self, parts: &[String]) -> bool {
        let m = self.module;
        let target = match (m.imports.get(&parts[0]), parts) {
            (Some(Import::Symbol { module, name }), [_]) => (module, name),
            (Some(Import::Module(module)), [_, attr]) => (module, attr),
            (None, [name]) => {
                return matches!(name.as_str(), "exit" | "quit")
                    && !m.defs.contains_key(name)
                    && m.star_imports.is_empty();
            }
            _ => return false,
        };
        NO_RETURN_FUNCTIONS.contains(&(target.0.as_str(), target.1.as_str()))
            && self.index.find_module(target.0, &m.name).is_none()
    }

    /// The names of `class_q`'s members that `pattern` matches: a value
    /// pattern `E.NAME` whose `E` resolves to the class, an or-pattern of
    /// them, or an `as` pattern around one.
    fn members_matched(&self, class_q: &str, pattern: &Pattern, out: &mut HashSet<String>) {
        match pattern {
            Pattern::MatchValue(v) => {
                let Some(parts) = dotted_parts(&v.value) else { return };
                let [prefix @ .., name] = parts.as_slice() else { return };
                if self.path_is_stable(prefix)
                    && self.index.resolve_dotted(&self.module.name, prefix)
                        == Some(Symbol::Class(class_q.to_string()))
                {
                    out.insert(name.clone());
                }
            }
            Pattern::MatchOr(o) => {
                for p in &o.patterns {
                    self.members_matched(class_q, p, out);
                }
            }
            Pattern::MatchAs(a) => {
                if let Some(p) = &a.pattern {
                    self.members_matched(class_q, p, out);
                }
            }
            _ => {}
        }
    }
}

/// Whether a call of `g` never returns: its return annotation is `NoReturn`
/// or `Never`, no decorator wraps it, it is no overload stub and no
/// generator, and its module does not rebind its name (a method: its class's name).
fn never_returns(index: &ProjectIndex, g: &FunctionInfo) -> bool {
    g.wrapping_decorators().next().is_none()
        // A plain name (`cache`, `wraps`) that a project function defines
        // may wrap the call and swallow what it raises.
        && g.decorators.iter().all(|d| index.resolve_dotted(&g.module, d).is_none())
        && !g.is_overload
        && !g.is_generator
        && index.modules.get(&g.module).is_some_and(|m| {
            // A method belongs to a class its module may define twice (the
            // index keeps the later definition); a function's own name may be
            // bound again.
            !m.rebound.contains(g.class_name.as_deref().unwrap_or(&g.name))
        })
        && g.return_annotation
            .as_ref()
            .is_some_and(|a| is_no_return_annotation(index, &g.module, a))
}

/// Whether the global `name` of `module` is the value of its one import or
/// definition, through every module its imports pass through: none binds the
/// name again, imports it from two places, or reaches it by a star import.
/// (`from .impl import f` in a package that then does `f = wrap(f)` makes `f`
/// unstable for a module that imports it from the package.)
fn chain_is_stable(index: &ProjectIndex, module: &str, name: &str, depth: usize) -> bool {
    let Some(m) = index.modules.get(module) else {
        return true;
    };
    if depth > 8 || m.rebound.contains(name) {
        return false;
    }
    if m.defs.contains_key(name) {
        return true;
    }
    match m.imports.get(name) {
        Some(Import::Symbol { module: from, name: n }) => index
            .find_module(from, module)
            .is_none_or(|t| chain_is_stable(index, t, n, depth + 1)),
        Some(Import::Module(_)) => true,
        // A builtin or a submodule in the function's own module; in another
        // module a name no import or definition names comes from a star import.
        None => depth == 0 || m.star_imports.is_empty(),
    }
}

/// `NoReturn` / `Never` from `typing` or `typing_extensions`, imported by
/// name or through the module, or the same in quotes.
fn is_no_return_annotation(index: &ProjectIndex, module: &str, annotation: &Expr) -> bool {
    if let Expr::StringLiteral(s) = annotation {
        return ruff_python_parser::parse_expression(s.value.to_str().trim())
            .is_ok_and(|p| is_no_return_annotation(index, module, p.expr()));
    }
    let Some(info) = index.modules.get(module) else {
        return false;
    };
    let Some(parts) = dotted_parts(annotation) else {
        return false;
    };
    if info.rebound.contains(&parts[0]) {
        return false;
    }
    let (from, name) = match (info.imports.get(&parts[0]), parts.as_slice()) {
        (Some(Import::Symbol { module, name }), [_]) => (module, name),
        (Some(Import::Module(module)), [_, attr]) => (module, attr),
        _ => return false,
    };
    matches!(from.as_str(), "typing" | "typing_extensions")
        && matches!(name.as_str(), "NoReturn" | "Never")
        && index.find_module(from, module).is_none()
}

struct ExitFinder<'i> {
    cx: Cx<'i>,
    f: &'i FunctionInfo,
    /// Names the function body binds (parameters not included).
    bound: HashSet<String>,
    exits: Exits,
}

impl<'i> ExitFinder<'i> {
    /// Whether every member of the subject's enum is matched by an unguarded case.
    fn covers_enum(&self, m: &ast::StmtMatch) -> bool {
        let Expr::Name(subject) = m.subject.as_ref() else {
            return false;
        };
        if self.bound.contains(subject.id.as_str()) {
            return false;
        }
        let Some(param) = self.f.params.iter().find(|p| p.name == subject.id.as_str()) else {
            return false;
        };
        if !matches!(
            param.kind,
            ParamKind::PositionalOnly | ParamKind::Normal | ParamKind::KeywordOnly
        ) || param.default_none
        {
            return false;
        }
        let Some(class) = param.annotation.as_ref().and_then(|a| self.enum_class(a)) else {
            return false;
        };
        let mut matched = HashSet::new();
        for case in m.cases.iter().filter(|c| c.guard.is_none()) {
            self.cx.members_matched(&class.qualified, &case.pattern, &mut matched);
        }
        let members = class.enum_members.iter().flatten();
        members.into_iter().all(|n| matched.contains(n))
    }

    /// The project enum an annotation names (not `Optional`, a union or a
    /// subscript, which resolve to no enum), when its members are certain and
    /// each matches itself.
    fn enum_class(&self, annotation: &Expr) -> Option<&'i ClassInfo> {
        let parsed;
        let annotation = match annotation {
            Expr::StringLiteral(s) => {
                parsed = ruff_python_parser::parse_expression(s.value.to_str().trim())
                    .ok()?
                    .into_expr();
                &parsed
            }
            other => other,
        };
        let index = self.cx.index;
        if !dotted_parts(annotation).is_some_and(|p| self.cx.path_is_stable(&p)) {
            return None;
        }
        let class = index.resolve_class(&self.f.module, annotation)?;
        let plain_bases = class.bases.iter().all(|b| {
            dotted_parts(b)
                .and_then(|p| p.last().cloned())
                .is_some_and(|last| ENUM_BASES.contains(&last.as_str()))
                && !matches!(index.resolve_expr(&class.module, b), Some(Symbol::Class(_)))
        });
        // An enum without members may be subclassed by one with members.
        let members = class.enum_members.as_ref().is_some_and(|m| !m.is_empty());
        // A class its module defines twice (`if`/`else`) has one indexed list.
        let defined_once = index
            .modules
            .get(&class.module)
            .is_some_and(|m| !m.rebound.contains(&class.name));
        (class.enum_kind.is_some() && plain_bases && members && defined_once).then_some(class)
    }
}

impl<'a> Visitor<'a> for ExitFinder<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => return,
            Stmt::Expr(e) if self.cx.is_exit_call(&e.value) => {
                self.exits.calls.insert(stmt.start().to_u32());
            }
            Stmt::Match(m) if self.covers_enum(m) => {
                self.exits.matches.insert(stmt.start().to_u32());
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }
}

#[cfg(test)]
mod tests {
    use crate::extractor::Project;
    use crate::flow::FunctionFlow;

    /// The first line of each statement of `code.<func>` that `function_exits`
    /// marks: `(calls, matches)`, in source order.
    fn marked(files: &[(&str, &str)], func: &str) -> (Vec<String>, Vec<String>) {
        let p = Project::from_sources(
            files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        );
        let f = p.index.function(&format!("code.{func}")).expect("function");
        let src = files.iter().find(|(p, _)| *p == "code.py").unwrap().1;
        let lines = |offsets: &std::collections::HashSet<u32>| {
            let mut o: Vec<u32> = offsets.iter().copied().collect();
            o.sort();
            o.into_iter()
                .map(|at| src[at as usize..].lines().next().unwrap().to_string())
                .collect::<Vec<_>>()
        };
        (lines(&f.exits.calls), lines(&f.exits.matches))
    }

    fn calls(src: &str, func: &str) -> Vec<String> {
        marked(&[("code.py", src)], func).0
    }

    #[test]
    fn test_standard_library_exits() {
        let src = "import os\nimport sys\nimport typing as t\nfrom sys import exit as bye\nfrom typing_extensions import assert_never\n\ndef f(x):\n    if x:\n        sys.exit(1)\n    elif x == 2:\n        bye(2)\n    for i in x:\n        os._exit(1)\n    with x:\n        os.abort()\n    try:\n        t.assert_never(x)\n    except E:\n        assert_never(x)\n    sys.stdout.write(x)\n    os.exit(1)\n    def g():\n        sys.exit(1)\n";
        assert_eq!(
            calls(src, "f"),
            [
                "sys.exit(1)",
                "bye(2)",
                "os._exit(1)",
                "os.abort()",
                "t.assert_never(x)",
                "assert_never(x)"
            ]
        );
    }

    #[test]
    fn test_standard_library_exit_needs_a_plain_call_of_the_real_module() {
        // A project module named `sys`, an awaited call, a local import.
        let files = [
            ("sys.py", "def exit(code): return code\n"),
            (
                "code.py",
                "import sys\nimport os\n\ndef f(x):\n    sys.exit(1)\n    os.abort()\n\nasync def g(x):\n    await os.abort()\n\ndef h(x):\n    import os\n    os._exit(1)\n",
            ),
        ];
        assert_eq!(marked(&files, "f").0, ["os.abort()"]);
        assert_eq!(marked(&files, "g").0, Vec::<String>::new());
        assert_eq!(marked(&files, "h").0, Vec::<String>::new());
        let rebound = "import sys\nsys = make()\n\ndef f(x):\n    sys.exit(1)\n";
        assert_eq!(calls(rebound, "f"), Vec::<String>::new());
    }

    #[test]
    fn test_builtin_exit_unless_the_name_is_bound() {
        let src = "def f(x):\n    exit(1)\n    quit()\n\ndef g(x):\n    exit = print\n    exit(1)\n\ndef h(exit):\n    exit(1)\n";
        assert_eq!(calls(src, "f"), ["exit(1)", "quit()"]);
        assert_eq!(calls(src, "g"), Vec::<String>::new());
        assert_eq!(calls(src, "h"), Vec::<String>::new());
        for module in [
            "exit = print\n",
            "def exit(code): pass\n",
            "from helpers import exit\n",
            "from helpers import *\n",
        ] {
            let src = format!("{module}\ndef f(x):\n    exit(1)\n");
            assert_eq!(calls(&src, "f"), Vec::<String>::new(), "{module}");
        }
    }

    #[test]
    fn test_project_no_return_functions() {
        let src = "import typing\nfrom typing import NoReturn, Never\nfrom typing_extensions import Never as Nev\n\ndef a(m) -> NoReturn:\n    raise E(m)\n\ndef b(m) -> Never:\n    raise E(m)\n\ndef c(m) -> typing.NoReturn:\n    raise E(m)\n\ndef d(m) -> 'Nev':\n    raise E(m)\n\nclass K:\n    @staticmethod\n    def s(m) -> NoReturn:\n        raise E(m)\n\ndef plain(m) -> int:\n    return 1\n\ndef other(m) -> typing.Any:\n    return 1\n\ndef f(x):\n    a(x)\n    b(x)\n    c(x)\n    d(x)\n    K.s(x)\n    plain(x)\n    other(x)\n";
        assert_eq!(calls(src, "f"), ["a(x)", "b(x)", "c(x)", "d(x)", "K.s(x)"]);
    }

    #[test]
    fn test_no_return_function_exceptions() {
        let src = "from typing import NoReturn, overload\n\ndef wrap(fn):\n    return fn\n\n@wrap\ndef decorated(m) -> NoReturn:\n    raise E(m)\n\ndef gen(m) -> NoReturn:\n    yield m\n\nasync def later(m) -> NoReturn:\n    raise E(m)\n\ndef sync(m) -> NoReturn:\n    raise E(m)\n\nclass K:\n    def m(self) -> NoReturn:\n        raise E()\n\nasync def f(x, k: K):\n    decorated(x)\n    gen(x)\n    later(x)\n    await sync(x)\n    k.m()\n    await later(x)\n    sync(x)\n";
        assert_eq!(calls(src, "f"), ["await later(x)", "sync(x)"]);
    }

    #[test]
    fn test_no_return_function_whose_name_or_annotation_is_rebound() {
        let head = "from typing import NoReturn\n\n";
        let f = "\ndef f(x):\n    a(x)\n";
        let cases = [
            // The function's name is bound again.
            format!("{head}def a(m) -> NoReturn:\n    raise E(m)\n\na = other\n{f}"),
            // `NoReturn` is bound again.
            format!("{head}NoReturn = int\n\ndef a(m) -> NoReturn:\n    raise E(m)\n{f}"),
        ];
        for src in cases {
            assert_eq!(calls(&src, "f"), Vec::<String>::new(), "{src}");
        }
        let src = format!("{head}def a(m) -> NoReturn:\n    raise E(m)\n{f}");
        assert_eq!(calls(&src, "f"), ["a(x)"]);
        // Imported from a module that binds the name again.
        let code = "from helpers import a\n\ndef f(x):\n    a(x)\n";
        let helper = format!("{head}def a(m) -> NoReturn:\n    raise E(m)\n\na = other\n");
        let files = [("helpers.py", helper.as_str()), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, Vec::<String>::new());
        let helper = format!("{head}def a(m) -> NoReturn:\n    raise E(m)\n");
        let files = [("helpers.py", helper.as_str()), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, ["a(x)"]);
    }

    #[test]
    fn test_a_project_decorator_with_a_plain_name_may_wrap() {
        let head = "from typing import NoReturn\n";
        let tail = "\n@cache\ndef a(m) -> NoReturn:\n    raise E(m)\n\ndef f(x):\n    a(x)\n";
        let own = format!("{head}\ndef cache(fn):\n    return fn\n{tail}");
        assert_eq!(calls(&own, "f"), Vec::<String>::new());
        let std = format!("{head}from functools import cache\n{tail}");
        assert_eq!(calls(&std, "f"), ["a(x)"]);
    }

    #[test]
    fn test_enum_defined_twice_in_another_module() {
        let enums = "import sys\nfrom enum import Enum\n\nif sys.version_info >= (3, 11):\n    class E(Enum):\n        A = 1\n        B = 2\n        C = 3\nelse:\n    class E(Enum):\n        A = 1\n        B = 2\n";
        let code = "from enums import E\n\ndef f(e: E):\n    match e:\n        case E.A | E.B:\n            return 1\n";
        let files = [("enums.py", enums), ("code.py", code)];
        assert_eq!(marked(&files, "f").1, Vec::<String>::new());
        let once = "from enum import Enum\n\nclass E(Enum):\n    A = 1\n    B = 2\n";
        let files = [("enums.py", once), ("code.py", code)];
        assert_eq!(marked(&files, "f").1, ["match e:"]);
    }

    #[test]
    fn test_function_flow_reads_the_function_exits() {
        // The wiring from `function_exits` to `FunctionFlow::of_info`: the
        // marked call ends the flow and narrows `x`.
        let src = "import sys\n\ndef f(x):\n    if x is None:\n        sys.exit(1)\n    return x\n\ndef g(x):\n    sys.exit(1)\n";
        let p = Project::from_sources(vec![("code.py".to_string(), src.to_string())]);
        let flow = |name: &str| FunctionFlow::of_info(p.index.function(name).expect("function"));
        assert!(flow("code.f").returns[0].1.contains("x"));
        assert!(!flow("code.g").falls_through);
        let src = "def f(x):\n    if x is None:\n        stop(1)\n    return x\n\ndef g(x):\n    stop(1)\n";
        let p = Project::from_sources(vec![("code.py".to_string(), src.to_string())]);
        let flow = |name: &str| FunctionFlow::of_info(p.index.function(name).expect("function"));
        assert!(!flow("code.f").returns[0].1.contains("x"));
        assert!(flow("code.g").falls_through);
    }

    #[test]
    fn test_no_return_function_that_a_module_re_exports_and_rebinds() {
        let head = "from typing import NoReturn\n\n";
        let imp = format!("{head}def abort(m) -> NoReturn:\n    raise E(m)\n");
        let code = "from pkg import abort\n\ndef f(x):\n    abort(x)\n";
        let rebound = ("pkg.py", "from impl import abort\nabort = swallow(abort)\n");
        let files = [("impl.py", imp.as_str()), rebound, ("code.py", code)];
        assert_eq!(marked(&files, "f").0, Vec::<String>::new());
        let plain = ("pkg.py", "from impl import abort\n");
        let files = [("impl.py", imp.as_str()), plain, ("code.py", code)];
        assert_eq!(marked(&files, "f").0, ["abort(x)"]);
    }

    #[test]
    fn test_exit_through_a_hop_that_rebinds_or_star_imports() {
        let imp = "from typing import NoReturn\n\ndef abort(m) -> NoReturn:\n    raise E(m)\n";
        let enums = "from enum import Enum\n\nclass Color(Enum):\n    RED = 1\n    GREEN = 2\n";
        let code = "from pkg import abort, Color\nimport pkg\n\ndef f(x):\n    abort(x)\n\ndef g(x):\n    pkg.abort(x)\n\ndef h(c: Color):\n    match c:\n        case Color.RED | Color.GREEN:\n            return 1\n";
        let marked_with = |pkg: &str| {
            let files = [("impl.py", imp), ("colors.py", enums), ("pkg.py", pkg), ("code.py", code)];
            (marked(&files, "f"), marked(&files, "g"), marked(&files, "h"))
        };
        let clean = "from impl import abort\nfrom colors import Color\n";
        let ((f, _), (g, _), (_, h)) = marked_with(clean);
        assert_eq!((f, g, h), (vec!["abort(x)".to_string()], vec!["pkg.abort(x)".to_string()], vec!["match c:".to_string()]));
        for pkg in [
            // The re-export binds the names again.
            "from impl import abort\nfrom colors import Color\nabort = wrap(abort)\nColor = wrap(Color)\n",
            // The names come from a star import.
            "from impl import *\nfrom colors import *\n",
            // An alias chain whose last hop is rebound.
            "from impl import abort as stop\nfrom colors import Color as Hue\nstop = wrap(stop)\nHue = wrap(Hue)\nabort, Color = stop, Hue\n",
            // Two sources for a name.
            "try:\n    from extlib import abort, Color\nexcept ImportError:\n    from impl import abort\n    from colors import Color\n",
        ] {
            let ((f, _), (g, _), (_, h)) = marked_with(pkg);
            assert_eq!((f, g, h), (vec![], vec![], vec![]), "{pkg}");
        }
    }

    #[test]
    fn test_enum_annotation_and_patterns_each_need_a_stable_path() {
        // `Color` reaches the enum through a package that rebinds it; `Hue`
        // reaches it directly. Each name alone must not make the match exhaustive.
        let enums = "from enum import Enum\n\nclass Color(Enum):\n    RED = 1\n    GREEN = 2\n";
        let pkg = "from colors import Color\nColor = wrap(Color)\n";
        let head = "from pkg import Color\nfrom colors import Color as Hue\n\n";
        let cases = [
            ("def f(c: Color):\n    match c:\n        case Hue.RED | Hue.GREEN:\n            return 1\n", false),
            ("def f(c: Hue):\n    match c:\n        case Color.RED | Color.GREEN:\n            return 1\n", false),
            ("def f(c: Hue):\n    match c:\n        case Hue.RED | Hue.GREEN:\n            return 1\n", true),
        ];
        for (body, exhaustive) in cases {
            let code = format!("{head}{body}");
            let files = [("colors.py", enums), ("pkg.py", pkg), ("code.py", code.as_str())];
            assert_eq!(marked(&files, "f").1.len(), usize::from(exhaustive), "{body}");
        }
    }

    #[test]
    fn test_name_that_a_module_both_imports_and_defines_with_a_plain_import() {
        let fallback = "try:\n    import extlib as fail\nexcept ImportError:\n    from typing import NoReturn\n\n    def fail(m) -> NoReturn:\n        raise E(m)\n";
        let code = "from compat import fail\n\ndef f(x):\n    fail(x)\n";
        let files = [("compat.py", fallback), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, Vec::<String>::new());
        let plain = "from typing import NoReturn\n\ndef fail(m) -> NoReturn:\n    raise E(m)\n";
        let files = [("compat.py", plain), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, ["fail(x)"]);
    }

    #[test]
    fn test_no_return_method_of_a_class_defined_twice() {
        let helpers = "import sys\nfrom typing import NoReturn\n\nif sys.version_info >= (3, 11):\n    class K:\n        @staticmethod\n        def s(m):\n            return None\nelse:\n    class K:\n        @staticmethod\n        def s(m) -> NoReturn:\n            raise E(m)\n";
        let code = "from helpers import K\n\ndef f(x):\n    K.s(x)\n";
        let files = [("helpers.py", helpers), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, Vec::<String>::new());
        let once = "from typing import NoReturn\n\nclass K:\n    @staticmethod\n    def s(m) -> NoReturn:\n        raise E(m)\n";
        let files = [("helpers.py", once), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, ["K.s(x)"]);
    }

    #[test]
    fn test_name_that_a_module_both_imports_and_defines() {
        // `try: from extlib import X / except ImportError: <define X>`: the
        // imported object may be the one in use.
        let fallback = "try:\n    from extlib import fail, Color\nexcept ImportError:\n    from enum import Enum\n    from typing import NoReturn\n\n    def fail(m) -> NoReturn:\n        raise E(m)\n\n    class Color(Enum):\n        RED = 1\n        GREEN = 2\n";
        let code = "from compat import fail, Color\n\ndef f(x):\n    fail(x)\n\ndef g(c: Color):\n    match c:\n        case Color.RED | Color.GREEN:\n            return 1\n";
        let files = [("compat.py", fallback), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, Vec::<String>::new());
        assert_eq!(marked(&files, "g").1, Vec::<String>::new());
    }

    #[test]
    fn test_enum_with_a_member_method() {
        // `@enum.member` makes a method a member, so the list is not certain.
        let enums = |deco: &str| format!("from enum import Enum, member\n\nclass E(Enum):\n    A = 1\n    {deco}\n    def B(self):\n        return 2\n");
        let code = "from enums import E\n\ndef f(e: E):\n    match e:\n        case E.A:\n            return 1\n";
        for deco in ["@member", "@enum.member", "@member()"] {
            let e = enums(deco);
            let files = [("enums.py", e.as_str()), ("code.py", code)];
            assert_eq!(marked(&files, "f").1, Vec::<String>::new(), "{deco}");
        }
        let e = enums("@staticmethod");
        let files = [("enums.py", e.as_str()), ("code.py", code)];
        assert_eq!(marked(&files, "f").1, ["match e:"]);
    }

    #[test]
    fn test_no_return_stub_and_project_typing_module() {
        // An `@overload` stub says nothing about the call.
        let src = "from typing import NoReturn, overload\n\n@overload\ndef a(m) -> NoReturn: ...\n\ndef f(x):\n    a(x)\n";
        assert_eq!(calls(src, "f"), Vec::<String>::new());
        let plain = src.replace("@overload\n", "").replace(", overload", "");
        assert_eq!(calls(&plain, "f"), ["a(x)"]);
        // A project module named `typing` is not the standard library.
        let code = "from typing import NoReturn\n\ndef a(m) -> NoReturn:\n    raise E(m)\n\ndef f(x):\n    a(x)\n";
        let files = [("typing.py", "NoReturn = int\n"), ("code.py", code)];
        assert_eq!(marked(&files, "f").0, Vec::<String>::new());
    }

    const COLOR: &str = "from enum import Enum, Flag, IntFlag, auto\nfrom typing import Optional\n\nclass Color(Enum):\n    \"\"\"Colors.\"\"\"\n    RED = 1\n    GREEN = auto()\n    BLUE = 3\n    _hidden_ = 4\n    def label(self):\n        return self.name\n\nclass Perm(Flag):\n    R = 1\n    W = 2\n\n";

    fn matches(body: &str, func: &str) -> Vec<String> {
        marked(&[("code.py", &format!("{COLOR}{body}"))], func).1
    }

    #[test]
    fn test_match_over_every_enum_member() {
        let body = "def f(c: Color):\n    match c:\n        case Color.RED | Color.GREEN:\n            return 1\n        case (Color.BLUE as b):\n            return 2\n\ndef g(c: 'Color'):\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n";
        assert_eq!(matches(body, "f"), ["match c:"]);
        assert_eq!(matches(body, "g"), ["match c:"]);
    }

    #[test]
    fn test_match_that_may_miss_a_value() {
        let cases = [
            // A member without a case.
            "def f(c: Color):\n    match c:\n        case Color.RED | Color.GREEN:\n            return 1\n",
            // A guarded case.
            "def f(c: Color, ok):\n    match c:\n        case Color.RED | Color.GREEN:\n            return 1\n        case Color.BLUE if ok:\n            return 2\n",
            // A flag.
            "def f(p: Perm):\n    match p:\n        case Perm.R | Perm.W:\n            return 1\n",
            // None.
            "def f(c: Optional[Color]):\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
            "def f(c: Color = None):\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
            // The subject rebound.
            "def f(c: Color, raw):\n    c = raw\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
            // The enum name rebound locally.
            "def f(c: Color, Other):\n    Color = Other\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
            // Not a single value.
            "def f(*c: Color):\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
            "def f(**c: Color):\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
            // Not a parameter.
            "def f(raw):\n    c = Color(raw)\n    match c:\n        case Color.RED | Color.GREEN | Color.BLUE:\n            return 1\n",
        ];
        for body in cases {
            assert_eq!(matches(body, "f"), Vec::<String>::new(), "{body}");
        }
    }

    #[test]
    fn test_enum_whose_members_are_not_certain() {
        let classes = [
            "class E(Enum):\n    A = 1\n    B: int = 2\n",
            "class E(Enum):\n    _ignore_ = ['B']\n    A = 1\n",
            "class E(Enum):\n    A = 1\n    def __eq__(self, other):\n        return False\n",
            "class E(Enum):\n    A = 1\n    if X:\n        B = 2\n",
            "class Base(Enum):\n    pass\n\nclass E(Base):\n    A = 1\n",
            "class E(float, Enum):\n    A = float('nan')\n",
            "class E(Enum, metaclass=Meta):\n    A = 1\n",
            "@decorate\nclass E(Enum):\n    A = 1\n",
        ];
        let f = "\ndef f(e: E):\n    match e:\n        case E.A:\n            return 1\n";
        for class in classes {
            let src = format!("from enum import Enum\n\n{class}{f}");
            assert_eq!(marked(&[("code.py", &src)], "f").1, Vec::<String>::new(), "{class}");
        }
        // An enum without members: the value may be a subclass's member.
        let src = "from enum import Enum\n\nclass Base(Enum):\n    pass\n\nclass E(Base):\n    A = 1\n\ndef f(b: Base):\n    match b:\n        case E.A:\n            return 1\n";
        assert_eq!(marked(&[("code.py", src)], "f").1, Vec::<String>::new());
        // A project class named `Enum` may define any equality.
        let files = [
            ("helpers.py", "class Enum:\n    pass\n"),
            ("code.py", &format!("from helpers import Enum\n\nclass E(Enum):\n    A = 1\n{f}")),
        ];
        assert_eq!(marked(&files, "f").1, Vec::<String>::new());
        let src = format!("from enum import Enum, unique\n\n@unique\nclass E(str, Enum):\n    A = 'a'\n{f}");
        assert_eq!(marked(&[("code.py", &src)], "f").1, ["match e:"]);
    }
}
