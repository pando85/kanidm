# Maintenance Control Protocol

This document is the operator-facing protocol reference for the implementation described in [Node Drain, Replication Fences, and Maintenance Control Plane](node_drain_replication_fences_maintenance.md).

## Read-only HTTP surface

`GET /maintenance` returns the node-local maintenance state, active operation ID, last error, and capability set. It is intended for capability discovery and diagnostics. `GET /readyz` remains the normal serving-readiness endpoint and returns `503` whenever maintenance state is not `serving`.

Neither endpoint mutates directory or database state.

## Privileged local control surface

Mutating operations use the existing Unix admin socket, protected by its root/server-UID peer-credential check. The version-1 request set is:

- `MaintenanceCapabilities`
- `MaintenanceStatus`
- `MaintenanceDrain { operation_id }`
- `MaintenanceRun { operation_id, operation }`
- `ReplicationFence`
- `ReplicationSyncUntil { operation_id, fence, timeout_seconds }`
- `MaintenanceResume { operation_id }`

Supported native maintenance operations are `reindex` and `verify`. `vacuum` and `restore` are intentionally reported as unsupported because they require stronger backend/recovery lifecycle semantics.

## Rolling handoff

For replicas A and B, the safe maintenance sequence for B is:

1. drain B and persist the returned fence `F_B`;
2. require A to satisfy `F_B`;
3. run the maintenance operation on B;
4. capture a fresh fence `F_A` from A;
5. require B to satisfy `F_A` while B remains non-serving;
6. resume B only after fence satisfaction succeeds.

The fence is replication state, not wall-clock time and not Kubernetes readiness. Domain or replication-generation mismatches are hard failures.

## Transport stability

The Unix admin transport is the initial privileged control path. The maintenance semantics and request/result model are intentionally transport-independent so a future dedicated mTLS operator endpoint can expose the same operations without moving orchestration logic into Kubidm.
