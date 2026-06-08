// Rust guideline compliant 2026-06-08

//! Deterministic evaluator for activation criteria (ADR 0096; WOS-ACT-0401..0405).
//!
//! Evaluates an [`ActivationCriteria`](crate::model::activation::ActivationCriteria)
//! against an [`ActivationContext`] in the spec-mandated order
//! (trigger → actor → required-data → FEL guard → deadline hint), short-circuiting
//! on the first failing clause (Governance §16.1.2). FEL is the only expression
//! language; `where` MUST be boolean — a non-boolean or error result fails
//! activation rather than being coerced by truthiness.
//!
//! This is pure: it reads the criteria and context and returns an
//! [`ActivationDecision`]; it never mutates state or schedules timers.

use std::collections::HashMap;

use fel_core::{MapEnvironment, evaluate, json_to_fel, parse, types::Value};
use wos_events::ActorKind;

use crate::model::activation::{
    ActivationCriteria, ActivationTrigger, ActorConstraint, EventScope,
};

/// Runtime inputs to activation evaluation.
///
/// Constructed from the event being processed plus the current process state.
pub struct ActivationContext<'a> {
    /// Concrete runtime event name.
    pub event_name: &'a str,
    /// Event payload object, if any.
    pub event_data: Option<&'a serde_json::Value>,
    /// Semantic tags carried by the event.
    pub event_tags: &'a [String],
    /// Kernel event family, if known.
    pub event_kind: Option<crate::model::activation::EventKind>,
    /// Acting actor identifier.
    pub actor_id: Option<&'a str>,
    /// Acting actor roles.
    pub actor_roles: &'a [String],
    /// Acting actor kind.
    pub actor_type: Option<ActorKind>,
    /// Case state object (keys are case-file fields, as in deontic evaluation).
    pub case_state: &'a serde_json::Value,
    /// Semantic tags of the firing transition.
    pub transition_tags: &'a [String],
    /// Current wall-clock time in epoch milliseconds.
    pub now_ms: u64,
    /// Actor that activated the obligation, for `notSameAsTriggerActor`
    /// (satisfaction/violation context only; `None` on activation).
    pub trigger_actor_id: Option<&'a str>,
    /// Whether this event was surfaced from a *related* case rather than the
    /// case being evaluated (ADR 0096; WOS-INTEG-REL-2101). A trigger whose
    /// `event_scope` is `related` matches ONLY when this is `true`; a trigger
    /// scoped to `this` (the default) matches ONLY when this is `false`.
    /// Defaults to `false` (own-case event); the related-case event source is a
    /// runtime follow-up, so the reference drain currently always passes `false`.
    pub is_related_event: bool,
}

impl<'a> ActivationContext<'a> {
    /// Construct a minimal context for an event with no actor/tag information.
    pub fn for_event(event_name: &'a str, case_state: &'a serde_json::Value, now_ms: u64) -> Self {
        ActivationContext {
            event_name,
            event_data: None,
            event_tags: &[],
            event_kind: None,
            actor_id: None,
            actor_roles: &[],
            actor_type: None,
            case_state,
            transition_tags: &[],
            now_ms,
            trigger_actor_id: None,
            is_related_event: false,
        }
    }
}

/// The outcome of evaluating an activation criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivationDecision {
    /// Whether the criteria matched.
    pub matched: bool,
    /// Why it matched or did not (deterministic, for traces).
    pub reason: ActivationDecisionReason,
    /// Deadline hint, present only on a match with a `within` clause.
    pub deadline_hint: Option<DeadlineHint>,
}

/// Deterministic reason for an [`ActivationDecision`] (Governance §16.1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationDecisionReason {
    /// All declared clauses matched.
    Matched,
    /// The trigger did not match the event/transition.
    TriggerMismatch,
    /// The actor constraint did not match.
    ActorMismatch,
    /// A required-data path was missing or null (carries the failing path).
    MissingRequiredData(String),
    /// The FEL guard evaluated to boolean false.
    GuardFalse,
    /// The FEL guard evaluated to a non-boolean value (fails activation).
    GuardNonBoolean,
    /// The FEL guard failed to parse or evaluate.
    GuardError,
}

