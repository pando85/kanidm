import { MascotState, Severity } from "./guide_contract.mjs";
import { consumeConfirmedAuthenticationArrival } from "./guide_handoff.mjs";

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
let arrivalConsumed = false;
let timers = [];

function clearTimers() {
    for (const timer of timers) window.clearTimeout(timer);
    timers = [];
}

function setSceneState(
    scene,
    { action, mascotState, severity = Severity.NEUTRAL, travelDirection = "right", lookX = 0, lookY = 0 },
) {
    scene.dataset.guideAction = action;
    scene.dataset.guideState = mascotState;
    scene.dataset.guideSeverity = severity;
    scene.dataset.guideTravelDirection = travelDirection;
    scene.dataset.guideLookX = String(lookX);
    scene.dataset.guideLookY = String(lookY);
}

function settle(scene) {
    setSceneState(scene, {
        action: "applications",
        mascotState: MascotState.IDLE,
        severity: Severity.NEUTRAL,
        lookX: 0.15,
    });
}

function playConfirmedArrival(scene) {
    clearTimers();
    setSceneState(scene, {
        action: "authentication_confirmed",
        mascotState: MascotState.SUCCESS,
        severity: Severity.POSITIVE,
        lookX: 0,
    });

    if (reducedMotion.matches) {
        timers.push(window.setTimeout(() => settle(scene), 350));
        return;
    }

    timers.push(
        window.setTimeout(() => {
            setSceneState(scene, {
                action: "applications_arrival",
                mascotState: MascotState.TRAVEL,
                severity: Severity.NEUTRAL,
                travelDirection: "right",
                lookX: 0.85,
            });
        }, 650),
    );
    timers.push(window.setTimeout(() => settle(scene), 1550));
}

function syncApplicationsScene() {
    const scene = document.querySelector('[data-guide-scene="applications"]');
    if (!scene) {
        clearTimers();
        return;
    }

    if (!arrivalConsumed && consumeConfirmedAuthenticationArrival()) {
        arrivalConsumed = true;
        playConfirmedArrival(scene);
        return;
    }

    if (!timers.length) settle(scene);
}

document.body.addEventListener("htmx:afterSettle", syncApplicationsScene);
reducedMotion.addEventListener("change", () => {
    const scene = document.querySelector('[data-guide-scene="applications"]');
    if (!scene) return;
    clearTimers();
    settle(scene);
});

syncApplicationsScene();
