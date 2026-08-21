# Cluster Recovery, Replica Replacement, and Replication Generations

Status: **Proposed**

Related issues: #363, #357, #358, #361, #27

## Summary

Kubidm uses decentralised, eventually-consistent multi-master replication. Recovery must preserve that model rather than introducing a leader, witness, or write quorum.

This document defines three operations that must not be treated as equivalent:

1. **Replica replacement** — at least one trustworthy replica survives and a new node is populated from it.
2. **Complete-cluster disaster recovery** — no trustworthy live replica remains and the cluster is reconstructed from a verified backup.
3. **Historical rollback / PITR** — an operator intentionally makes an older state authoritative after newer replicated history has existed.

The first operation is normal replication bootstrap. The second and third establish a new recovery authority boundary. Historical rollback cannot be implemented correctly as a naive restore of one replica while peers with newer history remain active.

The target architecture introduces an explicit **replication generation** (also called an epoch in distributed-systems literature) for full-cluster restore and historical rollback. Replicas from an older generation are fenced from replication until they are wiped/refreshed into the new generation.

## Goals

- Preserve Kubidm's multi-master/eventual-consistency replication architecture.
- Make recovery semantics deterministic and automatable.
- Prevent stale pre-restore replicas from contaminating a recovered topology.
- Make node replacement cheap when a healthy peer exists.
- Make backup restore a real whole-cluster disaster-recovery primitive.
- Define the safety boundary required by PITR.
- Integrate recovery with backup verification and serving readiness.

## Non-goals

- Leader election or an authoritative primary during normal operation.
- Consensus/quorum-based writes.
- A witness/coordinator service.
- Treating replication as a substitute for backups.
- Treating backups as a mechanism for reverting one live replica independently.
- Implementing PITR in this document.

## Terminology

### Replica identity

The stable identity used by the replication protocol to identify one server/replica.

### Domain identity

The stable identity of the Kubidm logical domain. A disaster restore preserves the logical domain identity unless the operator is intentionally cloning/importing into a new domain.

### Replication generation

A monotonically changing opaque value that identifies one authoritative lineage of replicated history.

All replicas in one active topology must belong to the same generation. A replication handshake between different generations fails closed and reports that the stale node must be refreshed/reinitialised.

A generation is **not** a leader term and does not order normal writes. It is changed only by an explicit recovery operation that intentionally invalidates the previous cluster history.

## Invariants

### Normal multi-master operation

- All active replicas share the same domain identity and replication generation.
- Any writable replica may accept writes according to normal Kubidm policy.
- Temporary disconnection does not change the generation.
- Reconnection inside the supported replication window converges normally.
- Conflict handling remains part of the replication algorithm; a conflict is not, by itself, a reason to elect a primary or change generation.

### Successful backup

A backup reported as successful must satisfy the semantic-validity contract in #357. A backup selected for disaster recovery should additionally pass the restore-level verification defined by #358.

Storage checksums prove transport integrity, not database semantic validity or restorability.

### Serving readiness

A restored or replacement replica must not serve production traffic merely because its HTTP process is alive. The replication/local-database readiness model in #361 determines when it is safe to enter a Service/load-balancer pool.

## Scenario 1: Replica replacement

### Preconditions

- At least one surviving replica is trusted and healthy.
- The surviving topology remains in the current replication generation.
- The operation is not intended to undo logical history.

### Workflow

1. Remove/fence the failed instance at the infrastructure level.
2. Start a new empty replica with a new replica identity unless the implementation explicitly supports safe identity replacement.
3. Configure it to replicate from a healthy peer.
4. Perform the normal initial/full refresh.
5. Validate local database consistency and replication state.
6. Wait for `serving_safe=true` according to #361.
7. Add the replacement replica to production traffic.
8. Remove obsolete replication-peer metadata when the protocol requires explicit cleanup.

### Properties

- The replication generation does **not** change.
- No backup is required.
- No historical state is made authoritative.
- A surviving healthy replica is the recovery source because it already belongs to the active history.

This should be the cheapest and preferred recovery path for an individual-node loss.

## Scenario 2: Complete-cluster disaster recovery

### Preconditions

- No live replica is considered a trustworthy recovery source, or all replicas have been lost.
- The operator has selected a backup and verified it according to #357/#358.
- All pre-disaster replicas are fenced from the recovery network before restoration begins.

