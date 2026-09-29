//! Assertion sites: `<runtime read> as T`, their enclosing function, and
//! the guard evidence that discharges a property of `T`.

use std::collections::HashMap;

use oxc_allocator::Vec as ArenaVec;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::types::{self, FileId, Shape, TypeIndex};
use super::ParsedModule;
use crate::value_analysis::ValueFacts;

/// The runtime reads whose result tsc types as `any` or `string | null`.
/// This table is the whole policy of where tsc's guarantee is void.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSource {
    /// `JSON.parse(..)`: `any`, guarantees nothing.
    JsonParse,
    /// `localStorage.getItem(..)` / `sessionStorage.getItem(..)`: `string | null`.
    StorageGetItem,
    /// `<x>.searchParams.get(..)`, `new URLSearchParams(..).get(..)`,
    /// `searchParams.get(..)`: `string | null`.
    SearchParamsGet,
}

impl RuntimeSource {
    /// The documented result as value facts (the edge's guarantee).
    pub fn facts(self) -> ValueFacts {
        match self {
            RuntimeSource::JsonParse => ValueFacts::default(),
            RuntimeSource::StorageGetItem | RuntimeSource::SearchParamsGet => ValueFacts {
                nullable: Some(true),
                ..Default::default()
            },
        }
    }

    /// Match a call against the table by callee shape.
    pub fn of_call(call: &CallExpression<'_>) -> Option<RuntimeSource> {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return None;
        };
        let object = member.object.get_inner_expression();
        match member.property.name.as_str() {
            "parse" if object.is_specific_id("JSON") => Some(RuntimeSource::JsonParse),
            "getItem" if matches!(last_name(object), Some("localStorage" | "sessionStorage")) => {
                Some(RuntimeSource::StorageGetItem)
            }
            "get" => {
                let params = match object {
                    Expression::NewExpression(n) => n.callee.is_specific_id("URLSearchParams"),
                    o => last_name(o) == Some("searchParams"),
                };
                params.then_some(RuntimeSource::SearchParamsGet)
            }
            _ => None,
        }
    }
}

/// The last identifier of `a`, `a.b` or `a.b.c`.
fn last_name<'x>(expr: &'x Expression<'_>) -> Option<&'x str> {
    match expr {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(m) => Some(m.property.name.as_str()),
        _ => None,
    }
}

/// The enclosing function of a site (a path head).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRef {
    pub qualified: String,
    pub short: String,
    pub file: String,
    pub line: u32,
}

/// The asserted type of a site.
#[derive(Debug)]
pub struct Target {
    /// A named alias / enum / interface, or the type's source text.
    pub name: String,
    pub file: FileId,
    pub line: u32,
    pub shape: Shape,
}

/// One `<runtime read> as T`.
#[derive(Debug)]
pub struct Site {
    pub function: FunctionRef,
    pub line: u32,
    pub source: RuntimeSource,
    pub target: Target,
    /// Per property of `T`: what a guard on the cast's binding proves.
    pub guard: HashMap<String, ValueFacts>,
}

/// Every site in one module (not a `.d.ts`).
pub fn collect<'a>(index: &TypeIndex<'a>, file: FileId, module: &ParsedModule<'a>) -> Vec<Site> {
    // Real scopes: every identifier reference resolves to the binding it names,
    // so no shadowing, hoisting or block rule is modelled here.
    let scoping = SemanticBuilder::new().build(&module.program).semantic.into_scoping();
    let mut v = SiteVisitor {
        index,
        file,
        module,
        scoping,
        sources: HashMap::new(),
        typed: HashMap::new(),
        frames: vec![FunctionRef {
            qualified: format!("{}.<module>", module.rel),
            short: format!("<module {}>", module.rel),
            file: module.rel.clone(),
            line: 1,
        }],
        guards: HashMap::new(),
        sites: Vec::new(),
    };
    v.visit_program(&module.program);
    v.sites
}

/// Functions whose first argument is the function they wrap.
const WRAPPERS: [&str; 4] = ["useCallback", "useMemo", "memo", "forwardRef"];

