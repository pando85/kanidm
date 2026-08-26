from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement, found {count}")
    p.write_text(text.replace(old, new))


# Hard QueryServer write-admission gate, checked before and after semaphore wait.
replace(
    "server/lib/src/server/mod.rs",
    """    pub async fn write(\n        &self,\n        curtime: Duration,\n    ) -> Result<QueryServerWriteTransaction<'_>, OperationError> {\n        let (write_ticket, db_ticket) = self\n""",
    """    pub async fn write(\n        &self,\n        curtime: Duration,\n    ) -> Result<QueryServerWriteTransaction<'_>, OperationError> {\n        if !crate::maintenance::maintenance_write_allowed() {\n            return Err(OperationError::InvalidState);\n        }\n\n        let (write_ticket, db_ticket) = self\n""",
)
replace(
    "server/lib/src/server/mod.rs",
    """        let (write_ticket, db_ticket) = self\n            .write_acquire_ticket()\n            .await\n            .ok_or(OperationError::DatabaseLockAcquisitionTimeout)?;\n\n        // Point of no return - we now have a DB thread AND the write ticket, we MUST complete\n""",
    """        let (write_ticket, db_ticket) = self\n            .write_acquire_ticket()\n            .await\n            .ok_or(OperationError::DatabaseLockAcquisitionTimeout)?;\n\n        // Drain may have started while this writer was queued. Re-check only\n        // after owning the single-writer permit so no pre-drain waiter can slip\n        // through when a maintenance fence is released.\n        if !crate::maintenance::maintenance_write_allowed() {\n            return Err(OperationError::InvalidState);\n        }\n\n        // Point of no return - we now have a DB thread AND the write ticket, we MUST complete\n""",
)

# Tokio task-local privileged write bypass.
replace(
    "server/lib/src/maintenance.rs",
    """use serde::{Deserialize, Serialize};\nuse std::collections::BTreeMap;\n""",
    """use serde::{Deserialize, Serialize};\nuse std::collections::BTreeMap;\nuse std::future::Future;\n""",
)
replace(
    "server/lib/src/maintenance.rs",
    """static MAINTENANCE_STATE: AtomicU8 = AtomicU8::new(MaintenanceState::Serving as u8);\nstatic ACTIVE_OPERATION_ID: RwLock<Option<Uuid>> = RwLock::new(None);\nstatic LAST_ERROR: RwLock<Option<String>> = RwLock::new(None);\n\npub fn maintenance_state() -> MaintenanceState {\n""",
    """static MAINTENANCE_STATE: AtomicU8 = AtomicU8::new(MaintenanceState::Serving as u8);\nstatic ACTIVE_OPERATION_ID: RwLock<Option<Uuid>> = RwLock::new(None);\nstatic LAST_ERROR: RwLock<Option<String>> = RwLock::new(None);\n\ntokio::task_local! {\n    static MAINTENANCE_WRITE_BYPASS: ();\n}\n\nfn write_allowed_for_state(state: MaintenanceState, bypass: bool) -> bool {\n    state.is_serving() || bypass\n}\n\n/// Whether the current async task may start a QueryServer write transaction.\n///\n/// Ordinary tasks are admitted only while the node is serving. Privileged\n/// maintenance work and replication recovery are scoped with\n/// [`with_maintenance_write_bypass`], which is task-local and therefore can not\n/// accidentally admit unrelated concurrent writers.\npub fn maintenance_write_allowed() -> bool {\n    let bypass = MAINTENANCE_WRITE_BYPASS\n        .try_with(|()| true)\n        .unwrap_or(false);\n    write_allowed_for_state(maintenance_state(), bypass)\n}\n\n/// Run one future with privileged access to QueryServer writes.\n///\n/// The scope is inherited only by the future being directly awaited; unrelated\n/// Tokio tasks remain subject to the normal maintenance write gate.\npub async fn with_maintenance_write_bypass<F>(future: F) -> F::Output\nwhere\n    F: Future,\n{\n    MAINTENANCE_WRITE_BYPASS.scope((), future).await\n}\n\npub fn maintenance_state() -> MaintenanceState {\n""",
)
replace(
    "server/lib/src/maintenance.rs",
    """    #[test]\n    fn fence_rejects_domain_or_generation_mismatch() {\n""",
    """    #[test]\n    fn write_gate_allows_only_serving_or_privileged_tasks() {\n        assert!(write_allowed_for_state(MaintenanceState::Serving, false));\n        assert!(write_allowed_for_state(MaintenanceState::Serving, true));\n        assert!(!write_allowed_for_state(MaintenanceState::Draining, false));\n        assert!(!write_allowed_for_state(MaintenanceState::Fenced, false));\n        assert!(!write_allowed_for_state(MaintenanceState::Maintenance, false));\n        assert!(!write_allowed_for_state(MaintenanceState::Recovering, false));\n        assert!(!write_allowed_for_state(MaintenanceState::Failed, false));\n        assert!(write_allowed_for_state(MaintenanceState::Recovering, true));\n    }\n\n    #[test]\n    fn fence_rejects_domain_or_generation_mismatch() {\n""",
)

