import {
    JourneyStage,
    MascotState,
    Recommendation,
    Severity,
    normaliseGuideState,
} from "./guide_contract.mjs";

function step(story, stage, state) {
    return Object.freeze({
        story,
        stage,
        state: normaliseGuideState({ journeyStage: stage, ...state }),
    });
}

export const ScenarioId = Object.freeze({
    PASSKEY_FIRST_RUN: "passkey-first-run",
    PASSWORD_ALTERNATIVE: "password-alternative",
    RETURNING_USER: "returning-user",
    WEBAUTHN_CANCEL: "webauthn-cancel",
    POLICY_REQUIRED: "policy-required",
});

const applicationsArrival = () =>
    step("applications-arrival", JourneyStage.CONFIRM, {
        productState: "applications_arrival",
        recommendation: Recommendation.NONE,
        mascotState: MascotState.TRAVEL,
        severity: Severity.NEUTRAL,
    });

const applicationsSettled = () =>
    step("applications", JourneyStage.CONFIRM, {
        productState: "applications",
        recommendation: Recommendation.NONE,
        mascotState: MascotState.IDLE,
        severity: Severity.NEUTRAL,
    });

export const scenarios = Object.freeze({
    [ScenarioId.PASSKEY_FIRST_RUN]: Object.freeze({
        title: "Scenario A — new user, passkey recommended",
        description: "First-run teaching path through confirmed sign-in, Applications arrival, then optional resilience guidance.",
        steps: Object.freeze([
            step("first-login", JourneyStage.IDENTIFY, {
                productState: "identify",
                recommendation: Recommendation.NONE,
                mascotState: MascotState.WELCOME,
                severity: Severity.NEUTRAL,
            }),
            step("method-choice", JourneyStage.CHOOSE, {
                productState: "choose_method",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            }),
            step("passkey-story", JourneyStage.LEARN, {
                productState: "teach_passkey",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            }),
            step("passkey-working", JourneyStage.CONFIGURE, {
                productState: "webauthn_pending",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.WORKING,
                severity: Severity.NEUTRAL,
            }),
            step("success", JourneyStage.CONFIRM, {
                productState: "authentication_confirmed",
                recommendation: Recommendation.NONE,
                mascotState: MascotState.SUCCESS,
                severity: Severity.POSITIVE,
            }),
            applicationsArrival(),
            applicationsSettled(),
            step("resilience", JourneyStage.RESILIENCE, {
                productState: "resilience_available",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            }),
            step("credentials-progress", JourneyStage.RECOVERY, {
                productState: "credential_setup",
                recommendation: Recommendation.OPTIONAL,
                mascotState: MascotState.IDLE,
                severity: Severity.NEUTRAL,
            }),
            step("complete", JourneyStage.COMPLETE, {
                productState: "recommended_setup_complete",
                recommendation: Recommendation.NONE,
                mascotState: MascotState.SUCCESS,
                severity: Severity.POSITIVE,
            }),
        ]),
    }),
    [ScenarioId.PASSWORD_ALTERNATIVE]: Object.freeze({
        title: "Scenario B — valid password alternative",
        description: "Shows that a valid non-preferred choice is accepted without warning or shame.",
        steps: Object.freeze([
            step("first-login", JourneyStage.IDENTIFY, {
                productState: "identify",
                recommendation: Recommendation.NONE,
                mascotState: MascotState.WELCOME,
                severity: Severity.NEUTRAL,
            }),
            step("method-choice", JourneyStage.CHOOSE, {
                productState: "choose_method",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            }),
            step("password-ok", JourneyStage.CONFIGURE, {
                productState: "password_selected",
                recommendation: Recommendation.WORKS_OK,
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            }),
            step("success", JourneyStage.CONFIRM, {
                productState: "authentication_confirmed",
                recommendation: Recommendation.NONE,
                mascotState: MascotState.SUCCESS,
                severity: Severity.POSITIVE,
            }),
            applicationsArrival(),
            applicationsSettled(),
        ]),
    }),
    [ScenarioId.RETURNING_USER]: Object.freeze({
        title: "Scenario C — returning configured user",
        description: "Teaching has decayed and the normal authentication path stays quiet.",
        steps: Object.freeze([
            step("returning", JourneyStage.CHOOSE, {
                productState: "normal_login",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.IDLE,
                severity: Severity.NEUTRAL,
            }),
            step("passkey-working", JourneyStage.CONFIGURE, {
                productState: "webauthn_pending",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.WORKING,
                severity: Severity.NEUTRAL,
            }),
            step("success", JourneyStage.CONFIRM, {
                productState: "authentication_confirmed",
                recommendation: Recommendation.NONE,
                mascotState: MascotState.SUCCESS,
                severity: Severity.POSITIVE,
            }),
            applicationsArrival(),
            applicationsSettled(),
        ]),
    }),
    [ScenarioId.WEBAUTHN_CANCEL]: Object.freeze({
        title: "Scenario D — WebAuthn cancellation",
        description: "Cancellation is neutral and returns the user to an actionable state.",
        steps: Object.freeze([
            step("passkey-working", JourneyStage.CONFIGURE, {
                productState: "webauthn_pending",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.WORKING,
                severity: Severity.NEUTRAL,
            }),
            step("webauthn-cancel", JourneyStage.CHOOSE, {
                productState: "webauthn_cancelled",
                recommendation: Recommendation.RECOMMENDED,
                mascotState: MascotState.GUIDE,
                severity: Severity.NEUTRAL,
            }),
        ]),
    }),
    [ScenarioId.POLICY_REQUIRED]: Object.freeze({
        title: "Scenario E — policy-required action",
        description: "Authoritative policy takes priority and mascot behaviour becomes restrained.",
        steps: Object.freeze([
            step("policy-required", JourneyStage.CONFIGURE, {
                productState: "policy_required",
                recommendation: Recommendation.REQUIRED,
                mascotState: MascotState.PROTECT,
                severity: Severity.CAUTION,
            }),
            step("passkey-story", JourneyStage.LEARN, {
                productState: "teach_required_passkey",
                recommendation: Recommendation.REQUIRED,
                mascotState: MascotState.PROTECT,
                severity: Severity.CAUTION,
            }),
        ]),
    }),
});

export function scenarioById(id) {
    return scenarios[id] || scenarios[ScenarioId.PASSKEY_FIRST_RUN];
}

export function stepIndexForStory(scenario, story) {
    return scenario.steps.findIndex((entry) => entry.story === story);
}
