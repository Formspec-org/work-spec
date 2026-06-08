// Rust guideline compliant 2026-02-21

//! Intent-driven authoring API for WOS Kernel Documents.
//!
//! `wos-authoring` is the single seam every external consumer
//! (`wos-mcp`, future `wos-synth-core`, integration tests) uses to
//! mutate a `KernelDocument`. The crate exposes exactly one façade —
//! [`WosProject`] — plus the types needed to describe results
//! ([`AuthoringDiagnostic`], [`AuthoringResult`]) and identify common
//! enumerations from `wos-core` (re-exported for ergonomic call sites).
//!
//! # Layer overview
//!
//! ```text
//! WosProject (façade, this crate — only public API)
//!      |
//! RawWosProject + IWosProjectCore + Command  (pub(crate))
//!      |
//! KernelDocument (wos-core)
//! ```
//!
//! `Command` and `RawWosProject::dispatch` are `pub(crate)` on purpose:
//! the façade is the only way to issue a mutation, which keeps the public
//! surface stable as new commands land and guarantees every mutation
//! benefits from the crate's undo/redo machinery.

mod command;
mod diagnostics;
pub mod obligation_explain;
pub mod obligation_graph;
mod project;
mod raw;

pub use diagnostics::{AuthoringDiagnostic, Severity};
pub use obligation_explain::{
    explain_instance, explain_pending, explain_satisfied, explain_status, explain_violated,
};
pub use obligation_graph::{
    obligation_graph, EdgeKind, NodeKind, ObligationGraph, ObligationGraphEdge, ObligationGraphNode,
};
pub use project::{AuthoringResult, WosProject};

// Re-export the kernel types callers pass to `WosProject` helpers so they
// don't need a direct `wos-core` dependency for ergonomic authoring code.
pub use wos_core::{ActorKind, ImpactLevel, KernelDocument, StateKind};

// Re-export model types needed by wos-mcp's query tools for graph generation.
pub use wos_core::model::kernel::{Actor, State, Transition, TransitionEvent};
