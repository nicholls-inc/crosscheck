use crate::db::{ContractRole, ConstraintType};

/// A contract extracted from a docstring.
#[derive(Debug)]
pub struct DocstringContract {
    pub constraint_type: ConstraintType,
    pub role: ContractRole,
    /// The clause's bound: decimal places, length, range bound, or
    /// nullable flag (1 = may be None, 0 = never None), by constraint type.
    pub param_value: Option<i64>,
    pub dependent_expr: Option<String>,
    /// For range contracts: true for `>= N` (lower bound), false for `<= N`.
    pub is_lower_bound: bool,
    /// The name the clause constrains (`amount` in `precision(amount) <= 2`),
    /// as written. For preconditions this is the parameter, if it is one.
    pub subject: Option<String>,
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

fn contract(
    constraint_type: ConstraintType,
    role: ContractRole,
    subject: &str,
    param_value: Option<i64>,
) -> DocstringContract {
    DocstringContract {
        constraint_type,
        role,
        param_value,
        dependent_expr: None,
        is_lower_bound: false,
        subject: Some(subject.trim().to_string()),
    }
}

/// Parse a single requires/ensures clause.
///
/// Grammar (one clause per line; NAME is the constrained name, `result` for
/// the return value, otherwise usually a parameter):
/// - `precision(NAME) <= N` or `precision(NAME) <= DEPEXPR`
/// - `len(NAME) <= N`
/// - `nullable(NAME)`
/// - `non_null(NAME)`, `not_null(NAME)`, `NAME is not None`
/// - `NAME <= N` (range upper bound)
/// - `NAME >= N` (range lower bound)
fn parse_clause(clause: &str, role: ContractRole) -> Option<DocstringContract> {
    // Pattern: precision(NAME) <= N | DEPEXPR
    if let Some(inner) = clause.strip_prefix("precision(") {
        if let Some((name, rest)) = inner.split_once(") <=") {
            if is_simple_name(name) {
                let rest = rest.trim();
                if rest.contains("input_") || rest.contains("max(") || rest.contains("min(") {
                    return Some(DocstringContract {
                        dependent_expr: Some(rest.to_string()),
                        ..contract(ConstraintType::Precision, role, name, None)
                    });
                }
                if let Ok(n) = rest.parse::<i64>() {
                    return Some(contract(ConstraintType::Precision, role, name, Some(n)));
                }
            }
        }
    }

    // Pattern: len(NAME) <= N
    if let Some(inner) = clause.strip_prefix("len(") {
        if let Some((name, rest)) = inner.split_once(") <=") {
            if is_simple_name(name) {
                if let Ok(n) = rest.trim().parse::<i64>() {
                    return Some(contract(ConstraintType::Length, role, name, Some(n)));
                }
            }
        }
    }

    // Pattern: nullable(NAME)
    if let Some(name) = clause.strip_prefix("nullable(").and_then(|r| r.strip_suffix(')')) {
        if is_simple_name(name) {
            return Some(contract(ConstraintType::Nullability, role, name, Some(1)));
        }
    }

    // Pattern: non_null(NAME), not_null(NAME), NAME is not None
    let non_null_arg = clause
        .strip_prefix("non_null(")
        .or_else(|| clause.strip_prefix("not_null("))
        .and_then(|rest| rest.strip_suffix(')'))
        .or_else(|| clause.strip_suffix(" is not None"));
    if let Some(name) = non_null_arg.filter(|n| is_simple_name(n)) {
        return Some(contract(ConstraintType::Nullability, role, name, Some(0)));
    }

    // Pattern: NAME <= N (range max)
    if let Some((lhs, rhs)) = clause.split_once("<=") {
        if is_simple_name(lhs.trim()) {
            if let Ok(n) = rhs.trim().parse::<i64>() {
                return Some(contract(ConstraintType::Range, role, lhs, Some(n)));
            }
        }
    }

    // Pattern: NAME >= N (range min)
    if let Some((lhs, rhs)) = clause.split_once(">=") {
        if is_simple_name(lhs.trim()) {
            if let Ok(n) = rhs.trim().parse::<i64>() {
                return Some(DocstringContract {
                    is_lower_bound: true,
                    ..contract(ConstraintType::Range, role, lhs, Some(n))
                });
            }
        }
    }

    None
}

/// A Python identifier (e.g. `result`, `total`).
fn is_simple_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
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

    #[test]
    fn test_len_clause() {
        let c = parse_clause("len(result) <= 64", ContractRole::Postcondition).unwrap();
        assert_eq!(c.constraint_type.as_str(), "length");
        assert_eq!(c.param_value, Some(64));
    }

    #[test]
    fn test_range_upper_and_lower() {
        let upper = parse_clause("result <= 150", ContractRole::Postcondition).unwrap();
        assert_eq!(upper.constraint_type.as_str(), "range");
        assert_eq!((upper.param_value, upper.is_lower_bound), (Some(150), false));
        let lower = parse_clause("result >= 0", ContractRole::Postcondition).unwrap();
        assert_eq!((lower.param_value, lower.is_lower_bound), (Some(0), true));
    }

    #[test]
    fn test_subjects() {
        let c = parse_clause("precision(amount) <= 2", ContractRole::Precondition).unwrap();
        assert_eq!(c.subject.as_deref(), Some("amount"));
        let c = parse_clause("len(name) <= 5", ContractRole::Precondition).unwrap();
        assert_eq!(c.subject.as_deref(), Some("name"));
        let c = parse_clause("qty >= -3", ContractRole::Precondition).unwrap();
        assert_eq!((c.subject.as_deref(), c.param_value, c.is_lower_bound), (Some("qty"), Some(-3), true));
        let c = parse_clause("nullable(note)", ContractRole::Precondition).unwrap();
        assert_eq!((c.subject.as_deref(), c.param_value), (Some("note"), Some(1)));
        let c = parse_clause("x is not None", ContractRole::Precondition).unwrap();
        assert_eq!(c.subject.as_deref(), Some("x"));
        // A lower bound on something other than a plain name is not a range clause.
        assert!(parse_clause("len(x) >= 3", ContractRole::Precondition).is_none());
    }

    #[test]
    fn test_nullability_clauses() {
        let n = parse_clause("nullable(result)", ContractRole::Postcondition).unwrap();
        assert_eq!(n.param_value, Some(1));
        for clause in ["non_null(total)", "not_null(total)", "total is not None"] {
            let c = parse_clause(clause, ContractRole::Precondition).unwrap();
            assert_eq!(c.constraint_type.as_str(), "nullability", "{clause}");
            assert_eq!(c.param_value, Some(0), "{clause}");
        }
        assert!(parse_clause("a.b is not None", ContractRole::Precondition).is_none());
    }
}
