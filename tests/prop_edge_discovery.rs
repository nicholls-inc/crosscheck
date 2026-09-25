//! Property-based tests for edge_discovery.rs.
//!
//! Tests edge discovery from Django ORM patterns:
//! - N keyword args in Model.objects.create() → N writes_to edges
//! - Model(field=val) constructor produces equivalent edges
//! - Edge kind follows name resolution: a project class is a write target,
//!   a project function a call target, anything unresolved gives no edge

mod test_helpers;

use crosscheck_contracts::edge_discovery::discover_edges;
use proptest::prelude::*;
use test_helpers::parse_python_stmts;

/// A Django model `Model` with integer fields `f0`..`f{n-1}`, `a` and `b`.
fn model_source(n: usize) -> String {
    let mut src = String::from("from django.db import models\nclass Model(models.Model):\n");
    for i in 0..n {
        src.push_str(&format!("    f{i} = models.IntegerField()\n"));
    }
    src.push_str("    a = models.IntegerField()\n    b = models.IntegerField()\n");
    src
}

proptest! {
    /// N keyword arguments in Model.objects.create() produce exactly N writes_to edges.
    #[test]
    fn prop_create_kwargs_count(n in 1..8usize) {
        let fields: Vec<String> = (0..n).map(|i| format!("f{i}=x")).collect();
        let source = format!(
            "{}def f(x):\n    Model.objects.create({})",
            model_source(n),
            fields.join(", ")
        );
        let stmts = parse_python_stmts(&source);
        let edges = discover_edges(&stmts);

        let writes_to: Vec<_> = edges
            .iter()
            .filter(|e| e.relationship == "writes_to")
            .collect();

        prop_assert_eq!(writes_to.len(), n);

        // Verify all edges target "Model" and come from `f` with an override
        // (the written value is a parameter, not another function's result)
        for edge in &writes_to {
            prop_assert_eq!(&edge.target_name, "Model");
            prop_assert_eq!(&edge.source_function, "f");
            prop_assert!(edge.override_rows.is_some());
        }

        // Verify field names are correct
        let field_names: Vec<String> = writes_to
            .iter()
            .filter_map(|e| e.target_field.clone())
            .collect();
        for i in 0..n {
            prop_assert!(field_names.contains(&format!("f{i}")),
                "missing field f{i} in {:?}", field_names);
        }
    }

    /// Model constructor call produces the same edges as Model.objects.create().
    #[test]
    fn prop_constructor_equivalent(n in 1..6usize) {
        let fields: Vec<String> = (0..n).map(|i| format!("f{i}=x")).collect();
        let fields_str = fields.join(", ");

        let create_src = format!(
            "{}def f(x):\n    Model.objects.create({fields_str})", model_source(n)
        );
        let ctor_src = format!(
            "{}def f(x):\n    Model({fields_str})", model_source(n)
        );

        let create_stmts = parse_python_stmts(&create_src);
        let ctor_stmts = parse_python_stmts(&ctor_src);

        let create_edges = discover_edges(&create_stmts);
        let ctor_edges = discover_edges(&ctor_stmts);

        let create_writes: Vec<_> = create_edges.iter()
            .filter(|e| e.relationship == "writes_to")
            .collect();
        let ctor_writes: Vec<_> = ctor_edges.iter()
            .filter(|e| e.relationship == "writes_to")
            .collect();

        prop_assert_eq!(create_writes.len(), n);
        prop_assert_eq!(create_writes.len(), ctor_writes.len(),
            "constructor and objects.create should produce same number of writes_to edges");

        // Same target fields
        let mut create_fields: Vec<_> = create_writes.iter()
            .filter_map(|e| e.target_field.as_deref())
            .collect();
        let mut ctor_fields: Vec<_> = ctor_writes.iter()
            .filter_map(|e| e.target_field.as_deref())
            .collect();
        create_fields.sort();
        ctor_fields.sort();
        prop_assert_eq!(create_fields, ctor_fields);
    }
}

// -- Resolution-based edge type tests --

#[test]
fn test_project_class_is_write_target() {
    let source = "from dataclasses import dataclass\n@dataclass\nclass Record:\n    field: int\ndef f(x):\n    Record(field=x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let writes = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .count();
    assert_eq!(writes, 1, "a data class constructor should produce a writes_to edge");
}

#[test]
fn test_unresolved_uppercase_name_gives_no_edge() {
    let source = "def f(x):\n    Record(field=x)";
    let edges = discover_edges(&parse_python_stmts(source));
    assert!(edges.is_empty(), "an undefined class is not a write target: {edges:?}");
}

#[test]
fn test_project_function_is_call_target() {
    let source = "def helper(y):\n    return y\ndef f(x):\n    helper(x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let calls: Vec<_> = edges
        .iter()
        .filter(|e| e.relationship == "calls")
        .collect();
    assert_eq!(calls.len(), 1, "a project function call should produce a calls edge");
    assert_eq!(calls[0].target_name, "helper");
    let flows: Vec<_> = edges.iter().filter(|e| e.relationship == "flows_to").collect();
    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].target_param.as_deref(), Some("y"));
}

#[test]
fn test_underscore_prefix_is_function_call() {
    let source = "def _process(y):\n    pass\ndef f(x):\n    _process(x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let calls = edges
        .iter()
        .filter(|e| e.relationship == "calls")
        .count();
    assert_eq!(
        calls, 1,
        "underscore-prefixed function should be treated as function call"
    );
}

#[test]
fn test_objects_filter_not_matched() {
    // Model.objects.filter should NOT produce edges — only .create() is matched
    let source = format!("{}def f():\n    Model.objects.filter(a=x)", model_source(0));
    let stmts = parse_python_stmts(&source);
    let edges = discover_edges(&stmts);

    let writes = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .count();
    assert_eq!(writes, 0, "objects.filter should not produce writes_to edges");
}

#[test]
fn test_edges_in_control_flow() {
    let source = format!(
        "{}def f(x):\n    if x:\n        Model.objects.create(a=x)\n    else:\n        Model.objects.create(b=x)",
        model_source(0)
    );
    let stmts = parse_python_stmts(&source);
    let edges = discover_edges(&stmts);

    let writes: Vec<_> = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .collect();
    assert_eq!(
        writes.len(),
        2,
        "should discover edges in both branches of if/else"
    );
}
