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
    // Obligation-policy structural / resolution checks
    // (ACT-003 event resolution, ACT-004 requiredData resolution, ACT-005
    // duration validity, ACT-006 business-day calendar pairing, ACT-007
    // activationCriteriaRef resolution).
    check_obligation_activation_structure(doc, diagnostics);
    // Milestone activationCriteria FEL (ACT-008 / WOS-INTEG-MILE-1302):
    // the new optional `activationCriteria.where` on milestones runs the same
    // FEL parse + boolean-shape checks as ACT-001/002, without duplicating the
    // legacy `condition` check (K-013 owns `condition`).
    check_milestone_activation_fel(doc, diagnostics);
    // Obligation authoring lints (ACT-009 / WOS-TOOL-2502 unreachable
    // satisfaction; ACT-010 / WOS-TOOL-2503 impossible violation action).
    check_obligation_authoring(doc, diagnostics);
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
// ACT-003 .. ACT-007: obligation-policy activation-criteria structure
// ---------------------------------------------------------------------------

/// Structural / resolution checks over `governance.obligationPolicies[*]`
/// (ADR 0096; Governance §16.4). Walks each policy's four clause slots plus
/// the policy-level `deadline` and dispatches:
///
/// - ACT-003 (warning): `on.event` SHOULD name a known workflow event.
/// - ACT-004 (warning): `requiredData` `caseFile.*` paths SHOULD resolve to a
///   declared case-file field (skipped when the case file is contract-backed).
/// - ACT-005 (error): `within` (clause + `deadline.within`) MUST be a valid
///   ISO-8601 duration or the WOS `P<N>BD` business-day form.
/// - ACT-006 (warning): a business-day `within` SHOULD pair with a
///   `calendarRef`.
/// - ACT-007 (error): `activationCriteriaRef` MUST resolve to a named criteria
///   (local `#/$defs/...` pointer resolution; no duplicate ids).
fn check_obligation_activation_structure(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(policies) = doc
        .value
        .pointer("/governance/obligationPolicies")
        .and_then(Value::as_array)
    else {
        return;
    };
    let known_events = collect_known_events(&doc.value);
    let case_file_fields = collect_case_file_fields(&doc.value);

    const CLAUSES: [&str; 4] = ["activateWhen", "satisfyWhen", "cancelWhen", "violateWhen"];
    for (i, policy) in policies.iter().enumerate() {
        for clause in CLAUSES {
            let Some(criteria) = policy.get(clause) else {
                continue;
            };
            let base = format!("/governance/obligationPolicies/{i}/{clause}");

            // ACT-007: a referenced criteria carries no inline body — resolve
            // the pointer instead of inspecting its fields.
            if let Some(ref_str) = criteria.get("activationCriteriaRef").and_then(Value::as_str) {
                check_activation_criteria_ref(ref_str, &doc.value, &base, diagnostics);
                continue;
            }

            check_activation_criteria_body(
                criteria,
                &base,
                &known_events,
                &case_file_fields,
                diagnostics,
            );
        }

        // Policy-level `deadline.within` is the same duration grammar (ACT-005)
        // and pairs with `deadline.calendarRef` for business-day forms (ACT-006).
        if let Some(deadline) = policy.get("deadline") {
            let base = format!("/governance/obligationPolicies/{i}/deadline");
            check_within_and_calendar(deadline, &base, diagnostics);
        }
    }
}

// ---------------------------------------------------------------------------
// ACT-008 (WOS-INTEG-MILE-1302): milestone activationCriteria FEL
// ---------------------------------------------------------------------------

