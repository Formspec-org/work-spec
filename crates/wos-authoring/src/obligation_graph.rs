// Rust guideline compliant 2026-06-08

//! Obligation-graph extraction (WOS-TOOL-2501).
//!
//! Given a workflow's `governance.obligationPolicies[]`, [`obligation_graph`]
//! returns a static dependency graph whose nodes are events and obligations
//! and whose edges form the shape
//! `activate-event → obligation → {satisfy | cancel | violate}-event`.
//!
//! This is an authoring-time visualization/analysis aid (Claim A): it lets
//! `wos-mcp` and authoring tooling render "what activates this duty, and what
//! discharges it?" without running the kernel. It reads only the declared
//! [`ActivationCriteria`] triggers on each policy clause; it does not evaluate
//! FEL, resolve deadlines, or simulate the runtime lifecycle. Deadline expiry
//! and authorized bypass are lifecycle transitions with no triggering event,
//! so they are not edges here — see [`crate::obligation_explain`] for the
//! human-readable per-state rendering and Governance §16.2.2 for the full
//! lifecycle.
//!
//! Determinism: nodes and edges are emitted in policy-declaration order, then
//! clause order (`activate`, `satisfy`, `cancel`, `violate`). The output is a
//! pure function of the input slice — stable for snapshot testing.

use serde::{Deserialize, Serialize};
use wos_core::{ActivationCriteria, ObligationPolicy};

/// What a node in the [`ObligationGraph`] represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    /// A kernel event/transition trigger drawn from an [`ActivationCriteria`].
    Event,
    /// A durable obligation declared by an [`ObligationPolicy`].
    Obligation,
}

/// The role an edge plays in the obligation lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    /// `activate-event → obligation`: the event creates a pending obligation.
    Activate,
    /// `obligation → satisfy-event`: the event discharges the obligation.
    Satisfy,
    /// `obligation → cancel-event`: the event cancels the obligation.
    Cancel,
    /// `obligation → violate-event`: the event violates the obligation while pending.
    Violate,
}

/// A node: either an event or an obligation, keyed by a stable `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationGraphNode {
    /// Stable node id. For events this is the trigger label (see
    /// [`trigger_label`]); for obligations it is the policy `id`.
    pub id: String,
    /// Whether this node is an event or an obligation.
    pub kind: NodeKind,
    /// Human-readable label (policy `description` for obligations; the trigger
    /// label for events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A directed edge connecting an event node and an obligation node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationGraphEdge {
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// The lifecycle role this edge plays.
    pub kind: EdgeKind,
}

/// A static obligation dependency graph extracted from a set of policies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationGraph {
    /// Event and obligation nodes, de-duplicated, in first-seen order.
    pub nodes: Vec<ObligationGraphNode>,
    /// Directed edges in policy-then-clause declaration order.
    pub edges: Vec<ObligationGraphEdge>,
}

/// Derive a stable, human-meaningful label for an [`ActivationCriteria`]
/// trigger. Prefers the exact `event`, then `eventTag`, `transitionTag`,
/// `eventKind`; falls back to `<unbound>` when the criteria carries no `on`
/// trigger (e.g. a pure `where`/`requiredData` predicate).
///
/// Public so authoring tooling can label edges consistently with the graph.
pub fn trigger_label(criteria: &ActivationCriteria) -> String {
    match criteria.on.as_ref() {
        Some(trigger) => {
            if let Some(event) = trigger.event.as_deref() {
                event.to_string()
            } else if let Some(tag) = trigger.event_tag.as_deref() {
                format!("tag:{tag}")
            } else if let Some(tag) = trigger.transition_tag.as_deref() {
                format!("transitionTag:{tag}")
            } else if let Some(kind) = trigger.event_kind.as_ref() {
                // EventKind is camelCase via serde; render through it for fidelity.
                match serde_json::to_value(kind) {
                    Ok(serde_json::Value::String(s)) => format!("kind:{s}"),
                    _ => "kind:<unknown>".to_string(),
                }
            } else {
                "<unbound>".to_string()
            }
        }
        None => "<unbound>".to_string(),
    }
}

/// Extract the obligation graph from a slice of obligation policies
/// (`governance.obligationPolicies[]`).
///
/// Pure and deterministic: each policy contributes one obligation node and an
/// `activate` edge from its trigger; `satisfy`/`cancel`/`violate` clauses
/// contribute outbound edges. Event nodes are de-duplicated by label so a
/// single event shared across policies appears once.
pub fn obligation_graph(policies: &[ObligationPolicy]) -> ObligationGraph {
    let mut graph = ObligationGraph::default();

    for policy in policies {
        // Obligation node (declaration order; policy ids are unique within a
        // governance block, so no de-dup needed for these).
        push_node(
            &mut graph,
            ObligationGraphNode {
                id: policy.id.clone(),
                kind: NodeKind::Obligation,
                label: policy.description.clone(),
            },
        );

        // activate-event → obligation
        link_event(
            &mut graph,
            &policy.activate_when,
            &policy.id,
            EdgeKind::Activate,
        );
        // obligation → satisfy-event
        link_event(
            &mut graph,
            &policy.satisfy_when,
            &policy.id,
            EdgeKind::Satisfy,
        );
        // obligation → cancel-event
        if let Some(cancel) = policy.cancel_when.as_ref() {
            link_event(&mut graph, cancel, &policy.id, EdgeKind::Cancel);
        }
        // obligation → violate-event
        if let Some(violate) = policy.violate_when.as_ref() {
            link_event(&mut graph, violate, &policy.id, EdgeKind::Violate);
        }
    }

    graph
}

