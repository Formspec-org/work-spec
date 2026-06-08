// Rust guideline compliant 2026-04-16

//! PROV-O JSON-LD serializer (WOS Semantic Profile §5).
//!
//! Serializes a [`ProvenanceLog`] into a W3C PROV-O graph expressed as JSON-LD.
//! Every Facts tier record becomes a `prov:Activity` node linked (when an actor
//! is known) to a deduplicated `prov:Agent` node, per §5.3.
//!
//! # Scope filter (§6.5)
//!
//! Higher-tier provenance records (Reasoning, Counterfactual, Narrative) are
//! excluded from the graph by default: §6.5 restricts process-mining/PROV-O
//! export to Facts-tier records. Records with `audit_layer = None` are treated
//! as Facts for backward compatibility with pre-extension runtimes.
//!
//! # Actor type mapping (§5.5)
//!
//! When `ProvenanceRecord.actor_type` is populated, the emitted `prov:Agent`
//! node carries the matching §5.5 subclass pair:
//!
//! - `Some("human")`  → `["prov:Person", "wos:HumanAgent"]`
//! - `Some("system")` → `["prov:SoftwareAgent", "wos:SystemAgent"]`
//! - `Some("agent")`  → `["prov:SoftwareAgent", "wos:AIAgent"]`
//! - `None`           → `"prov:Agent"` (actor not in any registry)
//!
//! Distinct `(actor_id, actor_type)` pairs produce distinct agent nodes. The
//! common case (one actor_type per actor_id) behaves identically to pure
//! actor-id dedup; the tuple key only matters when a single id appears under
//! multiple types, which the spec permits.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use wos_core::{ProvenanceLog, ProvenanceRecord};

use crate::{ExportConfig, camel_case_record_kind, is_facts_tier};

/// A PROV-O graph serialized as JSON-LD (§5.6).
#[derive(Debug, Serialize)]
pub struct ProvODocument {
    /// JSON-LD `@context` declaring the `prov:`, `xsd:`, and `wos:` prefixes.
    #[serde(rename = "@context")]
    pub context: Value,
    /// The PROV-O graph: activities and deduplicated agents.
    #[serde(rename = "@graph")]
    pub graph: Vec<Value>,
}

/// Serialize a provenance log to a PROV-O JSON-LD document (§5.3, §5.6).
///
/// Each Facts-tier [`ProvenanceRecord`] becomes a `prov:Activity`, plus one
/// `prov:Entity` per input (linked via `prov:used`) and one per output
/// (linked via `prov:generated`). Agents are deduplicated by
/// `(actor_id, actor_type)`.
///
/// Records with an empty `timestamp` (never persistence-stamped; see
/// [`ProvenanceRecord::timestamp`] docs) omit `prov:atTime` because the
/// canonical time is unknown — emitting an empty string would poison any
/// downstream `xsd:dateTime` consumer.
#[must_use]
pub fn export(log: &ProvenanceLog, config: &ExportConfig) -> ProvODocument {
    // `provenance_namespace` is concatenated directly with record ids to mint
    // IRIs (see `ExportConfig::provenance_namespace` docs). A caller that
    // forgets the trailing separator produces malformed IRIs; guard against
    // it in debug builds so tests and CI surface the misuse loudly, but keep
    // release builds hot (production paths should use validated config).
    debug_assert!(
        config.provenance_namespace.ends_with(':') || config.provenance_namespace.ends_with('/'),
        "ExportConfig::provenance_namespace must end with ':' or '/' to mint valid IRIs, got '{}'",
        config.provenance_namespace,
    );

    // §6.5 scope filter: Facts-tier records only. `None` is Facts for
    // backward compatibility with pre-extension runtimes.
    let facts_records: Vec<(usize, &ProvenanceRecord)> = log
        .records()
        .iter()
        .enumerate()
        .filter(|(_, record)| is_facts_tier(record))
        .collect();

    // Agents are emitted in first-seen order (the `agents` Vec records the
    // order; `seen_actors` is only a membership filter). Stable ordering
    // keeps snapshot diffs clean for Task 5 fixtures. Key includes
    // `actor_type` so the rare case of one id under multiple subclasses
    // yields distinct nodes (§5.5).
    let mut seen_actors: BTreeSet<(String, Option<String>)> = BTreeSet::new();
    let mut activities_and_entities: Vec<Value> = Vec::new();
    let mut agents: Vec<Value> = Vec::new();

    for (index, record) in &facts_records {
        let (activity, entities) = activity_with_entities(*index, record, config);
        activities_and_entities.push(activity);
        activities_and_entities.extend(entities);

        if let Some(actor_id) = record.actor_id.as_deref() {
            let key = (actor_id.to_owned(), record.actor_type.clone());
            if seen_actors.insert(key) {
                agents.push(agent_node(actor_id, record.actor_type.as_deref(), config));
            }
        }
    }

    let mut graph = activities_and_entities;
    graph.extend(agents);

    ProvODocument {
        context: context_object(),
        graph,
    }
}

