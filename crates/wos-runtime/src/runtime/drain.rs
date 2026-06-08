// Rust guideline compliant 2026-02-21

//! Event-drain command handling for the reference runtime.
//!
//! The drain path is the runtime's main orchestration loop: timer
//! materialization, companion-policy evaluation, kernel evaluation, milestone
//! firing, side-effect staging, and provenance persistence. Keeping it in its
//! own module makes the durable command boundary easier to inspect before
//! Temporal/Restate adapter spikes are introduced.

use wos_core::eval::Evaluator;
use wos_core::model::kernel::KernelDocument;
use wos_core::{ActorKind, ProvenanceKind, ProvenanceRecord};

use crate::milestones::evaluate_milestones;
use crate::obligations::{
    ObligationEvent, evaluate_activations, evaluate_cancellations, evaluate_pre_event_gate,
    evaluate_satisfactions, load_obligation_policies,
};

use super::timers::{
    annotate_timer_created_with_calendar_version, annotate_timer_created_with_convergence_error,
    materialize_due_timers, timers_to_state,
};
use super::{
    DrainOnceResult, RuntimeError, RuntimeEventContext, WosRuntime, compensation_provenance,
    format_timestamp, populate_provenance_record_fields, stamp_provenance,
};

/// Resolve an event actor's kind from the kernel actor registry, for
/// obligation actor-constraint evaluation. WOS actors carry an `id` and a
/// `kind`; an actor's `id` doubles as its role (the id-as-role convention used
/// by transition `actor` matching), so callers pass `[actor_id]` as the role set.
fn resolve_actor_kind(kernel: &KernelDocument, actor_id: Option<&str>) -> Option<ActorKind> {
    let id = actor_id?;
    kernel.actors.iter().find(|a| a.id == id).map(|a| a.kind)
}

