// Rust guideline compliant 2026-06-08

//! Deterministic obligation explanation renderer (WOS-TOOL-2504).
//!
//! Produces human-readable, single-sentence explanations of a durable
//! obligation in each lifecycle state (pending / satisfied / violated / …).
//! This is an authoring- and review-time aid (Claim A): given an
//! [`ObligationPolicy`] — and, where available, the runtime
//! [`PendingObligation`] it produced — it renders strings such as
//!
//! > "Income changed after submission. Underwriting review is required before
//! >  final approval and is due by 2026-06-10T12:00:00Z."
//!
//! The renderer is **pure and deterministic**: identical inputs always yield
//! identical strings (no clock reads, no locale lookups), so the output is
//! stable for snapshots and for embedding in provenance narrative. It does not
//! evaluate FEL or compute deadlines; it surfaces what the policy/instance
//! already declares. Wording is descriptive, not normative — Governance §16 is
//! the contract.

use wos_core::{ObligationPolicy, ObligationStatus, PendingObligation, ViolationActionKind};

/// Render the pending-state explanation for a policy, optionally specialized
/// with a concrete deadline from its runtime instance.
///
/// When `deadline` is `Some`, the sentence ends with "… and is due by <deadline>.";
/// otherwise the deadline clause is omitted. The lead clause prefers the
/// policy `description`; absent one, it is synthesized from the activate/satisfy
/// trigger labels.
pub fn explain_pending(policy: &ObligationPolicy, deadline: Option<&str>) -> String {
    let lead = lead_clause(policy);
    let duty = duty_clause(policy);
    match deadline {
        Some(when) => format!("{lead} {duty} and is due by {when}."),
        None => format!("{lead} {duty}."),
    }
}

/// Render the satisfied-state explanation.
pub fn explain_satisfied(policy: &ObligationPolicy) -> String {
    let duty = satisfy_target(policy);
    format!("Obligation \"{}\" is satisfied: {duty} completed.", policy.id)
}

/// Render the violated-state explanation, naming the effective action taken.
pub fn explain_violated(policy: &ObligationPolicy) -> String {
    let duty = satisfy_target(policy);
    let action = action_phrase(policy.on_violation.kind());
    format!(
        "Obligation \"{}\" is violated: {duty} did not complete in time; {action}.",
        policy.id
    )
}

/// Render the explanation for any [`ObligationStatus`].
///
/// `deadline` is used only for the `Pending` state; it is ignored otherwise.
/// This is the single entry point authoring tooling should prefer.
pub fn explain_status(
    policy: &ObligationPolicy,
    status: ObligationStatus,
    deadline: Option<&str>,
) -> String {
    match status {
        ObligationStatus::Pending => explain_pending(policy, deadline),
        ObligationStatus::Satisfied => explain_satisfied(policy),
        ObligationStatus::Violated => explain_violated(policy),
        ObligationStatus::Cancelled => format!(
            "Obligation \"{}\" was cancelled before it became due.",
            policy.id
        ),
        ObligationStatus::Expired => {
            let action = action_phrase(policy.on_violation.kind());
            format!(
                "Obligation \"{}\" expired at its deadline; {action}.",
                policy.id
            )
        }
        ObligationStatus::Bypassed => format!(
            "Obligation \"{}\" was bypassed by an authorized actor.",
            policy.id
        ),
    }
}

/// Convenience: explain a concrete runtime [`PendingObligation`] against the
/// policy that produced it, threading the instance deadline automatically.
pub fn explain_instance(policy: &ObligationPolicy, instance: &PendingObligation) -> String {
    explain_status(policy, instance.status, instance.deadline.as_deref())
}

// ── Sentence fragments ──────────────────────────────────────────────────────

/// The descriptive lead: prefer the authored `description`, else synthesize
/// "<activate-event> occurred." from the activation trigger.
fn lead_clause(policy: &ObligationPolicy) -> String {
    if let Some(desc) = policy.description.as_deref() {
        let trimmed = desc.trim_end_matches('.');
        return format!("{trimmed}.");
    }
    let trigger = crate::obligation_graph::trigger_label(&policy.activate_when);
    format!("{trigger} occurred.")
}

/// The duty clause: "<satisfy-target> is required" plus any responsible role.
fn duty_clause(policy: &ObligationPolicy) -> String {
    let target = satisfy_target(policy);
    match policy.responsible_role.as_deref() {
        Some(role) => format!("{target} is required (responsible: {role})"),
        None => format!("{target} is required"),
    }
}