struct SiteVisitor<'i, 'a> {
    index: &'i TypeIndex<'a>,
    file: FileId,
    module: &'i ParsedModule<'a>,
    scoping: Scoping,
    /// `const v = JSON.parse(..)` bindings.
    sources: HashMap<SymbolId, RuntimeSource>,
    /// Parameters and variables with an annotation: what the binding guarantees.
    typed: HashMap<SymbolId, ValueFacts>,
    /// Enclosing functions, innermost last: sites are attributed to the top.
    frames: Vec<FunctionRef>,
    /// Guard evidence per cast (by the cast's span start), found from the
    /// statement after `const v = <cast>`.
    guards: HashMap<u32, HashMap<String, ValueFacts>>,
    sites: Vec<Site>,
}

impl<'a> SiteVisitor<'_, 'a> {
    fn line(&self, offset: u32) -> u32 {
        self.module.lines.line(offset)
    }

    fn text(&self, span: Span) -> String {
        self.module.source[span.start as usize..span.end as usize]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Enter a named function. Its qualified name carries the line, so two
    /// same-named functions in one file (methods of two classes) stay two nodes.
    fn push_function(&mut self, name: &str, span: Span, params: Option<&FormalParameters<'a>>) {
        let line = self.line(span.start);
        self.frames.push(FunctionRef {
            qualified: format!("{}.{name}@{line}", self.module.rel),
            short: name.to_string(),
            file: self.module.rel.clone(),
            line,
        });
        self.record_params(params);
    }

    /// Enter an anonymous function, attributed to the enclosing named function.
    fn push_anonymous(&mut self, params: Option<&FormalParameters<'a>>) {
        let function = self.frames.last().expect("module frame").clone();
        self.frames.push(function);
        self.record_params(params);
    }

    fn record_params(&mut self, params: Option<&FormalParameters<'a>>) {
        for p in params.into_iter().flat_map(|p| p.items.iter()) {
            let (BindingPattern::BindingIdentifier(id), Some(ann)) = (&p.pattern, &p.type_annotation) else {
                continue;
            };
            let mut facts = self.index.resolve(self.file, &ann.type_annotation).facts();
            if p.optional {
                facts.nullable = Some(true);
            }
            self.typed.insert(id.symbol_id(), facts);
        }
    }

    /// The binding a reference resolves to; `None` for a global.
    fn symbol(&self, id: &IdentifierReference<'a>) -> Option<SymbolId> {
        self.scoping.get_reference(id.reference_id()).symbol_id()
    }

    /// The runtime source behind a cast operand: the call itself, or a
    /// `const` bound to one; `as unknown` and parentheses are unwrapped.
    fn site_source(&self, expr: &Expression<'a>) -> Option<RuntimeSource> {
        let mut expr = expr;
        loop {
            expr = expr.without_parentheses();
            match expr {
                Expression::TSAsExpression(a) if matches!(a.type_annotation, TSType::TSUnknownKeyword(_)) => {
                    expr = &a.expression;
                }
                Expression::CallExpression(c) => return RuntimeSource::of_call(c),
                Expression::Identifier(id) => {
                    return self.symbol(id).and_then(|s| self.sources.get(&s).copied());
                }
                _ => return None,
            }
        }
    }

    fn target(&self, ty: &TSType<'a>) -> Target {
        let mut ty = ty;
        loop {
            match ty {
                TSType::TSParenthesizedType(p) => ty = &p.type_annotation,
                TSType::TSArrayType(a) => ty = &a.element_type,
                TSType::TSUnionType(u) => {
                    let mut rest = u.types.iter().filter(|t| !types::is_null_type(t));
                    let (Some(mut only), None) = (rest.next(), rest.next()) else {
                        break;
                    };
                    while let TSType::TSParenthesizedType(p) = only {
                        only = &p.type_annotation;
                    }
                    let TSType::TSArrayType(a) = only else {
                        break;
                    };
                    ty = &a.element_type;
                }
                _ => break,
            }
        }
        let shape = self.index.resolve(self.file, ty);
        if let TSType::TSTypeReference(r) = ty {
            if let (TSTypeName::IdentifierReference(id), None) = (&r.type_name, &r.type_arguments) {
                if let Some((file, name, line)) = self.index.declaration(self.file, id.name.as_str()) {
                    return Target { name, file, line, shape };
                }
            }
        }
        Target {
            name: self.text(ty.span()),
            file: self.file,
            line: self.line(ty.span().start),
            shape,
        }
    }

    /// What comparing against `x` proves: the annotation of a parameter or
    /// `const`, or the literal itself.
    fn operand_facts(&self, x: &Expression<'a>) -> Option<ValueFacts> {
        let one = |s: String| ValueFacts {
            nullable: Some(false),
            choices: Some(vec![s]),
            ..Default::default()
        };
        match x.without_parentheses() {
            Expression::Identifier(id) => self.symbol(id).and_then(|s| self.typed.get(&s).cloned()),
            Expression::StringLiteral(s) => Some(one(s.value.as_str().to_string())),
            Expression::NumericLiteral(n) => Some(one(types::numeric_text(n))),
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => Some(one(
                t.quasis
                    .iter()
                    .map(|q| q.value.cooked.map(|c| c.as_str().to_string()).unwrap_or_default())
                    .collect(),
            )),
            Expression::BooleanLiteral(_) => Some(ValueFacts::non_null()),
            _ => None,
        }
    }

    /// `v.p === x` / `x === v.p` (`equal`), or the same with `!==`: (p, x, equal).
    fn comparison<'x>(v: &str, test: &'x Expression<'a>) -> Option<(String, &'x Expression<'a>, bool)> {
        let Expression::BinaryExpression(b) = test.without_parentheses() else {
            return None;
        };
        let equal = match b.operator {
            BinaryOperator::StrictEquality => true,
            BinaryOperator::StrictInequality => false,
            _ => return None,
        };
        let property = |e: &Expression<'a>| match e.without_parentheses() {
            Expression::StaticMemberExpression(m)
                if !m.optional && m.object.without_parentheses().is_specific_id(v) =>
            {
                Some(m.property.name.as_str().to_string())
            }
            _ => None,
        };
        if let Some(p) = property(&b.left) {
            Some((p, &b.right, equal))
        } else {
            property(&b.right).map(|p| (p, &b.left, equal))
        }
    }

    /// Evidence from the statement right after `const v = <cast>`:
    /// `return v.p === x ? v : null` (either operand order, or `!==` with
    /// the branches swapped) or `if (v.p !== x) return null;`.
    fn guard_evidence(&self, v: &str, next: &Statement<'a>) -> Option<HashMap<String, ValueFacts>> {
        let is_nullish = |e: &Expression<'a>| {
            let e = e.without_parentheses();
            e.is_null() || e.is_undefined()
        };
        let (p, x) = match next {
            Statement::ReturnStatement(r) => {
                let Expression::ConditionalExpression(c) = r.argument.as_ref()?.without_parentheses() else {
                    return None;
                };
                let (p, x, equal) = Self::comparison(v, &c.test)?;
                let (kept, dropped) = if equal {
                    (&c.consequent, &c.alternate)
                } else {
                    (&c.alternate, &c.consequent)
                };
                if !(kept.without_parentheses().is_specific_id(v) && is_nullish(dropped)) {
                    return None;
                }
                (p, x)
            }
            Statement::IfStatement(i) if i.alternate.is_none() => {
                let (p, x, equal) = Self::comparison(v, &i.test)?;
                let returns_nullish = |s: &Statement<'a>| match s {
                    Statement::ReturnStatement(r) => r.argument.as_ref().is_none_or(is_nullish),
                    Statement::ThrowStatement(_) => true,
                    _ => false,
                };
                let exits = match &i.consequent {
                    Statement::BlockStatement(b) => b.body.len() == 1 && returns_nullish(&b.body[0]),
                    s => returns_nullish(s),
                };
                if equal || !exits {
                    return None;
                }
                (p, x)
            }
            _ => return None,
        };
        let facts = self.operand_facts(x)?;
        Some(HashMap::from([(p, facts)]))
    }
}

