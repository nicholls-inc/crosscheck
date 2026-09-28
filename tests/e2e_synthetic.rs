//! E2E tests with inline synthetic fixtures for edge cases.
//!
//! These tests construct Python files in a TempDir, run the full extraction
//! pipeline, and verify the resulting SQLite database.

mod test_helpers;

use std::fs;
use test_helpers::*;
use tempfile::TempDir;

/// Helper: write a single Python file to a temp dir, extract, return connection.
fn extract_source(source: &str) -> (TempDir, rusqlite::Connection) {
    let fixture_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();
    fs::write(fixture_dir.path().join("models.py"), source).unwrap();
    let db_path = db_dir.path().join("contracts.sqlite");
    let result_path = crosscheck_contracts::extractor::extract(
        fixture_dir.path(),
        None,
        "4.2",
        Some(&db_path),
    )
    .unwrap();
    let conn = rusqlite::Connection::open(&result_path).unwrap();
    // Keep both TempDirs alive by returning fixture_dir (db_dir is kept by Connection)
    (fixture_dir, conn)
}

/// Helper: write models.py and utils.py to a temp dir, extract, return connection.
fn extract_sources(models_py: &str, utils_py: &str) -> (TempDir, TempDir, rusqlite::Connection) {
    let fixture_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();
    fs::write(fixture_dir.path().join("models.py"), models_py).unwrap();
    fs::write(fixture_dir.path().join("utils.py"), utils_py).unwrap();
    let db_path = db_dir.path().join("contracts.sqlite");
    let result_path = crosscheck_contracts::extractor::extract(
        fixture_dir.path(),
        None,
        "4.2",
        Some(&db_path),
    )
    .unwrap();
    let conn = rusqlite::Connection::open(&result_path).unwrap();
    (fixture_dir, db_dir, conn)
}

