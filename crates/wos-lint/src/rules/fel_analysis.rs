// Rust guideline compliant 2026-02-21

//! FEL AST analysis rules (T2-ast tier).
//!
//! Parses FEL expression strings found in WOS documents and checks normative
//! constraints that require inspecting the AST: expression validity, cross-case
//! reference prohibition, function catalog conformance, and SMT-subset rules.
//!
//! # Rule coverage
//!
//! | Rule   | Category           | What is checked                                  |
//! |--------|--------------------|--------------------------------------------------|
//! | K-012  | expression-validity | Guard expressions are valid FEL                |
//! | K-013  | expression-validity | Milestone condition fields are valid FEL       |
//! | K-017  | expression-validity | Guards must not reference related-case state   |
//! | K-019  | expression-validity | Only built-in + extension functions used       |
//! | G-042  | expression-validity | Assertion `expression` fields are valid FEL    |
//! | G-043  | expression-validity | Delegation scope `conditions` are valid FEL    |
//! | AI-024 | expression-validity | Escalation conditions are valid FEL + use `@agent` |
//! | AI-057 | expression-validity | Capability `preconditions` entries are valid FEL |
//! | AG-010 | smt-compatibility   | Verifiable constraints satisfy all SMT rules   |
//! | AG-011 | smt-compatibility   | `let` bindings are not recursive               |
//! | AG-012 | smt-compatibility   | `every`/`some` with arity ≠ 2 need manual review |
//! | AG-013 | smt-compatibility   | Arithmetic is linear (no variable × variable)  |
//! | AG-014 | smt-compatibility   | No extension function calls in verifiable subset|
//!
//! **AG-010 (finite equality):** warns when both sides of `==` / `!=` are simple
//! field/context accesses and neither side is a literal, unless a path is a known
//! WOS enumeration field or listed in `finiteDomainDeclarations`.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use fel_core::{
    Package,
    ast::{BinaryOp, Expr, PathSegment, UnaryOp},
    builtin_function_catalog, builtin_function_catalog_for, parse,
};
use serde_json::Value;

use crate::diagnostic::LintDiagnostic;
use crate::document::{DocumentKind, WosDocument, WosProject};

/// Format a [`fel_core::Error`] for lint output; appends lexer char span when present.
fn fel_parse_failure_message(prefix: &str, err: &fel_core::Error) -> String {
    let mut msg = format!("{prefix}: {err}");
    match err {
        fel_core::Error::Parse(pe) => {
            if let Some(sp) = &pe.span {
                use std::fmt::Write;
                let _ = write!(msg, " (chars {}..{})", sp.start, sp.end);
            }
        }
    }
    msg
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run all FEL AST analysis checks across every document in the project.
///
/// After ADR 0076, all content lives in `$wosWorkflow` embedded blocks.
/// FEL checks look inside the envelope's sub-fields: `lifecycle.states`,
/// `governance`, `agents`, `advanced`, etc.
pub fn check(project: &WosProject, diagnostics: &mut Vec<LintDiagnostic>) {
    for doc in project.documents() {
        match doc.kind {
            // $wosWorkflow carries lifecycle FEL, governance FEL, agents FEL,
            // advanced FEL, and assertion-library FEL in one envelope.
            DocumentKind::Workflow => check_workflow_fel(doc, diagnostics),
            // Delivery, OntologyAlignment, Process, ProvenanceLog, Tooling
            // carry no FEL expressions.
            DocumentKind::Delivery
            | DocumentKind::OntologyAlignment
            | DocumentKind::Process
            | DocumentKind::ProvenanceLog
            | DocumentKind::Tooling => {}
        }
    }
}

/// FEL checks for a `$wosWorkflow` document.
///
/// Dispatches to per-block checkers that match where FEL appears in the
/// merged envelope.
fn check_workflow_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    // Kernel-surface FEL (lifecycle guards, conditions, etc.)
    check_kernel_fel(doc, diagnostics);
    // Governance embedded block FEL
    check_governance_fel(doc, diagnostics);
    // Obligation-policy activation criteria FEL (ACT-001, ACT-002)
    check_obligation_activation_fel(doc, diagnostics);
    // AI integration FEL (agent conditions, deontic expressions)
    check_ai_integration_fel(doc, diagnostics);
    // Advanced governance FEL (equity expressions, SMT constraints)
    check_advanced_governance_fel(doc, diagnostics);
    // Assertion library FEL (embedded in governance.assertionLibrary)
    check_assertion_library_fel(doc, diagnostics);
}

// ---------------------------------------------------------------------------
// Per-document-kind dispatchers
// ---------------------------------------------------------------------------

/// Check FEL in a Kernel document (K-012, K-013, K-017, K-019).
fn check_kernel_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if let Some(states) = doc
        .value
        .pointer("/lifecycle/states")
        .and_then(Value::as_object)
    {
        check_states_fel(states, "/lifecycle/states", diagnostics);
    }
    check_milestones_fel(&doc.value, diagnostics);
}

/// Check FEL in a WorkflowGovernance document (G-043).
///
/// Post-ADR-0076 the governance block lives at `$wosWorkflow.governance`.
/// We accept either the embedded path or, for legacy single-document fixtures
/// that pre-date the merge, the top-level placement.
fn check_governance_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let (delegations, base_prefix) = if let Some(arr) = doc
        .value
        .pointer("/governance/delegations")
        .and_then(Value::as_array)
    {
        (Some(arr), "/governance/delegations")
    } else if let Some(arr) = doc.value.get("delegations").and_then(Value::as_array) {
        (Some(arr), "/delegations")
    } else {
        (None, "/delegations")
    };
    if let Some(delegations) = delegations {
        for (i, delegation) in delegations.iter().enumerate() {
            let base_path = format!("{base_prefix}/{i}");
            check_delegation_conditions(delegation, &base_path, diagnostics);
        }
    }
}

