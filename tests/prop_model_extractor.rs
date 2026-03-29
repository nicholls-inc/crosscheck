//! Property-based tests for model_extractor.rs.
//!
//! Tests Django model field extraction and default handling:
//! - Explicit null= always overrides the default
//! - Field type mapping produces correct constraints

mod test_helpers;

use crosscheck_contracts::defaults::load_defaults;
use crosscheck_contracts::model_extractor::extract_models;
use test_helpers::parse_python_stmts;

#[test]
fn test_explicit_null_true_overrides_default() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    name = models.CharField(max_length=100, null=True)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].null, Some(true), "explicit null=True should win over default false");
    assert!(fields[0].null_is_explicit, "null_is_explicit should be true");
}

#[test]
fn test_explicit_null_false_overrides_default() {
    let defaults = load_defaults("4.2").unwrap();
    // Even when the default is already false, explicit null=False should be marked explicit
    let source = "class MyModel(models.Model):\n    name = models.CharField(max_length=100, null=False)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].null, Some(false));
    assert!(fields[0].null_is_explicit, "explicitly written null=False should be marked explicit");
}

#[test]
fn test_implicit_null_from_defaults() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    name = models.CharField(max_length=100)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].null, Some(false), "default null should be false for CharField");
    assert!(!fields[0].null_is_explicit, "implicit null should not be marked explicit");
}

#[test]
fn test_decimal_field_extraction() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    price = models.DecimalField(max_digits=10, decimal_places=2)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].field_name, "price");
    assert_eq!(fields[0].max_digits, Some(10));
    assert_eq!(fields[0].decimal_places, Some(2));
    assert_eq!(fields[0].field_type, "DecimalField");
}

#[test]
fn test_slug_field_default_max_length() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    slug = models.SlugField()";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].max_length, Some(50), "SlugField default max_length should be 50");
}

#[test]
fn test_slug_field_explicit_max_length_overrides() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    slug = models.SlugField(max_length=200)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].max_length, Some(200), "explicit max_length should override default 50");
}

#[test]
fn test_choices_extraction() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    status = models.CharField(max_length=1, choices=[('A', 'Active'), ('I', 'Inactive')])";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].choices.as_deref(), Some("A,I"), "choices should be extracted as comma-separated values");
}

#[test]
fn test_non_model_class_ignored() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class NotAModel(SomethingElse):\n    name = models.CharField(max_length=100)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert!(fields.is_empty(), "class not inheriting from models.Model should be ignored");
}

#[test]
fn test_model_both_inheritance_forms() {
    let defaults = load_defaults("4.2").unwrap();

    // Form 1: models.Model
    let source1 = "class MyModel(models.Model):\n    x = models.IntegerField()";
    let fields1 = extract_models(&parse_python_stmts(source1), "test.py", &defaults);
    assert_eq!(fields1.len(), 1, "models.Model form should be recognized");

    // Form 2: Model (bare import)
    let source2 = "class MyModel(Model):\n    x = models.IntegerField()";
    let fields2 = extract_models(&parse_python_stmts(source2), "test.py", &defaults);
    assert_eq!(fields2.len(), 1, "bare Model form should be recognized");
}

#[test]
fn test_multiple_fields_extracted() {
    let defaults = load_defaults("4.2").unwrap();
    let source = "class MyModel(models.Model):\n    a = models.IntegerField()\n    b = models.CharField(max_length=50)\n    c = models.DecimalField(max_digits=5, decimal_places=2)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 3, "should extract all 3 fields");
    let names: Vec<&str> = fields.iter().map(|f| f.field_name.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    assert!(names.contains(&"c"));
}

#[test]
fn test_annotated_assignment() {
    let defaults = load_defaults("4.2").unwrap();
    // Annotated assignment form: field: SomeType = models.CharField(...)
    let source = "class MyModel(models.Model):\n    name: str = models.CharField(max_length=100)";
    let stmts = parse_python_stmts(source);
    let fields = extract_models(&stmts, "test.py", &defaults);

    assert_eq!(fields.len(), 1, "annotated assignment should be recognized");
    assert_eq!(fields[0].field_name, "name");
    assert_eq!(fields[0].max_length, Some(100));
}
