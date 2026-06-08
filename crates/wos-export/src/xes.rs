// Rust guideline compliant 2026-04-16

//! IEEE 1849-2016 XES XML serializer (WOS Semantic Profile §6.3).
//!
//! Serializes a [`wos_core::ProvenanceLog`] into an XES XML
//! document as a single `<trace>` whose `<event>` elements correspond, in
//! order, to the Facts-tier records in the log.
//!
//! # Scope filter (§6.5)
//!
//! Higher-tier records (Reasoning, Counterfactual, Narrative) are excluded
//! from the trace by default. Records with `audit_layer = None` are treated
//! as Facts for backward compatibility with pre-extension runtimes.
//!
//! # Lifecycle attribute choice
//!
//! Per §6.3, WOS deliberately does NOT map its lifecycle states onto the
//! standard XES `lifecycle:transition` attribute. The standard XES Lifecycle
//! extension defines a fixed vocabulary (`start`, `complete`, `suspend`,
//! `resume`, …) that does not correspond to workflow-specific WOS state
//! names. We therefore emit a custom `wos:lifecycleState` attribute instead;
//! standards-compliant XES consumers will preserve it as an unknown
//! extension attribute rather than mis-interpreting WOS states as transition
//! codes.
//!
//! # Missing timestamps
//!
//! Records whose `timestamp` is empty have never been persistence-stamped
//! (see [`wos_core::ProvenanceRecord`]). For those events we
//! OMIT the `<date key="time:timestamp" ...>` element entirely — an empty
//! string is not a valid `xs:dateTime` value and would poison any
//! downstream tool. Consumers should treat the missing element as
//! "timestamp unknown".

use quick_xml::events::{BytesDecl, Event};
use quick_xml::writer::Writer;

use wos_core::{ProvenanceLog, ProvenanceRecord};

use crate::{ExportConfig, camel_case_record_kind, is_facts_tier};

/// Serialize a provenance log to an XES XML document (§6.3).
///
/// Writes into an in-memory `Vec<u8>` via [`quick_xml::writer::Writer`] and
/// returns the UTF-8 rendered document.
///
/// # Panics
///
/// Never panics in practice: `quick-xml` only returns errors on I/O
/// failures from the underlying writer, and writing into a `Vec<u8>`
/// cannot fail. Any surprise here is a contract violation in `quick-xml`
/// itself.
#[must_use]
pub fn export(log: &ProvenanceLog, config: &ExportConfig) -> String {
    let mut buffer: Vec<u8> = Vec::new();
    // Indent with two spaces so snapshot fixtures diff cleanly. The
    // XES spec does not mandate a specific whitespace style.
    let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);

    // Writing into a `Vec<u8>` is infallible, so every `?`-style call below
    // is expressed via `expect("write to Vec<u8> cannot fail")`.
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .expect("write to Vec<u8> cannot fail");

    writer
        .create_element("log")
        .with_attributes([
            ("xes.version", "2.0"),
            ("xes.features", "nested-attributes"),
            ("xmlns", "http://www.xes-standard.org/"),
        ])
        .write_inner_content::<_, quick_xml::Error>(|w| {
            write_extension_declarations(w)?;
            write_trace(w, log, config)?;
            Ok(())
        })
        .expect("write to Vec<u8> cannot fail");

    String::from_utf8(buffer).expect("quick-xml emits UTF-8 by construction")
}

/// Declare the XES standard extensions used by this exporter (§6.3).
///
/// Concept, Time, and Lifecycle are MUST; Organizational and Identity are
/// SHOULD, but we always emit them so round-trips stay lossless.
fn write_extension_declarations(writer: &mut Writer<&mut Vec<u8>>) -> Result<(), quick_xml::Error> {
    for (name, prefix, uri) in [
        (
            "Concept",
            "concept",
            "http://www.xes-standard.org/concept.xesext",
        ),
        ("Time", "time", "http://www.xes-standard.org/time.xesext"),
        (
            "Lifecycle",
            "lifecycle",
            "http://www.xes-standard.org/lifecycle.xesext",
        ),
        (
            "Organizational",
            "org",
            "http://www.xes-standard.org/org.xesext",
        ),
        (
            "Identity",
            "identity",
            "http://www.xes-standard.org/identity.xesext",
        ),
    ] {
        writer
            .create_element("extension")
            .with_attributes([("name", name), ("prefix", prefix), ("uri", uri)])
            .write_empty()?;
    }
    Ok(())
}