# Maintenance admin operations use the scoped bypass.
replace(
    "server/core/src/admin.rs",
    """    maintenance_public_status, set_maintenance_error, set_maintenance_state, FenceSatisfaction,\n    MaintenanceCapabilities, MaintenanceOperation, MaintenancePublicStatus, MaintenanceState,\n    ReplicationFence,\n""",
    """    maintenance_public_status, set_maintenance_error, set_maintenance_state,\n    with_maintenance_write_bypass, FenceSatisfaction, MaintenanceCapabilities,\n    MaintenanceOperation, MaintenancePublicStatus, MaintenanceState, ReplicationFence,\n""",
)
replace(
    "server/core/src/admin.rs",
    """        let proxy_write = match idms.proxy_write(duration_from_epoch_now()).await {\n""",
    """        let proxy_write = match with_maintenance_write_bypass(\n            idms.proxy_write(duration_from_epoch_now()),\n        )\n        .await\n        {\n""",
)
replace(
    "server/core/src/admin.rs",
    """        let proxy_write = idms\n            .proxy_write(duration_from_epoch_now())\n            .await\n            .map_err(|err| format!(\"unable to acquire reindex write transaction: {err:?}\"))?;\n""",
    """        let proxy_write = with_maintenance_write_bypass(\n            idms.proxy_write(duration_from_epoch_now()),\n        )\n        .await\n        .map_err(|err| format!(\"unable to acquire reindex write transaction: {err:?}\"))?;\n""",
)

# Replication apply is privileged only while Recovering; it is idle in other
# non-serving maintenance states.
replace(
    "server/core/src/repl/mod.rs",
    """use kubidmd_lib::prelude::duration_from_epoch_now;\nuse kubidmd_lib::prelude::IdmServer;\n""",
    """use kubidmd_lib::maintenance::{\n    maintenance_state, with_maintenance_write_bypass, MaintenanceState,\n};\nuse kubidmd_lib::prelude::duration_from_epoch_now;\nuse kubidmd_lib::prelude::IdmServer;\n""",
)
replace(
    "server/core/src/repl/mod.rs",
    """        idms.proxy_write(ct)\n            .await\n            .and_then(|mut write_txn| {\n                write_txn\n                    .qs_write\n                    .consumer_apply_refresh(refresh)\n                    .and_then(|cs| write_txn.commit().map(|()| cs))\n            })\n            .map_err(|err| error!(?err, \"Consumer was not able to apply refresh.\"))?;\n""",
    """        let write_txn = if maintenance_state() == MaintenanceState::Recovering {\n            with_maintenance_write_bypass(idms.proxy_write(ct)).await\n        } else {\n            idms.proxy_write(ct).await\n        };\n\n        write_txn\n            .and_then(|mut write_txn| {\n                write_txn\n                    .qs_write\n                    .consumer_apply_refresh(refresh)\n                    .and_then(|cs| write_txn.commit().map(|()| cs))\n            })\n            .map_err(|err| error!(?err, \"Consumer was not able to apply refresh.\"))?;\n""",
)
replace(
    "server/core/src/repl/mod.rs",
    """        match idms.proxy_write(ct).await.and_then(|mut write_txn| {\n            write_txn\n                .qs_write\n                .consumer_apply_changes(changes)\n                .and_then(|cs| write_txn.commit().map(|()| cs))\n        }) {\n""",
    """        let write_txn = if maintenance_state() == MaintenanceState::Recovering {\n            with_maintenance_write_bypass(idms.proxy_write(ct)).await\n        } else {\n            idms.proxy_write(ct).await\n        };\n\n        match write_txn.and_then(|mut write_txn| {\n            write_txn\n                .qs_write\n                .consumer_apply_changes(changes)\n                .and_then(|cs| write_txn.commit().map(|()| cs))\n        }) {\n""",
)
replace(
    "server/core/src/repl/mod.rs",
    """            _ = repl_interval.tick() => {\n                // Interval passed, attempt a replication run.\n                repl_run_consumer(\n""",
    """            _ = repl_interval.tick() => {\n                // A drained/fenced node remains a replication supplier, but its\n                // consumer must not start new apply work. Recovering is the sole\n                // non-serving state where replication writes receive the scoped\n                // maintenance bypass.\n                if !matches!(\n                    maintenance_state(),\n                    MaintenanceState::Serving | MaintenanceState::Recovering\n                ) {\n                    debug!(\"Skipping replication consumer while node is maintenance-fenced\");\n                    continue;\n                }\n\n                // Interval passed, attempt a replication run.\n                repl_run_consumer(\n""",
)

