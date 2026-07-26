export const Recommendation = Object.freeze({
    NONE: "none",
    REQUIRED: "required",
    RECOMMENDED: "recommended",
    WORKS_OK: "works_ok",
    OPTIONAL: "optional",
});

export const Severity = Object.freeze({
    NEUTRAL: "neutral",
    POSITIVE: "positive",
    CAUTION: "caution",
    CRITICAL: "critical",
});

export const MascotState = Object.freeze({
    IDLE: "idle",
    WELCOME: "welcome",
    GUIDE: "guide",
    PROTECT: "protect",
    WORKING: "working",
    SUCCESS: "success",
    WARNING: "warning",
    GOODBYE: "goodbye",
});

export const MotionLevel = Object.freeze({
    FULL: "full",
    REDUCED: "reduced",
    STATIC: "static",
});

export const JourneyStage = Object.freeze({
    MEET: "meet",
    IDENTIFY: "identify",
    CHOOSE: "choose",
    LEARN: "learn",
    CONFIGURE: "configure",
    CONFIRM: "confirm",
    RESILIENCE: "resilience",
    RECOVERY: "recovery",
    COMPLETE: "complete",
});

const allowed = Object.freeze({
    recommendation: new Set(Object.values(Recommendation)),
    severity: new Set(Object.values(Severity)),
    mascotState: new Set(Object.values(MascotState)),
    motionLevel: new Set(Object.values(MotionLevel)),
    journeyStage: new Set(Object.values(JourneyStage)),
});

export function isGuideValue(kind, value) {
    return allowed[kind]?.has(value) ?? false;
}

export function assertGuideValue(kind, value) {
    if (!isGuideValue(kind, value)) {
        throw new TypeError(`Invalid Kubidm guide ${kind}: ${String(value)}`);
    }
    return value;
}

export function normaliseGuideState(input = {}) {
    const state = {
        productState: String(input.productState || "unknown"),
        recommendation: input.recommendation || Recommendation.NONE,
        severity: input.severity || Severity.NEUTRAL,
        mascotState: input.mascotState || MascotState.IDLE,
        motionLevel: input.motionLevel || MotionLevel.FULL,
        journeyStage: input.journeyStage || null,
    };

    assertGuideValue("recommendation", state.recommendation);
    assertGuideValue("severity", state.severity);
    assertGuideValue("mascotState", state.mascotState);
    assertGuideValue("motionLevel", state.motionLevel);
    if (state.journeyStage !== null) {
        assertGuideValue("journeyStage", state.journeyStage);
    }

    return Object.freeze(state);
}

export function recommendationLabel(value) {
    assertGuideValue("recommendation", value);
    return {
        [Recommendation.NONE]: "",
        [Recommendation.REQUIRED]: "Required",
        [Recommendation.RECOMMENDED]: "Recommended",
        [Recommendation.WORKS_OK]: "Works OK",
        [Recommendation.OPTIONAL]: "Optional",
    }[value];
}

export function mascotStateForSeverity({ mascotState, severity }) {
    assertGuideValue("mascotState", mascotState);
    assertGuideValue("severity", severity);

    if (severity === Severity.CRITICAL) return MascotState.WARNING;
    if (severity === Severity.CAUTION && mascotState === MascotState.SUCCESS) {
        return MascotState.PROTECT;
    }
    return mascotState;
}
