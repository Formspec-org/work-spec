// Rust guideline compliant 2026-04-16

//! OCEL 2.0 JSON serializer (WOS Semantic Profile §6.4).
//!
//! Serializes a [`ProvenanceLog`] into an Object-Centric Event Log shaped per
//! the OCEL 2.0 JSON specification.
//!
//! # Scope filter (§6.5)
//!
//! Higher-tier records (Reasoning, Counterfactual, Narrative) are excluded
//! from the default export. The `wf-instance` object is emitted regardless
//! of filter outcome — it anchors the log even when no events survive.
//!
//! # Object modelling gap
//!
//! This implementation models only the workflow instance itself as an OCEL
//! Object (type `wf-instance`). Every event relates back to that single
//! object with qualifier `relates-to`. The spec (§6.4) mandates that each
//! **Case File Item** is also an OCEL Object, with E2O relationships
//! connecting mutating events to the items they touch and O2O relationships
//! representing cross-references between items. That richer shape is an
//! upstream-architectural gap: `ProvenanceRecord` does not today carry the
//! per-record read/write sets that E2O materialization requires. The gap is
//! tracked in a follow-up ticket and is NOT addressable from inside this
//! crate.
//!
//! # Empty timestamp handling
//!
//! Unstamped records (see [`ProvenanceRecord::timestamp`] docs) emit `time: ""`
//! verbatim. OCEL 2.0 requires `time` on every event, so omitting the field
//! would produce an invalid document; emitting an empty string surfaces the
//! missed stamping site to downstream consumers. This is consistent with the
//! push-stamped design's "surface, don't paper over" philosophy.

use serde_json::{Map, Value, json};

use wos_core::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

use crate::{ExportConfig, camel_case_record_kind, is_facts_tier};

/// Whether a record is a durable-obligation lifecycle record (ADR 0096).
///
/// Obligation records carry `obligationId` (and `policyId`) in their `data`
/// payload; this exporter promotes each distinct `obligationId` to a
/// first-class OCEL Object (type `obligation`) so process-mining tooling can
/// follow an obligation across its activate → satisfy/violate/expire/bypass
/// arc independent of the workflow instance object.
fn is_obligation_kind(kind: ProvenanceKind) -> bool {
    matches!(
        kind,
        ProvenanceKind::ObligationActivated
            | ProvenanceKind::ObligationSatisfied
            | ProvenanceKind::ObligationViolated
            | ProvenanceKind::ObligationCancelled
            | ProvenanceKind::ObligationExpired
            | ProvenanceKind::ObligationBypassed
            | ProvenanceKind::ObligationWarning
    )
}

/// Extract the `obligationId` carried in an obligation record's `data` payload.
/// Returns `None` for non-obligation records or when the payload is malformed.
fn obligation_id(record: &ProvenanceRecord) -> Option<&str> {
    if !is_obligation_kind(record.record_kind) {
        return None;
    }
    record
        .data
        .as_ref()
        .and_then(|data| data.get("obligationId"))
        .and_then(Value::as_str)
}

/// Serialize a provenance log as an OCEL 2.0 JSON document (§6.4).
///
/// The returned value is shaped exactly per the OCEL 2.0 spec: four top-level
/// arrays (`objectTypes`, `eventTypes`, `objects`, `events`). Every event
/// relates to a single `wf-instance` object whose id is `config.process_id`;
/// per-case-file-item E2O relationships are deferred — see module docs for
/// the object modelling gap.
#[must_use]
pub fn export(log: &ProvenanceLog, config: &ExportConfig) -> Value {
    // §6.5 scope filter: Facts-tier records only. `None` is Facts for
    // backward compatibility with pre-extension runtimes.
    let facts_records: Vec<&ProvenanceRecord> = log
        .records()
        .iter()
        .filter(|record| is_facts_tier(record))
        .collect();

    let events: Vec<Value> = facts_records
        .iter()
        .enumerate()
        .map(|(index, record)| event_node(index, record, &config.process_id))
        .collect();

    json!({
        "objectTypes": object_types(),
        "eventTypes": event_types(&facts_records),
        "objects": objects(config, &facts_records),
        "events": events,
    })
}