# Public read-only state/capability surface.
replace(
    "server/core/src/https/generic.rs",
    """use kubidmd_lib::maintenance::maintenance_public_status;\n""",
    """use kubidmd_lib::maintenance::{maintenance_public_status, MaintenancePublicStatus};\n""",
)
replace(
    "server/core/src/https/generic.rs",
    """pub async fn healthz(State(state): State<ServerState>) -> Json<LivenessStatus> {\n    state.status_ref.get_liveness_status().into()\n}\n\n#[utoipa::path(\n    get,\n    path = \"/readyz\",\n""",
    """pub async fn healthz(State(state): State<ServerState>) -> Json<LivenessStatus> {\n    state.status_ref.get_liveness_status().into()\n}\n\n#[utoipa::path(\n    get,\n    path = \"/maintenance\",\n    responses(\n        (status = 200, description = \"Node maintenance state and capabilities\", content_type = APPLICATION_JSON, body=MaintenancePublicStatus),\n    ),\n    tag = \"system\",\n    operation_id = \"maintenance_status\"\n)]\n/// Read-only node-local maintenance state and capability discovery.\npub async fn maintenance_status() -> Json<MaintenancePublicStatus> {\n    maintenance_public_status().into()\n}\n\n#[utoipa::path(\n    get,\n    path = \"/readyz\",\n""",
)
replace(
    "server/core/src/https/mod.rs",
    """        .route(\"/status\", get(generic::status))\n        .route(\"/healthz\", get(generic::healthz))\n        .route(\"/readyz\", get(generic::readyz))\n""",
    """        .route(\"/status\", get(generic::status))\n        .route(\"/healthz\", get(generic::healthz))\n        .route(\"/maintenance\", get(generic::maintenance_status))\n        .route(\"/readyz\", get(generic::readyz))\n""",
)
replace(
    "server/core/src/https/apidocs/mod.rs",
    """        super::generic::status,\n        super::generic::robots_txt,\n""",
    """        super::generic::status,\n        super::generic::maintenance_status,\n        super::generic::robots_txt,\n""",
)