/// Build the `@context` object. The three prefixes called out in §5.6 /
/// Appendix C are mandatory for a valid PROV-O JSON-LD emission.
fn context_object() -> Value {
    json!({
        "prov": "http://www.w3.org/ns/prov#",
        "xsd": "http://www.w3.org/2001/XMLSchema#",
        "wos": "https://wos-spec.org/ns/",
    })
}

/// Emit a `prov:Activity` plus every input/output `prov:Entity` it references.
/// The Activity gets `prov:used` / `prov:generated` IRI arrays; the entities
/// become sibling nodes in the @graph so JSON-LD consumers can resolve them.
fn activity_with_entities(
    index: usize,
    record: &ProvenanceRecord,
    config: &ExportConfig,
) -> (Value, Vec<Value>) {
    // `ProvenanceRecord` has no stable id today; mint a deterministic one
    // from the record's position in the log. When a record id is added
    // upstream this is the one site that needs to change.
    let activity_id = format!("{}{index}", config.provenance_namespace);
    let action_type = camel_case_record_kind(record);

    let mut node = serde_json::Map::new();
    node.insert("@id".into(), Value::String(activity_id));
    node.insert("@type".into(), Value::String("prov:Activity".into()));
    node.insert("wos:actionType".into(), Value::String(action_type));

    if !record.timestamp.is_empty() {
        node.insert(
            "prov:atTime".into(),
            Value::String(record.timestamp.clone()),
        );
    }

    if let Some(actor_id) = record.actor_id.as_deref() {
        node.insert(
            "prov:wasAssociatedWith".into(),
            Value::String(agent_iri(actor_id, config)),
        );
    }

    // §5.3 `wos:atLifecycleState`: authoritative source is `lifecycle_state`
    // once the runtime populates it. Task 3 guarantees transition records
    // carry `Some(...)`; records without it (e.g. counters, events outside
    // the lifecycle machine) omit the property.
    if let Some(lifecycle_state) = record.lifecycle_state.as_deref() {
        node.insert(
            "wos:atLifecycleState".into(),
            Value::String(lifecycle_state.to_owned()),
        );
    }

    // §5.3 `wos:definitionVersion`: governing document version.
    if let Some(version) = record.definition_version.as_deref() {
        node.insert(
            "wos:definitionVersion".into(),
            Value::String(version.to_owned()),
        );
    }

    // §5.3 `prov:used`: one Entity per input. The entity's `@id` is minted
    // from the `(activity_index, item_index)` coordinate, which addresses
    // the original `ProvenanceRecord.inputs[item_index]` slot. Consumers
    // that need the runtime's opaque reference look it up there — we do
    // NOT carry a redundant `wos:entityRef` property (not defined in §5.3).
    //
    // The digest covers the ENTIRE inputs/outputs vec, not the per-item
    // entity. Attaching it to the first entity is a convention; attaching
    // to every entity would falsely imply per-item digests. Consumers
    // should treat the digest as a property of the activity's input/output
    // bundle, not of any individual entity.
    let mut entities: Vec<Value> = Vec::new();
    if !record.inputs.is_empty() {
        let mut used_iris: Vec<Value> = Vec::with_capacity(record.inputs.len());
        for item_index in 0..record.inputs.len() {
            let entity_iri = format!(
                "{}entity/input/{index}/{item_index}",
                config.provenance_namespace
            );
            used_iris.push(Value::String(entity_iri.clone()));

            let mut entity = serde_json::Map::new();
            entity.insert("@id".into(), Value::String(entity_iri));
            entity.insert("@type".into(), Value::String("prov:Entity".into()));
            if item_index == 0
                && let Some(digest) = record.input_digest.as_deref()
            {
                entity.insert("wos:inputDigest".into(), Value::String(digest.to_owned()));
            }
            entities.push(Value::Object(entity));
        }
        node.insert("prov:used".into(), Value::Array(used_iris));
    }

    // §5.3 `prov:generated`: symmetric to `prov:used` for outputs. Same
    // digest-on-first-entity rationale as above.
    if !record.outputs.is_empty() {
        let mut generated_iris: Vec<Value> = Vec::with_capacity(record.outputs.len());
        for item_index in 0..record.outputs.len() {
            let entity_iri = format!(
                "{}entity/output/{index}/{item_index}",
                config.provenance_namespace
            );
            generated_iris.push(Value::String(entity_iri.clone()));

            let mut entity = serde_json::Map::new();
            entity.insert("@id".into(), Value::String(entity_iri));
            entity.insert("@type".into(), Value::String("prov:Entity".into()));
            if item_index == 0
                && let Some(digest) = record.output_digest.as_deref()
            {
                entity.insert("wos:outputDigest".into(), Value::String(digest.to_owned()));
            }
            entities.push(Value::Object(entity));
        }
        node.insert("prov:generated".into(), Value::Array(generated_iris));
    }

    (Value::Object(node), entities)
}