#[test]
fn test_choices_extraction() {
    let source = r#"
from django.db import models

class MyModel(models.Model):
    status = models.CharField(max_length=1, choices=[('A', 'Active'), ('I', 'Inactive')])
"#;
    let (_tmp, conn) = extract_source(source);

    let choices = query_contract_by_type(&conn, "MyModel.status", "choices");
    assert_eq!(choices.len(), 1, "should have a choices contract");
    // A JSON array (round 3); the legacy encoding was the comma list "A,I".
    assert_eq!(choices[0].param_choices.as_deref(), Some(r#"["A","I"]"#));
}

#[test]
fn test_optional_return_nullable() {
    let models = "from django.db import models\nclass M(models.Model):\n    val = models.DecimalField(max_digits=5, decimal_places=2)";
    let utils = r#"
from typing import Optional
from decimal import Decimal

def compute(x: Decimal) -> Optional[Decimal]:
    if x > 0:
        return x
    return None
"#;
    let (_tmp1, _tmp2, conn) = extract_sources(models, utils);

    // Should have nullability postcondition from Optional return type
    let null_contracts = query_contract_by_type(&conn, "compute", "nullability");
    let postconditions: Vec<_> = null_contracts
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .collect();
    assert!(
        !postconditions.is_empty(),
        "Optional[Decimal] return should produce nullability postcondition"
    );

    // Optional[Decimal] produces a Decimal type postcondition for the non-None
    // value; the None case is carried by the nullability postcondition above.
    let type_contracts = query_contract_by_type(&conn, "compute", "type");
    let decimal_post: Vec<_> = type_contracts
        .iter()
        .filter(|c| c.contract_role.as_deref() == Some("postcondition"))
        .filter(|c| c.param_type_name.as_deref() == Some("Decimal"))
        .collect();
    assert_eq!(
        decimal_post.len(),
        1,
        "Optional[Decimal] should produce one Decimal type postcondition"
    );

    // The `return None` path and the Optional annotation give one nullability
    // postcondition, not two.
    assert_eq!(postconditions.len(), 1, "expected a single nullability postcondition");
}

#[test]
fn test_slug_field_default_max_length_e2e() {
    let source = r#"
from django.db import models

class MyModel(models.Model):
    slug = models.SlugField()
"#;
    let (_tmp, conn) = extract_source(source);

    let length = query_contract_by_type(&conn, "MyModel.slug", "length");
    assert_eq!(length.len(), 1, "SlugField should have a length contract");
    assert_eq!(
        length[0].param_max_length,
        Some(50),
        "SlugField default max_length should be 50"
    );
}

#[test]
fn test_explicit_null_overrides_default_e2e() {
    let source = r#"
from django.db import models

class MyModel(models.Model):
    price = models.DecimalField(max_digits=5, decimal_places=2, null=True)
"#;
    let (_tmp, conn) = extract_source(source);

    let null_contracts = query_contract_by_type(&conn, "MyModel.price", "nullability");
    assert_eq!(null_contracts.len(), 1);
    assert_eq!(
        null_contracts[0].param_nullable,
        Some(1),
        "explicit null=True should be reflected"
    );
    assert!(
        !null_contracts[0].is_implicit,
        "explicit null should not be marked implicit"
    );
}

#[test]
fn test_non_model_class_ignored_e2e() {
    let source = r#"
class NotAModel:
    name = "not a field"

class AlsoNotAModel(SomethingElse):
    value = models.IntegerField()
"#;
    let (_tmp, conn) = extract_source(source);

    let model_nodes = query_nodes(&conn, "model");
    assert!(
        model_nodes.is_empty(),
        "non-Django-model classes should produce no model nodes"
    );
}

#[test]
fn test_unresolvable_edges_dropped() {
    let source = r#"
from django.db import models

class MyModel(models.Model):
    val = models.IntegerField()

def f(x):
    y = len(x)
    print(y)
    return y
"#;
    let (_tmp, conn) = extract_source(source);

    let edges = query_edges(&conn);
    assert!(
        edges.is_empty(),
        "edges to builtins (len, print) should be dropped when they don't match extracted nodes"
    );
}

#[test]
fn test_empty_file() {
    let source = "";
    let (_tmp, conn) = extract_source(source);

    assert_eq!(count_rows(&conn, "nodes"), 0);
    assert_eq!(count_rows(&conn, "contracts"), 0);
    assert_eq!(count_rows(&conn, "edges"), 0);
}

#[test]
fn test_tuple_uniform_element_type() {
    let models = "from django.db import models\nclass M(models.Model):\n    a = models.DecimalField(max_digits=5, decimal_places=2)";
    let utils = r#"
from decimal import Decimal

def split(total: Decimal) -> tuple[Decimal, Decimal]:
    half = total / 2
    return (half, total - half)
"#;
    let (_tmp1, _tmp2, conn) = extract_sources(models, utils);

    // tuple[Decimal, Decimal] with uniform element type should produce Decimal postcondition
    let type_contracts = query_contract_by_type(&conn, "split", "type");
    let decimal_post: Vec<_> = type_contracts
        .iter()
        .filter(|c| {
            c.param_type_name.as_deref() == Some("Decimal")
                && c.contract_role.as_deref() == Some("postcondition")
        })
        .collect();
    assert_eq!(
        decimal_post.len(),
        1,
        "tuple[Decimal, Decimal] should produce a single Decimal type postcondition"
    );
}

#[test]
fn test_manual_override_edges_resolve_by_suffix_or_display_name() {
    let fixture_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();
    fs::create_dir(fixture_dir.path().join("billing")).unwrap();
    fs::write(
        fixture_dir.path().join("billing/models.py"),
        "from django.db import models\nclass EnergyRecord(models.Model):\n    energy = models.DecimalField(max_digits=5, decimal_places=3)\n",
    )
    .unwrap();
    fs::write(fixture_dir.path().join("billing/utils.py"), "def split_energy(x):\n    return x\n").unwrap();
    let overrides = db_dir.path().join("overrides.toml");
    fs::write(
        &overrides,
        "[[edges]]\nsource = \"pkg.billing.utils.split_energy\"\ntarget = \"EnergyRecord.energy\"\nrelationship = \"writes_to\"\n",
    )
    .unwrap();
    let db_path = db_dir.path().join("contracts.sqlite");
    let result =
        crosscheck_contracts::extractor::extract(fixture_dir.path(), Some(&overrides), "4.2", Some(&db_path))
            .unwrap();
    let conn = rusqlite::Connection::open(result).unwrap();
    let edges = query_edges(&conn);
    assert_eq!(edges.len(), 1, "{edges:?}");
    assert_eq!(
        (edges[0].source_name.as_str(), edges[0].target_name.as_str(), edges[0].discovery.as_str()),
        ("split_energy", "EnergyRecord.energy", "manual")
    );
    assert!(!edges[0].source_override);
}
