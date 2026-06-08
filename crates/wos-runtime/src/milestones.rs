// Rust guideline compliant 2026-04-14

//! Milestone evaluation for WOS lifecycle (Kernel S4.13).
//!
//! After every durable data write the runtime evaluates all milestones in the
//! kernel document against the updated case state.  A milestone fires at most
//! once per workflow process: once its condition has been true, the id is recorded
//! in `WorkflowProcess::fired_milestones` and the milestone is never re-evaluated.

use std::collections::HashMap;

use fel_core::{evaluate, fel_to_json, has_error_diagnostics, parse};
use serde_json::json;
use wos_core::EvalContext;
use wos_core::activation::{ActivationContext, evaluate_activation_criteria};
use wos_core::instance::WorkflowProcess;
use wos_core::model::kernel::KernelDocument;
use wos_core::{ActorKind, ProvenanceKind, ProvenanceRecord};

/// Post-event context surfaced to milestone evaluation for the optional
/// `activationCriteria` path (WOS-INTEG-MILE-1301).
///
/// The legacy `condition` path needs only the post-event case state, so this
/// context is optional: when `None`, milestones fire purely on `condition`. When
/// present, a milestone declaring `activationCriteria` additionally fires when
/// that reusable predicate matches the draining event (event/transition trigger,
/// actor constraint, required-data presence, FEL guard).
pub struct MilestoneEventContext<'a> {
    /// Concrete runtime event name.
    pub event_name: &'a str,
    /// Event payload object, if any.
    pub event_data: Option<&'a serde_json::Value>,
    /// Acting actor identifier.
    pub actor_id: Option<&'a str>,
    /// Acting actor roles (the WOS id-as-role convention passes `[actor_id]`).
    pub actor_roles: &'a [String],
    /// Acting actor kind, resolved from the kernel actor registry.
    pub actor_type: Option<ActorKind>,
    /// Current wall-clock time in epoch milliseconds (for deadline hints).
    pub now_ms: u64,
}

/// Evaluate all un-fired milestones against `post_state`.
///
/// For each milestone whose `condition` now evaluates to truthy (or, when an
/// event context is supplied and the milestone declares `activationCriteria`,
/// whose criteria match the draining event — WOS-INTEG-MILE-1301):
/// - insert its id into `instance.fired_milestones`
/// - append a `MilestoneFired` provenance record carrying `{"milestoneId": id}`
///
/// Records are returned in lexicographic milestone-id order so the provenance
/// stream is deterministic regardless of `HashMap` iteration order.
pub fn evaluate_milestones(
    kernel: &KernelDocument,
    instance: &mut WorkflowProcess,
    post_state: &serde_json::Value,
) -> Vec<ProvenanceRecord> {
    evaluate_milestones_with_event(kernel, instance, post_state, None)
}

