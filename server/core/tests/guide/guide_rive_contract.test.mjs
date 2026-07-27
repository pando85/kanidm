import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
    MascotState,
    MotionLevel,
    Severity,
} from "../../static/modules/guide_contract.mjs";
import { createMockRiveRuntime } from "../../static/modules/guide_rive_mock.mjs";
import { validateGuideRiveContract } from "../../static/modules/guide_rive_runtime.mjs";

const contract = JSON.parse(
    await readFile(new URL("../../static/guide_rive_contract.json", import.meta.url), "utf8"),
);

function sorted(values) {
    return [...values].sort();
}

test("machine-readable Rive contract matches product semantic enums", () => {
    assert.equal(contract.artboard, "KubidmGuide");
    assert.equal(contract.stateMachine, "ProductGuide");
    assert.equal(contract.viewModel, "GuideState");
    assert.deepEqual(sorted(contract.properties.state.values), sorted(Object.values(MascotState)));
    assert.deepEqual(sorted(contract.properties.motion.values), sorted(Object.values(MotionLevel)));
    assert.deepEqual(sorted(contract.properties.severity.values), sorted(Object.values(Severity)));
    assert.deepEqual(sorted(contract.properties.travelDirection.values), ["left", "right"]);

    for (const trigger of ["attention", "successSmall", "successMajor", "goodbye"]) {
        assert.equal(contract.properties[trigger].type, "trigger");
    }
});

test("mock runtime satisfies the same Data Binding contract", async () => {
    const runtime = createMockRiveRuntime();
    const instance = await new Promise((resolve) => {
        let riveInstance;
        riveInstance = new runtime.Rive({ onLoad: () => resolve(riveInstance) });
    });

    const validated = validateGuideRiveContract(instance, contract);
    assert.equal(validated.instance.viewModelName, "GuideState");

    validated.instance.enum("state").value = "travel";
    validated.instance.enum("motion").value = "full";
    validated.instance.number("lookX").value = 0.5;
    validated.instance.trigger("attention").trigger();

    assert.equal(validated.instance.enum("state").value, "travel");
    assert.equal(validated.instance.enum("motion").value, "full");
    assert.equal(validated.instance.number("lookX").value, 0.5);
    assert.deepEqual(globalThis.__kubidmMockRiveStats.triggers.slice(-1), ["attention"]);
    instance.cleanup();
});
