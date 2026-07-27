import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
    MascotState,
    MotionLevel,
    Severity,
    TravelDirection,
} from "../../static/modules/guide_contract.mjs";
import { createMockRiveRuntime } from "../../static/modules/guide_rive_mock.mjs";
import {
    loadGuideRiveRuntime,
    resetGuideRiveRuntimeForTests,
    validateGuideRiveContract,
} from "../../static/modules/guide_rive_runtime.mjs";

const contract = JSON.parse(
    await readFile(new URL("../../static/guide_rive_contract.json", import.meta.url), "utf8"),
);
const runtimeVersion = JSON.parse(
    await readFile(new URL("../../static/rive/VERSION.json", import.meta.url), "utf8"),
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
    assert.deepEqual(
        sorted(contract.properties.travelDirection.values),
        sorted(Object.values(TravelDirection)),
    );

    for (const trigger of ["attention", "successSmall", "successMajor", "goodbye"]) {
        assert.equal(contract.properties[trigger].type, "trigger");
    }
});

test("machine contract and vendored Rive runtime cannot drift", () => {
    assert.equal(contract.runtime.package, runtimeVersion.package);
    assert.equal(contract.runtime.version, runtimeVersion.version);
    assert.equal(contract.runtime.selfHosted, true);
    assert.equal(contract.runtime.cdnAllowed, false);
    assert.equal(contract.runtime.javascript, "/pkg/rive/rive.js");
    assert.equal(contract.runtime.wasm, "/pkg/rive/rive.wasm");
    assert.equal(contract.rendererPolicy.full, "rive");
    assert.equal(contract.rendererPolicy.reduced, "static");
    assert.equal(contract.rendererPolicy.static, "static");
    assert.equal(contract.verification.minimumVisualScore, 4);
});

test("runtime loader disables Rive public WASM fallback", async () => {
    delete globalThis.__kubidmMockRiveStats;
    resetGuideRiveRuntimeForTests();
    const runtime = createMockRiveRuntime();
    globalThis.__kubidmRiveRuntimeOverride = runtime;
    try {
        await loadGuideRiveRuntime();
        assert.equal(globalThis.__kubidmMockRiveStats.wasmUrl, "/pkg/rive/rive.wasm");
        assert.equal(globalThis.__kubidmMockRiveStats.wasmFallbackUrl, null);
    } finally {
        delete globalThis.__kubidmRiveRuntimeOverride;
        resetGuideRiveRuntimeForTests();
    }
});

test("mock runtime satisfies the same Data Binding contract", async () => {
    delete globalThis.__kubidmMockRiveStats;
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