/// Emit the single `<trace>` and its events.
fn write_trace(
    writer: &mut Writer<&mut Vec<u8>>,
    log: &ProvenanceLog,
    config: &ExportConfig,
) -> Result<(), quick_xml::Error> {
    // §6.5 filter up front so both trace-level aggregation and event emission
    // see the same record set.
    let facts_records: Vec<&ProvenanceRecord> = log
        .records()
        .iter()
        .filter(|record| is_facts_tier(record))
        .collect();

    // §6.3 trace-level `wos:definitionVersion`: all records in a log share
    // one governing definition, so the first populated value is canonical.
    // In debug builds assert that every populated `definition_version`
    // matches — divergent values would silently first-wins and surface as
    // real-world drift otherwise. Release builds keep the hot path clean
    // and fall back to the first-wins behaviour; production pipelines
    // should catch divergence upstream before it reaches the exporter.
    let definition_version: Option<&str> = facts_records
        .iter()
        .find_map(|record| record.definition_version.as_deref());
    debug_assert!(
        facts_records
            .iter()
            .filter_map(|record| record.definition_version.as_deref())
            .all(|version| Some(version) == definition_version),
        "all records in a log must carry the same definition_version (one version per log)"
    );

    writer
        .create_element("trace")
        .write_inner_content::<_, quick_xml::Error>(|w| {
            write_string_attribute(w, "concept:name", &config.process_id)?;
            if let Some(version) = definition_version {
                write_string_attribute(w, "wos:definitionVersion", version)?;
            }
            for (index, record) in facts_records.iter().enumerate() {
                write_event(w, index, record)?;
            }
            Ok(())
        })?;
    Ok(())
}

/// Emit one `<event>` for a provenance record (§6.3 mapping table).
fn write_event(
    writer: &mut Writer<&mut Vec<u8>>,
    index: usize,
    record: &ProvenanceRecord,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("event")
        .write_inner_content::<_, quick_xml::Error>(|w| {
            write_string_attribute(w, "concept:name", &camel_case_record_kind(record))?;

            if !record.timestamp.is_empty() {
                write_date_attribute(w, "time:timestamp", &record.timestamp)?;
            }

            // §6.3: actorId → org:resource. Records without an actor are
            // attributed to the runtime itself ("system"), matching PROV-O
            // sibling behaviour where the actor becomes a synthetic agent.
            let resource = record.actor_id.as_deref().unwrap_or("system");
            write_string_attribute(w, "org:resource", resource)?;

            // §6.3: actorType → org:group (XES Organizational extension).
            if let Some(actor_type) = record.actor_type.as_deref() {
                write_string_attribute(w, "org:group", actor_type)?;
            }

            // ProvenanceRecord has no stable id; the record's position in the
            // log is a deterministic identifier that also doubles as a sort
            // key for consumers.
            write_string_attribute(w, "identity:id", &index.to_string())?;

            // CUSTOM attribute (see module docs) — deliberately NOT
            // lifecycle:transition. Source of truth is `lifecycle_state`
            // (populated by the runtime at stamp time, §5.3). Omit the
            // attribute entirely when the record carries no lifecycle state
            // so downstream consumers see "not applicable" rather than an
            // empty string masquerading as a known state.
            if let Some(lifecycle_state) = record.lifecycle_state.as_deref() {
                write_string_attribute(w, "wos:lifecycleState", lifecycle_state)?;
            }

            // §6.3 per-event inputs/outputs + digests. Each input/output is
            // emitted as its own `<string key="wos:input" .../>` element so
            // individual values are recoverable even when they legitimately
            // contain commas. XES allows repeated attribute keys; consumers
            // collect them by scanning the event for matching `key=`.
            for input in &record.inputs {
                write_string_attribute(w, "wos:input", input)?;
            }
            for output in &record.outputs {
                write_string_attribute(w, "wos:output", output)?;
            }
            if let Some(digest) = record.input_digest.as_deref() {
                write_string_attribute(w, "wos:inputDigest", digest)?;
            }
            if let Some(digest) = record.output_digest.as_deref() {
                write_string_attribute(w, "wos:outputDigest", digest)?;
            }

            Ok(())
        })?;
    Ok(())
}