### Workflow

1. **Fence the old topology.** Stop old instances and prevent their replication endpoints from reaching the recovered topology.
2. Select the recovery point and verify the backup artifact.
3. Restore a single bootstrap node into an isolated recovery environment.
4. Run semantic database verification, RUV/replication metadata reconstruction, migrations, and startup validation.
5. Establish a **new replication generation** for the recovered lineage.
6. Start the bootstrap node and require local health/readiness checks to pass.
7. Create every additional replica as an empty/new node and refresh it from a replica in the new generation.
8. Validate replication convergence and readiness.
9. Only then return the topology to production traffic.
10. Old pre-disaster nodes may rejoin only after their local data is discarded and they are refreshed into the new generation.

### Why a new generation is required

A backup captures state at time `T`. An old replica can contain valid changes from `T+1` onward. If that old replica is allowed to reconnect using the same logical replication lineage, the replication protocol cannot infer that the operator intentionally chose the older backup as authoritative. Replaying the newer history may therefore undo the recovery intent.

The generation change communicates exactly that missing fact: **history before the recovery boundary is no longer an eligible replication source**.

## Scenario 3: Historical rollback / PITR

Historical rollback has the strongest fencing requirement.

Consider:

```text
T0  account exists
T1  account deleted and deletion replicates to A and B
T2  operator restores A from a T0 backup
T3  B reconnects
```

Without an explicit history boundary, the deletion at T1 is a legitimate newer replicated operation. Restoring A does not make T0 globally authoritative; B can correctly propagate the deletion again.

Therefore Kubidm MUST NOT advertise "restore one live replica" as a supported cluster rollback procedure.

### Supported target workflow

1. Select a PITR/backup recovery point.
2. Fence **all** replicas from the old topology.
3. Restore one bootstrap node to the selected point.
4. Validate it completely.
5. Bump/create a new replication generation.
6. Recreate every other replica from that recovered node/topology.
7. Re-enable production traffic after readiness and convergence checks pass.

PITR is therefore a **cluster-lineage operation**, not a local database operation.

## Replication generation design

### Required properties

The generation value should:

- be persisted in database/replication metadata;
- be included in backup artifacts;
- be exchanged in the replication handshake before changes are accepted;
- be compared for exact equality during normal replication;
- be changed only by an explicit administrative recovery operation;
- be visible in diagnostics/status output;
- never be inferred from wall-clock time;
- not depend on a continuously available coordinator.

A UUID is sufficient as the generation identifier. A numeric counter is not necessary because generations do not need total ordering; equality/fencing is the relevant property.

### Handshake behaviour

Conceptually:

```text
consumer generation G2  <---- handshake ----> supplier generation G1

G1 == G2  -> normal replication negotiation
G1 != G2  -> reject before applying data
            report ReplicationGenerationMismatch
            require explicit wipe/refresh/recovery action
```

A mismatch must never trigger an automatic bidirectional merge. The entire purpose of the generation is to make an operator-selected history boundary fail closed.

### Creating a new generation

Generation rotation must be an explicit privileged recovery operation, for example conceptually:

```text
kubidmd database recovery-finalize --new-replication-generation
```

The exact CLI/API is a follow-up implementation decision.

The operation should only be valid while the server is in an offline/recovery state or otherwise protected from normal replication. It must be audited prominently.

### Backups and generations

Backups should record the generation from which they were produced for provenance and diagnostics.

Restoring a backup for ordinary inspection does not inherently need to mutate the generation. Finalising that restore as the new authoritative production cluster does.

This distinction allows verification tooling to restore into a temporary backend without accidentally manufacturing a new production lineage.

### Compatibility before generation support

Until the replication protocol supports generations, disaster recovery must emulate fencing operationally:

- stop and isolate all old replicas;
- rotate/reissue replication credentials or otherwise prevent old peers from authenticating;
- restore only one bootstrap node;
- build every production peer from that restored source;
- never reconnect an old data directory to the restored topology.

This is less robust than protocol-level generation fencing and should be documented as the transitional procedure, not the final architecture.

## Alternatives considered

### Restore one replica and let replication converge

Rejected for rollback/DR. It does not encode operator intent that an older point should become authoritative. Newer legitimate changes on peers may reappear.

