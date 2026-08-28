import { MascotState } from "./guide_contract.mjs";

const MAJOR_SUCCESS_STATES = new Set([
    "authentication_confirmed",
    "recommended_setup_complete",
    "credential_update_complete",
]);

function clampLook(value) {
    const number = Number(value ?? 0);
    return Number.isFinite(number) ? Math.max(-1, Math.min(1, number)) : 0;
}

export function guideRiveBindingValues(state) {
    return Object.freeze({
        state: state.mascotState,
        motion: state.motionLevel,
        severity: state.severity,
        travelDirection: state.travelDirection === "left" ? "left" : "right",
        lookX: clampLook(state.lookX),
        lookY: clampLook(state.lookY),
    });
}

export function guideRiveTriggers(previousMascotState, state) {
    if (previousMascotState === state.mascotState) return Object.freeze([]);

    const triggers = [];
    if (state.mascotState === MascotState.GUIDE) triggers.push("attention");
    if (state.mascotState === MascotState.SUCCESS) {
        triggers.push(MAJOR_SUCCESS_STATES.has(state.productState) ? "successMajor" : "successSmall");
    }
    if (state.mascotState === MascotState.GOODBYE) triggers.push("goodbye");
    return Object.freeze(triggers);
}
