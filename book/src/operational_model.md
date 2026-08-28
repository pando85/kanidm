# Operational Model

Kubidm is designed to work in both simple deployments and orchestrated distributed environments.
The server does not require Kubernetes, but its operational interfaces are intended to be usable by
modern reconcilers rather than only by a human following an imperative runbook.

The central model is:

> **Kubidm owns identity and distributed-data correctness. The deployment control plane owns
> infrastructure policy and reconciliation.**

This separation keeps the server portable while allowing Kubernetes and other orchestration systems
to manage Kubidm without duplicating its replication or database semantics.

## Simple deployments

A single Kubidm server can be operated as a normal long-running service:

```text
systemd / container runtime / service supervisor
                    |
                    v
                 kubidmd
                    |
                    v
               local storage
```

For small deployments this may be all that is required. The administrator chooses when to start,
stop, upgrade, back up, or maintain the server.

Kubidm continues to support this model. Cloud-native operation does not mean that every installation
must run Kubernetes.

## Orchestrated distributed deployments

Once a deployment has multiple replicas, persistent storage, replacement workflows, upgrades,
maintenance, and failure-domain requirements, there are two distinct control problems:

1. **application correctness** — whether a replication, recovery, or database transition is valid;
2. **infrastructure orchestration** — which instance should change, when it should change, and how
   the surrounding resources should converge afterward.

Kubidm is authoritative for the first. An external control plane is authoritative for the second.

A Kubernetes deployment therefore looks conceptually like:

```text
                 Kubernetes API
                      |
               desired state
                      |
                      v
                   operator
             policy + reconciliation
                      |
           Kubidm semantic operations
                      |
          +-----------+-----------+
          |                       |
          v                       v
       kubidmd A <--- replication ---> kubidmd B
          |                       |
          v                       v
       storage A               storage B
```

The operator does not become the replication engine. It uses server-provided state and operations to
sequence cluster workflows while Kubernetes provides scheduling, workload lifecycle, storage
attachment, desired state, and reconciliation.

## What Kubidm should expose

An orchestrator should not need to parse logs, scrape human-oriented CLI output, sleep for an
assumed convergence interval, or reproduce Kubidm's database rules.

Where a cluster workflow depends on a Kubidm-specific invariant, Kubidm should expose that invariant
through a machine-oriented interface. Depending on the operation this can include:

- replication-aware health and state;
- replication generation identity;
- drain and resume semantics;
- replication fences and fence satisfaction;
- explicit synchronisation or refresh outcomes;
- recovery state and preconditions;
- maintenance capabilities and idempotent operations.

The transport may evolve. The important contract is semantic: the server is the component that can
prove whether the requested data-plane transition is safe.

## What the external control plane should own

The orchestration layer owns questions that are generic to operating workloads rather than to the
identity database itself, including:

- desired replica count;
- node and failure-domain placement;
- persistent-volume lifecycle;
- workload replacement after infrastructure failure;
- rollout and upgrade ordering;
- selection of a replica for maintenance;
- persistence of long-running workflow progress;
- retries and reconciliation after controller restart;
- cluster-level status presented to operators.

In Kubernetes these concerns naturally map to controllers, Custom Resources, StatefulSets, Services,
CSI-backed storage, topology constraints, status conditions, finalizers, and events.

## Safety over apparent automation

A distributed control plane can make an unsafe workflow look automated. Kubidm deliberately avoids
that trade.

For example, none of these alone prove that a replica can be removed safely:

```text
Pod Ready == true
last replication timestamp is recent
no errors appeared in logs
we waited N seconds
StatefulSet rollout completed
```

Those may be useful operational signals, but they do not express the replication invariant that the
workflow actually depends on.

When Kubidm can provide a semantic proof or state transition, automation should use it. When it
cannot, an orchestrator should prefer a conservative workflow—possibly including temporary service
downtime—over an optimistic heuristic.

## Reconciler-friendly operations

Modern controllers are not one-shot scripts. They can restart between any two steps and may repeat
a request after losing the response.

Operator-facing Kubidm operations should therefore be designed so that:

- requests are idempotent or use caller-provided operation identities;
- current state and progress are machine-readable;
- capabilities and protocol versions can be discovered;
- invalid generation/history combinations fail explicitly;
- failures do not silently return a node to service when safety is unknown;
- a caller can resume reconciliation without reconstructing hidden server state from logs.

These properties are useful outside Kubernetes too. Ansible, Nomad, VM-management systems, or a
purpose-built controller can consume the same primitives.

## Kubernetes is an orchestration target, not a server dependency

Kubidm has a strong Kubernetes focus because Kubernetes already supplies a mature generic control
plane for stateful workloads. Reusing that control plane avoids rebuilding scheduling, desired
state, leader election, resource lifecycle, storage integration, and reconciliation inside Kubidm.

That does not require importing Kubernetes libraries into `kubidmd`, nor does it prevent a Kubidm
binary from being started directly on a host.

The goal is narrower and more durable:

> **A serious external reconciler should be able to operate Kubidm through supported semantic
> primitives instead of pretending to be a human system administrator.**

For the developer-level architectural decision behind this model, see
[Orchestration Boundary](developers/designs/orchestration_boundary.md).