/// Enough metadata for a downstream surface to compute a deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineHint {
    /// The `within` duration string (ISO 8601; `P<N>BD` business days).
    pub within: String,
    /// Business Calendar sidecar URI, when business-day arithmetic applies.
    pub calendar_ref: Option<String>,
}

/// Evaluate an activation criteria against a context, deterministically.
pub fn evaluate_activation_criteria(
    criteria: &ActivationCriteria,
    ctx: &ActivationContext<'_>,
) -> ActivationDecision {
    // 1. Trigger.
    if let Some(trigger) = &criteria.on {
        if !trigger_matches(trigger, ctx) {
            return mismatch(ActivationDecisionReason::TriggerMismatch);
        }
    }

    // 2. Actor.
    if let Some(actor) = &criteria.actor {
        if !actor_matches(actor, ctx) {
            return mismatch(ActivationDecisionReason::ActorMismatch);
        }
    }

    // 3. Required data.
    for path in &criteria.required_data {
        if !required_data_present(path, ctx) {
            return mismatch(ActivationDecisionReason::MissingRequiredData(path.clone()));
        }
    }

    // 4. FEL guard.
    if let Some(expr) = &criteria.where_fel {
        match eval_guard(expr, ctx) {
            GuardEval::True => {}
            GuardEval::False => return mismatch(ActivationDecisionReason::GuardFalse),
            GuardEval::NonBoolean => return mismatch(ActivationDecisionReason::GuardNonBoolean),
            GuardEval::Error => return mismatch(ActivationDecisionReason::GuardError),
        }
    }

    // 5. Deadline hint.
    let deadline_hint = criteria.within.as_ref().map(|w| DeadlineHint {
        within: w.clone(),
        calendar_ref: criteria.calendar_ref.clone(),
    });

    ActivationDecision {
        matched: true,
        reason: ActivationDecisionReason::Matched,
        deadline_hint,
    }
}

fn mismatch(reason: ActivationDecisionReason) -> ActivationDecision {
    ActivationDecision {
        matched: false,
        reason,
        deadline_hint: None,
    }
}

fn trigger_matches(trigger: &ActivationTrigger, ctx: &ActivationContext<'_>) -> bool {
    // Event scope (WOS-INTEG-REL-2101): `related` matches only related-case
    // events; the default (`this`, or absent) matches only own-case events.
    // The scope gate runs first because a trigger that names the right event on
    // the wrong case MUST NOT match.
    match trigger.event_scope {
        Some(EventScope::Related) => {
            if !ctx.is_related_event {
                return false;
            }
        }
        Some(EventScope::This) | None => {
            if ctx.is_related_event {
                return false;
            }
        }
    }
    if let Some(event) = &trigger.event {
        if event != ctx.event_name {
            return false;
        }
    }
    if let Some(tag) = &trigger.event_tag {
        if !ctx.event_tags.iter().any(|t| t == tag) {
            return false;
        }
    }
    if let Some(kind) = trigger.event_kind {
        // Cannot confirm a kind we were not told about.
        if ctx.event_kind != Some(kind) {
            return false;
        }
    }
    if let Some(tag) = &trigger.transition_tag {
        if !ctx.transition_tags.iter().any(|t| t == tag) {
            return false;
        }
    }
    true
}

