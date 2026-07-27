import assert from "node:assert/strict";
import test from "node:test";

import {
    MascotState,
    MotionLevel,
    Severity,
    normaliseGuideState,
} from "../../static/modules/guide_contract.mjs";
import {
    guideRiveBindingValues,
    guideRiveTriggers,
} from "../../static/modules/guide_rive_binding.mjs";

test("Rive binding maps bounded semantic state", () => {
    const state = normaliseGuideState({
        productState: "choose_method",
        mascotState: MascotState.GUIDE,
        motionLevel: MotionLevel.FULL,
        severity: Severity.NEUTRAL,
    });
    assert.deepEqual(guideRiveBindingValues(state), {
        state: "guide",
        motion: "full",
        severity: "neutral",
        travelDirection: "right",
        lookX: 0,
        lookY: 0,
    });
});

test("Rive gaze is clamped and travel direction fails safe", () => {
    const values = guideRiveBindingValues({
        mascotState: "travel",
        motionLevel: "full",
        severity: "neutral",
        travelDirection: "left",
        lookX: 4,
        lookY: -9,
    });
    assert.equal(values.travelDirection, "left");
    assert.equal(values.lookX, 1);
    assert.equal(values.lookY, -1);

    assert.equal(
        guideRiveBindingValues({
            mascotState: "idle",
            motionLevel: "static",
            severity: "neutral",
            travelDirection: "invalid",
        }).travelDirection,
        "right",
    );
});

test("Rive trigger policy fires only on state entry", () => {
    assert.deepEqual(
        guideRiveTriggers("idle", {
            mascotState: "guide",
            productState: "choose_method",
        }),
        ["attention"],
    );
    assert.deepEqual(
        guideRiveTriggers("guide", {
            mascotState: "guide",
            productState: "choose_method",
        }),
        [],
    );
    assert.deepEqual(
        guideRiveTriggers("working", {
            mascotState: "success",
            productState: "authentication_confirmed",
        }),
        ["successMajor"],
    );
    assert.deepEqual(
        guideRiveTriggers("idle", {
            mascotState: "success",
            productState: "profile_saved",
        }),
        ["successSmall"],
    );
    assert.deepEqual(
        guideRiveTriggers("idle", {
            mascotState: "goodbye",
            productState: "logout",
        }),
        ["goodbye"],
    );
});
