# Cluster Disaster Recovery, Replica Replacement, and Rollback Semantics

- **Status**: Accepted
- **Date**: 2026-08-21
- **Related Issues**: #363, #357, #358, #361, #27
- **Related Upstream**: kanidm/kanidm#3941

## Context

Kubidm operates as a multi-master, eventually consistent, leaderless replication topology. Every node can
accept writes, and convergence is achieved through change identifiers (CIDs) that combine wall-clock time
with a server UUID to establish a total ordering of events. Replication uses a Replication Update Vector
(RUV) to track the range of changes known to each node, and supports two phases: *refresh* (full clone) and
*incremental* (differential exchange).

This architecture makes several recovery scenarios operationally and semantically distinct, yet they are
easy to conflate:

1. **Replica replacement** -- a failed or new node joins an existing healthy topology.
2. **Complete cluster disaster recovery** -- all replicas are lost and the cluster must be rebuilt from a
   backup.
3. **Historical rollback / Point-in-Time Recovery (PITR)** -- an operator intentionally reverts the entire
   cluster to a state older than the current replicated state.

The first is a replication/bootstrap operation. The second is disaster recovery. The third is a
distributed-history problem: existing replicas may legitimately contain newer changes that would conflict
with or overwrite the restored state.

Naive online restore of a single replica while peers continue operating is **not** a correct general
mechanism for any of these scenarios. This ADR defines first-class recovery semantics without introducing
a coordinator, leader, quorum, or consensus-based write architecture.

## Decision

### 1. Replica Replacement

**Definition**: Replacing a failed or new replica from a healthy peer in an existing topology.

**Invariants**:

- At least one healthy replica with a current RUV must exist.
- The replacement node starts with an empty database and performs a full refresh from a healthy supplier.
- The replacement node's server UUID is new (or the old server entry has been revoked/removed from the
  topology).
- The replacement node is not considered safe to serve traffic until it has completed a full refresh and
  its RUV overlaps with the supplier's RUV for all known servers.

**Supported Workflow**:

1. Identify a healthy supplier node with an up-to-date RUV.
2. If the failed node's identity is still registered in the topology (via the replication coordinator or
   manual configuration), revoke or remove its server entry to prevent it from rejoining with stale
   metadata.
3. Initialize the replacement node with a fresh database and replication configuration pointing at the
   healthy supplier.
4. The replacement node performs a full refresh from the supplier.
5. After the refresh completes, the node transitions to incremental replication mode.
6. Verify the node's RUV has converged with the supplier before directing client traffic.

**Stale identity handling**: If a dead replica's server UUID remains in the RUV of other nodes, it will
eventually be trimmed when the tombstone reap window passes and the RUV minimum advances. Until then, the
dead UUID occupies RUV space but does not affect correctness. Operators may accelerate this by revoking the
server entry via the replication coordinator.

### 2. Complete Cluster Disaster Recovery

**Definition**: Rebuilding the entire cluster when no healthy replica remains, using a backup.

**Invariants**:

- No surviving replica exists that can serve as a replication supplier.
- A verified backup from a known-good point in time is available.
- All pre-disaster replica identities are considered untrusted and must not rejoin the restored topology
  without a full refresh from the restored node.
- The restored node forms a new single-node topology initially.

**Supported Workflow**:

1. Select a backup and verify its integrity (checksum validation, version compatibility).
2. Stop all surviving pre-disaster Kubidm instances (if any are reachable) and isolate them from the
   network to prevent accidental rejoin.
3. Restore the backup onto a fresh node. This node becomes the *seed* of the new topology.
4. Start the seed node and verify it is serving correctly (health checks, authentication tests, schema
   validation).
5. Establish the new replication topology by adding replacement nodes via the replica replacement workflow
   (Section 1). Each new node performs a full refresh from the seed.
6. Only after the new topology is verified healthy should client traffic be restored.

**Preventing stale replica contamination**: Pre-disaster replicas that are brought back online after a
cluster restore possess a RUV containing CIDs from the old topology. If allowed to rejoin directly, they
could:

- Supply changes with CIDs that appear *newer* than the restored state (because wall-clock time advanced
  before the disaster), causing the restored node to accept stale or conflicting data.
- Reintroduce tombstoned entries that were deleted after the backup was taken.

To prevent this, pre-disaster replicas **must not** rejoin the restored topology via incremental
replication. They must either be decommissioned or re-initialized via a full refresh from the restored
seed node (treating them as replacement nodes).

### 3. Historical Rollback / Point-in-Time Recovery (PITR)

**Definition**: Intentionally making an older database state authoritative after newer replicated changes
already exist in the topology.

**Invariants**:

- This is a deliberate, operator-initiated operation that discards all changes newer than the target
  restore point.
- It is semantically distinct from replica replacement and disaster recovery because the operator is
  choosing to *lose* data that was valid under the previous replication history.
- After rollback, the replication history is discontinuous: the restored state has no knowledge of changes
  that occurred after the restore point.

**Why naive single-replica online rollback is rejected**: Restoring a backup onto one node while peers
continue with newer state is incorrect because:

- The restored node's RUV will be *behind* its peers, placing it in a *lagging* state.
- Peers will supply incremental changes that postdate the restore point, effectively undoing the rollback.
- The restored node may reintroduce entries that were legitimately deleted after the backup point,
  creating zombie entries.
- Attribute-level conflict resolution will merge the restored (older) values with the current (newer)
  values in unpredictable ways.

**Supported Workflow**:

1. Stop **all** nodes in the topology. This is mandatory to prevent any node from holding a newer state
   that could contaminate the rollback.
