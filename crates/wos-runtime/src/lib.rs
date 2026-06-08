// Rust guideline compliant 2026-02-21

//! Generic runtime orchestration for WOS processors.
//!
//! This crate owns runtime commands, atomic persistence boundaries, task
//! lifecycle coordination, and binding dispatch. Binding-specific behavior
//! lives in separate adapter crates.

pub mod binding;
pub mod cloudevents;
pub mod companion;
mod durable;
pub mod intake;
pub mod integration;
pub mod integration_handlers;
pub mod milestones;
pub mod obligations;
pub mod policy_decision;
pub mod restate_fixture_support;
pub mod runtime;
pub mod store;
pub mod studio_api;

pub use binding::{
    BindingError, BindingRegistry, CaseMutationBundle, ContractBindingAdapter, PreparedTask,
    SignatureAdmissionFailure, SignatureAdmissionFailureReason, SignatureEvidence,
    SignaturePrimitiveStatus, SubmissionValidation,
};
pub use companion::ReferenceCompanionPolicy;
#[doc(inline)]
pub use durable::DurableRuntime;
pub use intake::{
    AutoCreatePublicIntakePolicy, IntakeAcceptanceAdapter, IntakeAcceptanceDecision,
    IntakeAcceptanceOutcome, IntakeAcceptancePolicy, IntakeAcceptanceRegistry,
    IntakeAcceptanceRequest, IntakeCaseDefinition, IntakeCaseDisposition, IntakeCaseIntent,
    IntakeInterpretation, IntakePolicyContext, IntakeRecordStatus, ManualReviewIntakePolicy,
    NoopIntakeAcceptancePolicy, PublicIntakeDisabledPolicy,
};
pub use integration::{
    IntegrationBinding, IntegrationBindingKind, IntegrationContractRef, IntegrationProfileDocument,
    TargetWorkflow,
};
pub use restate_fixture_support::{
    FixtureResolverError, MinimalFixtureFormspecAdapter, SharedInMemoryStore,
    SignatureFixtureResolver, restate_signature_fixture_bindings,
    restate_signature_fixture_runtime, signature_runtime_fixture_kernel,
    signature_runtime_fixture_profile,
};
pub use runtime::{
    Clock, CompanionPolicy, CompletionRequirementKind, CreateProcessRequest, DrainOnceResult,
    HttpPostureResolver, MigrationMap, MigrationOutcome, PersistDraftResult, PostureDeclaration,
    PostureResolver, ResolvedPostureDeclaration, RuntimeError, RuntimeEventContext,
    RuntimeEventDecision, SIGNATURE_PROFILE_KEY_EXTENSION, SIGNATURE_PROFILE_REF_EXTENSION,
    SIGNATURE_STEP_ID_EXTENSION, SignatureProfileDocument, StaticPostureResolver, SystemClock,
    TaskSubmissionResult, TrellisCustodyAppendOutcome, TrellisCustodyAppender, WosRuntime,
    populate_provenance_record_fields, stamp_provenance,
};
pub use store::{
    InMemoryStore, IntakeRecord, ReplayKey, ReplayOperation, ReplayValue, RuntimeAuxFields,
    RuntimeRecord, RuntimeStore, StoreError, TaskArtifact, TaskArtifactKind, runtime_aux_from_json,
    runtime_aux_to_json,
};
pub use wos_core::business_calendar::BusinessCalendarDocument;
#[doc(inline)]
pub use wos_events::custody::{
    CustodyAppendContext, CustodyAppendError, CustodyAppendInput, CustodyAppendMetadata,
    CustodyAppendReceipt,
};

#[cfg(test)]
mod typeid_stack_common_contract {
    //! TWREF-004 — `stack-common-typeid` is the single mint/parse surface; regression guard for
    //! urn prefix handling that previously lived on the retired `wos_core::typeid` shim.

    use stack_common_typeid::{
        CASE_PREFIX, DEFAULT_TENANT, PROCESS_PREFIX, extract_tenant, is_case_ledger_id,
        is_process_id, is_valid_record_type_id, is_valid_type_id, mint_case_ledger_id,
        mint_process_id, mint_provenance_id, parse_case_ledger_id, parse_process_id,
    };

    #[test]
    fn given_minted_ids_when_round_tripped_through_urn_helpers_then_matches_shared_typeid_rules() {
        let case_id = mint_case_ledger_id();
        assert!(is_case_ledger_id(&case_id));
        assert_eq!(parse_case_ledger_id(&case_id), Some(case_id.as_str()));
        assert_eq!(extract_tenant(&case_id), Some(DEFAULT_TENANT));

        let case_urn = format!("urn:wos:{case_id}");
        assert_eq!(parse_case_ledger_id(&case_urn), Some(case_id.as_str()));
        assert!(!is_valid_type_id(&case_urn, Some(CASE_PREFIX)));
        assert!(!is_case_ledger_id(&case_urn));
        assert_eq!(extract_tenant(&case_urn), None);

        let process_id = mint_process_id();
        let process_urn = format!("urn:wos:{process_id}");
        assert_eq!(parse_process_id(&process_urn), Some(process_id.as_str()));
        assert!(!is_valid_type_id(&process_urn, Some(PROCESS_PREFIX)));
        assert!(!is_process_id(&process_urn));

        let record_id = mint_provenance_id();
        assert!(is_valid_record_type_id(&record_id));
        assert!(!is_valid_record_type_id(&format!("urn:wos:{record_id}")));
    }
}
