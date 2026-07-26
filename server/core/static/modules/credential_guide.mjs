import { MascotState, Severity } from "./guide_contract.mjs";

const scene = document.querySelector('[data-guide-scene="credentials"]');

if (scene) {
    const stateNode = scene.querySelector("[data-guide-credential-state]");
    const dialog = scene.querySelector("[data-guide-credential-dialog]");
    const title = scene.querySelector("[data-guide-credential-title]");
    const message = scene.querySelector("[data-guide-credential-message]");

    function hasPendingChanges(dynamicSection) {
        const discard = dynamicSection?.querySelector('[hx-post="/ui/api/cu_cancel"]');
        return Boolean(discard && !discard.disabled);
    }

    function hasConfiguredPasskey(dynamicSection) {
        return Boolean(dynamicSection?.querySelector('[hx-post="/ui/api/remove_passkey"]'));
    }

    function hasVisiblePrimaryCredential(dynamicSection) {
        if (!dynamicSection) return false;

        if (dynamicSection.querySelector('[hx-post="/ui/api/delete_alt_creds"]')) {
            return true;
        }

        return [...dynamicSection.querySelectorAll("h6")].some(
            (heading) => heading.textContent.trim() === "Password",
        );
    }

    function milestone(name) {
        return stateNode?.querySelector(`[data-guide-milestone="${name}"]`) || null;
    }

    function setMilestone(name, complete, detail) {
        const item = milestone(name);
        if (!item) return;

        item.dataset.complete = String(Boolean(complete));
        const detailNode = item.querySelector("[data-guide-milestone-detail]");
        if (detailNode && detail) detailNode.textContent = detail;
    }

    function syncMilestones(dynamicSection) {
        const hasPasskey = hasConfiguredPasskey(dynamicSection);
        const hasPrimary = hasVisiblePrimaryCredential(dynamicSection);
        const hasWarnings = Boolean(dynamicSection.querySelector(".alert-warning, .alert-danger"));
        const pending = hasPendingChanges(dynamicSection);

        setMilestone(
            "sign-in-method",
            hasPasskey || hasPrimary,
            hasPasskey || hasPrimary
                ? "A configured sign-in method is visible in this editor."
                : "No configured sign-in method is currently visible in this editor.",
        );
        setMilestone(
            "passkey",
            hasPasskey,
            hasPasskey
                ? "At least one passkey is configured."
                : "No passkey is currently visible here. This is optional unless policy says otherwise.",
        );
        setMilestone(
            "policy",
            !hasWarnings,
            hasWarnings
                ? "Review the authoritative warning below."
                : "The editor currently reports no unresolved policy warning.",
        );
        setMilestone(
            "saved",
            !pending,
            pending
                ? "There are pending edits that have not been committed yet."
                : "The editor currently reports no pending changes.",
        );
    }

    function setDialog({ variant = "orient", heading, text }) {
        if (dialog) dialog.dataset.variant = variant;
        if (title) title.textContent = heading;
        if (message) message.textContent = text;
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

        syncMilestones(dynamicSection);

        // These Bootstrap alerts are rendered from server-provided CURegWarning
        // values. JavaScript only translates their already-authoritative severity
        // into guide posture; it never decides whether policy is satisfied.
        if (dynamicSection.querySelector(".alert-danger")) {
            setDialog({
                heading: "Policy needs attention",
                text: "The credential editor reports a blocking policy conflict. Follow the authoritative warning below before trying to save.",
            });
            setState({
                action: "credential_policy_conflict",
                mascotState: MascotState.WARNING,
                severity: Severity.CRITICAL,
            });
            return;
        }

        if (dynamicSection.querySelector(".alert-warning")) {
            setDialog({
                variant: "suggest",
                heading: "A requirement needs attention",
                text: "Start with the warning below. It comes from your account policy, and the editor will tell you when the requirement is satisfied.",
            });
            setState({
                action: "credential_attention_required",
                mascotState: MascotState.PROTECT,
                severity: Severity.CAUTION,
            });
            return;
        }

        if (hasPendingChanges(dynamicSection)) {
            setDialog({
                variant: "suggest",
                heading: "Changes ready to review",
                text: "You have unsaved credential changes. Review them, then use Save Changes when you are ready.",
            });
            setState({
                action: "credential_changes_pending",
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            });
            return;
        }

        setDialog({
            heading: "Your sign-in setup",
            text: "I’ll put required policy first, explain unfamiliar choices, and keep optional improvements optional. The credential editor below remains the source of truth.",
        });
        setState({
            action: "credential_setup",
            mascotState: MascotState.IDLE,
            severity: Severity.NEUTRAL,
        });
    }

    document.body.addEventListener("htmx:afterSettle", syncCredentialGuide);
    syncCredentialGuide();
}
