//! Property-based tests for edge_discovery.rs.
//!
//! Tests edge discovery from Django ORM patterns:
//! - N keyword args in Model.objects.create() → N writes_to edges
//! - Model(field=val) constructor produces equivalent edges
//! - Case determines edge type: uppercase=model, lowercase=function

mod test_helpers;

use crosscheck_contracts::edge_discovery::discover_edges;
use proptest::prelude::*;
use test_helpers::parse_python_stmts;

proptest! {
    /// N keyword arguments in Model.objects.create() produce exactly N writes_to edges.
    #[test]
    fn prop_create_kwargs_count(n in 1..8usize) {
        let fields: Vec<String> = (0..n).map(|i| format!("f{i}=x")).collect();
        let source = format!(
            "def f(x):\n    Model.objects.create({})",
            fields.join(", ")
        );
        let stmts = parse_python_stmts(&source);
        let edges = discover_edges(&stmts);

        let writes_to: Vec<_> = edges
            .iter()
            .filter(|e| e.relationship == "writes_to")
            .collect();

        prop_assert_eq!(writes_to.len(), n);

        // Verify all edges target "Model"
        for edge in &writes_to {
            prop_assert_eq!(&edge.target_name, "Model");
            prop_assert_eq!(&edge.source_function, "f");
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
            "def f(x):\n    Model.objects.create({fields_str})"
        );
        let ctor_src = format!(
            "def f(x):\n    Model({fields_str})"
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

// -- Case-based edge type tests --

#[test]
fn test_uppercase_name_is_model_constructor() {
    let source = "def f(x):\n    Record(field=x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let writes = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .count();
    assert_eq!(writes, 1, "uppercase name should produce writes_to edge");
}

#[test]
fn test_lowercase_name_is_function_call() {
    let source = "def f(x):\n    helper(x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let calls: Vec<_> = edges
        .iter()
        .filter(|e| e.relationship == "calls")
        .collect();
    assert_eq!(calls.len(), 1, "lowercase name should produce calls edge");
    assert_eq!(calls[0].target_name, "helper");
}

#[test]
fn test_underscore_prefix_is_function_call() {
    // _Foo starts with underscore, should be treated as function call, not model
    let source = "def f(x):\n    _process(x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let calls = edges
        .iter()
        .filter(|e| e.relationship == "calls")
        .count();
    assert_eq!(
        calls, 1,
        "underscore-prefixed name should be treated as function call"
    );
}

#[test]
fn test_objects_filter_not_matched() {
    // Model.objects.filter should NOT produce edges — only .create() is matched
    let source = "def f():\n    Model.objects.filter(field=x)";
    let stmts = parse_python_stmts(source);
    let edges = discover_edges(&stmts);

    let writes = edges
        .iter()
        .filter(|e| e.relationship == "writes_to")
        .count();
    assert_eq!(writes, 0, "objects.filter should not produce writes_to edges");
}

#[test]
fn test_edges_in_control_flow() {
    let source = "def f(x):\n    if x:\n        Model.objects.create(a=x)\n    else:\n        Model.objects.create(b=x)";
    let stmts = parse_python_stmts(source);
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