/// `const v = <cast>` with one declarator: (cast span start, v).
fn const_cast<'x>(stmt: &'x Statement<'_>) -> Option<(u32, &'x str)> {
    let Statement::VariableDeclaration(d) = stmt else {
        return None;
    };
    if d.kind != VariableDeclarationKind::Const || d.declarations.len() != 1 {
        return None;
    }
    let decl = &d.declarations[0];
    let BindingPattern::BindingIdentifier(id) = &decl.id else {
        return None;
    };
    match decl.init.as_ref()?.without_parentheses() {
        Expression::TSAsExpression(a) => Some((a.span.start, id.name.as_str())),
        _ => None,
    }
}

/// The parameters of the function a declarator names: an arrow or function
/// expression, or one passed to `useCallback` / `useMemo` / `memo` / `forwardRef`.
fn named_function<'x, 'a>(init: &'x Expression<'a>) -> Option<&'x FormalParameters<'a>> {
    match init.without_parentheses() {
        Expression::ArrowFunctionExpression(a) => Some(&a.params),
        Expression::FunctionExpression(f) => Some(&f.params),
        Expression::CallExpression(c) => {
            let wrapper = match c.callee.get_inner_expression() {
                Expression::Identifier(id) => id.name.as_str(),
                Expression::StaticMemberExpression(m) => m.property.name.as_str(),
                _ => return None,
            };
            if !WRAPPERS.contains(&wrapper) {
                return None;
            }
            match c.arguments.first()?.as_expression()?.without_parentheses() {
                Expression::ArrowFunctionExpression(a) => Some(&a.params),
                Expression::FunctionExpression(f) => Some(&f.params),
                _ => None,
            }
        }
        _ => None,
    }
}

