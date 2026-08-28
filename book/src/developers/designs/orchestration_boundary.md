# Orchestration Boundary

Status: **Accepted**

Related:

- [Node Drain, Replication Fences, and Maintenance Control Plane](node_drain_replication_fences_maintenance.md)
- [Replication Coordinator](replication_coordinator.md) — superseded by this decision

## Context

Kubidm is both an identity system and a distributed stateful service. The server must enforce strong
application-specific invariants around authentication, authorization, persistence, replication,
recovery, and maintenance. Operating a replicated service also requires a different class of
concerns: placement, desired replica count, workload replacement, failure-domain policy, rollout
ordering, storage lifecycle, and continuous reconciliation toward an intended topology.

Those two classes of responsibility are related, but they are not the same problem.

Historically, the inherited Replication Coordinator design proposed embedding additional
cluster-level coordination into the identity service so that replicas could register, discover
membership, receive topology, rotate replication configuration, and be garbage-collected by a
Kubidm-managed coordinator. That design attempted to improve automation, but it also moved generic
orchestration policy and lifecycle state into the application.

Kubidm has since developed a different operational direction. Replication-aware readiness,
replication generations, node drain, replication fences, synchronisation barriers, and in-process
maintenance all expose or enforce **Kubidm-specific correctness semantics** while allowing an
external controller to decide how a cluster-level workflow should proceed.

This document makes that separation an explicit project architecture principle.

## Decision

**Kubidm is a distributed identity system, not a general-purpose infrastructure orchestrator.**

Kubidm owns the mechanisms and invariants required to manipulate its local and replicated state
safely. Cluster-level policy and infrastructure lifecycle are delegated to an external
orchestration layer.

Kubidm will therefore prefer **small, explicit, machine-oriented operational primitives** over an
embedded cluster-wide controller.

These primitives should allow an external reconciler to observe state and request safe transitions
without requiring it to understand Kubidm database internals or infer correctness from process
state, logs, timing, or deployment-environment details.

Kubernetes is a primary orchestration environment for Kubidm, but this decision does not make
Kubernetes a runtime dependency of `kubidmd`. The same server primitives may be consumed by other
controllers or automation systems.

## Responsibility Boundary

| Kubidm | External orchestrator |
| --- | --- |
| Identity and authorization semantics | Desired replica count and topology policy |
| Database and on-disk invariants | Workload creation, placement, and replacement |
| Replication protocol and conflict semantics | Failure-domain and scheduling policy |
| Replication generations and history validity | Deciding when a recovery workflow should run |
| Replication fences and fence satisfaction | Selecting peers and sequencing handoff |
| Refresh/recovery safety rules | Infrastructure-level recovery orchestration |
| Node drain and write-admission safety | Selecting and ordering nodes for maintenance |
| In-process database maintenance semantics | Rolling maintenance sequencing |
| Machine-readable node/data-plane status | Aggregated cluster status and user-facing conditions |
| Idempotent node-local control operations | Retry, replay, and convergence of the overall workflow |

The boundary is semantic rather than transport-specific. A primitive may be exposed through a Unix
socket, HTTP, a dedicated mTLS listener, or another versioned protocol. What matters is that the
server remains authoritative for the correctness condition represented by the operation.

## Principles

### 1. Correctness stays with the component that can prove it

An external controller should not need to reproduce Kubidm's replication algorithm in order to
operate Kubidm safely.

For example, a controller may need to know whether replica A contains all history known to replica
B before taking B out of service. The correct interface is not a timestamp, a sleep, a Pod-ready
condition, or an approximation of the replication update vector in controller code. Kubidm should
provide a semantic operation such as a replication fence and fence-satisfaction check.

The same rule applies to recovery generations, database maintenance, and future distributed
operations.

### 2. Policy stays with the orchestration environment

Kubidm should not decide generic infrastructure questions for which the deployment environment
already has a control plane.

Examples include:

- which machine or availability zone should run a replica;
- how many replicas a deployment desires;
- which replica should be replaced after infrastructure failure;
- whether a rollout should proceed one node at a time;
- which storage class or volume implementation should back a replica;
- which node should be selected first for a maintenance operation.

Kubidm may expose constraints that make a choice safe or unsafe, but it should not own the generic
scheduler or desired-state database for those decisions.

### 3. Operations are designed for reconcilers, not only humans

Operational interfaces should assume that callers can crash, retry, replay, and observe partial
progress.

Mutating operations intended for automation should therefore be idempotent, or accept stable
operation identities that provide idempotent semantics. Status must be machine-readable. Failure
states must distinguish conditions that are retryable from those that require a new plan or human
intervention.

Human-oriented CLI commands may remain useful frontends, but an external controller should not have
to parse CLI text or logs to determine correctness.

### 4. Fail closed when safety cannot be established

Distributed operations should not silently weaken their invariants for automation convenience.

