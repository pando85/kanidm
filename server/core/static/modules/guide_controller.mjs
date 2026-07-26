import {
    MascotState,
    MotionLevel,
    Recommendation,
    Severity,
    normaliseGuideState,
} from "./guide_contract.mjs";
import { createGuideRenderer } from "./guide_renderer.mjs";

const sceneRoot = document.querySelector("[data-guide-scene]");

if (sceneRoot) {
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let renderer = null;

    function currentMotionLevel() {
        const explicit = sceneRoot.dataset.guideMotion;
        if (Object.values(MotionLevel).includes(explicit)) return explicit;
        return reducedMotion.matches ? MotionLevel.REDUCED : MotionLevel.FULL;
    }

    function semanticNode() {
        return sceneRoot.querySelector("[data-guide-state]") || sceneRoot;
    }

    function readState(overrides = {}) {
        const node = semanticNode();
        return normaliseGuideState({
            productState: node.dataset.guideAction || sceneRoot.dataset.guideScene || "unknown",
            recommendation: node.dataset.guideRecommendation || Recommendation.NONE,
            severity: node.dataset.guideSeverity || Severity.NEUTRAL,
            mascotState: node.dataset.guideState || MascotState.IDLE,
            motionLevel: currentMotionLevel(),
            ...overrides,
        });
    }

    function ensureRenderer() {
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

    function publish(overrides = {}) {
        const state = readState(overrides);
        document.documentElement.dataset.guideScene = sceneRoot.dataset.guideScene || "unknown";
        document.documentElement.dataset.guideState = state.mascotState;
        document.documentElement.dataset.guideSeverity = state.severity;
        document.documentElement.dataset.guideMotion = state.motionLevel;

        ensureRenderer()?.setState(state);
        window.dispatchEvent(new CustomEvent("kubidm:guide-state", { detail: state }));
        return state;
    }

    window.addEventListener("kubidm:webauthn-start", () => {
        publish({
            productState: "webauthn_pending",
            mascotState: MascotState.WORKING,
            severity: Severity.NEUTRAL,
        });
    });

    window.addEventListener("kubidm:webauthn-submit", () => {
        // The browser produced an assertion, but the server still has to validate
        // it. Remain in Working rather than showing a success state.
        publish({
            productState: "webauthn_submitting",
            mascotState: MascotState.WORKING,
            severity: Severity.NEUTRAL,
        });
    });

    window.addEventListener("kubidm:webauthn-cancelled", () => {
        // NotAllowedError may mean cancellation, timeout, or no available
        // credential. Return to a neutral actionable posture without guessing.
        publish({
            productState: "webauthn_interrupted",
            mascotState: MascotState.GUIDE,
            severity: Severity.NEUTRAL,
        });
    });

    window.addEventListener("kubidm:webauthn-error", () => {
        publish({
            productState: "webauthn_error",
            mascotState: MascotState.WARNING,
            severity: Severity.CAUTION,
        });
    });

    document.body.addEventListener("htmx:beforeRequest", () => {
        publish({ mascotState: MascotState.WORKING });
    });

    document.body.addEventListener("htmx:afterSettle", () => publish());
    document.body.addEventListener("htmx:responseError", () => {
        publish({ mascotState: MascotState.WARNING, severity: Severity.CAUTION });
    });

    new MutationObserver(() => publish()).observe(sceneRoot, {
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

    reducedMotion.addEventListener("change", () => publish());
    publish();
}