/// Add the event node for `criteria` (de-duplicated) and the edge linking it to
/// the obligation in the direction implied by `kind`.
fn link_event(
    graph: &mut ObligationGraph,
    criteria: &ActivationCriteria,
    obligation_id: &str,
    kind: EdgeKind,
) {
    let label = trigger_label(criteria);
    push_node(
        graph,
        ObligationGraphNode {
            id: label.clone(),
            kind: NodeKind::Event,
            label: None,
        },
    );
    let edge = match kind {
        // Activation flows event → obligation; all others flow obligation → event.
        EdgeKind::Activate => ObligationGraphEdge {
            from: label,
            to: obligation_id.to_string(),
            kind,
        },
        EdgeKind::Satisfy | EdgeKind::Cancel | EdgeKind::Violate => ObligationGraphEdge {
            from: obligation_id.to_string(),
            to: label,
            kind,
        },
    };
    graph.edges.push(edge);
}

/// Push a node unless an identical-id node is already present.
fn push_node(graph: &mut ObligationGraph, node: ObligationGraphNode) {
    if !graph.nodes.iter().any(|existing| existing.id == node.id) {
        graph.nodes.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policies_from(value: serde_json::Value) -> Vec<ObligationPolicy> {
        serde_json::from_value(value).expect("fixture policies must deserialize")
    }

    /// The canonical income-change policy yields one obligation node, three
    /// event nodes, and activate/satisfy/violate edges in declaration order.
    #[test]
    fn income_change_policy_graph_shape() {
        let policies = policies_from(serde_json::json!([
            {
                "id": "income-change-review-required",
                "description": "Independent underwriting review after income change.",
                "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" },
                "satisfyWhen": {
                    "on": { "event": "underwritingReviewCompleted" },
                    "actor": { "role": "underwriter", "notSameAsTriggerActor": true }
                },
                "violateWhen": { "on": { "event": "finalApprovalRequested" } },
                "deadline": { "within": "P2D" },
                "onViolation": "block"
            }
        ]));

        let graph = obligation_graph(&policies);

        // 1 obligation + 3 distinct events.
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.nodes[0].kind, NodeKind::Obligation);
        assert_eq!(graph.nodes[0].id, "income-change-review-required");
        assert_eq!(
            graph.nodes[0].label.as_deref(),
            Some("Independent underwriting review after income change.")
        );

        // Edges in clause order: activate, satisfy, violate.
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(
            graph.edges[0],
            ObligationGraphEdge {
                from: "caseFileUpdated".into(),
                to: "income-change-review-required".into(),
                kind: EdgeKind::Activate,
            }
        );
        assert_eq!(graph.edges[1].kind, EdgeKind::Satisfy);
        assert_eq!(graph.edges[1].from, "income-change-review-required");
        assert_eq!(graph.edges[1].to, "underwritingReviewCompleted");
        assert_eq!(graph.edges[2].kind, EdgeKind::Violate);
        assert_eq!(graph.edges[2].to, "finalApprovalRequested");
    }

    /// An event shared across two policies appears as a single node.
    #[test]
    fn shared_event_node_is_deduplicated() {
        let policies = policies_from(serde_json::json!([
            {
                "id": "a",
                "activateWhen": { "on": { "event": "decisionPrepared" } },
                "satisfyWhen": { "on": { "event": "noticeSent" } },
                "onViolation": "block"
            },
            {
                "id": "b",
                "activateWhen": { "on": { "event": "decisionPrepared" } },
                "satisfyWhen": { "on": { "event": "auditLogged" } },
                "onViolation": "warn"
            }
        ]));

        let graph = obligation_graph(&policies);

        // 2 obligations + decisionPrepared (shared) + noticeSent + auditLogged = 5.
        assert_eq!(graph.nodes.len(), 5);
        let decision_nodes = graph
            .nodes
            .iter()
            .filter(|n| n.id == "decisionPrepared")
            .count();
        assert_eq!(decision_nodes, 1, "shared event must be a single node");
        // Both policies still get their own activate edge from it.
        let activate_from_decision = graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Activate && e.from == "decisionPrepared")
            .count();
        assert_eq!(activate_from_decision, 2);
    }

    /// A trigger without an `on` clause labels as `<unbound>` rather than panicking.
    #[test]
    fn unbound_trigger_labels_gracefully() {
        let policies = policies_from(serde_json::json!([
            {
                "id": "where-only",
                "activateWhen": { "where": "caseFile.flag = true" },
                "satisfyWhen": { "on": { "eventTag": "resolved" } },
                "onViolation": "warn"
            }
        ]));

        let graph = obligation_graph(&policies);
        assert_eq!(graph.edges[0].from, "<unbound>");
        assert_eq!(graph.edges[1].to, "tag:resolved");
    }
}