fn actor_matches(constraint: &ActorConstraint, ctx: &ActivationContext<'_>) -> bool {
    if let Some(id) = &constraint.actor_id {
        if ctx.actor_id != Some(id.as_str()) {
            return false;
        }
    }
    if let Some(role) = &constraint.role {
        if !ctx.actor_roles.iter().any(|r| r == role) {
            return false;
        }
    }
    if let Some(kind) = constraint.actor_type {
        if ctx.actor_type != Some(kind) {
            return false;
        }
    }
    if constraint.not_same_as_trigger_actor == Some(true) {
        // Only meaningful when a triggering actor was recorded; if the acting
        // actor equals the trigger actor, the constraint fails.
        if let (Some(cur), Some(trigger)) = (ctx.actor_id, ctx.trigger_actor_id) {
            if cur == trigger {
                return false;
            }
        }
    }
    true
}

/// Resolve a dotted path into the named namespace and require present-non-null.
fn required_data_present(path: &str, ctx: &ActivationContext<'_>) -> bool {
    let mut parts = path.split('.');
    let namespace = match parts.next() {
        Some(ns) => ns,
        None => return false,
    };
    let rest: Vec<&str> = parts.collect();

    let actor_obj;
    let root: &serde_json::Value = match namespace {
        "caseFile" => ctx.case_state,
        "event" => match ctx.event_data {
            Some(v) => v,
            None => return false,
        },
        "actor" => {
            actor_obj = actor_value(ctx);
            &actor_obj
        }
        // `context` and any other namespace carry no presence data here.
        _ => return false,
    };

    match resolve_path(root, &rest) {
        Some(v) => !v.is_null(),
        None => false,
    }
}

fn resolve_path<'v>(root: &'v serde_json::Value, parts: &[&str]) -> Option<&'v serde_json::Value> {
    let mut current = root;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

fn actor_value(ctx: &ActivationContext<'_>) -> serde_json::Value {
    serde_json::json!({
        "id": ctx.actor_id,
        "type": ctx.actor_type.map(actor_kind_str),
        "roles": ctx.actor_roles,
    })
}

fn actor_kind_str(kind: ActorKind) -> &'static str {
    match kind {
        ActorKind::Human => "human",
        ActorKind::System => "system",
        ActorKind::Agent => "agent",
    }
}

enum GuardEval {
    True,
    False,
    NonBoolean,
    Error,
}