/// Helper: `<string key="..." value="..."/>`.
fn write_string_attribute(
    writer: &mut Writer<&mut Vec<u8>>,
    key: &str,
    value: &str,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("string")
        .with_attributes([("key", key), ("value", value)])
        .write_empty()?;
    Ok(())
}

/// Helper: `<date key="..." value="..."/>`.
fn write_date_attribute(
    writer: &mut Writer<&mut Vec<u8>>,
    key: &str,
    value: &str,
) -> Result<(), quick_xml::Error> {
    writer
        .create_element("date")
        .with_attributes([("key", key), ("value", value)])
        .write_empty()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::Reader;
    use quick_xml::events::Event as ReaderEvent;
    use wos_core::{ProvenanceKind, ProvenanceLog, ProvenanceRecord};

    fn config() -> ExportConfig {
        ExportConfig {
            provenance_namespace: "urn:wos:prov:test:".to_string(),
            process_id: "benefits-001".to_string(),
        }
    }

    fn stamped_transition(actor: Option<&str>) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition("Draft", "Review", "submit", actor);
        record.timestamp = "2026-01-01T00:00:00Z".into();
        // Populate lifecycle_state so downstream assertions for
        // `wos:lifecycleState` have a value to check. Previously derived
        // from `from_state`; the runtime now populates this field directly.
        record.lifecycle_state = Some("Draft".into());
        record
    }

    /// Parse the XES string and count occurrences of the named element. Fails
    /// the test if any parse error surfaces.
    fn count_elements(xml: &str, tag: &str) -> usize {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut count = 0usize;
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(ReaderEvent::Eof) => break,
                Ok(ReaderEvent::Start(ref element)) | Ok(ReaderEvent::Empty(ref element)) => {
                    if element.name().as_ref() == tag.as_bytes() {
                        count += 1;
                    }
                }
                Ok(_) => {}
                Err(error) => panic!(
                    "XES parse failed at position {}: {error:?}",
                    reader.buffer_position()
                ),
            }
            buffer.clear();
        }
        count
    }

    #[test]
    fn exports_log_produces_valid_xml() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1")));
        log.push(stamped_transition(Some("user-2")));

        let xml = export(&log, &config());

        // Root + one trace + two events.
        assert_eq!(count_elements(&xml, "log"), 1);
        assert_eq!(count_elements(&xml, "trace"), 1);
        assert_eq!(count_elements(&xml, "event"), 2);
        // XML declaration is present.
        assert!(xml.starts_with("<?xml"), "missing XML declaration: {xml}");
    }

    #[test]
    fn each_event_has_concept_name_and_timestamp() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1")));
        log.push(stamped_transition(None));

        let xml = export(&log, &config());

        // Split on <event> ... </event> pairs and assert attributes are
        // present inside each event block. A stamped record must emit both
        // concept:name and time:timestamp.
        let events: Vec<&str> = xml.split("<event>").skip(1).collect();
        assert_eq!(events.len(), 2);
        for event_block in events {
            assert!(
                event_block.contains(r#"key="concept:name""#),
                "event missing concept:name: {event_block}"
            );
            assert!(
                event_block.contains(r#"key="time:timestamp""#),
                "event missing time:timestamp: {event_block}"
            );
        }
    }

    #[test]
    fn emits_org_group_when_actor_type_present() {
        // §6.3: actorType → org:group.
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("user-1"));
        record.actor_type = Some("human".into());
        log.push(record);

        let xml = export(&log, &config());

        assert!(
            xml.contains(r#"<string key="org:group" value="human"/>"#),
            "expected org:group=human attribute: {xml}"
        );
    }

    #[test]
    fn omits_org_group_when_actor_type_absent() {
        let mut log = ProvenanceLog::default();
        // stamped_transition leaves actor_type = None.
        log.push(stamped_transition(Some("user-1")));

        let xml = export(&log, &config());

        assert!(
            !xml.contains(r#"key="org:group""#),
            "record without actor_type must not emit org:group: {xml}"
        );
    }

    #[test]
    fn emits_inputs_outputs_and_digests_when_present() {
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("user-1"));
        // Include a value containing a comma to prove the repeated-key form
        // is lossless where the previous joined form would have been lossy.
        record.inputs = vec!["case/1".into(), "case/with,comma".into()];
        record.outputs = vec!["case/1#state".into()];
        record.input_digest = Some("sha256:aaaa".into());
        record.output_digest = Some("sha256:bbbb".into());
        log.push(record);

        let xml = export(&log, &config());

        // Each input/output is emitted as its own element; XES permits
        // repeated attribute keys.
        assert!(
            xml.contains(r#"<string key="wos:input" value="case/1"/>"#),
            "expected first wos:input attribute: {xml}"
        );
        assert!(
            xml.contains(r#"<string key="wos:input" value="case/with,comma"/>"#),
            "expected comma-bearing wos:input attribute: {xml}"
        );
        assert!(
            xml.contains(r#"<string key="wos:output" value="case/1#state"/>"#),
            "expected wos:output attribute: {xml}"
        );
        // The old joined plural forms must NOT appear.
        assert!(
            !xml.contains(r#"key="wos:inputs""#),
            "legacy joined wos:inputs must not be emitted: {xml}"
        );
        assert!(
            !xml.contains(r#"key="wos:outputs""#),
            "legacy joined wos:outputs must not be emitted: {xml}"
        );
        assert!(
            xml.contains(r#"<string key="wos:inputDigest" value="sha256:aaaa"/>"#),
            "expected wos:inputDigest: {xml}"
        );
        assert!(
            xml.contains(r#"<string key="wos:outputDigest" value="sha256:bbbb"/>"#),
            "expected wos:outputDigest: {xml}"
        );
    }

    #[test]
    fn omits_inputs_outputs_when_empty() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1"))); // inputs/outputs empty

        let xml = export(&log, &config());

        assert!(!xml.contains(r#"key="wos:input""#));
        assert!(!xml.contains(r#"key="wos:output""#));
        assert!(!xml.contains(r#"key="wos:inputs""#));
        assert!(!xml.contains(r#"key="wos:outputs""#));
        assert!(!xml.contains(r#"key="wos:inputDigest""#));
        assert!(!xml.contains(r#"key="wos:outputDigest""#));
    }

    #[test]
    fn emits_trace_level_definition_version_when_any_record_has_it() {
        // §6.3 requires `wos:definitionVersion` as a trace-level attribute.
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("user-1"));
        record.definition_version = Some("2.1.0".into());
        log.push(record);

        let xml = export(&log, &config());

        // The trace attribute must appear AFTER concept:name and BEFORE the
        // first <event>. Split on <event> to grab the trace header block.
        let trace_header = xml
            .split("<event>")
            .next()
            .expect("at least the pre-event prefix exists");
        assert!(
            trace_header.contains(r#"<string key="wos:definitionVersion" value="2.1.0"/>"#),
            "trace-level definitionVersion missing or misplaced: {xml}"
        );
    }

    #[test]
    fn omits_trace_level_definition_version_when_all_records_lack_it() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1"))); // definition_version = None

        let xml = export(&log, &config());

        assert!(
            !xml.contains(r#"key="wos:definitionVersion""#),
            "must not emit trace-level definitionVersion when no record supplies one: {xml}"
        );
    }

    #[test]
    fn omits_timestamp_when_record_unstamped() {
        let mut log = ProvenanceLog::default();
        // Unstamped — constructor leaves timestamp empty.
        log.push(ProvenanceRecord::state_transition(
            "Draft",
            "Review",
            "submit",
            Some("user-1"),
        ));

        let xml = export(&log, &config());

        // No <date ...> element anywhere — there is only one event and it is
        // unstamped.
        assert!(
            !xml.contains("<date"),
            "unstamped record must not emit <date>: {xml}"
        );
        // But concept:name still present, so we didn't accidentally skip the event.
        assert!(xml.contains(r#"key="concept:name" value="stateTransition""#));
    }

    #[test]
    fn trace_concept_name_matches_process_id() {
        let log = ProvenanceLog::default();
        let xml = export(&log, &config());

        // The trace header string must carry the configured instance id.
        assert!(
            xml.contains(r#"<string key="concept:name" value="benefits-001"/>"#),
            "trace concept:name not set to instance id: {xml}"
        );
    }

    #[test]
    fn uses_custom_wos_lifecycle_state_not_standard_lifecycle_transition() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(Some("user-1")));

        let xml = export(&log, &config());

        assert!(
            xml.contains(r#"key="wos:lifecycleState""#),
            "must emit wos:lifecycleState (custom WOS attribute): {xml}"
        );
        assert!(
            !xml.contains("lifecycle:transition"),
            "must NOT emit standard lifecycle:transition — WOS states are not XES lifecycle vocab: {xml}"
        );
    }

    #[test]
    fn omits_lifecycle_state_when_field_is_none() {
        // Records with no lifecycle_state (e.g. lifecycle-external events)
        // omit the attribute entirely — absence is truth.
        let mut log = ProvenanceLog::default();
        let mut record = stamped_transition(Some("user-1"));
        record.lifecycle_state = None;
        log.push(record);

        let xml = export(&log, &config());

        assert!(
            !xml.contains(r#"key="wos:lifecycleState""#),
            "lifecycle_state=None must not emit wos:lifecycleState: {xml}"
        );
    }

    #[test]
    fn identity_id_is_record_index() {
        let mut log = ProvenanceLog::default();
        for _ in 0..3 {
            log.push(stamped_transition(Some("user-1")));
        }

        let xml = export(&log, &config());

        // Each event carries its zero-based index as identity:id.
        for expected_index in 0..3 {
            let expected = format!(r#"<string key="identity:id" value="{expected_index}"/>"#);
            assert!(
                xml.contains(&expected),
                "missing identity:id={expected_index}: {xml}"
            );
        }
    }

    #[test]
    fn absent_actor_id_falls_back_to_system_resource() {
        let mut log = ProvenanceLog::default();
        log.push(stamped_transition(None));

        let xml = export(&log, &config());

        assert!(
            xml.contains(r#"<string key="org:resource" value="system"/>"#),
            "missing system fallback for org:resource: {xml}"
        );
    }

    #[test]
    fn camel_cases_all_record_kinds() {
        // Exhaustive parity check (cross-stack-scout 2026-04-28): every
        // `ProvenanceKind` variant exports its serde-camelCase string into
        // the XES `concept:name` attribute. Mirrors the prov_o.rs sibling
        // test; adding a new variant upstream forces an entry here.
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
        assert_eq!(all.len(), 122);

        let mut log = ProvenanceLog::default();
        for kind in all {
            let mut record = stamped_transition(None);
            record.record_kind = *kind;
            log.push(record);
        }

        let xml = export(&log, &config());
        for kind in all {
            let expected = serde_json::to_value(*kind)
                .expect("infallible serialize")
                .as_str()
                .expect("string serialize")
                .to_string();
            let needle = format!(r#"value="{expected}""#);
            assert!(
                xml.contains(&needle),
                "{kind:?} expected XES concept:name={expected:?} not found in export"
            );
        }
    }

    #[test]
    fn declares_five_extensions() {
        let log = ProvenanceLog::default();
        let xml = export(&log, &config());
        // Must include all five — Concept, Time, Lifecycle, Organizational, Identity.
        assert_eq!(count_elements(&xml, "extension"), 5);
        for prefix in ["concept", "time", "lifecycle", "org", "identity"] {
            let needle = format!(r#"prefix="{prefix}""#);
            assert!(
                xml.contains(&needle),
                "extension with prefix={prefix} missing: {xml}"
            );
        }
    }

    #[test]
    fn filters_non_facts_tier_records_per_section_6_5() {
        // §6.5: narrative-tier records excluded from default XES export.
        let mut log = ProvenanceLog::default();

        let mut facts = stamped_transition(Some("user-1"));
        facts.audit_layer = Some("facts".into());
        log.push(facts);

        let mut narrative = stamped_transition(Some("user-2"));
        narrative.audit_layer = Some("narrative".into());
        log.push(narrative);

        let xml = export(&log, &config());

        // Exactly one event survives (the facts-tier record). user-2 must
        // not appear anywhere in the XML.
        assert_eq!(count_elements(&xml, "event"), 1);
        assert!(
            !xml.contains("user-2"),
            "narrative-tier actor must be excluded from export: {xml}"
        );
    }
}