/// Emit a `prov:Agent` node for a unique (actor_id, actor_type) pair (§5.3, §5.5).
fn agent_node(actor_id: &str, actor_type: Option<&str>, config: &ExportConfig) -> Value {
    // §5.5 subclass pair. Unknown/missing actor_type falls back to plain
    // `prov:Agent` — this preserves backward compatibility with records
    // whose actor is not resolvable in any registry.
    let type_value: Value = match actor_type {
        Some("human") => json!(["prov:Person", "wos:HumanAgent"]),
        Some("system") => json!(["prov:SoftwareAgent", "wos:SystemAgent"]),
        Some("agent") => json!(["prov:SoftwareAgent", "wos:AIAgent"]),
        _ => Value::String("prov:Agent".into()),
    };

    json!({
        "@id": agent_iri(actor_id, config),
        "@type": type_value,
    })
}

/// Compose the IRI used for a `prov:Agent` node so activities can link to it.
fn agent_iri(actor_id: &str, config: &ExportConfig) -> String {
    format!("{}agent/{actor_id}", config.provenance_namespace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

    fn config() -> ExportConfig {
        ExportConfig {
            provenance_namespace: "urn:wos:prov:test:".to_string(),
            process_id: "test-instance".to_string(),
        }
    }

    fn stamped_transition(actor: Option<&str>) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", actor);
        record.timestamp = "2026-01-01T00:00:00Z".into();
        record
    }

    #[test]
    fn context_includes_prov_xsd_wos_namespaces() {
        let log = ProvenanceLog::default();
        let document = export(&log, &config());

        let context = &document.context;
        assert_eq!(context["prov"], "http://www.w3.org/ns/prov#");
        assert_eq!(context["xsd"], "http://www.w3.org/2001/XMLSchema#");
        assert_eq!(context["wos"], "https://wos-spec.org/ns/");
    }

    #[test]
    fn exports_state_transition_as_prov_activity() {
        let mut log = ProvenanceLog::default();
        // Populate inputs/outputs so this exercises the full §5.3 emission:
        // 1 Activity + 1 input Entity + 1 output Entity + 1 Agent = 4 nodes.
        let mut record = stamped_transition(Some("user-42"));
        record.inputs = vec!["case/123".into()];
        record.outputs = vec!["case/123#state".into()];
        record.input_digest = Some("sha256:aaaa".into());
        record.output_digest = Some("sha256:bbbb".into());
        record.lifecycle_state = Some("a".into());
        record.definition_version = Some("1.0.0".into());
        log.push(record);

        let document = export(&log, &config());

        // Activity + input Entity + output Entity + Agent.
        assert_eq!(document.graph.len(), 4);

        let activity = &document.graph[0];
        assert_eq!(activity["@id"], "urn:wos:prov:test:0");
        assert_eq!(activity["@type"], "prov:Activity");
        assert_eq!(activity["wos:actionType"], "stateTransition");
        assert_eq!(activity["prov:atTime"], "2026-01-01T00:00:00Z");
        assert_eq!(activity["wos:atLifecycleState"], "a");
        assert_eq!(activity["wos:definitionVersion"], "1.0.0");
        assert_eq!(
            activity["prov:wasAssociatedWith"],
            "urn:wos:prov:test:agent/user-42"
        );
        assert_eq!(
            activity["prov:used"],
            json!(["urn:wos:prov:test:entity/input/0/0"])
        );
        assert_eq!(
            activity["prov:generated"],
            json!(["urn:wos:prov:test:entity/output/0/0"])
        );

        let input_entity = &document.graph[1];
        assert_eq!(input_entity["@id"], "urn:wos:prov:test:entity/input/0/0");
        assert_eq!(input_entity["@type"], "prov:Entity");
        // §5.3 defines no `wos:entityRef` property — the `@id` coordinate
        // already decodes to the `ProvenanceRecord.inputs[item_index]` slot.
        assert!(
            input_entity.get("wos:entityRef").is_none(),
            "wos:entityRef is not in §5.3 and must not be emitted: {input_entity}"
        );
        assert_eq!(input_entity["wos:inputDigest"], "sha256:aaaa");

        let output_entity = &document.graph[2];
        assert_eq!(output_entity["@id"], "urn:wos:prov:test:entity/output/0/0");
        assert_eq!(output_entity["@type"], "prov:Entity");
        assert!(
            output_entity.get("wos:entityRef").is_none(),
            "wos:entityRef is not in §5.3 and must not be emitted: {output_entity}"
        );
        assert_eq!(output_entity["wos:outputDigest"], "sha256:bbbb");

        let agent = &document.graph[3];
        assert_eq!(agent["@id"], "urn:wos:prov:test:agent/user-42");
        // actor_type unset → plain prov:Agent fallback.
        assert_eq!(agent["@type"], "prov:Agent");
    }

    #[test]
    fn omits_prov_at_time_when_timestamp_is_empty() {
        let mut log = ProvenanceLog::default();
        // Unstamped: timestamp is empty. §5.3 documents this as "unknown".
        log.push(ProvenanceRecord::state_transition(
            "a",
            "b",
            "ev",
            Some("user-1"),
        ));

        let document = export(&log, &config());

        let activity = &document.graph[0];
        assert!(
            activity.get("prov:atTime").is_none(),
            "empty timestamp must not emit prov:atTime, got: {activity}"
        );
        // Other properties still present.
        assert_eq!(activity["@type"], "prov:Activity");
        assert_eq!(activity["wos:actionType"], "stateTransition");
    }

    #[test]
    fn deduplicates_agents_across_records() {
        let mut log = ProvenanceLog::default();
        for _ in 0..3 {
            log.push(stamped_transition(Some("user-42")));
        }

        let document = export(&log, &config());

        // 3 activities + exactly 1 agent (no inputs/outputs on these records).
        assert_eq!(document.graph.len(), 4);
        let agents: Vec<_> = document
            .graph
            .iter()
            .filter(|node| node["@type"] == "prov:Agent")
            .collect();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["@id"], "urn:wos:prov:test:agent/user-42");
    }

    #[test]
    fn omits_agent_link_when_actor_id_absent() {
        let mut log = ProvenanceLog::default();
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", None);
        record.timestamp = "2026-01-01T00:00:00Z".into();
        log.push(record);

        let document = export(&log, &config());

        // No agent node, just the activity.
        assert_eq!(document.graph.len(), 1);
        let activity = &document.graph[0];
        assert!(
            activity.get("prov:wasAssociatedWith").is_none(),
            "actor-less record must not emit prov:wasAssociatedWith"
        );
        assert!(
            !document
                .graph
                .iter()
                .any(|node| node["@type"] == "prov:Agent"),
            "actor-less record must not emit any prov:Agent node"
        );
    }

    #[test]
    fn emits_at_lifecycle_state_from_field() {
        // §5.3: authoritative source is `lifecycle_state`, populated by the
        // runtime at stamp time. Records without it omit the property.
        let mut log = ProvenanceLog::default();
        let mut with_state = stamped_transition(Some("user-1"));
        with_state.lifecycle_state = Some("draft".into());
        log.push(with_state);
        // Second record has no lifecycle_state → property omitted.
        log.push(ProvenanceRecord::state_entered("approved"));

        let document = export(&log, &config());

        let first = &document.graph[0];
        assert_eq!(
            first["wos:atLifecycleState"], "draft",
            "record with lifecycle_state must emit wos:atLifecycleState: {first}"
        );

        let second = &document.graph[1];
        assert!(
            second.get("wos:atLifecycleState").is_none(),
            "record without lifecycle_state must not emit the field: {second}"
        );
    }

    #[test]
    fn emits_human_agent_subclass_for_actor_type_human() {
        // §5.5: `actor_type = "human"` → [prov:Person, wos:HumanAgent].
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("user-42"));
        record.actor_type = Some("human".into());
        log.push(record);

        let document = export(&log, &config());

        let agent = document
            .graph
            .iter()
            .find(|node| {
                node.get("@id") == Some(&Value::String("urn:wos:prov:test:agent/user-42".into()))
            })
            .expect("agent node must exist");
        assert_eq!(agent["@type"], json!(["prov:Person", "wos:HumanAgent"]));
    }

    #[test]
    fn emits_system_agent_subclass_for_actor_type_system() {
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("runtime"));
        record.actor_type = Some("system".into());
        log.push(record);

        let document = export(&log, &config());

        let agent = document
            .graph
            .iter()
            .find(|node| {
                node.get("@id") == Some(&Value::String("urn:wos:prov:test:agent/runtime".into()))
            })
            .expect("agent node");
        assert_eq!(
            agent["@type"],
            json!(["prov:SoftwareAgent", "wos:SystemAgent"])
        );
    }

    #[test]
    fn emits_ai_agent_subclass_for_actor_type_agent() {
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("claude"));
        record.actor_type = Some("agent".into());
        log.push(record);

        let document = export(&log, &config());

        let agent = document
            .graph
            .iter()
            .find(|node| {
                node.get("@id") == Some(&Value::String("urn:wos:prov:test:agent/claude".into()))
            })
            .expect("agent node");
        assert_eq!(agent["@type"], json!(["prov:SoftwareAgent", "wos:AIAgent"]));
    }

    #[test]
    fn distinct_actor_types_for_same_id_produce_distinct_agents() {
        // §5.5 edge case: spec permits same actor_id under multiple subclasses.
        // Dedup key must include actor_type so each pair yields its own node.
        let mut log = ProvenanceLog::default();
        let mut human = stamped_transition(Some("id-1"));
        human.actor_type = Some("human".into());
        log.push(human);
        let mut system = stamped_transition(Some("id-1"));
        system.actor_type = Some("system".into());
        log.push(system);

        let document = export(&log, &config());

        let agents: Vec<_> = document
            .graph
            .iter()
            .filter(|node| {
                node.get("@id") == Some(&Value::String("urn:wos:prov:test:agent/id-1".into()))
            })
            .collect();
        assert_eq!(
            agents.len(),
            2,
            "distinct actor_type values for same id must yield distinct agent nodes"
        );
    }

    #[test]
    fn filters_non_facts_tier_records_per_section_6_5() {
        // §6.5: Narrative-tier records are excluded from default PROV-O export.
        let mut log = ProvenanceLog::default();

        let mut facts = stamped_transition(Some("user-1"));
        facts.audit_layer = Some("facts".into());
        log.push(facts);

        let mut narrative = stamped_transition(Some("user-2"));
        narrative.audit_layer = Some("narrative".into());
        log.push(narrative);

        let document = export(&log, &config());

        // Only the facts-tier activity survives; its agent is present; the
        // narrative record and its agent are absent.
        let activities: Vec<_> = document
            .graph
            .iter()
            .filter(|node| node["@type"] == "prov:Activity")
            .collect();
        assert_eq!(activities.len(), 1);

        assert!(
            !document.graph.iter().any(|node| node.get("@id")
                == Some(&Value::String("urn:wos:prov:test:agent/user-2".into()))),
            "narrative-tier agent must be excluded from default export"
        );
    }

    #[test]
    fn exports_obligation_violated_as_prov_activity_without_panic() {
        // WOS-OBL-EXPORT-1201: obligation records flow through the generic
        // §5.3 mapping. PROV-O does not project `data` (policyId/obligationId
        // live there), so the assertion is on the Activity envelope: a stable
        // `wos:actionType` and the actor association. The witness payload
        // round-trips inside the record's `data` for OCEL; here we only prove
        // the Facts-tier obligation record produces a valid Activity node.
        let mut log = ProvenanceLog::default();
        // Build the obligation record via the generic seam (record_kind + data)
        // rather than the typed constructor: `ObligationViolationWitness` lives
        // in wos-events and is not re-exported through wos-core, and this
        // exporter only needs the record_kind + envelope to exercise §5.3.
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", Some("caseworker-7"));
        record.record_kind = ProvenanceKind::ObligationViolated;
        record.data = Some(json!({
            "policyId": "policy-1",
            "obligationId": "obl-1",
            "reason": "deadline elapsed",
            "effectiveAction": "suspend",
            "responsibleActor": "caseworker-7",
        }));
        record.timestamp = "2026-01-01T00:00:00Z".into();
        log.push(record);

        let document = export(&log, &config());

        let activity = &document.graph[0];
        assert_eq!(activity["@type"], "prov:Activity");
        assert_eq!(activity["wos:actionType"], "obligationViolated");
        assert_eq!(
            activity["prov:wasAssociatedWith"],
            "urn:wos:prov:test:agent/caseworker-7"
        );
    }

    #[test]
    fn camel_cases_all_record_kinds() {
        // Exhaustive parity check: every `ProvenanceKind` variant exports as
        // its serde-camelCase string verbatim. Mirrors the
        // `audit_layer_for_kind_covers_every_variant` enumeration in
        // `wos-core/src/provenance/tests.rs` — adding a new variant upstream
        // forces an entry here. Catches the bug class where a new variant
        // ships with a wrong serde rename or where the export pipeline
        // accidentally drops `record_kind`.
        //
        // Background: cross-stack-scout 2026-04-28 surfaced this as a real
        // gap — the original spot-check covered 3 of 100 variants, leaving
        // 97 unverified. Extending mechanical because
        // `camel_case_record_kind` is a pure serde round-trip; the test
        // asserts the export pipeline preserves that round-trip end-to-end.
        let all: &[ProvenanceKind] = &[
            ProvenanceKind::StateTransition,
            ProvenanceKind::UnmatchedEvent,
            ProvenanceKind::CaseStateMutation,
            ProvenanceKind::CaseCreated,
            ProvenanceKind::IntakeAccepted,
            ProvenanceKind::IntakeRejected,
            ProvenanceKind::IntakeDeferred,
            ProvenanceKind::TimerCreated,
            ProvenanceKind::TimerFired,
            ProvenanceKind::TimerCancelled,
            ProvenanceKind::OnEntry,
            ProvenanceKind::OnExit,
            ProvenanceKind::ActionExecuted,
            ProvenanceKind::InvalidDuration,
            ProvenanceKind::ToleranceViolation,
            ProvenanceKind::ConvergenceCapReached,
            ProvenanceKind::CapabilityInvocation,
            ProvenanceKind::DeonticViolation,
            ProvenanceKind::DeonticEvaluation,
            ProvenanceKind::DeonticResolution,
            ProvenanceKind::DeonticBypass,
            ProvenanceKind::RightsViolation,
            ProvenanceKind::ConsistencyViolation,
            ProvenanceKind::AutonomyViolation,
            ProvenanceKind::AutonomyCapped,
            ProvenanceKind::AutonomyComputed,
            ProvenanceKind::HumanTaskCreated,
            ProvenanceKind::ToolViolation,
            ProvenanceKind::EscalationPending,
            ProvenanceKind::AutonomyDemotion,
            ProvenanceKind::ConfidenceViolation,
            ProvenanceKind::ConfidenceDecay,
            ProvenanceKind::CumulativeConfidenceViolation,
            ProvenanceKind::SessionPaused,
            ProvenanceKind::GroundTruthLabel,
            ProvenanceKind::AgentOutput,
            ProvenanceKind::ActorTypeViolation,
            ProvenanceKind::AgentProvenanceAnnotation,
            ProvenanceKind::AgentVersionChange,
            ProvenanceKind::NarrativeTierRecorded,
            ProvenanceKind::ConstraintTamperBlocked,
            ProvenanceKind::DriftReclassification,
            ProvenanceKind::AgentStateTransition,
            ProvenanceKind::ProxyInvocation,
            ProvenanceKind::DispositiveViolation,
            ProvenanceKind::FallbackTriggered,
            ProvenanceKind::FallbackAttempt,
            ProvenanceKind::FallbackTerminal,
            ProvenanceKind::NoticeSent,
            ProvenanceKind::SeparationViolation,
            ProvenanceKind::AppealFiled,
            ProvenanceKind::ProtocolViolation,
            ProvenanceKind::IndependentFirstEnforced,
            ProvenanceKind::SamplingDecision,
            ProvenanceKind::OverrideViolation,
            ProvenanceKind::OverrideRecorded,
            ProvenanceKind::PipelineStageCompleted,
            ProvenanceKind::PipelineRiskProfile,
            ProvenanceKind::PipelineRejection,
            ProvenanceKind::TaskCreated,
            ProvenanceKind::TaskPresented,
            ProvenanceKind::TaskDismissed,
            ProvenanceKind::TaskDraftPersisted,
            ProvenanceKind::TaskResponseSubmitted,
            ProvenanceKind::TaskResponseRejected,
            ProvenanceKind::DataMapping,
            ProvenanceKind::TaskCompleted,
            ProvenanceKind::TaskFailed,
            ProvenanceKind::TaskSkipped,
            ProvenanceKind::ParameterResolved,
            ProvenanceKind::CompensationLogEntry,
            ProvenanceKind::CompensationExecuted,
            ProvenanceKind::CompensationScopeBoundary,
            ProvenanceKind::DelegationViolation,
            ProvenanceKind::InstanceResumed,
            ProvenanceKind::StepResultPersisted,
            ProvenanceKind::IdempotencyDedup,
            ProvenanceKind::InstanceMigrated,
            ProvenanceKind::ContractValidation,
            ProvenanceKind::HistoryCleared,
            ProvenanceKind::DcrActivityExecuted,
            ProvenanceKind::DcrRelationEvaluated,
            ProvenanceKind::DcrResolutionError,
            ProvenanceKind::ZoneSatisfied,
            ProvenanceKind::EquityAlert,
            ProvenanceKind::VerificationReportProduced,
            ProvenanceKind::ImmutabilityViolation,
            ProvenanceKind::ActivationBlocked,
            ProvenanceKind::CalendarIgnored,
            ProvenanceKind::NotificationSuppressed,
            ProvenanceKind::ConfigurationWarning,
            ProvenanceKind::RelationshipChanged,
            ProvenanceKind::MilestoneFired,
            ProvenanceKind::EventEmitted,
            ProvenanceKind::EventConsumed,
            ProvenanceKind::CallbackReceived,
            ProvenanceKind::CallbackPending,
            ProvenanceKind::ArazzoStep,
            ProvenanceKind::ToolInvoked,
            ProvenanceKind::PolicyDecision,
            ProvenanceKind::SignatureAffirmation,
            ProvenanceKind::CorrectionAuthorized,
            ProvenanceKind::AmendmentAuthorized,
            ProvenanceKind::DeterminationAmended,
            ProvenanceKind::RescissionAuthorized,
            ProvenanceKind::DeterminationRescinded,
            ProvenanceKind::Reinstated,
            ProvenanceKind::AuthorizationAttestation,
            ProvenanceKind::ClockStarted,
            ProvenanceKind::ClockResolved,
            ProvenanceKind::IdentityAttestation,
            ProvenanceKind::ClockSkewObserved,
            ProvenanceKind::CommitAttemptFailure,
            ProvenanceKind::AuthorizationRejected,
            ProvenanceKind::MigrationPinChanged,
            ProvenanceKind::ObligationActivated,
            ProvenanceKind::ObligationSatisfied,
            ProvenanceKind::ObligationViolated,
            ProvenanceKind::ObligationCancelled,
            ProvenanceKind::ObligationExpired,
            ProvenanceKind::ObligationBypassed,
            ProvenanceKind::ObligationWarning,
        ];
        assert_eq!(
            all.len(),
            122,
            "ProvenanceKind has 122 variants at HEAD (115 prior + 7 durable-obligation kinds from ADR 0096); this test must enumerate all of them so a new variant forces a conscious entry"
        );

        let mut log = ProvenanceLog::default();
        for kind in all {
            let mut record = ProvenanceRecord::state_transition("a", "b", "ev", None);
            record.record_kind = *kind;
            record.timestamp = "2026-01-01T00:00:00Z".into();
            log.push(record);
        }

        let document = export(&log, &config());
        let activities: Vec<&Value> = document
            .graph
            .iter()
            .filter(|node| node["@type"] == "prov:Activity")
            .collect();
        assert_eq!(
            activities.len(),
            all.len(),
            "every Facts-tier record must produce one prov:Activity"
        );

        for (kind, activity) in all.iter().zip(activities.iter()) {
            let expected = serde_json::to_value(*kind)
                .expect("ProvenanceKind serialization is infallible (unit variants)")
                .as_str()
                .expect("ProvenanceKind serializes as a string")
                .to_string();
            assert_eq!(
                activity["wos:actionType"].as_str().unwrap(),
                expected,
                "{kind:?} export wos:actionType must match its serde camelCase rename"
            );
            assert!(
                expected
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_lowercase())
                    .unwrap_or(false),
                "{kind:?} camelCase rename must start with a lowercase letter (got {expected:?})"
            );
        }
    }
}