/// ACT-008: a milestone's optional `activationCriteria.where` MUST be valid FEL
/// (hard error) AND boolean-shaped (warning), reusing the same parse +
/// `is_boolean_shaped` checks as ACT-001/002.
///
/// This deliberately does NOT touch the legacy milestone `condition` field —
/// `K-013` (Kernel FEL, `check_milestones_fel`) owns `condition`, and emitting
/// here would double-report. ACT-008 fires only over the new
/// `activationCriteria` surface (ADR 0096; Governance §16.3).
///
/// Referenced criteria (`activationCriteriaRef`) carry no inline `where` and
/// are skipped (their resolution is ACT-007's concern; milestones reuse the
/// same `ActivationCriteria` shape).
fn check_milestone_activation_fel(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(milestones) = doc
        .value
        .pointer("/lifecycle/milestones")
        .and_then(Value::as_array)
    else {
        return;
    };
    for (i, milestone) in milestones.iter().enumerate() {
        // Only the inline activationCriteria carries a `where`; a ref form does not.
        let Some(criteria) = milestone.get("activationCriteria") else {
            continue;
        };
        let Some(expr_str) = criteria.get("where").and_then(Value::as_str) else {
            continue;
        };
        let path = format!("/lifecycle/milestones/{i}/activationCriteria/where");
        match parse(expr_str) {
            Err(err) => {
                diagnostics.push(LintDiagnostic::t2_error(
                    "ACT-008",
                    path,
                    fel_parse_failure_message(
                        "milestone activationCriteria `where` is not valid FEL",
                        &err,
                    ),
                ));
            }
            Ok(expr) => {
                if !is_boolean_shaped(&expr) {
                    diagnostics.push(LintDiagnostic::t2_warning(
                        "ACT-008",
                        path,
                        format!(
                            "milestone activationCriteria `where` `{expr_str}` does not have a \
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

// ---------------------------------------------------------------------------
// ACT-009 / ACT-010: obligation authoring lints
// ---------------------------------------------------------------------------

/// Obligation authoring lints over `governance.obligationPolicies[*]`:
///
/// - ACT-009 (WOS-TOOL-2502, warning): an obligation whose `satisfyWhen.on.event`
///   names a static event that cannot occur (not in the workflow's static event
///   graph) can never be discharged. A `$`-prefixed or `*` event name is a
///   dynamic-event escape hatch and is never flagged (mirrors ACT-003).
/// - ACT-010 (WOS-TOOL-2503, error): `onViolation.createTask.taskRef` MUST
///   resolve to a known task (governance task catalog / `tasks` / contracts) and
///   `onViolation.emitEvent.event` MUST be a valid (non-empty) event name in the
///   static event graph (with the same dynamic-event escape hatch).
fn check_obligation_authoring(doc: &WosDocument, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(policies) = doc
        .value
        .pointer("/governance/obligationPolicies")
        .and_then(Value::as_array)
    else {
        return;
    };
    let known_events = collect_known_events(&doc.value);
    let known_tasks = collect_known_task_refs(&doc.value);

    for (i, policy) in policies.iter().enumerate() {
        // ACT-009: unreachable satisfaction.
        if let Some(event) = policy
            .pointer("/satisfyWhen/on/event")
            .and_then(Value::as_str)
        {
            if let Some(events) = &known_events {
                if event_is_statically_unreachable(event, events) {
                    diagnostics.push(LintDiagnostic::t2_warning(
                        "ACT-009",
                        format!("/governance/obligationPolicies/{i}/satisfyWhen/on/event"),
                        format!(
                            "obligation `satisfyWhen` event '{event}' never occurs in the \
                             workflow's static event graph, so this obligation can never be \
                             satisfied; verify the event name or use a `$`-prefixed dynamic \
                             event if it is raised externally (WOS-TOOL-2502; Governance §16.4)"
                        ),
                    ));
                }
            }
        }

        // ACT-010: impossible violation action.
        let Some(on_violation) = policy.get("onViolation") else {
            continue;
        };
        let base = format!("/governance/obligationPolicies/{i}/onViolation");

        // `onViolation` is either a bare string (`warn`/`escalate`/`fail`/`block`)
        // or an object carrying `createTask` / `emitEvent`. A bare string has no
        // refs to resolve.
        if let Some(task_ref) = on_violation
            .pointer("/createTask/taskRef")
            .and_then(Value::as_str)
        {
            if !task_ref.is_empty() && !known_tasks.contains(task_ref) {
                diagnostics.push(LintDiagnostic::t2_error(
                    "ACT-010",
                    format!("{base}/createTask/taskRef"),
                    format!(
                        "onViolation.createTask.taskRef '{task_ref}' does not resolve to a known \
                         task (declare it under `governance.taskCatalog`/`governance.tasks` or a \
                         contract) (WOS-TOOL-2503; Governance §16.4)"
                    ),
                ));
            }
        }
        if let Some(emit_event) = on_violation
            .pointer("/emitEvent/event")
            .and_then(Value::as_str)
        {
            if emit_event.is_empty() {
                diagnostics.push(LintDiagnostic::t2_error(
                    "ACT-010",
                    format!("{base}/emitEvent/event"),
                    "onViolation.emitEvent.event is empty; it must name an event (WOS-TOOL-2503)"
                        .to_string(),
                ));
            } else if let Some(events) = &known_events {
                if event_is_statically_unreachable(emit_event, events) {
                    diagnostics.push(LintDiagnostic::t2_error(
                        "ACT-010",
                        format!("{base}/emitEvent/event"),
                        format!(
                            "onViolation.emitEvent.event '{emit_event}' is not a workflow event \
                             (no transition consumes it and no timer fires it); use a declared \
                             event or a `$`-prefixed dynamic event (WOS-TOOL-2503; Governance §16.4)"
                        ),
                    ));
                }
            }
        }
    }
}

/// True when `event` is a concrete name absent from the static event graph.
///
/// `$`-prefixed names and the `*` wildcard are dynamic / externally-raised and
/// are never judged (mirrors the ACT-003 escape hatch).
fn event_is_statically_unreachable(event: &str, known: &HashSet<String>) -> bool {
    !event.is_empty() && !event.starts_with('$') && event != "*" && !known.contains(event)
}

/// Collect identifiers a `taskRef` may resolve to for ACT-010.
///
/// Sources: `governance.taskCatalog[*].id`, the keys of a legacy
/// `governance.tasks` object, and the keys of the top-level `contracts` object
/// (a contract may stand in for a task binding). Absent sources contribute
/// nothing; an unknown `taskRef` is judged against whatever is present.
fn collect_known_task_refs(root: &Value) -> HashSet<String> {
    let mut refs = HashSet::new();
    if let Some(catalog) = root
        .pointer("/governance/taskCatalog")
        .and_then(Value::as_array)
    {
        for task in catalog {
            if let Some(id) = task.get("id").and_then(Value::as_str) {
                refs.insert(id.to_string());
            }
        }
    }
    if let Some(tasks) = root.pointer("/governance/tasks").and_then(Value::as_object) {
        for key in tasks.keys() {
            refs.insert(key.clone());
        }
    }
    if let Some(contracts) = root.get("contracts").and_then(Value::as_object) {
        for key in contracts.keys() {
            refs.insert(key.clone());
        }
    }
    refs
}

/// Run the body-level checks (ACT-003/004/005/006) over an inline
/// `ActivationCriteria` object.
fn check_activation_criteria_body(
    criteria: &Value,
    base: &str,
    known_events: &Option<HashSet<String>>,
    case_file_fields: &CaseFileFields,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    // ACT-003: trigger event resolution.
    if let Some(event) = criteria.pointer("/on/event").and_then(Value::as_str) {
        if let Some(events) = known_events {
            // Escape hatch: a `$`-prefixed or `*` event name is treated as a
            // dynamic / unknowable trigger and is never flagged.
            if !event.is_empty()
                && !event.starts_with('$')
                && event != "*"
                && !events.contains(event)
            {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "ACT-003",
                    format!("{base}/on/event"),
                    format!(
                        "activation trigger event '{event}' does not match any workflow \
                         transition event or declared timer-fire event; verify the trigger \
                         name (Governance §16.4)"
                    ),
                ));
            }
        }
    }

    // ACT-004: requiredData path resolution.
    if let Some(required) = criteria.get("requiredData").and_then(Value::as_array) {
        for (j, entry) in required.iter().enumerate() {
            let Some(path_str) = entry.as_str() else {
                continue;
            };
            check_required_data_path(path_str, &format!("{base}/requiredData/{j}"), case_file_fields, diagnostics);
        }
    }

    // ACT-005 + ACT-006: `within` validity and business-day calendar pairing.
    check_within_and_calendar(criteria, base, diagnostics);
}

/// ACT-005 + ACT-006: validate a `within` string in `obj` and, when it is a
/// business-day duration, check for a sibling `calendarRef`.
fn check_within_and_calendar(obj: &Value, base: &str, diagnostics: &mut Vec<LintDiagnostic>) {
    let Some(within) = obj.get("within").and_then(Value::as_str) else {
        return;
    };
    match classify_duration(within) {
        DurationKind::Invalid => {
            diagnostics.push(LintDiagnostic::t2_error(
                "ACT-005",
                format!("{base}/within"),
                format!(
                    "`within` '{within}' is not a valid ISO-8601 duration or WOS business-day \
                     form `P<N>BD`; 'indefinite' and empty/garbage values are rejected \
                     (Governance §16.4)"
                ),
            ));
        }
        DurationKind::BusinessDay => {
            // ACT-006: business-day arithmetic needs a calendar to resolve.
            if obj.get("calendarRef").and_then(Value::as_str).is_none() {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "ACT-006",
                    format!("{base}/within"),
                    format!(
                        "business-day duration '{within}' should declare a sibling `calendarRef` \
                         so business days resolve against a known calendar (Governance §16.4; \
                         cf. G-023)"
                    ),
                ));
            }
        }
        DurationKind::Iso => {}
    }
}

/// ACT-004: check one `requiredData` dotted path against the declared
/// case-file fields. `caseFile.*` paths whose top field is undeclared warn;
/// non-`caseFile` roots are out of scope here. Obviously malformed paths
/// (empty, trailing/leading/double dots) always warn.
fn check_required_data_path(
    path_str: &str,
    path: &str,
    fields: &CaseFileFields,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if path_str.is_empty()
        || path_str.starts_with('.')
        || path_str.ends_with('.')
        || path_str.contains("..")
    {
        diagnostics.push(LintDiagnostic::t2_warning(
            "ACT-004",
            path,
            format!("requiredData path '{path_str}' is malformed (empty or dangling segment)"),
        ));
        return;
    }
    let Some(rest) = path_str.strip_prefix("caseFile.") else {
        // Not a case-file path (e.g. `event.field`) — out of ACT-004 scope.
        return;
    };
    match fields {
        // Contract-backed / external case file: field set is not knowable
        // from this document, so resolution is skipped.
        CaseFileFields::External => {}
        CaseFileFields::Inline(declared) => {
            let top = rest.split('.').next().unwrap_or(rest);
            if !declared.contains(top) {
                diagnostics.push(LintDiagnostic::t2_warning(
                    "ACT-004",
                    path,
                    format!(
                        "requiredData path '{path_str}' references undeclared case-file field \
                         '{top}'; declare it under `caseFile.fields` (Governance §16.4)"
                    ),
                ));
            }
        }
    }
}

/// ACT-007: resolve an `activationCriteriaRef`. Local JSON-pointer references
/// (`#/$defs/...`) MUST resolve within the document; external URIs are accepted
/// (cross-document resolution is out of scope at T2 here).
fn check_activation_criteria_ref(
    ref_str: &str,
    root: &Value,
    base: &str,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let path = format!("{base}/activationCriteriaRef");
    if ref_str.is_empty() {
        diagnostics.push(LintDiagnostic::t2_error(
            "ACT-007",
            path,
            "activationCriteriaRef is empty; it must name a criteria",
        ));
        return;
    }
    // Only local fragment pointers are resolvable here; treat a bare `#...`
    // or `#/...` as a JSON pointer into this document.
    let Some(fragment) = ref_str.strip_prefix('#') else {
        // External URI (`https://.../#/$defs/x`, relative file, etc.) — accept;
        // cross-document resolution is not a single-document concern.
        return;
    };
    if root.pointer(fragment).is_none() {
        diagnostics.push(LintDiagnostic::t2_error(
            "ACT-007",
            path,
            format!(
                "activationCriteriaRef '{ref_str}' does not resolve to a node in this document \
                 (Governance §16.4)"
            ),
        ));
    }
}

/// Declared case-file field set, or a sentinel for a contract-backed file.
enum CaseFileFields {
    /// Inline `caseFile.fields` — the set of declared top-level field names.
    Inline(HashSet<String>),
    /// `caseFile.contractRef` — fields defined externally; resolution skipped.
    External,
}

/// Collect the inline case-file field names, or detect a contract-backed file.
///
/// Mirrors the `caseFile` `oneOf` in `wos-workflow.schema.json` (inline
/// `fields` map vs. `contractRef`). Absent `caseFile` is treated as an empty
/// inline declaration so obviously-undeclared paths still warn.
fn collect_case_file_fields(root: &Value) -> CaseFileFields {
    let Some(case_file) = root.get("caseFile") else {
        return CaseFileFields::Inline(HashSet::new());
    };
    if case_file.get("contractRef").is_some() {
        return CaseFileFields::External;
    }
    let mut names = HashSet::new();
    if let Some(fields) = case_file.get("fields").and_then(Value::as_object) {
        for key in fields.keys() {
            names.insert(key.clone());
        }
    }
    CaseFileFields::Inline(names)
}

/// Collect statically-present workflow event names for ACT-003.
///
/// Candidates: every `lifecycle.states.*.transitions[*].event` (recursing into
/// compound substates and parallel regions) plus any `startTimer` action's
/// `fireEvent`/`event` field (timer-fire events). Returns `None` when no
/// lifecycle is present — in which case ACT-003 cannot judge and stays silent.
fn collect_known_events(root: &Value) -> Option<HashSet<String>> {
    let states = root.pointer("/lifecycle/states").and_then(Value::as_object)?;
    let mut events = HashSet::new();
    collect_events_from_states(states, &mut events);
    Some(events)
}

/// Recursively harvest transition events and timer-fire events from a state map.
fn collect_events_from_states(
    states: &serde_json::Map<String, Value>,
    events: &mut HashSet<String>,
) {
    for state in states.values() {
        if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
            for transition in transitions {
                if let Some(event) = transition.get("event").and_then(Value::as_str) {
                    events.insert(event.to_string());
                }
            }
        }
        // Timer-fire events: `startTimer` actions may name the event they emit.
        collect_timer_fire_events(state, events);
        if let Some(substates) = state.get("states").and_then(Value::as_object) {
            collect_events_from_states(substates, events);
        }
        if let Some(regions) = state.get("regions").and_then(Value::as_object) {
            for region in regions.values() {
                if let Some(rstates) = region.get("states").and_then(Value::as_object) {
                    collect_events_from_states(rstates, events);
                }
            }
        }
    }
}

/// Harvest `fireEvent` / `event` names from any `startTimer` actions on a state.
///
/// Actions can live under `entryActions`, `exitActions`, or transition
/// `actions`; we scan every array of objects on the state for an action whose
/// `action == "startTimer"` that statically names the event it will fire.
fn collect_timer_fire_events(state: &Value, events: &mut HashSet<String>) {
    let Some(obj) = state.as_object() else {
        return;
    };
    let mut harvest = |arr: &Value| {
        if let Some(actions) = arr.as_array() {
            for action in actions {
                if action.get("action").and_then(Value::as_str) == Some("startTimer") {
                    for key in ["fireEvent", "event"] {
                        if let Some(ev) = action.get(key).and_then(Value::as_str) {
                            events.insert(ev.to_string());
                        }
                    }
                }
            }
        }
    };
    for (key, value) in obj {
        if key == "entryActions" || key == "exitActions" {
            harvest(value);
        }
    }
    if let Some(transitions) = obj.get("transitions").and_then(Value::as_array) {
        for transition in transitions {
            if let Some(actions) = transition.get("actions") {
                harvest(actions);
            }
        }
    }
}

/// Outcome of duration classification for ACT-005 / ACT-006.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DurationKind {
    /// A standard ISO-8601 duration (`P…`, with optional time component).
    Iso,
    /// The WOS business-day form `P<N>BD` (N a positive integer).
    BusinessDay,
    /// Not a recognized duration (empty, `indefinite`, garbage, malformed).
    Invalid,
}

/// Classify a `within` string per ADR 0096 / Governance §16.4.
///
/// Hand-rolled rather than regex-backed (the crate has no `regex` dep): we
/// accept the WOS business-day form `P<N>BD` (positive integer N) and a
/// conservative ISO-8601 duration grammar
/// `P[nY][nM][nW][nD][T[nH][nM][nS]]` with at least one component and digits
/// in every component slot. `indefinite` and empty strings are explicitly
/// rejected (unlike G-055's hold `expectedDuration`, activation `within` has
/// no indefinite form).
fn classify_duration(s: &str) -> DurationKind {
    if s == "P0BD" {
        // Zero business days is a degenerate trigger window — reject as garbage.
        return DurationKind::Invalid;
    }
    if let Some(n) = s.strip_prefix('P').and_then(|r| r.strip_suffix("BD")) {
        // `P<N>BD`: N must be a non-empty run of ASCII digits, N >= 1.
        if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
            return DurationKind::BusinessDay;
        }
        return DurationKind::Invalid;
    }
    if is_iso8601_duration(s) {
        DurationKind::Iso
    } else {
        DurationKind::Invalid
    }
}

/// Conservative ISO-8601 duration validator: `P[nY][nM][nW][nD][T[nH][nM][nS]]`.
///
/// Requires the leading `P`, at least one numeric component, digits preceding
/// every designator, and no trailing/garbage characters. A lone `P` or `PT`
/// is rejected.
fn is_iso8601_duration(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (rest, None),
    };
    let mut saw_component = false;
    // Date designators in canonical order Y, M, W, D.
    if !consume_designators(date_part, &['Y', 'M', 'W', 'D'], &mut saw_component) {
        return false;
    }
    if let Some(time_part) = time_part {
        // A `T` with no following time component is malformed.
        if time_part.is_empty() {
            return false;
        }
        if !consume_designators(time_part, &['H', 'M', 'S'], &mut saw_component) {
            return false;
        }
    }
    saw_component
}

