// Rust guideline compliant 2026-04-14

//! Handler for `policy-engine` integration bindings.
//!
//! Dispatches a policy evaluation request to an external engine and normalizes
//! its response into the canonical `PolicyDecision` shape.
//!
//! Engine selection is driven by `binding.extensions.engineType`:
//! - `"opa"` — OPA `{result: true|false, reasons?: [...]}` format
//! - `"cedar"` — Cedar `{decision: "Allow"|"Deny", determining_policies: [...]}` format
//! - `"canonical"` (default) — the canonical `{decision: "allow"|"deny"|"indeterminate"}` format
//!
//! `Indeterminate` decisions are emitted as-is. The handler does NOT coerce
//! them to Allow or Deny — the caller governs downstream behavior.

use wos_core::eval::ObservedAction;
use wos_core::instance::WorkflowProcess;
use wos_core::model::kernel::KernelDocument;
use wos_core::model::obligation::{ObligationStatus, PendingObligation, PolicyObligationHandling};
use wos_core::{ProvenanceKind, ProvenanceRecord};

use crate::integration::{IntegrationBinding, IntegrationBindingKind};
use crate::milestones::evaluate_milestones;
use crate::policy_decision::{DecisionEffect, PolicyDecision};
use crate::runtime::RuntimeError;
use crate::store::RuntimeRecord;

use super::IntegrationBindingHandler;
use super::request_response::{
    InvocationContext, apply_output_binding, build_integration_input,
    load_or_invoke_service_result, validate_integration_contract,
};

/// Handler for external policy engine evaluation bindings.
pub(crate) struct PolicyEngineHandler;

impl IntegrationBindingHandler for PolicyEngineHandler {
    fn kind(&self) -> IntegrationBindingKind {
        IntegrationBindingKind::PolicyEngine
    }

    fn execute(
        &self,
        ctx: &InvocationContext<'_>,
        record: &mut RuntimeRecord,
        kernel: &KernelDocument,
        observed: &ObservedAction,
        service_ref: &str,
        binding: &IntegrationBinding,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let mut provenance = Vec::new();

        // Build the policy input (context) from the input_mapping expressions.
        let input = build_integration_input(binding, kernel, observed, &record.process)?;

        if let Some(prov_record) = validate_integration_contract(
            ctx.validator,
            service_ref,
            "request",
            binding.request_contract.as_ref(),
            &input,
            observed.actor_id.as_deref(),
        )? {
            provenance.push(prov_record);
        }

        let (step_result, _reused) = load_or_invoke_service_result(
            ctx.service,
            record,
            service_ref,
            &input,
            None, // policy evaluation is not idempotency-keyed at the binding level
            now_iso,
        )?;

        // Determine the engine adapter from `extensions.engineType`.
        let engine_type = binding
            .extensions
            .get("engineType")
            .and_then(|v| v.as_str())
            .unwrap_or("canonical");

        let decision = normalize_decision(engine_type, &step_result.output, service_ref)?;

        // Emit PolicyDecision provenance with the canonical shape.
        provenance.push(ProvenanceRecord {
            id: ProvenanceRecord::mint_id(),
            record_kind: ProvenanceKind::PolicyDecision,
            timestamp: String::new(),
            actor_id: observed.actor_id.clone(),
            from_state: None,
            to_state: None,
            event: None,
            data: Some(serde_json::json!({
                "serviceRef": service_ref,
                "engineType": engine_type,
                "decision": decision.decision,
                "reasonsCount": decision.reasons.len(),
                "obligationsCount": decision.obligations.len(),
                "reasons": decision.reasons,
                "obligations": decision.obligations,
            })),
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        });

        // WOS-INTEG-POLICY-1801/1802: bridge policy-engine obligation directives
        // into WOS durable obligations per the binding's `obligationHandling`.
        // `recordOnly` (the default) is satisfied by the PolicyDecision record
        // above — the directives are already logged in `data.obligations`. The
        // `materialize` form instantiates one `PendingObligation` per directive
        // from the named template. An `indeterminate` decision is NEVER
        // materialized (no obligations are coerced from an undecided result).
        provenance.extend(materialize_policy_obligations(
            binding,
            &decision,
            &mut record.process,
            observed.actor_id.as_deref(),
            now_iso,
        )?);

        // Apply the output binding using the canonical decision as the source document.
        // Callers may map `$.decision` to a case-state field (e.g. `caseFile.policyAllowed`).
        // An Indeterminate decision propagates as-is — null coercion is the caller's choice.
        let decision_value = serde_json::to_value(&decision).map_err(|e| {
            RuntimeError::Integration(format!(
                "policy-engine '{service_ref}': failed to serialize decision: {e}"
            ))
        })?;

        let updates = apply_output_binding(
            &mut record.process.case_state,
            &binding.output_binding,
            &decision_value,
        )?;
        if !updates.is_empty() {
            provenance.push(ProvenanceRecord {
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::DataMapping,
                timestamp: String::new(),
                actor_id: observed.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "serviceRef": service_ref,
                    "integrationType": binding.kind,
                    "updatedPaths": updates,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                canonical_event_hash: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
                outcome: None,
            });
        }

        let post_state = record.process.case_state.clone();
        let milestone_records = evaluate_milestones(kernel, &mut record.process, &post_state);
        provenance.extend(milestone_records);

        Ok(provenance)
    }
}

