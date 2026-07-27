import { expect, test } from "@playwright/test";

function labUrl({
    story = "first-login",
    motion = "full",
    rive = "mock",
    theme = "light",
    viewport = "desktop",
} = {}) {
    const query = rive ? `?rive=${rive}` : "";
    return `/ui/_lab${query}#story=${story}&theme=${theme}&viewport=${viewport}&motion=${motion}`;
}

async function waitForRive(page) {
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true);
    return page.evaluate(() => globalThis.__kubidmGuideDiagnostics);
}

async function waitForFallback(page) {
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.fallbackActive === true);
    return page.evaluate(() => globalThis.__kubidmGuideDiagnostics);
}

function requireRealRive() {
    test.skip(process.env.KUBIDM_EXPECT_REAL_RIVE !== "1", "Run after kubidm-guide.riv is exported");
}

test("self-hosted Rive runtime artifacts are served locally", async ({ request }) => {
    const javascript = await request.get("/pkg/rive/rive.js");
    expect(javascript.ok()).toBe(true);
    expect(javascript.headers()["content-type"]).toContain("javascript");

    const wasm = await request.get("/pkg/rive/rive.wasm");
    expect(wasm.ok()).toBe(true);
    expect(wasm.headers()["content-type"]).toContain("application/wasm");

    const contract = await request.get("/pkg/guide_rive_contract.json");
    expect(contract.ok()).toBe(true);
    expect((await contract.json()).viewModel).toBe("GuideState");
});

test("full motion uses the production Rive renderer contract", async ({ page }) => {
    await page.goto(labUrl({ story: "method-choice" }));
    const diagnostics = await waitForRive(page);

    expect(diagnostics.renderer).toBe("rive");
    expect(diagnostics.mockRuntime).toBe(true);
    expect(diagnostics.artboard).toBe("KubidmGuide");
    expect(diagnostics.stateMachine).toBe("ProductGuide");
    expect(diagnostics.viewModel).toBe("GuideState");
    expect(diagnostics.semanticState).toBe("guide");
    expect(diagnostics.riveState).toBe("guide");
    expect(diagnostics.fallbackActive).toBe(false);
    await expect(page.locator("[data-guide-rive-canvas]")).toBeVisible();
    await expect(page.locator("[data-lab-mascot-image]")).toBeHidden();
});

test("static and reduced modes never instantiate full Rive motion", async ({ page }) => {
    for (const motion of ["static", "reduced"]) {
        await page.goto(labUrl({ story: "method-choice", motion }));
        await expect(page.locator("[data-lab-mascot-image]")).toBeVisible();
        await expect(page.locator("[data-guide-rive-canvas]")).toHaveCount(0);
        const created = await page.evaluate(() => globalThis.__kubidmMockRiveStats?.created || 0);
        expect(created).toBe(0);
    }
});

test("Rive load failure degrades to static artwork and leaves UI usable", async ({ page }) => {
    await page.goto(labUrl({ story: "first-login", rive: "mock-fail" }));
    const diagnostics = await waitForFallback(page);
    expect(diagnostics.renderer).toBe("static");
    expect(diagnostics.loaded).toBe(false);
    expect(diagnostics.lastError).toContain("Injected mock Rive load failure");
    await expect(page.locator("[data-lab-mascot-image]")).toBeVisible();
    await expect(page.getByRole("button", { name: "Continue" })).toBeEnabled();

    const stats = await page.evaluate(() => globalThis.__kubidmMockRiveStats);
    expect(stats.active).toBe(0);
    expect(stats.created).toBe(stats.cleaned);
});

test("100 story transitions keep at most one Rive instance alive", async ({ page }) => {
    await page.goto(labUrl({ story: "first-login" }));
    await waitForRive(page);

    const expectedState = {
        "method-choice": "guide",
        "passkey-story": "guide",
        "passkey-working": "working",
        success: "success",
        "policy-required": "protect",
        returning: "idle",
        goodbye: "goodbye",
    };
    const sequence = Object.keys(expectedState);
    for (let index = 0; index < 100; index += 1) {
        const story = sequence[index % sequence.length];
        await page.locator(`[data-story="${story}"]`).click();
        await page.waitForFunction(
            (expected) => document.querySelector("#ui-lab-mascot-state")?.textContent === expected,
            expectedState[story],
        );
    }

    await page.waitForTimeout(0);
    const stats = await page.evaluate(() => globalThis.__kubidmMockRiveStats);
    expect(stats.active).toBeLessThanOrEqual(1);
    expect(stats.created - stats.cleaned).toBe(stats.active);
});

test("semantic and Rive states agree across representative scenarios", async ({ page }) => {
    for (const story of [
        "first-login",
        "method-choice",
        "passkey-working",
        "success",
        "webauthn-cancel",
        "policy-required",
        "goodbye",
        "applications-arrival",
    ]) {
        await page.goto(labUrl({ story }));
        const diagnostics = await waitForRive(page);
        const semantic = await page.locator("#ui-lab-mascot-state").textContent();
        expect(diagnostics.semanticState).toBe(semantic);
        expect(diagnostics.riveState).toBe(semantic);
    }
});

test("real Rive asset satisfies the runtime contract when required", async ({ page }) => {
    requireRealRive();
    const external = [];
    page.on("request", (request) => {
        const url = request.url();
        if (/^https?:/.test(url) && new URL(url).origin !== new URL(page.url() || "https://localhost:8443").origin) {
            external.push(url);
        }
    });
    await page.goto(labUrl({ story: "method-choice", rive: null }));
    const diagnostics = await waitForRive(page);
    expect(diagnostics.renderer).toBe("rive");
    expect(diagnostics.mockRuntime).not.toBe(true);
    expect(diagnostics.fallbackActive).toBe(false);
    expect(diagnostics.artboard).toBe("KubidmGuide");
    expect(diagnostics.stateMachine).toBe("ProductGuide");
    expect(diagnostics.viewModel).toBe("GuideState");
    expect(external).toEqual([]);
});

test("real .riv 404 falls back without blocking the task", async ({ page }) => {
    requireRealRive();
    await page.route("**/pkg/img/guide/kubidm-guide.riv", (route) =>
        route.fulfill({ status: 404, contentType: "application/octet-stream", body: "" }),
    );
    await page.goto(labUrl({ story: "first-login", rive: null }));
    const diagnostics = await waitForFallback(page);
    expect(diagnostics.renderer).toBe("static");
    await expect(page.locator("[data-lab-mascot-image]")).toBeVisible();
    await expect(page.getByRole("button", { name: "Continue" })).toBeEnabled();
});

test("real WASM failure falls back without blocking the task", async ({ page }) => {
    requireRealRive();
    await page.route("**/pkg/rive/rive.wasm", (route) => route.abort("failed"));
    await page.goto(labUrl({ story: "first-login", rive: null }));
    const diagnostics = await waitForFallback(page);
    expect(diagnostics.renderer).toBe("static");
    await expect(page.locator("[data-lab-mascot-image]")).toBeVisible();
    await expect(page.getByRole("button", { name: "Continue" })).toBeEnabled();
});