impl WosRuntime {
    /// Drain a single event from the instance queue.
    pub fn drain_once(&mut self, process_id: &str) -> Result<DrainOnceResult, RuntimeError> {
        let now_ms = self.clock.now_ms();
        let now_iso = format_timestamp(now_ms)?;
        let mut record = self.store.load_record(process_id)?;
        let mut appended_provenance =
            materialize_due_timers(&mut record.process, now_ms, &now_iso)?;

        let Some(event) = record.process.pending_events.first().cloned() else {
            if !appended_provenance.is_empty() {
                // Resolve kernel for SP §5.3/§5.4 field population (due-timer
                // materialization path). The kernel is always resolvable here
                // because the instance is persisted.
                let kernel = self.resolver.resolve_kernel(
                    &record.process.definition_url,
                    &record.process.definition_version,
                )?;
                populate_provenance_record_fields(
                    &mut appended_provenance,
                    &kernel,
                    &record.process.definition_version,
                );
                stamp_provenance(&mut appended_provenance, &now_iso);
                record.process.updated_at = now_iso;
                record.process.provenance_position += appended_provenance.len() as u64;
                record.provenance_log.extend(appended_provenance);
                self.store.save_record(record)?;
            }
            return Ok(DrainOnceResult::default());
        };

        record.process.pending_events.remove(0);
        let kernel = self.resolver.resolve_kernel(
            &record.process.definition_url,
            &record.process.definition_version,
        )?;
        // Durable obligation policies (ADR 0096) live in the governance block.
        let obligation_policies = load_obligation_policies(kernel.governance.as_ref());
        let mut runtime_result = DrainOnceResult {
            processed_event: Some(event.event.clone()),
            processed_event_token: event.idempotency_token.clone(),
            transitions: Vec::new(),
            provenance: Vec::new(),
            created_task_ids: Vec::new(),
            emitted_events: Vec::new(),
            guard_evaluations: Vec::new(),
        };

        let drained_event_name = event.event.clone();
        let decision = self.companion_policy.evaluate_event(RuntimeEventContext {
            kernel: kernel.clone(),
            instance: record.process.clone(),
            event,
            now_ms,
            now_iso: now_iso.clone(),
        })?;
        appended_provenance.extend(decision.provenance);

        let Some(event) = decision.event else {
            populate_provenance_record_fields(
                &mut appended_provenance,
                &kernel,
                &record.process.definition_version,
            );
            stamp_provenance(&mut appended_provenance, &now_iso);
            record.process.updated_at = now_iso;
            record.process.provenance_position += appended_provenance.len() as u64;
            record.provenance_log.extend(appended_provenance.clone());
            self.store.save_record(record)?;
            runtime_result.provenance = appended_provenance;
            return Ok(runtime_result);
        };

        appended_provenance.extend(self.signature_expiry_records_for_event(
            &mut record,
            &event.event,
            event.actor_id.as_deref(),
            &now_iso,
        )?);

        // Pre-event obligation gate (ADR 0096 §16.2.3 step 4): a pending
        // obligation's `violateWhen` may block this event before the kernel
        // applies it. Runs against the pre-event case state; mirrors the
        // companion-policy block path on a `block` outcome.
        if !obligation_policies.is_empty() {
            let pre_case_state = record.process.case_state.clone();
            let ev_name = event.event.clone();
            let ev_actor = event.actor_id.clone();
            let ev_data = event.data.clone();
            let actor_roles: Vec<String> = ev_actor.iter().cloned().collect();
            let gate = {
                let obl_event = ObligationEvent {
                    event_name: &ev_name,
                    event_data: ev_data.as_ref(),
                    event_tags: &[],
                    actor_id: ev_actor.as_deref(),
                    actor_roles: &actor_roles,
                    actor_type: resolve_actor_kind(&kernel, ev_actor.as_deref()),
                    case_state: &pre_case_state,
                    transition_tags: &[],
                    now_ms,
                    now_iso: &now_iso,
                };
                evaluate_pre_event_gate(&obligation_policies, &mut record.process, &obl_event)
            };
            appended_provenance.extend(gate.provenance);
            if gate.block {
                populate_provenance_record_fields(
                    &mut appended_provenance,
                    &kernel,
                    &record.process.definition_version,
                );
                stamp_provenance(&mut appended_provenance, &now_iso);
                record.process.updated_at = now_iso;
                record.process.provenance_position += appended_provenance.len() as u64;
                record.provenance_log.extend(appended_provenance.clone());
                self.store.save_record(record)?;
                runtime_result.provenance = appended_provenance;
                return Ok(runtime_result);
            }
        }

        let mut evaluator = Evaluator::from_instance(kernel.clone(), &record.process, now_ms)
            .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;
        evaluator
            .process_event(&event.event, event.actor_id.as_deref(), event.data.as_ref())
            .map_err(|error| RuntimeError::Evaluator(error.to_string()))?;
        runtime_result.transitions = evaluator.transitions().to_vec();
        runtime_result.guard_evaluations = evaluator.take_guard_evaluations();

        appended_provenance.extend(evaluator.provenance().records().to_vec());
        // Annotate any newly created timers with calendarVersion when a calendar
        // is attached (provenance approach a — augment data field, no new variant).
        if let Some(cal) = &self.business_calendar {
            annotate_timer_created_with_calendar_version(&mut appended_provenance, cal);
        }
        appended_provenance.extend(compensation_provenance(
            &kernel,
            &record.provenance_log,
            &appended_provenance,
        ));
        record.process.configuration = evaluator.configuration().active_states().to_vec();
        record.process.case_state = evaluator.case_state_json();
        let (timer_states, convergence_error_ids) =
            timers_to_state(evaluator.timers(), self.business_calendar.as_ref())?;
        // Annotate TimerCreated records for any timers whose calendar deadline did not converge.
        annotate_timer_created_with_convergence_error(
            &mut appended_provenance,
            &convergence_error_ids,
        );
        record.process.timers = timer_states;
        record.process.history_store = evaluator.history_store().clone();
        record.process.updated_at = now_iso.clone();

        let case_state_can_mutate_explicitly = record
            .provenance_log
            .iter()
            .chain(appended_provenance.iter())
            .any(|record| record.record_kind == ProvenanceKind::CaseStateMutation);
        if !runtime_result.transitions.is_empty() && case_state_can_mutate_explicitly {
            appended_provenance.push(ProvenanceRecord {
                id: ProvenanceRecord::mint_id(),
                record_kind: ProvenanceKind::StateTransition,
                timestamp: String::new(),
                actor_id: event.actor_id.clone(),
                from_state: None,
                to_state: None,
                event: Some(event.event.clone()),
                data: Some(serde_json::json!({ "caseStateUnchangedByTransition": true })),
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

        // Milestone firing: evaluate after the event's transition tree completes
        // (including all onEntry/onExit setData), before side-effect realization
        // (createTask/emitEvent) that would enqueue follow-on events (Kernel S4.13).
        // Records are appended in lexicographic milestone-id order so the provenance
        // stream is deterministic.
        let post_state = record.process.case_state.clone();
        let milestone_records = evaluate_milestones(&kernel, &mut record.process, &post_state);
        appended_provenance.extend(milestone_records);

        // Post-event obligation lifecycle (ADR 0096 §16.2.3 step 6): evaluate
        // satisfactions/cancellations of existing obligations, then activations
        // of new ones, against the post-event case state.
        if !obligation_policies.is_empty() {
            let ev_name = event.event.clone();
            let ev_actor = event.actor_id.clone();
            let ev_data = event.data.clone();
            let actor_roles: Vec<String> = ev_actor.iter().cloned().collect();
            let obl_event = ObligationEvent {
                event_name: &ev_name,
                event_data: ev_data.as_ref(),
                event_tags: &[],
                actor_id: ev_actor.as_deref(),
                actor_roles: &actor_roles,
                actor_type: resolve_actor_kind(&kernel, ev_actor.as_deref()),
                case_state: &post_state,
                transition_tags: &[],
                now_ms,
                now_iso: &now_iso,
            };
            appended_provenance.extend(evaluate_satisfactions(
                &obligation_policies,
                &mut record.process,
                &obl_event,
            ));
            appended_provenance.extend(evaluate_cancellations(
                &obligation_policies,
                &mut record.process,
                &obl_event,
            ));
            appended_provenance.extend(evaluate_activations(
                &obligation_policies,
                &mut record.process,
                &obl_event,
            ));
        }

        let actions = evaluator.take_executed_actions();
        let (created_task_ids, emitted_events, runtime_provenance) =
            self.apply_observed_actions(&kernel, &mut record, &actions, &now_iso)?;
        appended_provenance.extend(runtime_provenance);

        let (pending_presentations, presentation_provenance) =
            self.stage_pending_tasks_for_presentation(&mut record, &now_iso)?;
        appended_provenance.extend(presentation_provenance);

        // Stamp the drain's event onto policy-application provenance
        // records that left `event = None`. The governance / AI / autonomy
        // / confidence constructors all set `event: None` because they
        // don't carry the triggering event in their construction context,
        // but the trace teaching-signal (§5.3) needs this association so
        // conformance traces can scope `policies_applied` to the right
        // trace step. Scoped strictly to `is_policy_application()` kinds
        // — kernel-layer records (state transitions, action executions)
        // already set `event` correctly in their constructors and the
        // field is load-bearing there (see `ProvenanceRecord::state_transition`).
        for prov_record in &mut appended_provenance {
            if prov_record.event.is_none() && prov_record.record_kind.is_policy_application() {
                prov_record.event = Some(drained_event_name.clone());
            }
        }
        populate_provenance_record_fields(
            &mut appended_provenance,
            &kernel,
            &record.process.definition_version,
        );
        stamp_provenance(&mut appended_provenance, &now_iso);
        record.process.provenance_position += appended_provenance.len() as u64;
        record.provenance_log.extend(appended_provenance.clone());
        self.store.save_record(record.clone())?;

        self.deliver_pending_presentations(&pending_presentations)?;

        runtime_result.provenance = appended_provenance;
        runtime_result.created_task_ids = created_task_ids;
        runtime_result.emitted_events = emitted_events;
        Ok(runtime_result)
    }

    /// Drain events until the queue is empty and no timers are due.
    pub fn drain_until_idle(
        &mut self,
        process_id: &str,
    ) -> Result<Vec<DrainOnceResult>, RuntimeError> {
        let mut results = Vec::new();

        loop {
            let result = self.drain_once(process_id)?;
            let should_stop = result.processed_event.is_none();
            if should_stop {
                break;
            }
            results.push(result);
        }

        Ok(results)
    }
}