impl<'a> Visit<'a> for SiteVisitor<'_, 'a> {
    fn visit_statements(&mut self, stmts: &ArenaVec<'a, Statement<'a>>) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let Some((cast, v)) = const_cast(stmt) {
                if let Some(guard) = stmts.get(i + 1).and_then(|next| self.guard_evidence(v, next)) {
                    self.guards.insert(cast, guard);
                }
            }
            self.visit_statement(stmt);
        }
    }

    fn visit_declaration(&mut self, decl: &Declaration<'a>) {
        match decl {
            Declaration::FunctionDeclaration(f) if f.id.is_some() => {
                let name = f.id.as_ref().map(|i| i.name.as_str().to_string()).unwrap_or_default();
                self.push_function(&name, f.span, Some(&f.params));
                walk::walk_declaration(self, decl);
                self.frames.pop();
            }
            _ => walk::walk_declaration(self, decl),
        }
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        if let ExportDefaultDeclarationKind::FunctionDeclaration(f) = &decl.declaration {
            let name = f.id.as_ref().map_or("default", |i| i.name.as_str()).to_string();
            self.push_function(&name, f.span, Some(&f.params));
            walk::walk_export_default_declaration(self, decl);
            self.frames.pop();
        } else {
            walk::walk_export_default_declaration(self, decl);
        }
    }

    fn visit_method_definition(&mut self, m: &MethodDefinition<'a>) {
        match m.key.static_name() {
            Some(name) => {
                self.push_function(&name, m.span, Some(&m.value.params));
                walk::walk_method_definition(self, m);
                self.frames.pop();
            }
            None => walk::walk_method_definition(self, m),
        }
    }

    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        for d in &decl.declarations {
            let BindingPattern::BindingIdentifier(id) = &d.id else {
                self.visit_variable_declarator(d);
                continue;
            };
            let name = id.name.as_str().to_string();
            if let Some(ann) = &d.type_annotation {
                let facts = self.index.resolve(self.file, &ann.type_annotation).facts();
                self.typed.insert(id.symbol_id(), facts);
            }
            let init = d.init.as_ref();
            // Only JSON.parse: tsc narrows a `string | null` binding by control
            // flow (`if (!raw) return;`), which this frontend does not model,
            // while nothing narrows `any` into the asserted type.
            if decl.kind == VariableDeclarationKind::Const {
                if let Some(Expression::CallExpression(c)) = init.map(Expression::without_parentheses) {
                    if RuntimeSource::of_call(c) == Some(RuntimeSource::JsonParse) {
                        self.sources.insert(id.symbol_id(), RuntimeSource::JsonParse);
                    }
                }
            }
            match init.and_then(named_function) {
                Some(params) => {
                    self.push_function(&name, d.span, Some(params));
                    self.visit_variable_declarator(d);
                    self.frames.pop();
                }
                None => self.visit_variable_declarator(d),
            }
        }
    }

    fn visit_function(&mut self, f: &Function<'a>, flags: ScopeFlags) {
        self.push_anonymous(Some(&f.params));
        walk::walk_function(self, f, flags);
        self.frames.pop();
    }

    fn visit_arrow_function_expression(&mut self, f: &ArrowFunctionExpression<'a>) {
        self.push_anonymous(Some(&f.params));
        walk::walk_arrow_function_expression(self, f);
        self.frames.pop();
    }

    fn visit_ts_as_expression(&mut self, e: &TSAsExpression<'a>) {
        let intermediate = matches!(e.type_annotation, TSType::TSUnknownKeyword(_) | TSType::TSAnyKeyword(_));
        if let Some(source) = self.site_source(&e.expression).filter(|_| !intermediate) {
            let target = self.target(&e.type_annotation);
            self.sites.push(Site {
                function: self.frames.last().expect("module frame").clone(),
                line: self.line(e.span.start),
                source,
                target,
                guard: self.guards.remove(&e.span.start).unwrap_or_default(),
            });
        }
        walk::walk_ts_as_expression(self, e);
    }
}