It remains valid only for cases where the restored node is **not** intended to roll history back and a healthy peer remains authoritative; in that case refreshing a new node from the healthy peer is simpler and safer than restoring it from backup.

### Elect a primary/leader after restore

Rejected. This introduces normal-operation coordination and availability coupling merely to solve a rare recovery-boundary problem. Kubidm does not need consensus to represent a recovery generation.

### Add a witness/coordinator

Rejected. A coordinator becomes an additional availability dependency and does not remove the need to define backup/rollback lineage semantics.

### Use timestamps to decide which side wins

Rejected. Wall-clock ordering does not encode administrative recovery intent, is vulnerable to clock assumptions, and would make rollback impossible by definition because the discarded history is newer.

### Only rotate replication credentials

Acceptable as an interim operational fence, but insufficient as the long-term semantic model. Credentials answer "is this peer authenticated?"; a replication generation answers "does this peer belong to the authoritative history I am willing to merge?". Keeping those concepts separate produces safer diagnostics and automation.

### Change the domain identity on restore

Rejected as the default. A disaster restore is recovery of the same logical identity domain, not creation of an unrelated domain. Changing domain identity may also affect integrations and identity semantics beyond replication.

## Interaction with replication retention

A replica temporarily disconnected but still inside the supported replication retention window remains in the same generation and should catch up normally.

Being outside the retention window may require a full refresh, but does not itself imply a generation change. The source is still the same authoritative lineage.

Generation changes are reserved for intentional invalidation of old history, not routine lag recovery.

## Interaction with conflict entries

Conflict entries are part of normal multi-master convergence. Their existence must not automatically make a replica belong to a different generation or force recovery.

A generation boundary is a coarse administrative lineage fence. Conflict resolution is an intra-generation replication mechanism. The two concepts should remain independent.

## Failure handling

Recovery tooling should fail closed when:

- the selected backup cannot pass semantic/restore verification;
- the restored database cannot complete startup/reconstruction;
- a peer attempts to replicate with a different generation;
- an old replica is detected during recovery finalisation;
- local database consistency is not sufficient for serving readiness.

Errors must identify the domain, local replica, local generation, remote replica, and remote generation where safe to do so. Sensitive credential data must not be logged.

## Kubernetes/operator implications

Kaniop/Kubernetes automation can build on these primitives without becoming part of the consistency protocol:

- readiness gates traffic according to #361;
- replacement Pods start empty and refresh from healthy peers;
- DR automation restores exactly one bootstrap node before scaling the topology;
- a generation mismatch is a hard condition requiring reinitialisation, not a retry loop;
- StatefulSet/PVC reuse must not allow an old-generation data directory to silently rejoin after rollback.

The operator orchestrates the documented protocol; it does not elect a database leader.

## Follow-up implementation work

1. Add replication-generation metadata to the database and backup format with migration/backward-compatibility handling.
2. Add generation exchange/checking to replication handshakes.
3. Add an explicit recovery-finalisation operation that rotates the generation safely.
4. Expose generation and mismatch state through diagnostics/readiness metrics.
5. Add E2E tests proving an old-generation replica cannot contaminate a restored topology.
6. Update PITR (#27) so finalising a historical recovery establishes a new generation.
7. Add operator/runbook documentation for the transitional credential-fencing procedure.

## Required tests for generation support

At minimum:

- normal disconnected replicas with equal generations reconnect and converge;
- a newly bootstrapped replica joins the current generation via refresh;
- restoring a backup and finalising recovery produces a new generation;
- an old replica from the previous generation is rejected before applying changes;
- wiping that old replica and refreshing it allows it to join the new generation;
- PITR followed by generation rotation does not replay post-target history from an old peer;
- backup verification in a temporary backend does not mutate production generation semantics;
- generation mismatch is observable and causes `serving_safe=false` where appropriate.

## Decision

Kubidm will keep its decentralised multi-master replication model.

Replica replacement remains ordinary refresh/bootstrap inside the current replication generation.

Complete-cluster disaster recovery and intentional historical rollback establish a new authoritative lineage. The target architecture represents that lineage with an explicit replication generation and fences peers from older generations before they can exchange replicated data.

Until protocol-level generation fencing is implemented, operators must achieve the same safety property by isolating every old replica and rebuilding all peers from the verified restored bootstrap node.