/// ACT-001 + ACT-002: each obligation-policy activation-criteria `where`
/// expression MUST be valid FEL (ACT-001, hard error) AND its AST root MUST be
/// boolean-shaped (ACT-002, warning — non-boolean fails activation at runtime
/// per Governance §16.1.2, mirroring AI-058 for capability preconditions).
///
/// Walks `governance.obligationPolicies[*].{activateWhen,satisfyWhen,
/// cancelWhen,violateWhen}.where`. Referenced criteria (`activationCriteriaRef`)
/// carry no inline `where` and are skipped here (resolution is ACT-007).
fn check_obligation_activation_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(policies) = doc
        .value
        .pointer("/governance/obligationPolicies")
        .and_then(Value::as_array)
    else {
        return;
    };
    const CLAUSES: [&str; 4] = ["activateWhen", "satisfyWhen", "cancelWhen", "violateWhen"];
    for (i, policy) in policies.iter().enumerate() {
        for clause in CLAUSES {
            let Some(expr_str) = policy.pointer(&format!("/{clause}/where")).and_then(Value::as_str)
            else {
                continue;
            };
            let path = format!("/governance/obligationPolicies/{i}/{clause}/where");
            match parse(expr_str) {
                Err(err) => {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "ACT-001",
                        path,
                        fel_parse_failure_message(
                            "activation criteria `where` is not valid FEL",
                            &err,
                        ),
                    ));
                }
                Ok(expr) => {
                    if !is_boolean_shaped(&expr) {
                        diagnostics.push(LintDiagnostic::t2_warning(
                            "ACT-002",
                            path,
                            format!(
                                "activation criteria `where` `{expr_str}` does not have a \
                                 boolean-shaped AST root; `where` must evaluate to a boolean \
                                 (Governance §16.1.2; non-boolean fails activation, no truthy \
                                 coercion)"
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Check FEL in an AI Integration document (AI-024, AI-057).
fn check_ai_integration_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if let Some(agents) = doc.value.get("agents").and_then(Value::as_object) {
        for (agent_name, agent) in agents {
            let base_path = format!("/agents/{agent_name}");
            check_escalation_conditions(agent, &base_path, diagnostics);
            check_capability_preconditions(agent, &base_path, diagnostics);
        }
    }
}

/// AI-057 + AI-058: Each capability `preconditions` entry MUST be valid FEL
/// (AI-057) AND its AST root MUST be a boolean-shaped expression (AI-058).
///
/// The two rules fire on the same inputs: AI-057 catches parse failures
/// (hard error); AI-058 catches parse-clean expressions whose AST root does
/// not type to `boolean` (warning). Core §4.3.1 / §5.2.1 type bind/shape
/// slots as `→ boolean` and §3.4.3 forbids truthy coercion, so a
/// parse-clean `caseFile.amount` or `"open"` in a boolean slot is a bug.
fn check_capability_preconditions(
    agent: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(capabilities) = agent.get("capabilities").and_then(Value::as_array) else {
        return;
    };
    for (cap_idx, capability) in capabilities.iter().enumerate() {
        let Some(preconditions) = capability.get("preconditions").and_then(Value::as_array) else {
            continue;
        };
        for (pre_idx, entry) in preconditions.iter().enumerate() {
            let Some(expr_str) = entry.as_str() else {
                continue;
            };
            let path = format!("{base_path}/capabilities/{cap_idx}/preconditions/{pre_idx}");
            match parse(expr_str) {
                Err(err) => {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "AI-057",
                        path,
                        fel_parse_failure_message("capability precondition is not valid FEL", &err),
                    ));
                }
                Ok(expr) => {
                    if !is_boolean_shaped(&expr) {
                        diagnostics.push(LintDiagnostic::t2_warning(
                            "AI-058",
                            path,
                            format!(
                                "capability precondition `{expr_str}` does not have a \
                                 boolean-shaped AST root; preconditions must evaluate to a \
                                 boolean (AI Integration §3.3.1; Core §3.4.3 forbids truthy \
                                 coercion)"
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Return true when `expr`'s AST root syntactically produces a boolean.
///
/// The predicate is deliberately conservative: it matches operator shapes
/// whose FEL semantics return boolean, and a hard-coded set of
/// boolean-returning builtins. Anything else — bare field refs, string
/// literals, arithmetic — is treated as non-boolean. Ternary / if-then-else
/// require both branches to satisfy the predicate recursively.
///
/// See AI Integration §3.3.1 and Core §4.3.1 / §5.2.1 for the slot-type
/// requirement this predicate enforces at lint time.
pub(super) fn is_boolean_shaped(expr: &Expr) -> bool {
    match expr {
        Expr::Boolean(_) => true,
        Expr::BinaryOp { op, .. } => matches!(
            op,
            BinaryOp::Or
                | BinaryOp::And
                | BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
        ),
        Expr::UnaryOp {
            op: UnaryOp::Not, ..
        } => true,
        Expr::Membership { .. } => true,
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        }
        | Expr::IfThenElse {
            then_branch,
            else_branch,
            ..
        } => is_boolean_shaped(then_branch) && is_boolean_shaped(else_branch),
        Expr::FunctionCall { name, .. } => is_boolean_returning_builtin(name),
        Expr::LetBinding { body, .. } => is_boolean_shaped(body),
        // `a ?? b` is boolean-shaped when both operands are boolean-shaped,
        // e.g. `$flag ?? true`. One branch returning a non-boolean (a path,
        // a number) taints the whole expression — fall through to the `_`
        // arm by short-circuiting here. Review A Finding 4.
        Expr::NullCoalesce { left, right } => is_boolean_shaped(left) && is_boolean_shaped(right),
        _ => false,
    }
}

/// Set of Core FEL builtins whose return type is boolean.
///
/// Derived at first use from `fel_core::builtin_function_catalog()` by
/// filtering entries whose `returns` field is `FelType::Boolean`. This
/// keeps AI-058 honest against spec drift: adding a new boolean-returning
/// builtin in `fel-core` immediately makes it allowlisted here, and a
/// name like `isBoolean` that never existed in the catalog correctly
/// fails the check.
///
/// See `specs/ai/ai-integration.md` §3.3.1 and Core §4.3.1 / §5.2.1 for
/// the boolean-slot typing obligation this predicate enforces.
static BOOLEAN_RETURNING_BUILTINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    builtin_function_catalog()
        .iter()
        .filter(|entry| matches!(entry.returns, fel_core::extensions::FelType::Boolean))
        .map(|entry| entry.name)
        .collect()
});

fn is_boolean_returning_builtin(name: &str) -> bool {
    BOOLEAN_RETURNING_BUILTINS.contains(name)
}

/// Check FEL in an Advanced Governance document (AG-010 through AG-014).
///
/// Post-ADR-0076 advanced lives at `$wosWorkflow.advanced`. Accept either the
/// embedded path or, for legacy single-document fixtures, the top-level form.
fn check_advanced_governance_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let (constraints, prefix) = if let Some(arr) = doc
        .value
        .pointer("/advanced/verifiableConstraints")
        .and_then(Value::as_array)
    {
        (Some(arr), "/advanced/verifiableConstraints")
    } else if let Some(arr) = doc
        .value
        .pointer("/advanced/verifiableConstraints")
        .or_else(|| doc.value.get("verifiableConstraints"))
        .and_then(Value::as_array)
    {
        (Some(arr), "/verifiableConstraints")
    } else {
        (None, "/verifiableConstraints")
    };
    if let Some(constraints) = constraints {
        for (i, constraint) in constraints.iter().enumerate() {
            let path = format!("{prefix}/{i}");
            if let Some(expr_str) = constraint.get("expression").and_then(Value::as_str) {
                let decls =
                    parse_finite_domain_declarations(constraint.get("finiteDomainDeclarations"));
                check_smt_expression(expr_str, &path, diagnostics, &decls);
            }
        }
    }
}
/// Check FEL in an Assertion Library document (G-042).
fn check_assertion_library_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    if let Some(assertions) = doc.value.get("assertions").and_then(Value::as_array) {
        for (i, assertion) in assertions.iter().enumerate() {
            let path = format!("/assertions/{i}/expression");
            if let Some(expr_str) = assertion.get("expression").and_then(Value::as_str) {
                check_expression_syntax("G-042", expr_str, &path, diagnostics);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// K-012, K-013, K-017, K-019: Kernel FEL checks
// ---------------------------------------------------------------------------

/// Recursively check guard expressions in all states and their substates.
fn check_states_fel(
    states: &serde_json::Map<String, Value>,
    path_prefix: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for (state_name, state) in states {
        let state_path = format!("{path_prefix}/{state_name}");

        // K-012: Guards on transitions must be valid FEL.
        if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
            for (ti, transition) in transitions.iter().enumerate() {
                let t_path = format!("{state_path}/transitions/{ti}");
                if let Some(guard) = transition.get("guard").and_then(Value::as_str) {
                    check_guard_expression(guard, &format!("{t_path}/guard"), diagnostics);
                }
            }
        }

        // Recurse into compound substates.
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            check_states_fel(substates, &format!("{state_path}/states"), diagnostics);
        }

        // Recurse into parallel regions.
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for (region_name, region) in regions {
                let region_path = format!("{state_path}/regions/{region_name}");
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    check_states_fel(rstates, &format!("{region_path}/states"), diagnostics);
                }
            }
        }
    }
}

/// K-012 + K-017 + K-019: Parse a guard expression and run structural checks.
fn check_guard_expression(expr_str: &str, path: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let expr = match parse(expr_str) {
        Ok(e) => e,
        Err(err) => {
            diagnostics.push(LintDiagnostic::t2_error(
                "K-012",
                path,
                fel_parse_failure_message("guard expression is not valid FEL", &err),
            ));
            return;
        }
    };

    // K-017: Guards must not reference related-case state.
    check_no_related_case_refs(&expr, "K-017", path, diagnostics);

    // K-019: Only built-in + extension functions.
    check_only_builtin_functions(&expr, "K-019", path, diagnostics);
}

/// K-013: Milestone condition fields must be valid FEL.
fn check_milestones_fel(root: &Value, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(milestones) = root
        .pointer("/lifecycle/milestones")
        .and_then(Value::as_array)
    else {
        return;
    };

    for (i, milestone) in milestones.iter().enumerate() {
        let path = format!("/lifecycle/milestones/{i}/condition");
        if let Some(condition) = milestone.get("condition").and_then(Value::as_str) {
            check_expression_syntax("K-013", condition, &path, diagnostics);
        }
    }
}

// ---------------------------------------------------------------------------
// G-043: Delegation conditions
// ---------------------------------------------------------------------------

/// G-043: `conditions` array entries in a delegation must be valid FEL.
fn check_delegation_conditions(
    delegation: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(conditions) = delegation.get("conditions").and_then(Value::as_array) else {
        return;
    };

    for (i, condition) in conditions.iter().enumerate() {
        let path = format!("{base_path}/conditions/{i}");
        if let Some(expr_str) = condition.as_str() {
            check_expression_syntax("G-043", expr_str, &path, diagnostics);
        }
    }
}

// ---------------------------------------------------------------------------
// AI-024: Escalation conditions
// ---------------------------------------------------------------------------

/// AI-024: Escalation conditions must be valid FEL that references `@agent` context.
fn check_escalation_conditions(
    agent: &Value,
    base_path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Some(escalation) = agent.get("escalation") else {
        return;
    };

    let Some(conditions) = escalation.get("conditions").and_then(Value::as_array) else {
        return;
    };

    for (i, condition) in conditions.iter().enumerate() {
        let path = format!("{base_path}/escalation/conditions/{i}");
        if let Some(expr_str) = condition.as_str() {
            let expr = match parse(expr_str) {
                Ok(e) => e,
                Err(err) => {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "AI-024",
                        &path,
                        fel_parse_failure_message("escalation condition is not valid FEL", &err),
                    ));
                    continue;
                }
            };

            if !references_agent_context(&expr) {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "AI-024",
                    &path,
                    "escalation condition should reference @agent context",
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AG-010 through AG-014: SMT verifiable subset
// ---------------------------------------------------------------------------

/// Load `finiteDomainDeclarations` paths from a constraint JSON object.
///
/// Shape: `{ "path.to.field": { "domain": ["v1", "v2", ...] }, ... }`.
/// Entries without a non-empty `domain` array of strings are ignored.
fn parse_finite_domain_declarations(value: Option<&Value>) -> HashMap<String, ()> {
    let mut out = HashMap::new();
    let Some(Value::Object(map)) = value else {
        return out;
    };
    for (key, entry) in map {
        let Some(domain) = entry.get("domain").and_then(Value::as_array) else {
            continue;
        };
        if domain.is_empty() || !domain.iter().all(|v| v.as_str().is_some()) {
            continue;
        }
        out.insert(key.clone(), ());
    }
    out
}

/// AG-010: Entry point for all SMT subset checks on a single expression.
///
/// Applies AG-011, AG-012, AG-013, AG-014, and finite-domain equality (AG-010)
/// in sequence. Each violation is reported with its own rule ID.
fn check_smt_expression(
    expr_str: &str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
    finite_domain_paths: &HashMap<String, ()>,
) {
    let expr = match parse(expr_str) {
        Ok(e) => e,
        Err(err) => {
            diagnostics.push(LintDiagnostic::t2_error(
                "AG-010",
                path,
                fel_parse_failure_message("verifiable constraint is not valid FEL", &err),
            ));
            return;
        }
    };

    // AG-011: no recursive let bindings.
    let mut let_names: HashSet<String> = HashSet::new();
    check_no_recursive_let(&expr, &mut let_names, "AG-011", path, diagnostics);

    // AG-012: non-standard every/some arity (partial check).
    check_finite_quantifiers(&expr, "AG-012", path, diagnostics);

    // AG-013: arithmetic must be linear.
    check_linear_arithmetic(&expr, "AG-013", path, diagnostics);

    // AG-014: no extension function calls.
    check_no_extension_functions(&expr, "AG-014", path, diagnostics);

    // AG-010 (finite equality): variable-to-variable equality on simple paths.
    check_finite_domain_equality(&expr, path, diagnostics, finite_domain_paths);
}

// ---------------------------------------------------------------------------
// Helpers: syntax-only parse (K-013, G-042, G-043)
// ---------------------------------------------------------------------------

/// Parse `expr_str` and emit a diagnostic with `rule_id` if it fails.
fn check_expression_syntax(
    rule_id: &'static str,
    expr_str: &str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Err(err) = parse(expr_str) {
        diagnostics.push(LintDiagnostic::t2_error(
            rule_id,
            path,
            fel_parse_failure_message("expression is not valid FEL", &err),
        ));
    }
}

// ---------------------------------------------------------------------------
// AST walkers
// ---------------------------------------------------------------------------

/// K-017: Detect references to related-case state in an expression.
///
/// "Related case" references are `$` field-refs whose first path segment
/// begins with `relatedCase` or uses a wildcard to dereference it, as well
/// as `@relatedCase` context refs. This covers the explicit patterns the
/// spec prohibits: `$relatedCase.status`, `@relatedCase.field`, etc.
fn check_no_related_case_refs(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    walk_expr(expr, &mut |e| {
        match e {
            Expr::FieldRef { name, .. } => {
                if name.as_deref().is_some_and(is_related_case_name) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        rule_id,
                        path,
                        format!(
                            "guard references related-case field '{}'; guards must not access \
                             related case state",
                            name.as_deref().unwrap_or_default()
                        ),
                    ));
                }
            }
            Expr::VarRef { name, .. } => {
                if is_related_case_name(name) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        rule_id,
                        path,
                        format!(
                            "guard references related-case field '{name}'; guards must not access \
                             related case state"
                        ),
                    ));
                }
            }
            Expr::ContextRef { name, .. } => {
                if is_related_case_name(name) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        rule_id,
                        path,
                        format!(
                            "guard references related-case context '@{name}'; guards must not \
                             access related case state"
                        ),
                    ));
                }
            }
            Expr::PostfixAccess {
                expr: inner,
                path: segments,
            } => {
                // Postfix chains like `$someField.relatedCase` — check the first dot segment.
                if let Some(PathSegment::Dot(first)) = segments.first() {
                    if is_related_case_name(first) {
                        // We only warn if the base is a field ref without its own name,
                        // meaning it could be a bare `$` dereferencing into relatedCase.
                        if matches!(inner.as_ref(), Expr::FieldRef { name: None, .. }) {
                            diagnostics.push(LintDiagnostic::t2_error(
                                rule_id,
                                path,
                                format!(
                                    "guard accesses '.{first}' on bare '$'; this may reference \
                                     related case state"
                                ),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
        false // continue walking
    });
}

/// Return true if an identifier looks like a related-case accessor.
///
/// Matches `relatedCase` and common capitalisation variants. The spec (Kernel S5.5)
/// calls this concept "related case state". We match the canonical camelCase and
/// a few predictable alias patterns.
fn is_related_case_name(name: &str) -> bool {
    name == "relatedCase"
        || name == "relatedCases"
        || name.starts_with("relatedCase.")
        || name.starts_with("relatedCases.")
}

/// K-019: Check that every function call in the expression is a known built-in.
///
/// Extension functions are permitted by K-019 ("built-in and extension functions");
/// this check flags anything not in the built-in catalog. At Tier 2 we have no
/// extension registry to consult, so we emit a warning (not an error) for unknown
/// names to avoid false positives against valid registered extensions.
fn check_only_builtin_functions(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let builtin_names: HashSet<&str> = builtin_function_catalog_for(Package::Universal)
        .map(|e| e.name)
        .collect();

    walk_expr(expr, &mut |e| {
        if let Expr::FunctionCall { name, .. } = e {
            if !builtin_names.contains(name.as_str()) {
                diagnostics.push(LintDiagnostic::t2_warning(
                    rule_id,
                    path,
                    format!(
                        "function '{name}' is not in the built-in catalog; if it is an extension \
                         function it must be declared in the extension registry"
                    ),
                ));
            }
        }
        false
    });
}

/// AG-011: Detect recursive `let` bindings.
///
/// A `let x = ... in body` is recursive if `x` is referenced anywhere inside
/// its own value expression. We track the binding name being defined and scan
/// the value sub-tree for any `FieldRef` or `FunctionCall` with the same name.
fn check_no_recursive_let(
    expr: &Expr,
    outer_names: &mut HashSet<String>,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    match expr {
        Expr::LetBinding { name, value, body } => {
            // Check whether the value expression references the name being bound
            // (direct self-recursion) or any name currently being defined in an
            // enclosing let (mutual recursion through shadowing — not actually
            // possible in FEL's single-binding let, but we check for completeness).
            let mut self_set = outer_names.clone();
            self_set.insert(name.clone());

            if let_value_references_name(value, &self_set) {
                diagnostics.push(LintDiagnostic::t2_error(
                    rule_id,
                    path,
                    format!("let binding '{name}' references itself recursively"),
                ));
            }

            // Add this name to the outer scope and recurse into body.
            outer_names.insert(name.clone());
            check_no_recursive_let(body, outer_names, rule_id, path, diagnostics);
            outer_names.remove(name);
        }
        // For any other expression shape, recurse into children.
        _ => {
            visit_children(expr, &mut |child| {
                check_no_recursive_let(child, outer_names, rule_id, path, diagnostics);
            });
        }
    }
}

/// Return true if `expr` contains a `FieldRef` whose name is in `names`.
fn let_value_references_name(expr: &Expr, names: &HashSet<String>) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if let Expr::FieldRef { name: Some(n), .. } = e {
            if names.contains(n) {
                found = true;
                return true;
            }
        }
        if let Expr::VarRef { name: n, .. } = e {
            if names.contains(n) {
                found = true;
                return true;
            }
        }
        false
    });
    found
}

/// AG-012: Warn when `every` or `some` are used with arity other than two (partial check).
///
/// Core FEL defines `every(array, predicate)` and `some(array, predicate)` with `$` rebound
/// per element — iteration is over a concrete array value. Calls with a different arity are
/// likely extensions or mistakes; Tier 2 cannot verify their domains, so we flag them.
fn check_finite_quantifiers(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    walk_expr(expr, &mut |e| {
        if let Expr::FunctionCall { name, args } = e {
            if (name == "every" || name == "some") && args.len() != 2 {
                diagnostics.push(LintDiagnostic::t2_warning(
                    rule_id,
                    path,
                    format!(
                        "'{name}()' expects two arguments (array, predicate); non-standard arity \
                         may be an extension — verify finite iteration manually"
                    ),
                ));
            }
        }
        false
    });
}

/// AG-013: Detect non-linear arithmetic (variable × variable or variable ÷ variable).
///
/// A multiplication or division is non-linear if both operands contain at
/// least one variable reference (`FieldRef` or `ContextRef`). One constant
/// side is allowed (e.g. `$qty * 2`).
fn check_linear_arithmetic(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    walk_expr(expr, &mut |e| {
        if let Expr::BinaryOp { op, left, right } = e {
            if matches!(op, BinaryOp::Mul | BinaryOp::Div) {
                let left_has_var = contains_variable(left);
                let right_has_var = contains_variable(right);

                if left_has_var && right_has_var {
                    let op_symbol = if *op == BinaryOp::Mul { "*" } else { "/" };
                    diagnostics.push(LintDiagnostic::t2_error(
                        rule_id,
                        path,
                        format!(
                            "non-linear arithmetic: '{op_symbol}' has variable references on both \
                             sides; the SMT subset requires linear arithmetic"
                        ),
                    ));
                }
            }
        }
        false
    });
}

/// Return true if `expr` contains any `FieldRef` or `ContextRef` node.
fn contains_variable(expr: &Expr) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if matches!(
            e,
            Expr::FieldRef { .. } | Expr::VarRef { .. } | Expr::ContextRef { .. }
        ) {
            found = true;
            return true; // short-circuit
        }
        false
    });
    found
}

/// AG-014: Extension function calls are forbidden in the SMT verifiable subset.
///
/// Unlike K-019 (which only warns), AG-014 is a hard error: the SMT prover
/// cannot reason about extension semantics, so their presence makes a
/// constraint unverifiable.
fn check_no_extension_functions(
    expr: &Expr,
    rule_id: &'static str,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let builtin_names: HashSet<&str> = builtin_function_catalog().iter().map(|e| e.name).collect();

    walk_expr(expr, &mut |e| {
        if let Expr::FunctionCall { name, .. } = e {
            if !builtin_names.contains(name.as_str()) {
                diagnostics.push(LintDiagnostic::t2_error(
                    rule_id,
                    path,
                    format!(
                        "extension function '{name}' is not permitted in the SMT verifiable \
                         subset; only Core S3.5 built-ins may appear in verifiable constraints"
                    ),
                ));
            }
        }
        false
    });
}

/// AG-010 (finite enumerations): warn on simple variable-to-variable `==` / `!=`.
///
/// Passes silently when either side is a literal (including comparisons such as
/// `$instance.impactLevel == "rights-impacting"`) or when either side's dotted path is
/// listed in `finiteDomainDeclarations`.
///
/// When both operands resolve to dotted paths, at most one warning is emitted per
/// unordered path pair (avoids duplicate diagnostics for the same comparison shape).
fn check_finite_domain_equality(
    expr: &Expr,
    path: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
    finite_paths: &HashMap<String, ()>,
) {
    let mut warned_path_pairs: HashSet<(String, String)> = HashSet::new();
    walk_expr(expr, &mut |e| {
        if let Expr::BinaryOp {
            op: BinaryOp::Eq | BinaryOp::NotEq,
            left,
            right,
        } = e
        {
            if smt_equality_is_decidable(left, right, finite_paths) {
                return false;
            }
            if is_simple_access_expr(left.as_ref()) && is_simple_access_expr(right.as_ref()) {
                let skip_duplicate = match (
                    simple_access_path_string(left.as_ref()),
                    simple_access_path_string(right.as_ref()),
                ) {
                    (Some(a), Some(b)) => {
                        let pair = if a <= b { (a, b) } else { (b, a) };
                        !warned_path_pairs.insert(pair)
                    }
                    _ => false,
                };
                if skip_duplicate {
                    return false;
                }
                diagnostics.push(LintDiagnostic::t2_warning(
                    "AG-010",
                    path,
                    "`==` or `!=` compares two non-literal field or context accesses; use a \
                     literal on one side, add `finiteDomainDeclarations` for a path, or avoid \
                     variable-to-variable comparison (AdvGov S8.2)"
                        .to_string(),
                ));
            }
        }
        false
    });
}

/// True when AdvGov S8.2 finite-domain reasoning is obvious from the AST.
fn smt_equality_is_decidable(
    left: &Expr,
    right: &Expr,
    finite_paths: &HashMap<String, ()>,
) -> bool {
    if is_literal_expr(left) || is_literal_expr(right) {
        return true;
    }
    path_declared_finite(left, finite_paths) || path_declared_finite(right, finite_paths)
}

fn path_declared_finite(expr: &Expr, finite_paths: &HashMap<String, ()>) -> bool {
    simple_access_path_string(expr).is_some_and(|p| finite_paths.contains_key(&p))
}

/// Scalar or aggregate of literals only (no `$` / `@`).
fn is_literal_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Null
        | Expr::Boolean(_)
        | Expr::Number(_)
        | Expr::String(_)
        | Expr::DateLiteral(_)
        | Expr::DateTimeLiteral(_) => true,
        Expr::Array(elements) => elements.iter().all(is_literal_expr),
        Expr::Object(pairs) => pairs.iter().all(|(_, v)| is_literal_expr(v)),
        _ => false,
    }
}

