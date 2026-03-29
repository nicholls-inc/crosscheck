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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clause_range_min() {
        let result = parse_clause("result >= 0", ContractRole::Precondition);
        assert!(result.is_some());
        let c = result.unwrap();
        assert_eq!(c.constraint_type.as_str(), "range");
        assert_eq!(c.param_value, Some(0));
    }

    #[test]
    fn test_parse_clause_range_positive() {
        let result = parse_clause("value >= 42", ContractRole::Precondition);
        assert!(result.is_some());
        assert_eq!(result.unwrap().param_value, Some(42));
    }

    #[test]
    fn test_parse_clause_input_precision() {
        let result = parse_clause("precision(offpeak) <= 10", ContractRole::Precondition);
        assert!(result.is_some());
        let c = result.unwrap();
        assert_eq!(c.constraint_type.as_str(), "precision");
        assert_eq!(c.param_value, Some(10));
    }

    /// nullable(result) must match exactly — no trailing whitespace.
    #[test]
    fn test_nullable_exact_match() {
        let result = parse_clause("nullable(result)", ContractRole::Postcondition);
        assert!(result.is_some());
        assert_eq!(result.unwrap().constraint_type.as_str(), "nullability");
    }

    #[test]
    fn test_nullable_with_trailing_space_no_match() {
        // The implementation uses exact equality: clause == "nullable(result)"
        // Trailing space should not match
        let result = parse_clause("nullable(result) ", ContractRole::Postcondition);
        // This will fall through to the >= check, which won't match either
        // So it returns None — no false positive from the >= branch
        assert!(result.is_none(), "trailing space should prevent nullable match");
    }

    #[test]
    fn test_unrecognized_clause() {
        let result = parse_clause("something else entirely", ContractRole::Precondition);
        assert!(result.is_none());
    }

    #[test]
    fn test_dependent_expr_with_max() {
        let result =
            parse_clause("precision(result) <= max(input_a, 3)", ContractRole::Postcondition);
        assert!(result.is_some());
        let c = result.unwrap();
        assert_eq!(c.dependent_expr.as_deref(), Some("max(input_a, 3)"));
        assert!(c.param_value.is_none());
    }
}
