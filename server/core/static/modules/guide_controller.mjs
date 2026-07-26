import {
    MascotState,
    MotionLevel,
    Recommendation,
    Severity,
    normaliseGuideState,
} from "./guide_contract.mjs";
import {
    clearAuthenticationAttempt,
    markAuthenticationAttempt,
} from "./guide_handoff.mjs";
import { createGuideRenderer } from "./guide_renderer.mjs";

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const AUTH_CREDENTIAL_PATHS = new Set([
    "/ui/login/begin",
    "/ui/login/passkey",
    "/ui/login/seckey",
    "/ui/login/pw",
    "/ui/login/totp",
    "/ui/login/backup_code",
]);

let sceneRoot = null;
let renderer = null;
let sceneObserver = null;

function currentMotionLevel() {
    const explicit = sceneRoot?.dataset.guideMotion;
    if (Object.values(MotionLevel).includes(explicit)) return explicit;
    return reducedMotion.matches ? MotionLevel.REDUCED : MotionLevel.FULL;
}

function semanticNode() {
    return sceneRoot?.querySelector("[data-guide-state]") || sceneRoot;
}

function statusNode() {
    return sceneRoot?.querySelector("[data-guide-status]") || null;
}

function setStatus(text, severity = Severity.NEUTRAL) {
    const node = statusNode();
    if (!node) return;
    node.textContent = text;
    node.dataset.severity = severity;
}

function tracksApplicationsArrival() {
    return sceneRoot?.dataset.guideScene === "auth" && sceneRoot.dataset.guideAuthArrival === "applications";
}

function readState(overrides = {}) {
    if (!sceneRoot) return null;

    const node = semanticNode();
    return normaliseGuideState({
        productState: node?.dataset.guideAction || sceneRoot.dataset.guideScene || "unknown",
        recommendation: node?.dataset.guideRecommendation || Recommendation.NONE,
        severity: node?.dataset.guideSeverity || Severity.NEUTRAL,
        mascotState: node?.dataset.guideState || MascotState.IDLE,
        motionLevel: currentMotionLevel(),
        ...overrides,
    });
}

function ensureRenderer() {
    if (!sceneRoot) return null;

    const slot = sceneRoot.querySelector("[data-guide-slot]");
    if (!slot) {
        renderer?.destroy();
        renderer = null;
        return null;
    }

    if (!renderer || renderer.slot !== slot) {
        renderer?.destroy();
        renderer = createGuideRenderer(slot, { renderer: "static" });
    }
    return renderer;
}

function clearDocumentState() {
    delete document.documentElement.dataset.guideScene;
    delete document.documentElement.dataset.guideState;
    delete document.documentElement.dataset.guideSeverity;
    delete document.documentElement.dataset.guideMotion;
}

function publish(overrides = {}) {
    const state = readState(overrides);
    if (!state || !sceneRoot) return null;

    if (state.productState === "authentication_denied" || state.productState === "identify") {
        clearAuthenticationAttempt();
    }

    document.documentElement.dataset.guideScene = sceneRoot.dataset.guideScene || "unknown";
    document.documentElement.dataset.guideState = state.mascotState;
    document.documentElement.dataset.guideSeverity = state.severity;
    document.documentElement.dataset.guideMotion = state.motionLevel;

    ensureRenderer()?.setState(state);
    window.dispatchEvent(new CustomEvent("kubidm:guide-state", { detail: state }));
    return state;
}

function observeScene() {
    sceneObserver?.disconnect();
    sceneObserver = null;

    if (!sceneRoot) return;

    sceneObserver = new MutationObserver(() => publish());
    sceneObserver.observe(sceneRoot, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: [
            "data-guide-state",
            "data-guide-action",
            "data-guide-recommendation",
            "data-guide-severity",
            "data-guide-motion",
        ],
    });
}

function syncScene() {
    const nextScene = document.querySelector("[data-guide-scene]");
    if (nextScene === sceneRoot) {
        publish();
        return;
    }

    sceneObserver?.disconnect();
    renderer?.destroy();
    renderer = null;
    sceneRoot = nextScene;

    if (!sceneRoot) {
        clearDocumentState();
        return;
    }

    // A reauthentication or OAuth flow may use the same credential endpoints as
    // normal login, but it must never create an Applications arrival celebration.
    if (sceneRoot.dataset.guideScene === "auth" && !tracksApplicationsArrival()) {
        clearAuthenticationAttempt();
    }

    observeScene();
    publish();
}

function maybeMarkFormAuthentication(form) {
    if (!(form instanceof HTMLFormElement) || !tracksApplicationsArrival()) return;

    let path;
    try {
        path = new URL(form.action, window.location.href).pathname;
    } catch {
        return;
    }

    if (AUTH_CREDENTIAL_PATHS.has(path)) {
        markAuthenticationAttempt();
    }
}

window.addEventListener("kubidm:webauthn-start", () => {
    syncScene();
    setStatus("Waiting for your browser or device…");
    publish({
        productState: "webauthn_pending",
        mascotState: MascotState.WORKING,
        severity: Severity.NEUTRAL,
    });
});

window.addEventListener("kubidm:webauthn-submit", () => {
    // The browser produced an assertion, but the server still has to validate
    // it. Remain in Working rather than showing a success state.
    if (tracksApplicationsArrival()) markAuthenticationAttempt();
    syncScene();
    setStatus("Checking your identity…");
    publish({
        productState: "webauthn_submitting",
        mascotState: MascotState.WORKING,
        severity: Severity.NEUTRAL,
    });
});

window.addEventListener("kubidm:webauthn-cancelled", () => {
    // NotAllowedError may mean cancellation, timeout, or no available
    // credential. Return to a neutral actionable posture without guessing.
    clearAuthenticationAttempt();
    syncScene();
    setStatus("That request did not complete. You can try again when you are ready.");
    publish({
        productState: "webauthn_interrupted",
        mascotState: MascotState.GUIDE,
        severity: Severity.NEUTRAL,
    });
});

window.addEventListener("kubidm:webauthn-error", () => {
    clearAuthenticationAttempt();
    syncScene();
    setStatus("The browser could not complete this request. Try again.", Severity.CAUTION);
    publish({
        productState: "webauthn_error",
        mascotState: MascotState.WARNING,
        severity: Severity.CAUTION,
    });
});

document.body.addEventListener(
    "submit",
    (event) => {
        maybeMarkFormAuthentication(event.target);
    },
    true,
);

document.body.addEventListener("htmx:beforeRequest", () => {
    publish({ mascotState: MascotState.WORKING });
});

document.body.addEventListener("htmx:afterSettle", () => syncScene());
document.body.addEventListener("htmx:responseError", () => {
    syncScene();
    setStatus("The request could not be completed. Try again.", Severity.CAUTION);
    publish({ mascotState: MascotState.WARNING, severity: Severity.CAUTION });
});

reducedMotion.addEventListener("change", () => publish());
syncScene();