#[cfg(test)]
mod tests {
    use crate::ts::{extract_sources, TsGraph};
    use crate::ts::types::TsConfig;

    fn graph(files: &[(&str, &str)]) -> TsGraph {
        extract_sources(
            files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
            std::path::Path::new("/app"),
            &TsConfig::default(),
        )
    }

    /// `function -> slot: facts` per assertion, sorted.
    fn edges(g: &TsGraph) -> Vec<String> {
        let mut out: Vec<String> = g
            .assertions
            .iter()
            .map(|a| {
                let mut facts = Vec::new();
                if let Some(n) = a.facts.nullable {
                    facts.push(format!("nullable={}", n as u8));
                }
                if let Some(c) = &a.facts.choices {
                    facts.push(format!("choices={}", c.join("|")));
                }
                format!("{} -> {} @{}: [{}]", a.function, a.slot, a.site_line, facts.join(", "))
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn runtime_source_matching() {
        let g = graph(&[(
            "m.ts",
            "type T = 'a' | 'b';
export const f = (raw: string, sp: URLSearchParams, req: Request) => {
  const a = JSON.parse(raw) as T;
  const b = localStorage.getItem('k') as T;
  const c = window.sessionStorage.getItem('k') as T;
  const d = sp.get('k') as T;
  const e = new URL(raw).searchParams.get('k') as T;
  const g = new URLSearchParams(raw).get('k') as T;
  const h = searchParams.get('k') as T;
  const i = req.headers.get('k') as T;
  const j = raw as T;
  const k = other.parse(raw) as T;
  return [a, b, c, d, e, g, h, i, j, k];
};",
        )]);
        assert_eq!(
            edges(&g),
            vec![
                "m.ts.f@2 -> m.ts.T @3: []",
                "m.ts.f@2 -> m.ts.T @4: [nullable=1]",
                "m.ts.f@2 -> m.ts.T @5: [nullable=1]",
                "m.ts.f@2 -> m.ts.T @7: [nullable=1]",
                "m.ts.f@2 -> m.ts.T @8: [nullable=1]",
                "m.ts.f@2 -> m.ts.T @9: [nullable=1]",
            ]
        );
        assert_eq!(g.sites, 6);
    }

    #[test]
    fn const_bound_read_and_double_cast_are_sites() {
        let g = graph(&[(
            "m.ts",
            "type T = 'a';
function f() {
  const parsed = JSON.parse('{}');
  const x = parsed as T;
  const y = (JSON.parse('{}') as unknown) as T;
  const z = JSON.parse('{}') as unknown as T;
  return [x, y, z];
}
function g(raw: string) {
  return raw as T;
}",
        )]);
        assert_eq!(
            edges(&g),
            vec![
                "m.ts.f@2 -> m.ts.T @4: []",
                "m.ts.f@2 -> m.ts.T @5: []",
                "m.ts.f@2 -> m.ts.T @6: []",
            ]
        );
    }

    #[test]
    fn storage_bound_consts_narrowed_or_shadowed_are_not_sites() {
        let g = graph(&[(
            "m.ts",
            "type T = 'a';
export function narrowed(): T | null {
  const raw = localStorage.getItem('t');
  if (!raw) return null;
  return raw as T;
}
export function shadow(): T {
  const parsed = JSON.parse('{}');
  return ['x'].map((parsed: string) => parsed as T)[0];
}
export function blockScoped(): T {
  if (Math.random()) { const late = JSON.parse('{}'); void late; }
  const late = 'a';
  return late as T;
}",
        )]);
        assert_eq!(edges(&g), Vec::<String>::new());
    }

    #[test]
    fn optional_or_shadowed_comparand_does_not_discharge() {
        let g = graph(&[(
            "m.ts",
            "interface S { client: string; }
export function optionalParam(client?: string): S | null {
  const s = JSON.parse('') as S;
  return s.client === client ? s : null;
}
export function shadowed(client: string) {
  return (client) => { const s = JSON.parse('') as S; return s.client === client ? s : null; };
}",
        )]);
        assert_eq!(
            edges(&g),
            vec![
                "m.ts.optionalParam@2 -> m.ts.S.client @3: [nullable=1]",
                "m.ts.shadowed@6 -> m.ts.S.client @7: []",
            ]
        );
    }

    #[test]
    fn every_binding_form_shadows_an_outer_comparand() {
        let g = graph(&[(
            "m.ts",
            "interface S { client: string; }
export function outer(client: string, xs: (string | null)[]) {
  const a = ({ client }: { client: string | null }) => { const s = JSON.parse('') as S; return s.client === client ? s : null; };
  const b = (...client: string[]) => { const s = JSON.parse('') as S; return s.client === client ? s : null; };
  try { a({ client: null }); } catch (client) { const s = JSON.parse('') as S; return s.client === client ? s : null; }
  for (const client of xs) { const s = JSON.parse('') as S; return s.client === client ? s : null; }
  { const { client } = { client: xs[0] }; const s = JSON.parse('') as S; return s.client === client ? s : null; }
  return b;
}",
        )]);
        assert_eq!(
            edges(&g),
            vec![
                "m.ts.a@3 -> m.ts.S.client @3: []",
                "m.ts.b@4 -> m.ts.S.client @4: []",
                "m.ts.outer@2 -> m.ts.S.client @5: []",
                "m.ts.outer@2 -> m.ts.S.client @6: []",
                "m.ts.outer@2 -> m.ts.S.client @7: []",
            ]
        );
    }

    #[test]
    fn declarations_and_hoisted_bindings_shadow_an_outer_comparand() {
        let g = graph(&[(
            "m.ts",
            "interface S { client: string; }
export function fnDecl(client: 'a') { { function client() {} const s = JSON.parse('') as S; return s.client === client ? s : null; } }
export function classDecl(client: 'a') { { class client {} const s = JSON.parse('') as S; return s.client === client ? s : null; } }
export function lateConst(client: 'a') { {
  const f = () => { const s = JSON.parse('') as S; return s.client === client ? s : null; };
  const client = 'x' as string | null;
  return f; }
}
export function varInBlock(client: 'a', c: boolean) {
  return () => { if (c) { var client = null; } const s = JSON.parse('') as S; return s.client === client ? s : null; };
}",
        )]);
        assert_eq!(
            edges(&g),
            vec![
                "m.ts.classDecl@3 -> m.ts.S.client @3: []",
                "m.ts.f@5 -> m.ts.S.client @5: []",
                "m.ts.fnDecl@2 -> m.ts.S.client @2: []",
                "m.ts.varInBlock@9 -> m.ts.S.client @10: []",
            ]
        );
    }

    #[test]
    fn block_bindings_stay_in_their_block() {
        let g = graph(&[(
            "m.ts",
            "interface S { client: string; }
declare const client: string | null;
declare const d: string;
export function leak(c: boolean, x: string) {
  if (c) { const client: 'a' = 'a'; }
  const s = JSON.parse(x) as S;
  return s.client === client ? s : null;
}
export function leakSrc(c: boolean, x: string) {
  if (c) { const d = JSON.parse(x); }
  return d as S;
}
export function sib(c: boolean, x: string) {
  if (c) { const d = JSON.parse(x); return d as S; }
  else { const d = JSON.parse(x); return d as S; }
}",
        )]);
        assert_eq!(
            edges(&g),
            vec![
                "m.ts.leak@4 -> m.ts.S.client @6: [nullable=1]",
                "m.ts.sib@13 -> m.ts.S.client @14: []",
                "m.ts.sib@13 -> m.ts.S.client @15: []",
            ]
        );
    }

    #[test]
    fn same_named_methods_are_distinct_functions() {
        let g = graph(&[(
            "a.ts",
            "type T = 'x';
class A { load() { return JSON.parse('') as T; } }
class B { load() { return localStorage.getItem('a') as T; } }",
        )]);
        assert_eq!(
            edges(&g),
            vec!["a.ts.load@2 -> a.ts.T @2: []", "a.ts.load@3 -> a.ts.T @3: [nullable=1]"]
        );
        assert_eq!(g.functions.len(), 2);
    }

    #[test]
    fn enclosing_function_names() {
        let g = graph(&[(
            "a/b.tsx",
            "type T = 'a';
const top = JSON.parse('') as T;
export function decl() { return JSON.parse('') as T; }
export const arrow = () => JSON.parse('') as T;
const hooked = useCallback(() => [1].map(() => JSON.parse('') as T), []);
class C { method() { return JSON.parse('') as T; } }
export default function () { return JSON.parse('') as T; }",
        )]);
        let mut functions: Vec<String> = g.functions.iter().map(|f| format!("{} ({}:{})", f.short, f.qualified, f.line)).collect();
        functions.sort();
        assert_eq!(
            functions,
            vec![
                "<module a/b.tsx> (a/b.tsx.<module>:1)",
                "arrow (a/b.tsx.arrow@4:4)",
                "decl (a/b.tsx.decl@3:3)",
                "default (a/b.tsx.default@7:7)",
                "hooked (a/b.tsx.hooked@5:5)",
                "method (a/b.tsx.method@6:6)",
            ]
        );
    }

    #[test]
    fn guard_discharges_property() {
        let src = |guard: &str| {
            format!(
                "interface S {{ client: string; origin: 'login' | 'signup'; extra?: string; }}
export const peek = (client: string): S | null => {{
  const s = JSON.parse('') as S;
  {guard}
}};"
            )
        };
        let forms = [
            "return s.client === client ? s : null;",
            "return client === s.client ? s : null;",
            "return s.client !== client ? null : s;",
            "return s.client !== client ? undefined : s;",
            "if (s.client !== client) return null;\n  return s;",
            "if (s.client !== client) { return; }\n  return s;",
            "if (s.client !== client) throw new Error('x');\n  return s;",
        ];
        for form in forms {
            let g = graph(&[("m.ts", &src(form))]);
            assert_eq!(
                edges(&g),
                vec!["m.ts.peek@2 -> m.ts.S.client @3: [nullable=0]", "m.ts.peek@2 -> m.ts.S.origin @3: []"],
                "{form}"
            );
        }
    }

    #[test]
    fn guard_operand_kinds() {
        let cases = [
            ("'x'", "[nullable=0, choices=x]"),
            ("1", "[nullable=0, choices=1]"),
            ("KEY", "[nullable=0, choices=k]"),
            ("mode", "[nullable=0, choices=a|b]"),
            ("maybe", "[nullable=1]"),
            ("untyped", "[]"),
        ];
        for (operand, expected) in cases {
            let g = graph(&[(
                "m.ts",
                &format!(
                    "interface S {{ client: string; }}
const KEY: 'k' = 'k';
const untyped = 'u';
export function peek(mode: 'a' | 'b', maybe: string | null) {{
  const s = JSON.parse('') as S;
  return s.client === {operand} ? s : null;
}}"
                ),
            )]);
            assert_eq!(edges(&g), vec![format!("m.ts.peek@4 -> m.ts.S.client @5: {expected}")], "{operand}");
        }
    }

    #[test]
    fn non_dominating_guard_gives_nothing() {
        let forms = [
            "if (flag) { if (s.client !== client) return null; }\n  return s;",
            "return flag ? s : (s.client === client ? s : null);",
            "if (s.client !== client) { log(s); return null; }\n  return s;",
            "if (s.client !== client) return null; else return s;",
            "if (s.client === client) return s;\n  return null;",
            "log(s);\n  return s.client === client ? s : null;",
            "return s.client == client ? s : null;",
            "return s?.client === client ? s : null;",
            "return s.client === client ? s : s;",
        ];
        for form in forms {
            let g = graph(&[(
                "m.ts",
                &format!(
                    "interface S {{ client: string; }}
export const peek = (client: string, flag: boolean) => {{
  const s = JSON.parse('') as S;
  {form}
}};"
                ),
            )]);
            assert_eq!(edges(&g), vec!["m.ts.peek@2 -> m.ts.S.client @3: []"], "{form}");
        }
    }
}