/// Event-aware milestone evaluation (WOS-INTEG-MILE-1301).
///
/// Identical to [`evaluate_milestones`] but threads an optional
/// [`MilestoneEventContext`] so a milestone's `activationCriteria` can be
/// evaluated against the draining event. The `condition` path is unchanged and
/// still governs every milestone; `activationCriteria` is an additive fire
/// signal (either-matches), preserving fire-once semantics via
/// `fired_milestones`.
pub fn evaluate_milestones_with_event(
    kernel: &KernelDocument,
    instance: &mut WorkflowProcess,
    post_state: &serde_json::Value,
    event_ctx: Option<&MilestoneEventContext<'_>>,
) -> Vec<ProvenanceRecord> {
    if kernel.lifecycle.milestones.is_empty() {
        return Vec::new();
    }

    let case_map = match case_state_as_map(post_state) {
        Some(map) => map,
        None => return Vec::new(),
    };

    // Sort milestone ids lexically for deterministic output.
    let mut ids: Vec<&String> = kernel.lifecycle.milestones.keys().collect();
    ids.sort();

    let mut records = Vec::new();

    for id in ids {
        // Already fired — skip without re-evaluating.
        if instance.fired_milestones.contains(id) {
            continue;
        }

        let milestone = &kernel.lifecycle.milestones[id];

        // Either the FEL `condition` is truthy over the post-event case state,
        // or (WOS-INTEG-MILE-1301) the optional `activationCriteria` matches the
        // draining event. Both are additive fire signals; fire-once is preserved
        // by the `fired_milestones` guard above.
        let condition_fires = milestone_condition_true(&milestone.condition, &case_map);
        let criteria_fires = match (&milestone.activation_criteria, event_ctx) {
            (Some(criteria), Some(ctx)) => {
                let act_ctx = ActivationContext {
                    event_name: ctx.event_name,
                    event_data: ctx.event_data,
                    event_tags: &[],
                    event_kind: None,
                    actor_id: ctx.actor_id,
                    actor_roles: ctx.actor_roles,
                    actor_type: ctx.actor_type,
                    case_state: post_state,
                    transition_tags: &[],
                    now_ms: ctx.now_ms,
                    trigger_actor_id: None,
                    is_related_event: false,
                };
                evaluate_activation_criteria(criteria, &act_ctx).matched
            }
            _ => false,
        };

        if condition_fires || criteria_fires {
            instance.fired_milestones.insert(id.clone());
            records.push(ProvenanceRecord {
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::MilestoneFired,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(json!({ "milestoneId": id })),
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
    }

    records
}

/// Evaluate a FEL condition against the case-state map.
///
/// Returns `true` if the expression parses, evaluates without errors, and
/// the resulting JSON value is truthy (`true`, non-zero number, non-empty string).
/// Any parse or evaluation error is treated as `false` — the milestone does not fire.
fn milestone_condition_true(
    expression: &str,
    case_map: &HashMap<String, serde_json::Value>,
) -> bool {
    let Ok(parsed) = parse(expression) else {
        return false;
    };

    let context = EvalContext::from_case_state(case_map, None);
    let result = evaluate(&parsed, &context.to_fel_environment());

    if has_error_diagnostics(&result.diagnostics) {
        return false;
    }

    is_truthy(&fel_to_json(&result.value))
}

/// Convert a `serde_json::Value` case-state object into a field map.
///
/// Returns `None` when the state is not an object (should never happen in
/// well-formed instances, but we defend here rather than panic).
fn case_state_as_map(case_state: &serde_json::Value) -> Option<HashMap<String, serde_json::Value>> {
    case_state
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// FEL truthiness: `true` bool, non-zero number, non-empty string.
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
        serde_json::Value::Null => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Build a minimal kernel document with the given milestones by deserializing JSON.
    fn kernel_with_milestones(milestones: &serde_json::Value) -> KernelDocument {
        let kernel_json = serde_json::json!({
            "$wosWorkflow": "1.0",
            "lifecycle": {
                "initialState": "open",
                "states": {
                    "open": { "type": "atomic" }
                },
                "milestones": milestones
            }
        });
        serde_json::from_value(kernel_json).expect("valid kernel JSON")
    }

    fn bare_instance() -> WorkflowProcess {
        WorkflowProcess {
            process_id: "test".to_string(),
            case_ledger_id: "test-case".to_string(),
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
            fired_milestones: HashSet::new(),
            pending_callbacks: Default::default(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            extensions: Default::default(),
        }
    }

    #[test]
    fn fires_when_condition_is_true() {
        let kernel = kernel_with_milestones(&serde_json::json!({
            "approved": { "condition": "caseFile.approved == true" }
        }));
        let mut instance = bare_instance();
        let state = serde_json::json!({ "approved": true });

        let records = evaluate_milestones(&kernel, &mut instance, &state);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_kind, ProvenanceKind::MilestoneFired);
        assert_eq!(
            records[0].data,
            Some(serde_json::json!({ "milestoneId": "approved" }))
        );
        assert!(instance.fired_milestones.contains("approved"));
    }

    #[test]
    fn does_not_fire_when_condition_is_false() {
        let kernel = kernel_with_milestones(&serde_json::json!({
            "approved": { "condition": "caseFile.approved == true" }
        }));
        let mut instance = bare_instance();
        let state = serde_json::json!({ "approved": false });

        let records = evaluate_milestones(&kernel, &mut instance, &state);

        assert!(records.is_empty());
        assert!(!instance.fired_milestones.contains("approved"));
    }

    #[test]
    fn does_not_refire_once_in_fired_set() {
        let kernel = kernel_with_milestones(&serde_json::json!({
            "approved": { "condition": "caseFile.approved == true" }
        }));
        let mut instance = bare_instance();
        instance.fired_milestones.insert("approved".to_string());
        let state = serde_json::json!({ "approved": true });

        let records = evaluate_milestones(&kernel, &mut instance, &state);

        assert!(
            records.is_empty(),
            "milestone already in fired_milestones must not produce a second record"
        );
    }

    #[test]
    fn multiple_milestones_fire_in_lexicographic_order() {
        let kernel = kernel_with_milestones(&serde_json::json!({
            "zMilestone": { "condition": "caseFile.z == true" },
            "aMilestone": { "condition": "caseFile.a == true" }
        }));
        let mut instance = bare_instance();
        let state = serde_json::json!({ "a": true, "z": true });

        let records = evaluate_milestones(&kernel, &mut instance, &state);

        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].data.as_ref().unwrap()["milestoneId"],
            "aMilestone"
        );
        assert_eq!(
            records[1].data.as_ref().unwrap()["milestoneId"],
            "zMilestone"
        );
    }

    // ── WOS-INTEG-MILE-1301: activationCriteria fires on the draining event ──

    #[test]
    fn activation_criteria_fires_on_event_when_condition_false() {
        // `condition` references a field that is never true; `activationCriteria`
        // fires on the event name instead.
        let kernel = kernel_with_milestones(&serde_json::json!({
            "intakeAcknowledged": {
                "condition": "caseFile.acknowledged == true",
                "activationCriteria": { "on": { "event": "intakeReceived" } }
            }
        }));
        let mut instance = bare_instance();
        let state = serde_json::json!({ "acknowledged": false });
        let roles: Vec<String> = Vec::new();
        let ctx = MilestoneEventContext {
            event_name: "intakeReceived",
            event_data: None,
            actor_id: None,
            actor_roles: &roles,
            actor_type: None,
            now_ms: 0,
        };

        // Without the matching event: condition is false → no fire.
        let none =
            evaluate_milestones_with_event(&kernel, &mut instance, &state, None);
        assert!(none.is_empty());
        assert!(!instance.fired_milestones.contains("intakeAcknowledged"));

        // With the matching event: activationCriteria fires it.
        let records =
            evaluate_milestones_with_event(&kernel, &mut instance, &state, Some(&ctx));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_kind, ProvenanceKind::MilestoneFired);
        assert_eq!(
            records[0].data,
            Some(serde_json::json!({ "milestoneId": "intakeAcknowledged" }))
        );
        assert!(instance.fired_milestones.contains("intakeAcknowledged"));
    }

    #[test]
    fn activation_criteria_respects_fire_once() {
        let kernel = kernel_with_milestones(&serde_json::json!({
            "m1": {
                "condition": "caseFile.never == true",
                "activationCriteria": { "on": { "event": "ping" } }
            }
        }));
        let mut instance = bare_instance();
        let state = serde_json::json!({});
        let roles: Vec<String> = Vec::new();
        let ctx = MilestoneEventContext {
            event_name: "ping",
            event_data: None,
            actor_id: None,
            actor_roles: &roles,
            actor_type: None,
            now_ms: 0,
        };
        let first = evaluate_milestones_with_event(&kernel, &mut instance, &state, Some(&ctx));
        assert_eq!(first.len(), 1);
        // A second matching event must not re-fire.
        let second = evaluate_milestones_with_event(&kernel, &mut instance, &state, Some(&ctx));
        assert!(second.is_empty(), "milestone must fire at most once");
    }

    #[test]
    fn activation_criteria_does_not_fire_on_event_mismatch() {
        let kernel = kernel_with_milestones(&serde_json::json!({
            "m1": {
                "condition": "caseFile.never == true",
                "activationCriteria": { "on": { "event": "wanted" } }
            }
        }));
        let mut instance = bare_instance();
        let state = serde_json::json!({});
        let roles: Vec<String> = Vec::new();
        let ctx = MilestoneEventContext {
            event_name: "other",
            event_data: None,
            actor_id: None,
            actor_roles: &roles,
            actor_type: None,
            now_ms: 0,
        };
        let records = evaluate_milestones_with_event(&kernel, &mut instance, &state, Some(&ctx));
        assert!(records.is_empty());
    }

    #[test]
    fn empty_milestones_returns_no_records() {
        let kernel = kernel_with_milestones(&serde_json::json!({}));
        let mut instance = bare_instance();
        let state = serde_json::json!({ "anything": true });

        let records = evaluate_milestones(&kernel, &mut instance, &state);
        assert!(records.is_empty());
    }
}