/// Consume `segment` as a run of `<digits><designator>` pairs whose designators
/// appear in `order` (each at most once, in order). Returns false on any stray
/// character, missing digits, or out-of-order/duplicate designator.
fn consume_designators(segment: &str, order: &[char], saw_component: &mut bool) -> bool {
    let mut idx = 0usize; // position within `order`
    let mut digits = String::new();
    for ch in segment.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        // Must be a designator; find it at or after the current order position.
        let Some(pos) = order[idx..].iter().position(|d| *d == ch) else {
            return false;
        };
        if digits.is_empty() {
            return false; // designator with no preceding digits
        }
        idx += pos + 1;
        digits.clear();
        *saw_component = true;
    }
    // Any leftover digits without a trailing designator is malformed.
    digits.is_empty()
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

    // --- ACT-003 .. ACT-007: activation-criteria structure ---

    /// Build a workflow with one obligation policy whose `activateWhen` is the
    /// given criteria object, plus a minimal lifecycle and inline case file.
    fn workflow_with_activate_when(activate_when: serde_json::Value) -> WosDocument {
        make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "caseFile": { "fields": { "income": { "type": "number" } } },
                "lifecycle": {
                    "states": {
                        "open": {
                            "transitions": [{ "event": "caseFileUpdated", "target": "review" }]
                        },
                        "review": {}
                    }
                },
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": activate_when,
                        "onViolation": "block"
                    }]
                }
            }),
        )
    }

    #[test]
    fn act003_known_event_is_clean() {
        let doc = workflow_with_activate_when(json!({ "on": { "event": "caseFileUpdated" } }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-003"),
            "unexpected ACT-003: {diag:?}"
        );
    }

    #[test]
    fn act003_unknown_event_warns() {
        let doc = workflow_with_activate_when(json!({ "on": { "event": "neverDeclared" } }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-003" && d.severity == LintSeverity::Warning),
            "expected ACT-003 warning: {diag:?}"
        );
    }

    #[test]
    fn act003_dynamic_event_escape_hatch_is_clean() {
        let doc = workflow_with_activate_when(json!({ "on": { "event": "$dynamic" } }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-003"),
            "unexpected ACT-003 on dynamic event: {diag:?}"
        );
    }

    #[test]
    fn act004_declared_case_file_field_is_clean() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "requiredData": ["caseFile.income"]
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-004"),
            "unexpected ACT-004: {diag:?}"
        );
    }

    #[test]
    fn act004_undeclared_case_file_field_warns() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "requiredData": ["caseFile.notAField"]
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-004" && d.severity == LintSeverity::Warning),
            "expected ACT-004 warning: {diag:?}"
        );
    }

    #[test]
    fn act004_malformed_path_warns() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "requiredData": ["caseFile..income"]
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "ACT-004"),
            "expected ACT-004 warning for malformed path: {diag:?}"
        );
    }

    #[test]
    fn act004_contract_backed_case_file_is_skipped() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "caseFile": { "contractRef": "https://agency.gov/c.json" },
                "lifecycle": { "states": { "open": {} } },
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": {
                            "on": { "event": "$x" },
                            "requiredData": ["caseFile.anything"]
                        }
                    }]
                }
            }),
        );
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-004"),
            "unexpected ACT-004 against contract-backed case file: {diag:?}"
        );
    }

    #[test]
    fn act005_valid_iso_duration_is_clean() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "within": "P3D"
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-005"),
            "unexpected ACT-005: {diag:?}"
        );
    }

    #[test]
    fn act005_valid_iso_datetime_duration_is_clean() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "within": "P1DT12H30M"
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-005"),
            "unexpected ACT-005: {diag:?}"
        );
    }

    #[test]
    fn act005_garbage_within_errors() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "within": "indefinite"
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-005" && d.severity == LintSeverity::Error),
            "expected ACT-005 error for 'indefinite': {diag:?}"
        );
    }

    #[test]
    fn act005_empty_within_errors() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "within": ""
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-005" && d.severity == LintSeverity::Error),
            "expected ACT-005 error for empty within: {diag:?}"
        );
    }

    #[test]
    fn act005_deadline_within_is_checked() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": { "states": { "open": {} } },
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": { "on": { "event": "$x" } },
                        "deadline": { "within": "notaduration" }
                    }]
                }
            }),
        );
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter().any(|d| d.rule_id == "ACT-005"
                && d.path == "/governance/obligationPolicies/0/deadline/within"),
            "expected ACT-005 on deadline.within: {diag:?}"
        );
    }

    #[test]
    fn act006_business_day_without_calendar_warns() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "within": "P5BD"
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-006" && d.severity == LintSeverity::Warning),
            "expected ACT-006 warning: {diag:?}"
        );
        // A valid `P5BD` must not also trip ACT-005.
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-005"),
            "unexpected ACT-005 on valid business-day duration: {diag:?}"
        );
    }

    #[test]
    fn act006_business_day_with_calendar_is_clean() {
        let doc = workflow_with_activate_when(json!({
            "on": { "event": "caseFileUpdated" },
            "within": "P5BD",
            "calendarRef": "https://agency.gov/calendars/federal.json"
        }));
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-006"),
            "unexpected ACT-006: {diag:?}"
        );
    }

    #[test]
    fn act007_resolvable_local_ref_is_clean() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": { "states": { "open": {} } },
                "$defs": {
                    "filedCriteria": { "on": { "event": "$x" } }
                },
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": { "activationCriteriaRef": "#/$defs/filedCriteria" }
                    }]
                }
            }),
        );
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-007"),
            "unexpected ACT-007: {diag:?}"
        );
    }

    #[test]
    fn act007_missing_local_ref_errors() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": { "states": { "open": {} } },
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": { "activationCriteriaRef": "#/$defs/missing" }
                    }]
                }
            }),
        );
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-007" && d.severity == LintSeverity::Error),
            "expected ACT-007 error: {diag:?}"
        );
    }

    #[test]
    fn act007_external_uri_ref_is_accepted() {
        let doc = make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": { "states": { "open": {} } },
                "governance": {
                    "obligationPolicies": [{
                        "id": "p1",
                        "activateWhen": {
                            "activationCriteriaRef": "https://agency.gov/lib.json#/$defs/x"
                        }
                    }]
                }
            }),
        );
        let mut diag = Vec::new();
        check_obligation_activation_structure(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-007"),
            "unexpected ACT-007 for external URI: {diag:?}"
        );
    }

    // --- ACT-008: milestone activationCriteria FEL (WOS-INTEG-MILE-1302) ---

    fn workflow_with_milestone(milestone: serde_json::Value) -> WosDocument {
        make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": { "open": {} },
                    "milestones": [milestone]
                }
            }),
        )
    }

    #[test]
    fn act008_valid_boolean_where_is_clean() {
        let doc = workflow_with_milestone(json!({
            "id": "m1",
            "activationCriteria": { "where": "$amount > 0" }
        }));
        let mut diag = Vec::new();
        check_milestone_activation_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-008"),
            "unexpected ACT-008: {diag:?}"
        );
    }

    #[test]
    fn act008_invalid_where_errors() {
        let doc = workflow_with_milestone(json!({
            "id": "m1",
            "activationCriteria": { "where": ">>> broken <<<" }
        }));
        let mut diag = Vec::new();
        check_milestone_activation_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-008" && d.severity == LintSeverity::Error),
            "expected ACT-008 error: {diag:?}"
        );
    }

    #[test]
    fn act008_non_boolean_where_warns() {
        let doc = workflow_with_milestone(json!({
            "id": "m1",
            "activationCriteria": { "where": "$amount" }
        }));
        let mut diag = Vec::new();
        check_milestone_activation_fel(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-008" && d.severity == LintSeverity::Warning),
            "expected ACT-008 warning: {diag:?}"
        );
    }

    #[test]
    fn act008_does_not_touch_legacy_condition() {
        // A milestone with only the legacy `condition` (no activationCriteria)
        // must NOT trip ACT-008 — K-013 owns `condition`.
        let doc = workflow_with_milestone(json!({
            "id": "m1",
            "condition": ">>> broken <<<"
        }));
        let mut diag = Vec::new();
        check_milestone_activation_fel(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-008"),
            "unexpected ACT-008 on legacy condition: {diag:?}"
        );
    }

    // --- ACT-009 / ACT-010: obligation authoring lints ---

    fn workflow_with_policy(policy: serde_json::Value) -> WosDocument {
        make_doc(
            DocumentKind::Workflow,
            json!({
                "$wosWorkflow": true,
                "lifecycle": {
                    "states": {
                        "open": {
                            "transitions": [
                                { "event": "prepared", "target": "review" },
                                { "event": "signed", "target": "done" }
                            ]
                        },
                        "review": {},
                        "done": {}
                    }
                },
                "governance": {
                    "taskCatalog": [{ "id": "reviewTask" }],
                    "obligationPolicies": [policy]
                }
            }),
        )
    }

    #[test]
    fn act009_reachable_satisfaction_is_clean() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "signed" } },
            "onViolation": "block"
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-009"),
            "unexpected ACT-009: {diag:?}"
        );
    }

    #[test]
    fn act009_unreachable_satisfaction_warns() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "neverHappens" } },
            "onViolation": "block"
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-009" && d.severity == LintSeverity::Warning),
            "expected ACT-009 warning: {diag:?}"
        );
    }

    #[test]
    fn act009_dynamic_satisfaction_escape_hatch_is_clean() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "$externalSignal" } },
            "onViolation": "block"
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-009"),
            "unexpected ACT-009 on dynamic event: {diag:?}"
        );
    }

    #[test]
    fn act010_known_task_ref_is_clean() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "signed" } },
            "onViolation": { "createTask": { "taskRef": "reviewTask" } }
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-010"),
            "unexpected ACT-010: {diag:?}"
        );
    }

    #[test]
    fn act010_unknown_task_ref_errors() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "signed" } },
            "onViolation": { "createTask": { "taskRef": "noSuchTask" } }
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-010" && d.severity == LintSeverity::Error),
            "expected ACT-010 error for unknown taskRef: {diag:?}"
        );
    }

    #[test]
    fn act010_invalid_emit_event_errors() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "signed" } },
            "onViolation": { "emitEvent": { "event": "bogusEvent" } }
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            diag.iter()
                .any(|d| d.rule_id == "ACT-010" && d.severity == LintSeverity::Error),
            "expected ACT-010 error for unknown emitEvent: {diag:?}"
        );
    }

    #[test]
    fn act010_bare_string_on_violation_is_clean() {
        let doc = workflow_with_policy(json!({
            "id": "p1",
            "activateWhen": { "on": { "event": "prepared" } },
            "satisfyWhen": { "on": { "event": "signed" } },
            "onViolation": "escalate"
        }));
        let mut diag = Vec::new();
        check_obligation_authoring(&doc, &mut diag);
        assert!(
            !diag.iter().any(|d| d.rule_id == "ACT-010"),
            "unexpected ACT-010 on bare-string onViolation: {diag:?}"
        );
    }

    #[test]
    fn classify_duration_cases() {
        assert_eq!(classify_duration("P3D"), DurationKind::Iso);
        assert_eq!(classify_duration("PT30M"), DurationKind::Iso);
        assert_eq!(classify_duration("P1Y2M10DT2H30M"), DurationKind::Iso);
        assert_eq!(classify_duration("P10BD"), DurationKind::BusinessDay);
        assert_eq!(classify_duration("P0BD"), DurationKind::Invalid);
        assert_eq!(classify_duration("PBD"), DurationKind::Invalid);
        assert_eq!(classify_duration("P"), DurationKind::Invalid);
        assert_eq!(classify_duration("PT"), DurationKind::Invalid);
        assert_eq!(classify_duration(""), DurationKind::Invalid);
        assert_eq!(classify_duration("indefinite"), DurationKind::Invalid);
        assert_eq!(classify_duration("3D"), DurationKind::Invalid);
        assert_eq!(classify_duration("PXD"), DurationKind::Invalid);
    }
}