/// A noun phrase for the satisfying event (the thing that must happen).
fn satisfy_target(policy: &ObligationPolicy) -> String {
    crate::obligation_graph::trigger_label(&policy.satisfy_when)
}

/// Describe the effective violation action in plain language.
fn action_phrase(kind: ViolationActionKind) -> String {
    match kind {
        ViolationActionKind::Warn => "a warning was recorded".to_string(),
        ViolationActionKind::Escalate => "the case was escalated".to_string(),
        ViolationActionKind::Fail => "the workflow was failed".to_string(),
        ViolationActionKind::Block => "the triggering action was blocked".to_string(),
        ViolationActionKind::CreateTask => "a remediation task was created".to_string(),
        ViolationActionKind::EmitEvent => "a compensating event was emitted".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn income_policy() -> ObligationPolicy {
        serde_json::from_value(serde_json::json!({
            "id": "income-change-review-required",
            "description": "Income changed after submission",
            "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" },
            "satisfyWhen": {
                "on": { "event": "underwritingReviewCompleted" },
                "actor": { "role": "underwriter", "notSameAsTriggerActor": true }
            },
            "violateWhen": { "on": { "event": "finalApprovalRequested" } },
            "deadline": { "within": "P2D" },
            "responsibleRole": "underwriter",
            "onViolation": "block"
        }))
        .unwrap()
    }

    #[test]
    fn pending_with_deadline_reads_naturally() {
        let p = income_policy();
        let s = explain_pending(&p, Some("2026-06-10T12:00:00Z"));
        assert_eq!(
            s,
            "Income changed after submission. underwritingReviewCompleted is required \
             (responsible: underwriter) and is due by 2026-06-10T12:00:00Z."
        );
    }

    #[test]
    fn pending_without_deadline_omits_due_clause() {
        let p = income_policy();
        let s = explain_pending(&p, None);
        assert!(s.ends_with("is required (responsible: underwriter)."));
        assert!(!s.contains("due by"));
    }

    #[test]
    fn satisfied_and_violated_name_the_target_and_action() {
        let p = income_policy();
        assert_eq!(
            explain_satisfied(&p),
            "Obligation \"income-change-review-required\" is satisfied: \
             underwritingReviewCompleted completed."
        );
        assert_eq!(
            explain_violated(&p),
            "Obligation \"income-change-review-required\" is violated: \
             underwritingReviewCompleted did not complete in time; \
             the triggering action was blocked."
        );
    }

    #[test]
    fn explain_status_covers_every_state() {
        let p = income_policy();
        for status in [
            ObligationStatus::Pending,
            ObligationStatus::Satisfied,
            ObligationStatus::Violated,
            ObligationStatus::Cancelled,
            ObligationStatus::Expired,
            ObligationStatus::Bypassed,
        ] {
            let s = explain_status(&p, status, Some("2026-06-10T12:00:00Z"));
            assert!(!s.is_empty());
            assert!(s.ends_with('.'));
        }
        // Expired surfaces the block action; bypass and cancel do not.
        assert!(explain_status(&p, ObligationStatus::Expired, None).contains("blocked"));
        assert!(explain_status(&p, ObligationStatus::Cancelled, None).contains("cancelled"));
    }

    #[test]
    fn lead_clause_synthesizes_when_no_description() {
        let p: ObligationPolicy = serde_json::from_value(serde_json::json!({
            "id": "notice-required",
            "activateWhen": { "on": { "event": "adverseDecisionPrepared" } },
            "satisfyWhen": { "on": { "event": "noticeSent" } },
            "onViolation": "warn"
        }))
        .unwrap();
        let s = explain_pending(&p, None);
        assert!(s.starts_with("adverseDecisionPrepared occurred."));
        assert!(s.contains("noticeSent is required"));
    }

    #[test]
    fn explain_instance_threads_runtime_deadline_and_status() {
        use std::collections::HashMap;
        let p = income_policy();
        let inst = PendingObligation {
            obligation_id: "obl-1".into(),
            policy_id: p.id.clone(),
            status: ObligationStatus::Pending,
            trigger_event: Some("caseFileUpdated".into()),
            trigger_actor_id: Some("caseworker-7".into()),
            activated_at: "2026-06-08T12:00:00Z".into(),
            deadline: Some("2026-06-10T12:00:00Z".into()),
            responsible_actor: None,
            responsible_role: Some("underwriter".into()),
            correlation_key: None,
            extensions: HashMap::new(),
        };
        let s = explain_instance(&p, &inst);
        assert!(s.contains("due by 2026-06-10T12:00:00Z"));
    }
}