/// Materialize policy-engine obligation directives into durable WOS pending
/// obligations when the binding declares `obligationHandling: materialize`
/// (WOS-INTEG-POLICY-1801/1802).
///
/// The handling mode is read from `binding.extensions.obligationHandling`
/// (same convention as `engineType`), parsed into a [`PolicyObligationHandling`].
/// Absent or `recordOnly` ⇒ no pending obligation is created (the directives are
/// already recorded on the PolicyDecision provenance). `materialize` ⇒ one
/// [`PendingObligation`] per directive, with a deterministic id
/// (`policyEngineDirective:<directiveId>#<seq>`, `seq` = prior count for that
/// directive id) and an `ObligationActivated` provenance record carrying the
/// template ref. An `indeterminate` decision is never materialized — WOS does
/// not coerce a durable duty out of an undecided result.
fn materialize_policy_obligations(
    binding: &IntegrationBinding,
    decision: &PolicyDecision,
    process: &mut WorkflowProcess,
    actor_id: Option<&str>,
    now_iso: &str,
) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
    // Indeterminate is never materialized (WOS-INTEG-POLICY-1802).
    if decision.decision == DecisionEffect::Indeterminate || decision.obligations.is_empty() {
        return Ok(Vec::new());
    }

    let handling = match binding.extensions.get("obligationHandling") {
        Some(raw) => serde_json::from_value::<PolicyObligationHandling>(raw.clone()).map_err(|e| {
            RuntimeError::Integration(format!(
                "policy-engine: malformed obligationHandling on binding: {e}"
            ))
        })?,
        // Default is recordOnly: nothing to materialize.
        None => return Ok(Vec::new()),
    };

    let Some(template_ref) = handling.template_ref() else {
        // recordOnly — directives stay logged on the PolicyDecision record.
        return Ok(Vec::new());
    };

    let mut provenance = Vec::new();
    let governance = process.governance_state.get_or_insert_with(Default::default);
    for directive in &decision.obligations {
        let policy_id = format!("policyEngineDirective:{}", directive.id);
        // Deterministic id: directive policy id + count of all obligations ever
        // materialized for it (mirrors the obligation-monitor id scheme).
        let seq = governance
            .pending_obligations
            .iter()
            .filter(|o| o.policy_id == policy_id)
            .count();
        let obligation_id = format!("{policy_id}#{seq}");
        let mut extensions = std::collections::HashMap::new();
        extensions.insert(
            "x-wos-policy-engine-template-ref".to_string(),
            serde_json::Value::String(template_ref.to_string()),
        );
        if !directive.data.is_null() {
            extensions.insert(
                "x-wos-policy-engine-directive-data".to_string(),
                directive.data.clone(),
            );
        }
        governance.pending_obligations.push(PendingObligation {
            obligation_id: obligation_id.clone(),
            policy_id: policy_id.clone(),
            status: ObligationStatus::Pending,
            trigger_event: None,
            trigger_actor_id: actor_id.map(str::to_string),
            activated_at: now_iso.to_string(),
            deadline: None,
            responsible_actor: None,
            responsible_role: None,
            correlation_key: None,
            extensions,
        });
        provenance.push(ProvenanceRecord::obligation_activated(
            &policy_id,
            &obligation_id,
            None,
            None,
        ));
    }
    Ok(provenance)
}

