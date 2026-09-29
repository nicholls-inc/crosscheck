use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Default field parameters for a Django field type.
#[derive(Debug, Deserialize, Clone)]
pub struct FieldDefaults {
    pub null: Option<bool>,
    pub blank: Option<bool>,
    pub max_length: Option<i64>,
    pub implicit_validators: Option<Vec<String>>,
}

/// Top-level defaults file structure.
#[derive(Debug, Deserialize)]
pub struct DefaultsFile {
    pub fields: HashMap<String, FieldDefaults>,
}

/// Load defaults for a Django version.
pub fn load_defaults(django_version: &str) -> Result<HashMap<String, FieldDefaults>> {
    let version_key = django_version.replace('.', "_");
    let defaults_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("defaults")
        .join(format!("django_{}.toml", version_key));

    if !defaults_path.exists() {
        anyhow::bail!(
            "Defaults file not found for Django {}: {}",
            django_version,
            defaults_path.display()
        );
    }

    let content = std::fs::read_to_string(&defaults_path)?;
    let defaults_file: DefaultsFile = toml::from_str(&content)?;

    Ok(defaults_file.fields)
}

/// Generate defaults by parsing Django source.
pub fn generate_defaults(django_source: &Path, version: &str) -> Result<()> {
    let fields_init = django_source
        .join("django")
        .join("db")
        .join("models")
        .join("fields")
        .join("__init__.py");

    if !fields_init.exists() {
        anyhow::bail!(
            "Django fields module not found: {}",
            fields_init.display()
        );
    }

    let source = std::fs::read_to_string(&fields_init)?;
    let parsed = ruff_python_parser::parse_unchecked(
        &source,
        ruff_python_parser::Mode::Module.into(),
    );

    let mut defaults = HashMap::new();

    // Walk the AST looking for field class definitions
    let stmts = match parsed.syntax() {
        ruff_python_ast::Mod::Module(module) => &module.body,
        _ => return Ok(()),
    };
    for stmt in stmts.iter() {
        if let ruff_python_ast::Stmt::ClassDef(class_def) = stmt {
            let name = class_def.name.to_string();
            if name.ends_with("Field") {
                let field_defaults = extract_field_class_defaults(class_def);
                defaults.insert(name, field_defaults);
            }
        }
    }

    // Output TOML
    let version_key = version.replace('.', "_");
    let output = format_defaults_toml(&defaults);
    let output_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("defaults")
        .join(format!("django_{}.toml", version_key));

    std::fs::write(&output_path, output)?;
    eprintln!("Wrote defaults to {}", output_path.display());

    Ok(())
}

/// Extract default keyword arguments from a field class's __init__.
fn extract_field_class_defaults(class_def: &ruff_python_ast::StmtClassDef) -> FieldDefaults {
    let mut defaults = FieldDefaults {
        null: Some(false),  // Django default
        blank: Some(false), // Django default
        max_length: None,
        implicit_validators: None,
    };

    for stmt in &class_def.body {
        if let ruff_python_ast::Stmt::FunctionDef(func_def) = stmt {
            if func_def.name.as_str() == "__init__" {
                // Extract default values from __init__ parameters
                for param_with_default in &func_def.parameters.posonlyargs {
                    check_param_default(
                        &param_with_default.parameter.name,
                        param_with_default.default.as_deref(),
                        &mut defaults,
                    );
                }
                for param_with_default in &func_def.parameters.args {
                    check_param_default(
                        &param_with_default.parameter.name,
                        param_with_default.default.as_deref(),
                        &mut defaults,
                    );
                }
                for param_with_default in &func_def.parameters.kwonlyargs {
                    check_param_default(
                        &param_with_default.parameter.name,
                        param_with_default.default.as_deref(),
                        &mut defaults,
                    );
                }
            }
        }
    }

    defaults
}

/// Check a parameter's default value and update field defaults.
fn check_param_default(
    name: &str,
    default: Option<&ruff_python_ast::Expr>,
    defaults: &mut FieldDefaults,
) {
    if let Some(expr) = default {
        match name {
            "null" => {
                if let ruff_python_ast::Expr::BooleanLiteral(b) = expr {
                    defaults.null = Some(b.value);
                }
            }
            "blank" => {
                if let ruff_python_ast::Expr::BooleanLiteral(b) = expr {
                    defaults.blank = Some(b.value);
                }
            }
            "max_length" => {
                if let ruff_python_ast::Expr::NumberLiteral(n) = expr {
                    if let ruff_python_ast::Number::Int(i) = &n.value {
                        defaults.max_length = i.as_i64();
                    }
                }
            }
            _ => {}
        }
    }
}

/// Format defaults as TOML string.
fn format_defaults_toml(defaults: &HashMap<String, FieldDefaults>) -> String {
    let mut output = String::new();
    output.push_str("# Generated Django field defaults\n\n");

    let mut sorted_keys: Vec<&String> = defaults.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        let field = &defaults[key];
        output.push_str(&format!("[fields.{}]\n", key));
        if let Some(null) = field.null {
            output.push_str(&format!("null = {}\n", null));
        }
        if let Some(blank) = field.blank {
            output.push_str(&format!("blank = {}\n", blank));
        }
        if let Some(max_length) = field.max_length {
            output.push_str(&format!("max_length = {}\n", max_length));
        }
        if let Some(ref validators) = field.implicit_validators {
            output.push_str(&format!(
                "implicit_validators = [{}]\n",
                validators
                    .iter()
                    .map(|v| format!("\"{}\"", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        output.push('\n');
    }

    output
}
