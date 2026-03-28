use crate::db::{ContractRole, ConstraintType};

/// A contract extracted from a docstring.
#[derive(Debug)]
pub struct DocstringContract {
    pub constraint_type: ConstraintType,
    pub role: ContractRole,
    pub param_value: Option<i64>,
    pub dependent_expr: Option<String>,
}

/// Parse docstring for requires/ensures clauses.
pub fn parse_docstring(docstring: &str) -> Vec<DocstringContract> {
    let mut contracts = Vec::new();

    for line in docstring.lines() {
        let trimmed = line.trim();

        if let Some(clause) = trimmed.strip_prefix("requires:") {
            if let Some(contract) = parse_clause(clause.trim(), ContractRole::Precondition) {
                contracts.push(contract);
            }
        } else if let Some(clause) = trimmed.strip_prefix("ensures:") {
            if let Some(contract) = parse_clause(clause.trim(), ContractRole::Postcondition) {
                contracts.push(contract);
            }
        }
    }

    contracts
}

/// Parse a single requires/ensures clause.
fn parse_clause(clause: &str, role: ContractRole) -> Option<DocstringContract> {
    // Pattern: precision(result) <= N
    if let Some(rest) = clause.strip_prefix("precision(result) <=") {
        let rest = rest.trim();
        // Check for dependent expression
        if rest.contains("input_") || rest.contains("max(") || rest.contains("min(") {
            return Some(DocstringContract {
                constraint_type: ConstraintType::Precision,
                role,
                param_value: None,
                dependent_expr: Some(rest.to_string()),
            });
        }
        if let Ok(n) = rest.parse::<i64>() {
            return Some(DocstringContract {
                constraint_type: ConstraintType::Precision,
                role,
                param_value: Some(n),
                dependent_expr: None,
            });
        }
    }

    // Pattern: precision(input) <= N or precision(PARAM) <= N
    if clause.starts_with("precision(") && clause.contains(") <=") {
        if let Some(rest) = clause.split(") <=").nth(1) {
            let rest = rest.trim();
            if let Ok(n) = rest.parse::<i64>() {
                return Some(DocstringContract {
                    constraint_type: ConstraintType::Precision,
                    role,
                    param_value: Some(n),
                    dependent_expr: None,
                });
            }
        }
    }

    // Pattern: nullable(result)
    if clause == "nullable(result)" {
        return Some(DocstringContract {
            constraint_type: ConstraintType::Nullability,
            role,
            param_value: None,
            dependent_expr: None,
        });
    }

    // Pattern: EXPR >= N (range min)
    if clause.contains(">=") {
        if let Some(rest) = clause.split(">=").nth(1) {
            if let Ok(n) = rest.trim().parse::<i64>() {
                return Some(DocstringContract {
                    constraint_type: ConstraintType::Range,
                    role,
                    param_value: Some(n),
                    dependent_expr: None,
                });
            }
        }
    }

    None
}