/// Evaluate the `where` FEL guard over the canonical activation namespaces.
///
/// Builds the same flattened `namespace.field` environment as deontic
/// evaluation ([`crate::deontic`]): `caseFile.*`, `event.*`, `actor.*`.
fn eval_guard(expression: &str, ctx: &ActivationContext<'_>) -> GuardEval {
    let parsed = match parse(expression) {
        Ok(ast) => ast,
        Err(_) => return GuardEval::Error,
    };

    let mut fields: HashMap<String, Value> = HashMap::new();

    if let serde_json::Value::Object(map) = ctx.case_state {
        for (key, value) in map {
            fields.insert(format!("caseFile.{key}"), json_to_fel(value));
        }
    }
    if let Some(serde_json::Value::Object(map)) = ctx.event_data {
        for (key, value) in map {
            fields.insert(format!("event.{key}"), json_to_fel(value));
        }
    }
    if let Some(id) = ctx.actor_id {
        fields.insert("actor.id".to_string(), json_to_fel(&serde_json::json!(id)));
    }
    if let Some(kind) = ctx.actor_type {
        fields.insert(
            "actor.type".to_string(),
            json_to_fel(&serde_json::json!(actor_kind_str(kind))),
        );
    }
    fields.insert(
        "actor.roles".to_string(),
        json_to_fel(&serde_json::json!(ctx.actor_roles)),
    );

    let env = MapEnvironment::with_fields(fields);
    match evaluate(&parsed, &env).value {
        Value::Boolean(true) => GuardEval::True,
        Value::Boolean(false) => GuardEval::False,
        _ => GuardEval::NonBoolean,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::activation::{ActivationTrigger, ActorConstraint};

    fn ctx_for<'a>(
        event: &'a str,
        event_data: Option<&'a serde_json::Value>,
        case_state: &'a serde_json::Value,
    ) -> ActivationContext<'a> {
        ActivationContext {
            event_name: event,
            event_data,
            event_tags: &[],
            event_kind: None,
            actor_id: None,
            actor_roles: &[],
            actor_type: None,
            case_state,
            transition_tags: &[],
            now_ms: 0,
            trigger_actor_id: None,
            is_related_event: false,
        }
    }

    #[test]
    fn event_mismatch_does_not_match() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            on: Some(ActivationTrigger {
                event: Some("incomeChanged".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = ctx_for("somethingElse", None, &cs);
        let d = evaluate_activation_criteria(&c, &ctx);
        assert!(!d.matched);
        assert_eq!(d.reason, ActivationDecisionReason::TriggerMismatch);
    }

    #[test]
    fn event_match_matches() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            on: Some(ActivationTrigger {
                event: Some("incomeChanged".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = ctx_for("incomeChanged", None, &cs);
        assert!(evaluate_activation_criteria(&c, &ctx).matched);
    }

    #[test]
    fn missing_required_data_does_not_match() {
        let cs = serde_json::json!({ "name": "a" });
        let c = ActivationCriteria {
            required_data: vec!["caseFile.income".into()],
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        let d = evaluate_activation_criteria(&c, &ctx);
        assert!(!d.matched);
        assert_eq!(
            d.reason,
            ActivationDecisionReason::MissingRequiredData("caseFile.income".into())
        );
    }

    #[test]
    fn null_required_data_does_not_match() {
        let cs = serde_json::json!({ "income": null });
        let c = ActivationCriteria {
            required_data: vec!["caseFile.income".into()],
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        assert!(!evaluate_activation_criteria(&c, &ctx).matched);
    }

    #[test]
    fn present_required_data_matches_including_nested() {
        let cs = serde_json::json!({ "applicant": { "householdSize": 3 } });
        let c = ActivationCriteria {
            required_data: vec!["caseFile.applicant.householdSize".into()],
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        assert!(evaluate_activation_criteria(&c, &ctx).matched);
    }

    #[test]
    fn fel_true_matches_and_false_does_not() {
        let cs = serde_json::json!({ "income": 60000, "priorIncome": 50000 });
        let c = ActivationCriteria {
            where_fel: Some("caseFile.income > caseFile.priorIncome".into()),
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        assert!(evaluate_activation_criteria(&c, &ctx).matched);

        let cs2 = serde_json::json!({ "income": 40000, "priorIncome": 50000 });
        let ctx2 = ctx_for("e", None, &cs2);
        let d = evaluate_activation_criteria(&c, &ctx2);
        assert!(!d.matched);
        assert_eq!(d.reason, ActivationDecisionReason::GuardFalse);
    }

    #[test]
    fn fel_over_event_payload() {
        let cs = serde_json::json!({});
        let ev = serde_json::json!({ "field": "income" });
        let c = ActivationCriteria {
            where_fel: Some("event.field = 'income'".into()),
            ..Default::default()
        };
        let ctx = ctx_for("caseFileUpdated", Some(&ev), &cs);
        assert!(evaluate_activation_criteria(&c, &ctx).matched);
    }

    #[test]
    fn fel_non_boolean_fails_activation() {
        let cs = serde_json::json!({ "income": 60000 });
        let c = ActivationCriteria {
            where_fel: Some("caseFile.income".into()),
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        let d = evaluate_activation_criteria(&c, &ctx);
        assert!(!d.matched);
        assert_eq!(d.reason, ActivationDecisionReason::GuardNonBoolean);
    }

    #[test]
    fn fel_parse_error_is_deterministic() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            where_fel: Some("this is not (valid fel".into()),
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        assert_eq!(
            evaluate_activation_criteria(&c, &ctx).reason,
            ActivationDecisionReason::GuardError
        );
    }

    #[test]
    fn actor_role_membership() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            actor: Some(ActorConstraint {
                role: Some("underwriter".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let roles = vec!["underwriter".to_string(), "reviewer".to_string()];
        let mut ctx = ctx_for("e", None, &cs);
        ctx.actor_roles = &roles;
        assert!(evaluate_activation_criteria(&c, &ctx).matched);

        let other = vec!["clerk".to_string()];
        let mut ctx2 = ctx_for("e", None, &cs);
        ctx2.actor_roles = &other;
        let d = evaluate_activation_criteria(&c, &ctx2);
        assert!(!d.matched);
        assert_eq!(d.reason, ActivationDecisionReason::ActorMismatch);
    }

    #[test]
    fn actor_type_match() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            actor: Some(ActorConstraint {
                actor_type: Some(ActorKind::Agent),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut ctx = ctx_for("e", None, &cs);
        ctx.actor_type = Some(ActorKind::Agent);
        assert!(evaluate_activation_criteria(&c, &ctx).matched);
        ctx.actor_type = Some(ActorKind::Human);
        assert!(!evaluate_activation_criteria(&c, &ctx).matched);
    }

    #[test]
    fn not_same_as_trigger_actor_blocks_self_satisfaction() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            actor: Some(ActorConstraint {
                role: Some("reviewer".into()),
                not_same_as_trigger_actor: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        let roles = vec!["reviewer".to_string()];
        // Same actor as trigger: must not match.
        let mut ctx = ctx_for("reviewCompleted", None, &cs);
        ctx.actor_roles = &roles;
        ctx.actor_id = Some("agent-1");
        ctx.trigger_actor_id = Some("agent-1");
        assert!(!evaluate_activation_criteria(&c, &ctx).matched);
        // Different actor: matches.
        ctx.actor_id = Some("agent-2");
        assert!(evaluate_activation_criteria(&c, &ctx).matched);
    }

    #[test]
    fn event_scope_related_matches_only_related_events() {
        use crate::model::activation::EventScope;
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            on: Some(ActivationTrigger {
                event: Some("incomeChanged".into()),
                event_scope: Some(EventScope::Related),
                ..Default::default()
            }),
            ..Default::default()
        };
        // Own-case event with `related` scope: must not match.
        let own = ctx_for("incomeChanged", None, &cs);
        let d = evaluate_activation_criteria(&c, &own);
        assert!(!d.matched);
        assert_eq!(d.reason, ActivationDecisionReason::TriggerMismatch);
        // Related-case event with `related` scope: matches.
        let mut related = ctx_for("incomeChanged", None, &cs);
        related.is_related_event = true;
        assert!(evaluate_activation_criteria(&c, &related).matched);
    }

    #[test]
    fn default_scope_matches_only_own_case_events() {
        let cs = serde_json::json!({});
        // No `event_scope` → defaults to `this` (own-case only).
        let c = ActivationCriteria {
            on: Some(ActivationTrigger {
                event: Some("incomeChanged".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let own = ctx_for("incomeChanged", None, &cs);
        assert!(evaluate_activation_criteria(&c, &own).matched);
        // A related-case event MUST NOT satisfy a `this`-scoped (default) trigger.
        let mut related = ctx_for("incomeChanged", None, &cs);
        related.is_related_event = true;
        assert!(!evaluate_activation_criteria(&c, &related).matched);
    }

    #[test]
    fn deadline_hint_returned_on_match() {
        let cs = serde_json::json!({});
        let c = ActivationCriteria {
            within: Some("P2D".into()),
            calendar_ref: Some("urn:wos:calendar:federal-fy2026".into()),
            ..Default::default()
        };
        let ctx = ctx_for("e", None, &cs);
        let d = evaluate_activation_criteria(&c, &ctx);
        assert!(d.matched);
        let hint = d.deadline_hint.expect("hint");
        assert_eq!(hint.within, "P2D");
        assert_eq!(
            hint.calendar_ref.as_deref(),
            Some("urn:wos:calendar:federal-fy2026")
        );
    }
}