2. Select and verify the target backup (must be from the same server version).
3. Restore the backup onto **every** node in the topology, or destroy all existing nodes and rebuild from
   the backup using the complete cluster DR workflow (Section 2).
4. Start the restored topology and verify convergence.

This effectively reduces PITR to a full-cluster disaster recovery operation, which is the only safe
approach in a leaderless, eventually consistent system.

### 4. Replication Epoch / Generation Evaluation

**Question**: Should Kubidm introduce a replication epoch (generation) mechanism to allow a deliberate
full-cluster rollback to establish new authoritative replication history and reject stale replicas?

**Proposed Model**:

```text
backup at t0
    -> restore
    -> establish replication epoch E+1
    -> old E replicas may not rejoin directly
    -> new replicas refresh from the restored E+1 topology
```

**Trade-offs**:

| Aspect | With Epoch | Without Epoch |
|--------|-----------|---------------|
| Stale replica rejection | Automatic -- nodes from epoch E are rejected by epoch E+1 nodes | Manual -- operators must ensure stale nodes are isolated or refreshed |
| PITR safety | Strong -- epoch boundary prevents accidental mixing of old and new history | Relies on operator discipline to stop all nodes before restore |
| Implementation complexity | Requires new metadata on every entry/RUV, migration path, and protocol changes | No new infrastructure required |
| Operational overhead | Minimal after implementation -- epoch is checked automatically | Operators must follow strict procedural checklists |
| Risk of misconfiguration | Low -- protocol enforces epoch boundaries | Higher -- a single node restarted with old state can contaminate topology |

**Evaluation**:

An epoch mechanism would provide a *protocol-level guarantee* that stale replicas cannot contaminate a
restored topology. Without it, safety depends entirely on operator procedure (stopping all nodes,
isolating old replicas). Given that Kubidm's replication is leaderless and eventually consistent, the
consequences of a stale replica rejoining are severe (zombie entries, conflicting attributes, silent data
corruption).

However, implementing epochs requires:

- Adding an epoch identifier to every entry's change state and to the RUV.
- Defining epoch transition semantics (how a restore increments the epoch).
- Ensuring backward compatibility or defining a migration path.
- Modifying the incremental replication protocol to reject cross-epoch changes.

**Decision**: This ADR does **not** mandate an epoch mechanism at this time. The supported workflows
(Sections 1-3) are sufficient for safe operation when followed correctly. However, the epoch concept is
identified as a high-priority follow-up (see Follow-up Issues) because it would significantly reduce the
operational risk of PITR and disaster recovery by encoding safety invariants into the protocol rather than
relying on procedural compliance.

### 5. Integration with Backup Verification and Replication-Aware Readiness

**Backup verification**: Before any restore operation, the backup must be verified for:

- Integrity (SHA-256 checksum matches).
- Version compatibility (backup was created by the same server version).
- Completeness (database is not truncated or corrupt).

Kubidm provides `kubidmd database verify` and `kubidmd database verify-s3` commands for this purpose.

**Replication-aware readiness**: A restored or replacement node must not be considered *ready* to serve
client traffic until:

- It has completed a full refresh from a known-good supplier (or is the seed of a new topology).
- Its RUV indicates convergence with the supplier (no lagging state).
- Health checks pass (schema validation, access control integrity, plugin consistency).

Load balancers and orchestration systems should use these readiness signals before directing traffic to a
recovered node.

## Non-Goals

- Naive online restore of one active replica while peers continue unchanged (explicitly rejected).
- Leader election, quorum, witness, or consensus-based writes.
- Implementing PITR automation or tooling in this ADR (this document defines semantics only).
- Modifying the replication protocol (epoch/generation is evaluated but not mandated).

## Consequences

### Positive

- Clear, documented workflows for each recovery scenario reduce operational ambiguity.
- Explicit rejection of unsafe patterns (naive single-replica rollback) prevents data corruption.
- The design integrates with existing backup verification and replication mechanisms without protocol
  changes.
- The epoch evaluation provides a clear path for future hardening.

### Negative

- Without an epoch mechanism, PITR safety relies on operator discipline (stopping all nodes).
- The full-cluster stop requirement for PITR increases recovery time.
- Operators must understand the distinction between the three scenarios to choose the correct workflow.

### Neutral

- The replica replacement workflow is consistent with existing replication refresh semantics.
- The disaster recovery workflow is a natural extension of backup restore plus replica replacement.

## Follow-up Issues

1. **Replication epoch/generation implementation**: Design and implement a protocol-level epoch mechanism
   to enforce stale replica rejection automatically. This would harden PITR and DR workflows against
   operator error. (Related: #363)

2. **Replication-aware readiness probes**: Expose node readiness signals that account for replication
   state (RUV convergence, lagging/advanced status) so that load balancers and orchestrators can make
   informed routing decisions during recovery. (Related: #358)

3. **Backup verification automation**: Integrate backup verification into the restore workflow so that
   corrupted or incompatible backups are rejected before they can cause damage. (Related: #357)

4. **PITR tooling**: Build operator-facing tooling that automates the full-cluster stop, restore, and
   restart workflow for point-in-time recovery, reducing the risk of procedural errors. (Related: #27)

5. **Stale replica detection**: Implement detection and alerting for nodes that attempt to rejoin a
   topology with a divergent or outdated RUV, providing early warning of potential contamination.
   (Related: #361)

6. **Recovery runbook documentation**: Translate the workflows defined in this ADR into step-by-step
   operator runbooks with examples for common deployment topologies (single-site, multi-site,
   cloud-native).