/// Select and invoke the correct `PolicyDecision::from_*` constructor based on engine type.
fn normalize_decision(
    engine_type: &str,
    raw_response: &serde_json::Value,
    service_ref: &str,
) -> Result<PolicyDecision, RuntimeError> {
    let decision = match engine_type {
        "opa" => PolicyDecision::from_opa(raw_response),
        "cedar" => PolicyDecision::from_cedar(raw_response),
        "canonical" => PolicyDecision::from_canonical(raw_response),
        other => {
            return Err(RuntimeError::Integration(format!(
                "policy-engine '{service_ref}': unknown engineType '{other}' \
                 (expected opa|cedar|canonical)"
            )));
        }
    };

    decision.ok_or_else(|| {
        RuntimeError::Integration(format!(
            "policy-engine '{service_ref}': failed to normalize response using engine type \
             '{engine_type}': response was malformed or missing required fields"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy_decision::Obligation;

    fn bare_process() -> WorkflowProcess {
        WorkflowProcess {
            process_id: "p1".to_string(),
            case_ledger_id: "case-1".to_string(),
            tenant: stack_common_typeid::DEFAULT_TENANT.to_string(),
            definition_url: "urn:test".to_string(),
            definition_version: "1.0.0".to_string(),
            configuration: Vec::new(),
            case_state: serde_json::json!({}),
            provenance_position: 0,
            next_task_sequence: 0,
            timers: Vec::new(),
            active_tasks: Vec::new(),
            history_store: Default::default(),
            compensation_logs: Default::default(),
            status: wos_core::instance::InstanceStatus::Active,
            stalled_since: None,
            decline_reason: None,
            voided_by: None,
            voided_at: None,
            expired_at: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
            created_at: "2026-06-08T00:00:00Z".to_string(),
            updated_at: "2026-06-08T00:00:00Z".to_string(),
            extensions: Default::default(),
        }
    }

    fn binding_with(extensions: serde_json::Value) -> IntegrationBinding {
        let mut v = serde_json::json!({ "type": "policy-engine" });
        if let serde_json::Value::Object(map) = extensions {
            for (k, val) in map {
                v[k] = val;
            }
        }
        serde_json::from_value(v).expect("valid binding")
    }

    fn decision_with(
        effect: DecisionEffect,
        obligations: Vec<Obligation>,
    ) -> PolicyDecision {
        PolicyDecision {
            decision: effect,
            reasons: Vec::new(),
            obligations,
        }
    }

    #[test]
    fn record_only_default_does_not_materialize() {
        let binding = binding_with(serde_json::json!({}));
        let decision = decision_with(
            DecisionEffect::Deny,
            vec![Obligation { id: "notify-applicant".into(), data: serde_json::Value::Null }],
        );
        let mut process = bare_process();
        let prov =
            materialize_policy_obligations(&binding, &decision, &mut process, Some("svc"), "2026-06-08T00:00:00Z")
                .unwrap();
        assert!(prov.is_empty(), "recordOnly default must not materialize");
        assert!(process.governance_state.is_none());
    }

    #[test]
    fn materialize_creates_pending_and_provenance() {
        let binding = binding_with(serde_json::json!({
            "obligationHandling": {
                "mode": "materialize",
                "templateRef": "urn:wos:obligation-template:policy-engine-directive"
            }
        }));
        let decision = decision_with(
            DecisionEffect::Allow,
            vec![Obligation {
                id: "notify-applicant".into(),
                data: serde_json::json!({ "channel": "mail" }),
            }],
        );
        let mut process = bare_process();
        let prov = materialize_policy_obligations(
            &binding,
            &decision,
            &mut process,
            Some("svc"),
            "2026-06-08T00:00:00Z",
        )
        .unwrap();
        assert_eq!(prov.len(), 1);
        assert_eq!(prov[0].record_kind, ProvenanceKind::ObligationActivated);
        let g = process.governance_state.as_ref().unwrap();
        assert_eq!(g.pending_obligations.len(), 1);
        let o = &g.pending_obligations[0];
        assert_eq!(o.policy_id, "policyEngineDirective:notify-applicant");
        assert_eq!(o.obligation_id, "policyEngineDirective:notify-applicant#0");
        assert_eq!(o.status, ObligationStatus::Pending);
        assert_eq!(
            o.extensions.get("x-wos-policy-engine-template-ref").and_then(|v| v.as_str()),
            Some("urn:wos:obligation-template:policy-engine-directive")
        );
    }

    #[test]
    fn indeterminate_is_never_materialized() {
        let binding = binding_with(serde_json::json!({
            "obligationHandling": {
                "mode": "materialize",
                "templateRef": "urn:wos:obligation-template:policy-engine-directive"
            }
        }));
        let decision = decision_with(
            DecisionEffect::Indeterminate,
            vec![Obligation { id: "x".into(), data: serde_json::Value::Null }],
        );
        let mut process = bare_process();
        let prov = materialize_policy_obligations(
            &binding,
            &decision,
            &mut process,
            None,
            "2026-06-08T00:00:00Z",
        )
        .unwrap();
        assert!(prov.is_empty(), "indeterminate decision must not materialize obligations");
        assert!(process.governance_state.is_none());
    }
}