# Public endpoint integration contract.
replace(
    "server/testkit/tests/testkit/health_endpoints.rs",
    """#[kubidmd_testkit::test]\nasync fn test_readyz_endpoint_after_startup(rsclient: &kubidm_client::KubidmClient) {\n""",
    """#[kubidmd_testkit::test]\nasync fn test_maintenance_endpoint_after_startup(rsclient: &kubidm_client::KubidmClient) {\n    let client = rsclient.client();\n\n    let res = client\n        .get(rsclient.make_url(\"/maintenance\"))\n        .send()\n        .await\n        .expect(\"Failed to send request\");\n\n    assert_eq!(res.status(), 200);\n\n    let body: serde_json::Value = res.json().await.expect(\"Failed to parse JSON\");\n    assert_eq!(body[\"state\"], \"serving\");\n    assert!(body[\"active_operation_id\"].is_null());\n    assert_eq!(body[\"capabilities\"][\"api_version\"], \"v1\");\n    assert_eq!(body[\"capabilities\"][\"drain\"], true);\n    assert_eq!(body[\"capabilities\"][\"replication_fence\"], true);\n    assert_eq!(body[\"capabilities\"][\"sync_until\"], true);\n    assert_eq!(body[\"capabilities\"][\"reindex\"], true);\n    assert_eq!(body[\"capabilities\"][\"verify\"], true);\n    assert_eq!(body[\"capabilities\"][\"vacuum\"], false);\n    assert_eq!(body[\"capabilities\"][\"restore\"], false);\n}\n\n#[kubidmd_testkit::test]\nasync fn test_readyz_endpoint_after_startup(rsclient: &kubidm_client::KubidmClient) {\n""",
)

# Local admin wire contract.
admin = Path("server/core/src/admin.rs")
admin_text = admin.read_text()
if "mod maintenance_protocol_tests" in admin_text:
    raise SystemExit("maintenance protocol tests already exist")
admin.write_text(
    admin_text
    + r'''

#[cfg(test)]
mod maintenance_protocol_tests {
    use super::*;

    #[test]
    fn maintenance_request_json_round_trip_preserves_operation_id() {
        let operation_id = Uuid::new_v4();
        let encoded = serde_json::to_vec(&AdminTaskRequest::MaintenanceDrain { operation_id })
            .expect("maintenance request should serialize");
        let decoded: AdminTaskRequest =
            serde_json::from_slice(&encoded).expect("maintenance request should deserialize");

        match decoded {
            AdminTaskRequest::MaintenanceDrain {
                operation_id: decoded_id,
            } => assert_eq!(decoded_id, operation_id),
            other => panic!("unexpected decoded request: {other:?}"),
        }
    }

    #[test]
    fn maintenance_run_result_json_is_structured() {
        let operation_id = Uuid::new_v4();
        let response = AdminTaskResponse::MaintenanceRun {
            result: MaintenanceRunResult {
                operation_id,
                operation: MaintenanceOperation::Verify,
                success: true,
                verification_errors: Vec::new(),
                fence: None,
                error: None,
            },
        };

        let value = serde_json::to_value(response).expect("maintenance response should serialize");
        assert_eq!(
            value["MaintenanceRun"]["result"]["operation_id"],
            operation_id.to_string()
        );
        assert_eq!(value["MaintenanceRun"]["result"]["success"], true);
    }
}
'''
)

# Final implementation note without rewriting the first ADR commit.
adr = Path("book/src/developers/designs/node_drain_replication_fences_maintenance.md")
adr_text = adr.read_text()
if "## Implementation refinement" not in adr_text:
    adr.write_text(
        adr_text
        + r'''

## Implementation refinement

The implementation keeps the safety invariants above but deliberately uses a smaller
lock/control surface than the initial plan described.

- `QueryServer::write` is the hard admission boundary. It rejects ordinary writes in every
  non-`Serving` maintenance state, both before waiting for and after acquiring the writer
  permit. The second check closes the race with writers that entered the semaphore queue just
  before a drain began.
- Privileged maintenance work uses a Tokio task-local bypass. Because the bypass is scoped to
  one directly-awaited future, it cannot admit unrelated client/background writers in another
  task.
- The replication consumer is idle while a node is `Draining`, `Fenced`, `Maintenance`, or
  `Failed`. In `Recovering` only replication apply receives the task-local bypass; ordinary
  writes remain rejected while the node catches up to the supplied fence.
- This makes explicit `ReplCtrl` pause/resume messages unnecessary for the first implementation:
  write admission itself provides the pause acknowledgement once the maintenance fence owns the
  writer permit.
- `sync-until` currently observes the existing periodic incremental replication loop instead of
  forcing a new replication tick. This can add up to one configured replication interval of
  latency but does not weaken the fence proof. An immediate-sync control is an optimisation, not
  a correctness requirement.

The original plan remains in the first commit for design history; this section records the final
mechanism implemented by the branch.
'''
    )
