import { MascotState, Severity } from "./guide_contract.mjs";

const scene = document.querySelector('[data-guide-scene="credentials"]');

if (scene) {
    const stateNode = scene.querySelector("[data-guide-credential-state]");

    function hasPendingChanges(dynamicSection) {
        const discard = dynamicSection?.querySelector('[hx-post="/ui/api/cu_cancel"]');
        return Boolean(discard && !discard.disabled);
    }

    function setState({ action, mascotState, severity }) {
        if (!stateNode) return;
        stateNode.dataset.guideAction = action;
        stateNode.dataset.guideState = mascotState;
        stateNode.dataset.guideSeverity = severity;
    }

    function syncCredentialGuide() {
        const dynamicSection = scene.querySelector("#credentialUpdateDynamicSection");
        if (!dynamicSection || !stateNode) return;

        // These Bootstrap alerts are rendered from server-provided CURegWarning
        // values. JavaScript only translates their already-authoritative severity
        // into guide posture; it never decides whether policy is satisfied.
        if (dynamicSection.querySelector(".alert-danger")) {
            setState({
                action: "credential_policy_conflict",
                mascotState: MascotState.WARNING,
                severity: Severity.CRITICAL,
            });
            return;
        }

        if (dynamicSection.querySelector(".alert-warning")) {
            setState({
                action: "credential_attention_required",
                mascotState: MascotState.PROTECT,
                severity: Severity.CAUTION,
            });
            return;
        }

        if (hasPendingChanges(dynamicSection)) {
            setState({
                action: "credential_changes_pending",
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            });
            return;
        }

        setState({
            action: "credential_setup",
            mascotState: MascotState.IDLE,
            severity: Severity.NEUTRAL,
        });
    }

    document.body.addEventListener("htmx:afterSettle", syncCredentialGuide);
    syncCredentialGuide();
}