/// `objectTypes` array. OCEL 2.0 requires typed objects. This exporter models
/// the workflow instance (`wf-instance`) and — per ADR 0096 / WOS-OBL-EXPORT-1203
/// — each durable obligation (`obligation`) as a first-class object so an
/// obligation's lifecycle is followable across events. Case-File-Item objects
/// remain the documented upstream gap (see module docs).
fn object_types() -> Value {
    json!([
        {
            "name": "wf-instance",
            "attributes": [
                { "name": "processId", "type": "string" }
            ],
        },
        {
            "name": "obligation",
            "attributes": [
                { "name": "policyId", "type": "string" }
            ],
        }
    ])
}

/// Deduplicate `record_kind` occurrences into an OCEL `eventTypes` array.
/// Order is stable: first-seen wins, matching the activity emission order the
/// PROV-O exporter uses so snapshot diffs stay clean for Task 5 fixtures.
///
/// Each event type declares the full set of STATIC optional attributes a
/// `ProvenanceRecord` may carry (§6.3). Because `ProvenanceKind` is a uniform
/// struct — every variant shares the same optional field set — we emit the
/// same schema for every event type. The indexed `inputs.{i}` / `outputs.{i}`
/// attributes are NOT declared here because their count varies per record
/// (OCEL 2.0 tolerates undeclared attributes; strict schema validation is
/// optional).
fn event_types(records: &[&ProvenanceRecord]) -> Value {
    let mut seen: Vec<String> = Vec::new();
    for record in records {
        let name = camel_case_record_kind(record);
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen.into_iter()
        .map(|name| json!({ "name": name, "attributes": static_event_attribute_schema() }))
        .collect()
}

/// Static attribute schema shared by every event type. Mirrors the fields
/// emitted by [`event_attributes`] for non-indexed, non-dynamic values.
fn static_event_attribute_schema() -> Value {
    json!([
        { "name": "actorId", "type": "string" },
        { "name": "fromState", "type": "string" },
        { "name": "toState", "type": "string" },
        { "name": "event", "type": "string" },
        { "name": "data", "type": "string" },
        { "name": "actorType", "type": "string" },
        { "name": "lifecycleState", "type": "string" },
        { "name": "definitionVersion", "type": "string" },
        { "name": "inputDigest", "type": "string" },
        { "name": "outputDigest", "type": "string" }
    ])
}

/// Emit the `wf-instance` object plus one `obligation` object per distinct
/// `obligationId` (ADR 0096 / WOS-OBL-EXPORT-1203). The `wf-instance`
/// `processId` attribute carries the earliest non-empty record timestamp
/// (falling back to `""` when the log contains no stamped records) so
/// downstream OCEL tools can anchor the object in the event timeline.
fn objects(config: &ExportConfig, records: &[&ProvenanceRecord]) -> Value {
    // Lexicographic `.min()` over timestamps is only correct when every
    // timestamp uses the same UTC form (`...Z`). The WOS runtime emits
    // strict RFC 3339 UTC via `wos_runtime::stamp_provenance` (see
    // `crates/wos-runtime/src/lib.rs`), so this assumption holds for
    // runtime-produced logs. If a future source supplies offset-bearing
    // timestamps (`...+01:00`), normalize to UTC before comparing — a
    // lexicographic min over mixed offsets can silently pick the wrong
    // record.
    let earliest = records
        .iter()
        .map(|record| record.timestamp.as_str())
        .filter(|timestamp| !timestamp.is_empty())
        .min()
        .unwrap_or("");

    let mut objects: Vec<Value> = vec![json!({
        "id": config.process_id,
        "type": "wf-instance",
        "attributes": [{
            "name": "processId",
            "time": earliest,
            "value": config.process_id,
        }],
    })];

    // WOS-OBL-EXPORT-1203: one `obligation` object per distinct obligationId.
    // First-seen order keeps snapshot diffs stable (mirrors the eventTypes
    // dedup ordering). The object's `policyId` attribute is timestamped with
    // the earliest event that referenced the obligation so OCEL tooling can
    // anchor it in the timeline; `policyId` itself is taken from the first
    // record that carries one (an obligation has exactly one policy).
    let mut seen: Vec<String> = Vec::new();
    for record in records {
        let Some(id) = obligation_id(record) else {
            continue;
        };
        if seen.iter().any(|existing| existing == id) {
            continue;
        }
        seen.push(id.to_string());

        let policy_id = records
            .iter()
            .filter(|other| obligation_id(other) == Some(id))
            .find_map(|other| {
                other
                    .data
                    .as_ref()
                    .and_then(|data| data.get("policyId"))
                    .and_then(Value::as_str)
            })
            .unwrap_or("");
        let object_earliest = records
            .iter()
            .filter(|other| obligation_id(other) == Some(id))
            .map(|other| other.timestamp.as_str())
            .filter(|timestamp| !timestamp.is_empty())
            .min()
            .unwrap_or("");

        objects.push(json!({
            "id": id,
            "type": "obligation",
            "attributes": [{
                "name": "policyId",
                "time": object_earliest,
                "value": policy_id,
            }],
        }));
    }

    Value::Array(objects)
}

/// Emit a single OCEL event for a record at position `index`.
fn event_node(index: usize, record: &ProvenanceRecord, process_id: &str) -> Value {
    let mut node = Map::new();
    node.insert("id".into(), Value::String(format!("e-{index}")));
    node.insert("type".into(), Value::String(camel_case_record_kind(record)));
    // `time` is emitted verbatim even when empty — see module docs for the
    // rationale. OCEL requires the field to be present on every event.
    node.insert("time".into(), Value::String(record.timestamp.clone()));
    node.insert("attributes".into(), Value::Array(event_attributes(record)));

    // Every event relates to the workflow instance. Obligation lifecycle
    // events additionally link to their `obligation` object via the `concerns`
    // qualifier (WOS-OBL-EXPORT-1203) so the obligation's arc is recoverable.
    let mut relationships = vec![json!({ "objectId": process_id, "qualifier": "relates-to" })];
    if let Some(id) = obligation_id(record) {
        relationships.push(json!({ "objectId": id, "qualifier": "concerns" }));
    }
    node.insert("relationships".into(), Value::Array(relationships));

    Value::Object(node)
}

/// Collect the optional `ProvenanceRecord` fields into OCEL event attributes.
/// Only non-`None` / non-empty fields are included; `record_kind` and
/// `timestamp` are carried on the event envelope and are intentionally
/// excluded here.
fn event_attributes(record: &ProvenanceRecord) -> Vec<Value> {
    let mut attributes = Vec::new();
    if let Some(actor_id) = record.actor_id.as_deref() {
        attributes.push(json!({ "name": "actorId", "value": actor_id }));
    }
    if let Some(actor_type) = record.actor_type.as_deref() {
        attributes.push(json!({ "name": "actorType", "value": actor_type }));
    }
    if let Some(from_state) = record.from_state.as_deref() {
        attributes.push(json!({ "name": "fromState", "value": from_state }));
    }
    if let Some(to_state) = record.to_state.as_deref() {
        attributes.push(json!({ "name": "toState", "value": to_state }));
    }
    if let Some(lifecycle_state) = record.lifecycle_state.as_deref() {
        attributes.push(json!({ "name": "lifecycleState", "value": lifecycle_state }));
    }
    if let Some(version) = record.definition_version.as_deref() {
        attributes.push(json!({ "name": "definitionVersion", "value": version }));
    }
    if let Some(event) = record.event.as_deref() {
        attributes.push(json!({ "name": "event", "value": event }));
    }
    if let Some(data) = record.data.as_ref() {
        // OCEL 2.0 requires scalar attribute values matching the declared
        // `type` in `eventTypes[*].attributes` — `data` is declared `string`
        // in `static_event_attribute_schema()`. Structured `data` (objects,
        // arrays, numbers, bools) is JSON-string-encoded at emission so the
        // declared `string` type holds and consumers can round-trip with
        // `JSON.parse`. Same rationale as the `inputs.{i}` / `outputs.{i}`
        // flattening below.
        let data_text = serde_json::to_string(data).unwrap_or_default();
        attributes.push(json!({ "name": "data", "value": data_text }));
    }
    // §6.3 inputs/outputs. OCEL 2.0 requires attribute `value` to be a scalar
    // matching the declared type in `eventTypes[*].attributes`, so we CANNOT
    // emit a JSON array here. Instead, flatten each vec into indexed scalar
    // attributes using the convention `name: "inputs.{i}"` / `"outputs.{i}"`.
    // This preserves order and per-item identity while remaining spec-valid.
    // Consumers reconstruct the vec by filtering attribute names with the
    // prefix `inputs.` or `outputs.`. The indexed attributes are dynamic
    // (count varies per record) and are therefore NOT declared in the
    // `eventTypes` schema — only the static optional fields are.
    for (item_index, input) in record.inputs.iter().enumerate() {
        attributes.push(json!({ "name": format!("inputs.{item_index}"), "value": input }));
    }
    for (item_index, output) in record.outputs.iter().enumerate() {
        attributes.push(json!({ "name": format!("outputs.{item_index}"), "value": output }));
    }
    if let Some(digest) = record.input_digest.as_deref() {
        attributes.push(json!({ "name": "inputDigest", "value": digest }));
    }
    if let Some(digest) = record.output_digest.as_deref() {
        attributes.push(json!({ "name": "outputDigest", "value": digest }));
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

    fn config() -> ExportConfig {
        ExportConfig {
            provenance_namespace: "urn:wos:prov:test:".to_string(),
            process_id: "instance-abc".to_string(),
        }
    }

    fn stamped(kind: ProvenanceKind, timestamp: &str) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("a", "b", "ev", Some("user-1"));
        record.record_kind = kind;
        record.timestamp = timestamp.to_string();
        record
    }

    #[test]
    fn exports_log_has_required_top_level_keys() {
        let log = ProvenanceLog::default();
        let document = export(&log, &config());

        let object = document
            .as_object()
            .expect("top-level must be a JSON object");
        let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, ["eventTypes", "events", "objectTypes", "objects"]);
    }

    #[test]
    fn event_count_equals_record_count() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));
        log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));
        log.push(stamped(ProvenanceKind::OnExit, "2026-01-01T00:00:02Z"));

        let document = export(&log, &config());

        assert_eq!(
            document["events"].as_array().expect("events array").len(),
            3
        );
    }

    #[test]
    fn every_event_has_id_type_time_relationships() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));
        log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));

        let document = export(&log, &config());

        let events = document["events"].as_array().expect("events array");
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event["id"], format!("e-{index}"));
            assert!(
                event["type"].is_string(),
                "type must be string on event {index}"
            );
            assert!(
                event["time"].is_string(),
                "time must be string on event {index}"
            );
            assert!(
                event["relationships"].is_array(),
                "relationships must be array on event {index}"
            );
        }
    }

    #[test]
    fn every_event_relates_to_instance_object() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));
        log.push(stamped(ProvenanceKind::TimerFired, "2026-01-01T00:00:10Z"));

        let document = export(&log, &config());

        let expected = json!({ "objectId": "instance-abc", "qualifier": "relates-to" });
        for event in document["events"].as_array().expect("events array") {
            let relationships = event["relationships"].as_array().expect("relationships");
            assert!(
                relationships.iter().any(|r| r == &expected),
                "event missing instance relationship: {event}"
            );
        }
    }

    #[test]
    fn event_types_deduplicated_by_kind() {
        let mut log = ProvenanceLog::default();
        for _ in 0..3 {
            log.push(stamped(
                ProvenanceKind::StateTransition,
                "2026-01-01T00:00:00Z",
            ));
        }
        for _ in 0..2 {
            log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));
        }

        let document = export(&log, &config());

        let event_types = document["eventTypes"].as_array().expect("eventTypes array");
        assert_eq!(event_types.len(), 2);

        let mut names: Vec<_> = event_types
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_string())
            .collect();
        names.sort();
        assert_eq!(names, ["onEntry", "stateTransition"]);
    }

    #[test]
    fn preserves_empty_timestamp_as_empty_string() {
        let mut log = ProvenanceLog::default();
        // Unstamped record — constructors leave `timestamp` empty; the runtime
        // would normally push-stamp it, but tests may bypass that path.
        log.push(ProvenanceRecord::state_transition("a", "b", "ev", None));

        let document = export(&log, &config());

        let events = document["events"].as_array().expect("events array");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]["time"], "",
            "empty timestamp must round-trip as empty string"
        );

        // When every record is unstamped, the instance object's `processId`
        // attribute falls back to an empty `time` (documented at module level).
        let objects = document["objects"].as_array().expect("objects array");
        let instance_attrs = objects[0]["attributes"]
            .as_array()
            .expect("attributes array");
        assert_eq!(
            instance_attrs[0]["time"], "",
            "all-unstamped log must produce empty processId attribute time"
        );
    }

    #[test]
    fn single_instance_object_emitted() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));
        log.push(stamped(ProvenanceKind::OnEntry, "2026-01-01T00:00:01Z"));

        let document = export(&log, &config());

        // No obligation records present → exactly the single wf-instance object.
        let objects = document["objects"].as_array().expect("objects array");
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0]["id"], "instance-abc");
        assert_eq!(objects[0]["type"], "wf-instance");

        // `objectTypes` now always declares both the instance and obligation
        // types (WOS-OBL-EXPORT-1203); obligation objects only materialize when
        // obligation records are present.
        let object_types = document["objectTypes"]
            .as_array()
            .expect("objectTypes array");
        let type_names: Vec<&str> = object_types
            .iter()
            .map(|t| t["name"].as_str().expect("object type name"))
            .collect();
        assert_eq!(type_names, ["wf-instance", "obligation"]);
    }

    /// Build an obligation lifecycle record carrying the `obligationId` /
    /// `policyId` payload the OCEL exporter promotes to a first-class object.
    fn obligation_record(
        kind: ProvenanceKind,
        policy_id: &str,
        obligation_id: &str,
        timestamp: &str,
    ) -> ProvenanceRecord {
        let mut record = stamped(kind, timestamp);
        record.data = Some(json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
        }));
        record
    }

    #[test]
    fn promotes_each_obligation_to_a_first_class_ocel_object() {
        // WOS-OBL-EXPORT-1203: distinct obligationIds each become one
        // `obligation` object; repeated ids dedup to a single object.
        let mut log = ProvenanceLog::default();
        log.push(obligation_record(
            ProvenanceKind::ObligationActivated,
            "policy-1",
            "obl-1",
            "2026-01-01T00:00:00Z",
        ));
        log.push(obligation_record(
            ProvenanceKind::ObligationViolated,
            "policy-1",
            "obl-1",
            "2026-01-01T00:00:05Z",
        ));
        log.push(obligation_record(
            ProvenanceKind::ObligationActivated,
            "policy-2",
            "obl-2",
            "2026-01-01T00:00:10Z",
        ));

        let document = export(&log, &config());

        let objects = document["objects"].as_array().expect("objects array");
        // wf-instance + two distinct obligations.
        assert_eq!(objects.len(), 3);

        let obligation_objects: Vec<&Value> = objects
            .iter()
            .filter(|object| object["type"] == "obligation")
            .collect();
        assert_eq!(obligation_objects.len(), 2);
        // First-seen order: obl-1 before obl-2.
        assert_eq!(obligation_objects[0]["id"], "obl-1");
        assert_eq!(obligation_objects[1]["id"], "obl-2");
        // policyId attribute carried, timestamped with the earliest referencing event.
        let obl1_attr = &obligation_objects[0]["attributes"][0];
        assert_eq!(obl1_attr["name"], "policyId");
        assert_eq!(obl1_attr["value"], "policy-1");
        assert_eq!(obl1_attr["time"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn obligation_events_link_obligation_and_instance_objects() {
        // WOS-OBL-EXPORT-1203: an obligation event relates to BOTH the
        // wf-instance (`relates-to`) and its obligation object (`concerns`).
        let mut log = ProvenanceLog::default();
        log.push(obligation_record(
            ProvenanceKind::ObligationViolated,
            "policy-1",
            "obl-1",
            "2026-01-01T00:00:00Z",
        ));

        let document = export(&log, &config());

        let relationships = document["events"][0]["relationships"]
            .as_array()
            .expect("relationships array");
        assert!(
            relationships
                .iter()
                .any(|r| r == &json!({ "objectId": "instance-abc", "qualifier": "relates-to" })),
            "obligation event must still relate to wf-instance: {relationships:?}"
        );
        assert!(
            relationships
                .iter()
                .any(|r| r == &json!({ "objectId": "obl-1", "qualifier": "concerns" })),
            "obligation event must link its obligation object: {relationships:?}"
        );
    }

    #[test]
    fn non_obligation_events_carry_only_instance_relationship() {
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));

        let document = export(&log, &config());

        let relationships = document["events"][0]["relationships"]
            .as_array()
            .expect("relationships array");
        assert_eq!(
            relationships.len(),
            1,
            "non-obligation event must not gain a `concerns` link: {relationships:?}"
        );
        assert_eq!(relationships[0]["qualifier"], "relates-to");
    }

    #[test]
    fn emits_sp_section_6_3_attributes_when_populated() {
        // §6.3 symmetry with XES: actorType, lifecycleState, definitionVersion,
        // inputs/outputs as JSON arrays, and the two digests.
        let mut log = ProvenanceLog::default();
        let mut record = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z");
        record.actor_type = Some("human".into());
        record.lifecycle_state = Some("draft".into());
        record.definition_version = Some("3.2.1".into());
        record.inputs = vec!["case/1".into(), "case/2".into()];
        record.outputs = vec!["case/1#state".into()];
        record.input_digest = Some("sha256:aaaa".into());
        record.output_digest = Some("sha256:bbbb".into());
        log.push(record);

        let document = export(&log, &config());

        let attributes = document["events"][0]["attributes"]
            .as_array()
            .expect("attributes array");

        let find = |name: &str| -> Option<&Value> {
            attributes
                .iter()
                .find(|attr| attr["name"] == name)
                .map(|attr| &attr["value"])
        };

        assert_eq!(find("actorType"), Some(&Value::String("human".into())));
        assert_eq!(find("lifecycleState"), Some(&Value::String("draft".into())));
        assert_eq!(
            find("definitionVersion"),
            Some(&Value::String("3.2.1".into()))
        );
        // Inputs/outputs flatten to indexed scalar attributes so each `value`
        // is a primitive string (OCEL 2.0 spec requirement).
        assert_eq!(
            find("inputs.0"),
            Some(&Value::String("case/1".into())),
            "inputs must flatten to indexed scalar attributes",
        );
        assert_eq!(find("inputs.1"), Some(&Value::String("case/2".into())));
        assert_eq!(
            find("outputs.0"),
            Some(&Value::String("case/1#state".into()))
        );
        // The non-indexed `inputs` / `outputs` names must NOT appear.
        assert!(
            find("inputs").is_none(),
            "non-indexed `inputs` must not be emitted"
        );
        assert!(
            find("outputs").is_none(),
            "non-indexed `outputs` must not be emitted"
        );
        assert_eq!(
            find("inputDigest"),
            Some(&Value::String("sha256:aaaa".into()))
        );
        assert_eq!(
            find("outputDigest"),
            Some(&Value::String("sha256:bbbb".into()))
        );
    }

    #[test]
    fn inputs_outputs_attribute_values_are_primitive_strings_not_arrays() {
        // OCEL 2.0: `eventTypes[*].attributes[*].type` is a scalar primitive.
        // Therefore each `events[*].attributes[*].value` MUST be a primitive
        // matching that declared type. A JSON array or object would produce an
        // invalid OCEL document. Guard that contract explicitly — a weaker
        // `!is_array()` passes vacuously for `Value::Object(...)`, so we check
        // that every value is one of the primitive JSON scalars.
        let mut log = ProvenanceLog::default();
        let mut record = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z");
        record.inputs = vec!["a".into(), "b".into(), "c".into()];
        record.outputs = vec!["x".into()];
        log.push(record);

        let document = export(&log, &config());
        let attributes = document["events"][0]["attributes"]
            .as_array()
            .expect("attributes array");

        for attribute in attributes {
            let value = &attribute["value"];
            assert!(
                value.is_string() || value.is_number() || value.is_boolean() || value.is_null(),
                "OCEL 2.0 requires scalar attribute values; got {value:?} for attribute {attribute:?}"
            );
        }

        // And we should see the three indexed input entries + one indexed
        // output entry, matching the vec lengths.
        let indexed_inputs: Vec<_> = attributes
            .iter()
            .filter(|attr| attr["name"].as_str().unwrap_or("").starts_with("inputs."))
            .collect();
        assert_eq!(
            indexed_inputs.len(),
            3,
            "one indexed attribute per input: {attributes:?}"
        );
        let indexed_outputs: Vec<_> = attributes
            .iter()
            .filter(|attr| attr["name"].as_str().unwrap_or("").starts_with("outputs."))
            .collect();
        assert_eq!(indexed_outputs.len(), 1);
    }

    #[test]
    fn data_attribute_value_is_scalar_string_even_for_objects() {
        // OCEL 2.0 requires each event attribute `value` to be a scalar
        // matching the declared `type` in `eventTypes[*].attributes`. The
        // static schema declares `{"name": "data", "type": "string"}`, but
        // `ProvenanceRecord::data` is `Option<serde_json::Value>` and may
        // carry objects, arrays, numbers, or bools at runtime. The exporter
        // must JSON-string-encode structured `data` so the declared type
        // contract holds.
        let mut log = ProvenanceLog::default();
        let mut record = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z");
        record.data = Some(serde_json::json!({ "k": 1, "nested": { "x": true } }));
        log.push(record);

        let document = export(&log, &config());
        let attributes = document["events"][0]["attributes"]
            .as_array()
            .expect("attributes array");

        let data_attr = attributes
            .iter()
            .find(|attr| attr["name"] == "data")
            .expect("data attribute must be emitted when record.data is Some");

        assert!(
            data_attr["value"].is_string(),
            "OCEL 2.0: `data` is declared `type: string`, so `value` MUST be a JSON string (got {:?})",
            data_attr["value"],
        );
        let encoded = data_attr["value"].as_str().expect("data value is a string");
        let round_trip: serde_json::Value =
            serde_json::from_str(encoded).expect("stringified data must be valid JSON");
        assert_eq!(
            round_trip,
            serde_json::json!({ "k": 1, "nested": { "x": true } }),
            "round-trip must preserve the original structured data"
        );
    }

    #[test]
    fn event_types_declare_static_attribute_schema() {
        // OCEL 2.0 expects `eventTypes[*].attributes` to describe the shape
        // of event attributes so consumers can validate the log. The static
        // optional fields on `ProvenanceRecord` MUST be declared; the
        // dynamic indexed `inputs.{i}` / `outputs.{i}` attributes are not
        // (their cardinality varies per record).
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));
        let document = export(&log, &config());

        let event_types = document["eventTypes"].as_array().expect("eventTypes array");
        assert_eq!(event_types.len(), 1);
        let attributes = event_types[0]["attributes"]
            .as_array()
            .expect("eventType attributes array");

        let names: Vec<&str> = attributes
            .iter()
            .map(|attr| attr["name"].as_str().expect("attribute name"))
            .collect();
        for required in [
            "actorId",
            "fromState",
            "toState",
            "event",
            "data",
            "actorType",
            "lifecycleState",
            "definitionVersion",
            "inputDigest",
            "outputDigest",
        ] {
            assert!(
                names.contains(&required),
                "eventType schema missing static attribute {required}: {names:?}"
            );
        }

        // Every declared attribute must carry a primitive `type` string.
        for attribute in attributes {
            assert!(
                attribute["type"].is_string(),
                "eventType attribute needs a primitive `type`: {attribute}"
            );
        }
    }

    #[test]
    fn omits_optional_attributes_when_absent() {
        // Baseline record has actor_id + from/to/event only; the new fields
        // must be absent from the attribute list.
        let mut log = ProvenanceLog::default();
        log.push(stamped(
            ProvenanceKind::StateTransition,
            "2026-01-01T00:00:00Z",
        ));

        let document = export(&log, &config());
        let attributes = document["events"][0]["attributes"]
            .as_array()
            .expect("attributes array");
        let names: Vec<&str> = attributes
            .iter()
            .map(|attr| attr["name"].as_str().expect("name"))
            .collect();

        for omitted in [
            "actorType",
            "lifecycleState",
            "definitionVersion",
            "inputs",
            "outputs",
            "inputDigest",
            "outputDigest",
        ] {
            assert!(
                !names.contains(&omitted),
                "attribute {omitted} must not appear when field is unset: {names:?}"
            );
        }
        // Indexed input/output attributes must also be absent when vecs are empty.
        for name in &names {
            assert!(
                !name.starts_with("inputs.") && !name.starts_with("outputs."),
                "indexed input/output attribute leaked for empty vec: {name}"
            );
        }
    }

    #[test]
    fn filters_non_facts_tier_records_per_section_6_5() {
        // §6.5: Narrative-tier records excluded from default OCEL export.
        let mut log = ProvenanceLog::default();

        let mut facts = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:00Z");
        facts.audit_layer = Some("facts".into());
        log.push(facts);

        let mut narrative = stamped(ProvenanceKind::StateTransition, "2026-01-01T00:00:01Z");
        narrative.audit_layer = Some("narrative".into());
        narrative.actor_id = Some("narrator".into());
        log.push(narrative);

        let document = export(&log, &config());

        let events = document["events"].as_array().expect("events array");
        assert_eq!(events.len(), 1, "narrative record must be excluded");

        // The narrator should not appear as an event actor attribute.
        let narrator_present = events.iter().any(|event| {
            event["attributes"]
                .as_array()
                .map(|attrs| {
                    attrs
                        .iter()
                        .any(|attr| attr["name"] == "actorId" && attr["value"] == "narrator")
                })
                .unwrap_or(false)
        });
        assert!(!narrator_present, "narrator must be filtered: {events:?}");
    }
}