If Kubidm cannot prove that a replication fence is satisfied, that a recovery generation is
compatible, or that a database operation can run exclusively, the operation should fail with
structured state rather than let the caller infer success from timing or apparent health.

This may reduce availability during an exceptional workflow. That is preferable to presenting an
unproven state transition as safe.

### 5. Capability discovery precedes assumption

Operational APIs evolve. Controllers must be able to determine whether a node supports the
primitive and protocol version required by a workflow.

New operator-facing capabilities should therefore be versioned or otherwise capability-discoverable
from the beginning. A controller should be able to choose a conservative fallback or refuse an
operation without depending on the Kubidm release string alone.

### 6. Keep node-local mechanisms composable

Where practical, a server operation should describe one meaningful Kubidm transition rather than a
complete infrastructure workflow.

For example:

```text
node B: drain -> fence B
node A: sync until fence B
node B: run maintenance
node A: capture fence A
node B: sync until fence A
node B: resume
```

Kubidm defines the semantics of each transition. The external reconciler selects A and B, persists
workflow progress, handles retries, and decides what to do next.

This makes the primitives useful in Kubernetes without making them Kubernetes-specific.

## Kubernetes

Kubernetes is particularly well suited to the external-orchestration role because it already
provides the generic mechanisms required by a distributed workload controller:

- a durable desired-state API;
- watch/reconcile semantics;
- leader election and optimistic concurrency;
- scheduling and failure-domain constraints;
- StatefulSets and stable workload identities;
- storage lifecycle through CSI;
- status, conditions, events, and finalizers;
- rolling workload replacement;
- extensibility through Custom Resources and operators.

Kubidm should integrate cleanly with these semantics instead of recreating equivalent generic
control-plane machinery inside the identity server.

This does **not** mean that a simple Kubidm deployment requires Kubernetes. A single node or a
manually managed deployment can continue to run as a normal service under systemd, a container
runtime, or another supervisor. The architectural requirement is that advanced automation be able
to consume supported Kubidm primitives rather than emulate a human administrator.

## Operator-facing API Direction

The long-term operator-facing surface should expose only the semantic primitives that an external
controller cannot safely derive itself. Examples include:

- replication health and state;
- replication generation identity and compatibility;
- node drain/resume;
- versioned replication fences and fence satisfaction;
- explicit synchronisation/refresh requests and outcomes;
- recovery preconditions and progress;
- maintenance capabilities, state, and idempotent operations;
- safe replication membership or peer-management primitives if dynamic membership requires them.

These interfaces should not encode Kubernetes resources, scheduling policy, or deployment-specific
objects. They should describe Kubidm state transitions in Kubidm terms.

## Relationship to the Replication Coordinator Design

The earlier Replication Coordinator design attempted to solve automation by introducing a
Kubidm-managed topology coordinator responsible for node registration, membership, topology maps,
generation tracking, configuration distribution, and inactive-node cleanup.

This ADR supersedes that direction.

The underlying problems identified by that document are valid: manual certificate exchange,
static peer configuration, difficult node replacement, and administrator-driven topology changes
are poor interfaces for automated operation. The change is **where those problems are solved**.

Rather than adding a second cluster-level control plane inside Kubidm, the server should expose the
replication and lifecycle mechanisms required by an external reconciler. The reconciler already owns
the desired topology and infrastructure lifecycle and can compose those mechanisms into a workflow.

The historical coordinator document is retained because it describes useful requirements and the
reasoning that led to this boundary, but it is no longer the intended implementation direction.

## Consequences

### Positive

- Kubidm stays focused on identity and distributed-data correctness.
- Kubernetes operators can use native reconciliation rather than adapting to a parallel coordinator.
- Non-Kubernetes automation can consume the same semantic primitives.
- Operational correctness can be tested at the server boundary independently from orchestration
  policy.
- Controller failures and retries can be handled through explicit idempotency and status semantics.
- The project avoids maintaining another generic membership/lifecycle control plane.

### Costs

- Some operations require new server APIs before they can be automated safely.
- External controllers must understand the documented Kubidm operation state machines.
- Simple deployments and orchestrated deployments may use different levels of the operational API.
- Until a required primitive exists, an orchestrator may need a conservative workflow with reduced
  availability rather than a fully online operation.

These costs are intentional. They keep missing automation capabilities visible instead of hiding
them behind heuristics or duplicating distributed-system logic outside the server.

## Non-goals

This decision does not:

- require Kubernetes to run Kubidm;
- make Kubidm stateless;
- move replication correctness into an operator;
- require a specific Kubernetes operator implementation;
- prohibit helper tools for users who manage VMs or bare-metal services directly;
- prohibit server-side automation that is intrinsically part of maintaining Kubidm's own invariants;
- claim that every cluster operation can be reduced to a trivial node-local call.

The goal is a clear ownership boundary: **Kubidm provides the distributed-system mechanisms and
proofs; the deployment control plane provides policy and reconciliation.**