fn is_simple_access_expr(expr: &Expr) -> bool {
    match expr {
        Expr::FieldRef { .. } | Expr::VarRef { .. } | Expr::ContextRef { .. } => true,
        Expr::PostfixAccess { expr: inner, .. } => is_simple_access_expr(inner.as_ref()),
        _ => false,
    }
}

/// Dotted path for a simple field or context access (`$a.b` → `a.b`). Indices/wildcards excluded.
pub(super) fn simple_access_path_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::FieldRef {
            name,
            path: segments,
        } => {
            let root = name.as_deref()?;
            let mut s = root.to_string();
            for seg in segments {
                let PathSegment::Dot(part) = seg else {
                    return None;
                };
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        Expr::VarRef {
            name,
            path: segments,
        } => {
            let mut s = name.clone();
            for seg in segments {
                let PathSegment::Dot(part) = seg else {
                    return None;
                };
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        Expr::ContextRef { name, tail, .. } => {
            let mut s = name.clone();
            for part in tail {
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        Expr::PostfixAccess {
            expr: inner,
            path: segments,
        } => {
            let mut s = simple_access_path_string(inner.as_ref())?;
            for seg in segments {
                let PathSegment::Dot(part) = seg else {
                    return None;
                };
                s.push('.');
                s.push_str(part);
            }
            Some(s)
        }
        _ => None,
    }
}

/// AI-024: Return true if `expr` contains any `@agent` context reference.
fn references_agent_context(expr: &Expr) -> bool {
    let mut found = false;
    walk_expr(expr, &mut |e| {
        if let Expr::ContextRef { name, .. } = e {
            if name == "agent" {
                found = true;
                return true; // short-circuit
            }
        }
        false
    });
    found
}

// ---------------------------------------------------------------------------
// Generic AST traversal
// ---------------------------------------------------------------------------

/// Walk `expr` depth-first, calling `visitor` on every node.
///
/// If `visitor` returns `true` the traversal of the current subtree is
/// short-circuited (useful for early-exit searches). The visitor is
/// called in pre-order: the parent node is visited before its children.
///
/// Children are iterated inline via `visit_children` to avoid allocating
/// a `Vec` per node (Finding #2).
pub(super) fn walk_expr(expr: &Expr, visitor: &mut impl FnMut(&Expr) -> bool) {
    if visitor(expr) {
        return;
    }
    visit_children(expr, &mut |child| walk_expr(child, visitor));
}

/// Call `f` once for each direct child expression of `expr`.
///
/// Inlines child iteration without allocating a `Vec`, replacing the
/// previous `children_of` helper (Finding #2).
fn visit_children(expr: &Expr, f: &mut impl FnMut(&Expr)) {
    match expr {
        // Leaves — no children.
        Expr::Null
        | Expr::Boolean(_)
        | Expr::Number(_)
        | Expr::String(_)
        | Expr::DateLiteral(_)
        | Expr::DateTimeLiteral(_)
        | Expr::FieldRef { .. }
        | Expr::VarRef { .. }
        | Expr::ContextRef { .. } => {}

        Expr::Array(elements) => {
            for e in elements {
                f(e);
            }
        }

        Expr::Object(pairs) => {
            for (_, v) in pairs {
                f(v);
            }
        }

        Expr::UnaryOp { operand, .. } => f(operand.as_ref()),

        Expr::BinaryOp { left, right, .. } => {
            f(left.as_ref());
            f(right.as_ref());
        }

        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        }
        | Expr::IfThenElse {
            condition,
            then_branch,
            else_branch,
        } => {
            f(condition.as_ref());
            f(then_branch.as_ref());
            f(else_branch.as_ref());
        }

        Expr::Membership {
            value, container, ..
        } => {
            f(value.as_ref());
            f(container.as_ref());
        }

        Expr::NullCoalesce { left, right } => {
            f(left.as_ref());
            f(right.as_ref());
        }

        Expr::LetBinding { value, body, .. } => {
            f(value.as_ref());
            f(body.as_ref());
        }

        Expr::FunctionCall { args, .. } => {
            for arg in args {
                f(arg);
            }
        }

        Expr::PostfixAccess { expr: inner, .. } => f(inner.as_ref()),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_docs_in_private_items)]

    use std::collections::HashMap;

    use super::*;
    use crate::diagnostic::LintSeverity;
    use crate::document::{DocumentKind, WosDocument};
    use serde_json::json;

    fn make_doc(kind: DocumentKind, value: serde_json::Value) -> WosDocument {
        WosDocument {
            kind,
            value,
            source: None,
        }
    }

    // --- K-012: guard syntax ---

    #[test]
    fn k012_valid_guard_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "draft": {
                            "transitions": [{"event": "submit", "target": "review", "guard": "$amount > 0"}]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(diag.is_empty(), "unexpected: {diag:?}");
    }

    #[test]
    fn k012_invalid_guard_emits_error() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "draft": {
                            "transitions": [{"event": "submit", "target": "review", "guard": ">>> broken <<<"}]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "K-012" && d.severity == LintSeverity::Error),
            "expected K-012 error, got: {diag:?}"
        );
    }

    // --- K-013: milestone condition syntax ---

    #[test]
    fn k013_invalid_milestone_condition() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "milestones": [{"id": "m1", "condition": "((( invalid"}]
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "K-013"),
            "expected K-013 error, got: {diag:?}"
        );
    }

    // --- K-017: no related-case refs ---

    #[test]
    fn k017_guard_with_related_case_ref() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "close",
                                "target": "closed",
                                "guard": "$relatedCase.status = 'done'"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "K-017" && d.severity == LintSeverity::Error),
            "expected K-017 error, got: {diag:?}"
        );
    }

    #[test]
    fn k017_guard_without_related_case_ref_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "close",
                                "target": "closed",
                                "guard": "$status = 'done'"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "K-017"),
            "unexpected K-017: {diag:?}"
        );
    }

    // --- K-019: only built-in functions ---

    #[test]
    fn k019_unknown_function_emits_warning() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "go",
                                "target": "done",
                                "guard": "myCustomFn($amount) > 0"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "K-019" && d.severity == LintSeverity::Warning),
            "expected K-019 warning, got: {diag:?}"
        );
    }

    #[test]
    fn k019_known_builtin_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "active": {
                            "transitions": [{
                                "event": "go",
                                "target": "done",
                                "guard": "sum($items[*].amount) > 100"
                            }]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_kernel_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "K-019"),
            "unexpected K-019: {diag:?}"
        );
    }

    // --- G-042: assertion library expression syntax ---

    #[test]
    fn g042_invalid_assertion_expression() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "assertions": [{"id": "a1", "expression": "not ( valid"}]
            }),
        );
        let mut diag = Vec::new();
        check_assertion_library_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "G-042"),
            "expected G-042 error, got: {diag:?}"
        );
    }

    // --- G-043: delegation condition syntax ---

    #[test]
    fn g043_invalid_delegation_condition() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "delegations": [{"delegator": "alice", "conditions": ["$x >"]}]
            }),
        );
        let mut diag = Vec::new();
        check_governance_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "G-043"),
            "expected G-043 error, got: {diag:?}"
        );
    }

    #[test]
    fn g043_valid_delegation_condition_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "delegations": [{"delegator": "alice", "conditions": ["$level > 2"]}]
            }),
        );
        let mut diag = Vec::new();
        check_governance_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "G-043"),
            "unexpected G-043: {diag:?}"
        );
    }

    // --- AI-024: escalation condition references @agent ---

    #[test]
    fn ai024_condition_without_agent_ref_warns() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "classifier": {
                        "escalation": {
                            "conditions": ["$score > 0.9"]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-024" && d.severity == LintSeverity::Warning),
            "expected AI-024 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ai024_condition_with_agent_ref_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "classifier": {
                        "escalation": {
                            "conditions": ["@agent.confidence < 0.7"]
                        }
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-024"),
            "unexpected AI-024: {diag:?}"
        );
    }

    // --- AI-057: capability precondition FEL validity ---

    #[test]
    fn ai057_valid_precondition_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": ["caseFile.documentsReceived = true"]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-057"),
            "unexpected AI-057: {diag:?}"
        );
    }

    #[test]
    fn ai057_invalid_precondition_fails() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": ["!!! not FEL !!!"]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-057" && d.severity == LintSeverity::Error),
            "expected AI-057 error, got: {diag:?}"
        );
    }

    // --- AI-058: capability precondition boolean-AST-root ---

    #[test]
    fn ai058_binary_comparison_is_boolean_shaped() {
        // `caseFile.amount > 0` — binary comparison, boolean-shaped root.
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": ["caseFile.amount > 0"]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-058"),
            "unexpected AI-058: {diag:?}"
        );
    }

    #[test]
    fn ai058_bare_field_ref_fires() {
        // `caseFile.amount` alone is a path, not a boolean.
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": ["caseFile.amount"]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-058" && d.severity == LintSeverity::Warning),
            "expected AI-058 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ai058_string_literal_fires() {
        // `"open"` parses (as a string literal) but is not boolean-shaped.
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": ["\"open\""]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-058" && d.severity == LintSeverity::Warning),
            "expected AI-058 warning, got: {diag:?}"
        );
    }

    /// Helper: run AI-058 over a single precondition string; return the
    /// diagnostics that fired. Keeps the per-builtin allowlist tests compact.
    fn run_ai058(precondition: &str) -> Vec<LintDiagnostic> {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": [precondition]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        diag
    }

    // --- §4.3b #F4a: AI-058 allowlist derives from fel-core catalog ---
    //
    // These four tests pin the bugs Review A surfaced: the old hand-rolled
    // allowlist omitted `every`, `some`, and the `boolean(any)` cast, and
    // listed a bogus `isBoolean` that does not exist in `fel-core`.

    #[test]
    fn ai058_every_builtin_is_clean() {
        // `every(array, predicate) -> boolean` — aggregate builtin that was
        // missing from the pre-§4.3b hand-rolled allowlist (extensions.rs:114).
        let diag = run_ai058("every(caseFile.flags, $ = true)");
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-058"),
            "unexpected AI-058: {diag:?}"
        );
    }

    #[test]
    fn ai058_some_builtin_is_clean() {
        // `some(array, predicate) -> boolean` — also missing from the old
        // allowlist (extensions.rs:120).
        let diag = run_ai058("some(caseFile.flags, $ = true)");
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-058"),
            "unexpected AI-058: {diag:?}"
        );
    }

    #[test]
    fn ai058_boolean_cast_is_clean() {
        // `boolean(any) -> boolean` — the cast builtin (extensions.rs:378)
        // that was absent from the old allowlist.
        let diag = run_ai058("boolean(caseFile.flag)");
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-058"),
            "unexpected AI-058: {diag:?}"
        );
    }

    #[test]
    fn ai058_is_boolean_is_not_a_builtin() {
        // Behavior change introduced by §4.3b #F4a: `isBoolean` was in the
        // old hand-rolled allowlist but does not exist in `fel-core`
        // (grep-verified). The new catalog-derived predicate correctly
        // refuses it, so AI-058 now fires on a bare `isBoolean(...)` call.
        let diag = run_ai058("isBoolean(caseFile.flag)");
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-058" && d.severity == LintSeverity::Warning),
            "expected AI-058 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ai058_boolean_returning_builtin_is_clean() {
        // `present(caseFile.documentsReceived)` — builtin returning boolean.
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{
                            "id": "extract",
                            "preconditions": ["present(caseFile.documentsReceived)"]
                        }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-058"),
            "unexpected AI-058: {diag:?}"
        );
    }

    // --- §4.3b Finding 4: NullCoalesce in is_boolean_shaped ---

    #[test]
    fn ai058_null_coalesce_of_booleans_is_clean() {
        // `caseFile.flag ?? true` — null-coalesce of two boolean-shaped
        // operands. Before Finding 4, `is_boolean_shaped` had no arm for
        // `Expr::NullCoalesce` and fell into `_ => false`, firing AI-058
        // on a valid boolean expression.
        let diag = run_ai058("boolean(caseFile.flag) ?? true");
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-058"),
            "unexpected AI-058 on boolean null-coalesce: {diag:?}"
        );
    }

    #[test]
    fn ai058_null_coalesce_with_non_boolean_fires() {
        // `caseFile.amount ?? 0` — operands are a path and a number, neither
        // boolean-shaped. The new NullCoalesce arm must still fail this
        // expression (both branches boolean-shaped ⇒ whole is boolean).
        let diag = run_ai058("caseFile.amount ?? 0");
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AI-058" && d.severity == LintSeverity::Warning),
            "expected AI-058 warning on non-boolean null-coalesce, got: {diag:?}"
        );
    }

    #[test]
    fn ai057_missing_preconditions_is_noop() {
        // A capability without any preconditions MUST NOT trigger AI-057.
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "agents": {
                    "extractor": {
                        "capabilities": [{ "id": "extract" }]
                    }
                }
            }),
        );
        let mut diag = Vec::new();
        check_ai_integration_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "AI-057"),
            "unexpected AI-057: {diag:?}"
        );
    }

    // --- AG-011: recursive let binding ---

    #[test]
    fn ag011_self_recursive_let() {
        let expr_str = "let x = x + 1 in x";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            diag.iter().any(|d| d.rule_id == "AG-011"),
            "expected AG-011 error, got: {diag:?}"
        );
    }

    #[test]
    fn ag011_non_recursive_let_is_clean() {
        let expr_str = "let x = $amount * 2 in x > 100";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-011"),
            "unexpected AG-011: {diag:?}"
        );
    }

    // --- AG-013: linear arithmetic ---

    #[test]
    fn ag013_variable_times_variable() {
        let expr_str = "$qty * $price > 0";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-013" && d.severity == LintSeverity::Error),
            "expected AG-013 error, got: {diag:?}"
        );
    }

    #[test]
    fn ag013_variable_times_literal_is_linear() {
        let expr_str = "$qty * 2 > 0";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-013"),
            "unexpected AG-013: {diag:?}"
        );
    }

    // --- AG-014: no extension functions in SMT subset ---

    #[test]
    fn ag014_extension_function_in_verifiable_constraint() {
        let expr_str = "myExtFn($value) > 0";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-014" && d.severity == LintSeverity::Error),
            "expected AG-014 error, got: {diag:?}"
        );
    }

    #[test]
    fn ag014_builtin_function_is_allowed() {
        let expr_str = "abs($delta) < 5";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag.iter().any(|d| d.rule_id == "AG-014"),
            "unexpected AG-014: {diag:?}"
        );
    }

    // --- AG-010: finite-domain equality (variable-to-variable) ---

    #[test]
    fn ag010_literal_comparison_is_clean() {
        let expr_str = r#"$output.status == "approved""#;
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag
                .iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_boolean_comparison_is_clean() {
        let expr_str = "$output.eligible == true";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag
                .iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_membership_literal_array_is_clean() {
        let expr_str = r#"$tier in ["gold", "silver", "bronze"]"#;
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag
                .iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_known_enum_to_literal_is_clean() {
        let expr_str = r#"$instance.impactLevel == "rights-impacting""#;
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            !diag
                .iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_variable_to_variable_equality_warns() {
        let expr_str = "$output.status == $copy.status";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "expected AG-010 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ag010_variable_to_variable_inequality_warns() {
        let expr_str = "$output.status != $copy.status";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "expected AG-010 warning for !=, got: {diag:?}"
        );
    }

    #[test]
    fn ag010_duplicate_path_pair_emits_single_warning() {
        let expr_str = "($output.status == $copy.status) and ($copy.status == $output.status)";
        let mut diag = Vec::new();
        check_smt_expression(
            expr_str,
            "/verifiableConstraints/0",
            &mut diag,
            &HashMap::new(),
        );
        let n = diag
            .iter()
            .filter(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning)
            .count();
        assert_eq!(n, 1, "expected one deduped AG-010 warning, got: {diag:?}");
    }

    #[test]
    fn ag010_finite_domain_declaration_suppresses_var_var() {
        let expr_str = "$output.status == $copy.status";
        let mut decls = HashMap::new();
        decls.insert("output.status".to_string(), ());
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &decls);
        assert!(
            !diag
                .iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    #[test]
    fn ag010_invalid_declaration_entry_does_not_suppress() {
        let expr_str = "$output.status == $copy.status";
        let mut decls = HashMap::new();
        decls.insert("other.path".to_string(), ());
        let mut diag = Vec::new();
        check_smt_expression(expr_str, "/verifiableConstraints/0", &mut diag, &decls);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "expected AG-010 warning, got: {diag:?}"
        );
    }

    #[test]
    fn ag010_advanced_doc_parses_finite_domain_declarations() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": "1.0",
                "verifiableConstraints": [{
                    "constraintRef": "c1",
                    "verifiable": true,
                    "expression": "$output.status == $copy.status",
                    "finiteDomainDeclarations": {
                        "output.status": { "domain": ["a", "b"] },
                        "bad": { "domain": [] },
                        "alsoBad": "not-an-object"
                    }
                }]
            }),
        );
        let mut diag = Vec::new();
        check_advanced_governance_fel(&doc, &mut diag);
        assert!(
            !diag
                .iter()
                .any(|d| d.rule_id == "AG-010" && d.severity == LintSeverity::Warning),
            "unexpected AG-010 warning: {diag:?}"
        );
    }

    /// JSONPath-style `[?(...)]` is not FEL; restriction 6 is enforced by the parser.
    #[test]
    fn ag010_filter_bracket_syntax_does_not_parse() {
        assert!(
            parse("$items[?(@.x > 1)]").is_err(),
            "JSONPath filter expressions must not parse as FEL"
        );
    }

    // --- ACT-001 / ACT-002: obligation-policy activation criteria FEL ---

    fn workflow_with_obligation_where(where_expr: &str) -> WosDocument {
        make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": where_expr },
                        "satisfyWhen": { "on": { "event": "reviewCompleted" } },
                        "onViolation": "block"
                    }]
                }
            }),
        )
    }

    #[test]
    fn act001_valid_where_is_clean() {
        let doc = workflow_with_obligation_where("event.field = 'income'");
        let mut diag = Vec::new();
        check_obligation_activation_fel(&doc, &mut diag);
        assert!(diag.is_empty(), "unexpected: {diag:?}");
    }

    #[test]
    fn act001_invalid_where_emits_error() {
        let doc = workflow_with_obligation_where(">>> broken <<<");
        let mut diag = Vec::new();
        check_obligation_activation_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "ACT-001"
                && d.severity == LintSeverity::Error
                && d.path == "/governance/obligationPolicies/0/activateWhen/where"),
            "expected ACT-001 error: {diag:?}"
        );
    }

    #[test]
    fn act002_non_boolean_where_emits_warning() {
        // `caseFile.income` parses but is a bare field ref (non-boolean shape).
        let doc = workflow_with_obligation_where("caseFile.income");
        let mut diag = Vec::new();
        check_obligation_activation_fel(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "ACT-002"
                && d.severity == LintSeverity::Warning),
            "expected ACT-002 warning: {diag:?}"
        );
    }

    #[test]
    fn act002_boolean_where_is_clean() {
        let doc = workflow_with_obligation_where("caseFile.income > caseFile.priorIncome");
        let mut diag = Vec::new();
        check_obligation_activation_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-002"),
            "unexpected ACT-002: {diag:?}"
        );
    }

    #[test]
    fn act_no_obligation_policies_is_clean() {
        let doc = make_doc(DocumentKind::Workflow, json!({ "$wosWorkflow": true }));
        let mut diag = Vec::new();
        check_obligation_activation_fel(&doc, &mut diag);
        assert!(diag.is_empty(), "unexpected: {diag:?}");
    }
}
