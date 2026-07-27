import { expect, test } from "@playwright/test";

function labUrl({ story = "first-login", motion = "full", rive = "mock", theme = "light", viewport = "desktop" } = {}) {
    return `/ui/_lab?rive=${rive}#story=${story}&theme=${theme}&viewport=${viewport}&motion=${motion}`;
}

async function waitForRive(page) {
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true);
    return page.evaluate(() => globalThis.__kubidmGuideDiagnostics);
}

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

test("Rive load failure degrades to static artwork without leaking an instance", async ({ page }) => {
    await page.goto(labUrl({ story: "policy-required", rive: "mock-fail" }));
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.fallbackActive === true);

    const diagnostics = await page.evaluate(() => globalThis.__kubidmGuideDiagnostics);
    expect(diagnostics.renderer).toBe("static");
    expect(diagnostics.loaded).toBe(false);
    expect(diagnostics.lastError).toContain("Injected mock Rive load failure");
    await expect(page.locator("[data-lab-mascot-image]")).toBeVisible();

    const stats = await page.evaluate(() => globalThis.__kubidmMockRiveStats);
    expect(stats.active).toBe(0);
    expect(stats.created).toBe(stats.cleaned);
});

test("100 story transitions keep at most one Rive instance alive", async ({ page }) => {
    await page.goto(labUrl({ story: "first-login" }));
    await waitForRive(page);

    const sequence = ["method-choice", "passkey-story", "passkey-working", "success", "policy-required", "returning"];
    for (let index = 0; index < 100; index += 1) {
        const story = sequence[index % sequence.length];
        await page.locator(`[data-story="${story}"]`).click();
        await page.waitForFunction(
            (expected) => document.querySelector("#ui-lab-mascot-state")?.textContent === expected,
            ({ "method-choice": "guide", "passkey-story": "guide", "passkey-working": "working", success: "success", "policy-required": "protect", returning: "idle" })[story],
        );
    }

    await page.waitForTimeout(0);
    const stats = await page.evaluate(() => globalThis.__kubidmMockRiveStats);
    expect(stats.active).toBeLessThanOrEqual(1);
    expect(stats.created - stats.cleaned).toBe(stats.active);
});

test("semantic and Rive states agree across representative scenarios", async ({ page }) => {
    for (const story of ["first-login", "method-choice", "passkey-working", "success", "webauthn-cancel", "policy-required"]) {
        await page.goto(labUrl({ story }));
        const diagnostics = await waitForRive(page);
        const semantic = await page.locator("#ui-lab-mascot-state").textContent();
        expect(diagnostics.semanticState).toBe(semantic);
        expect(diagnostics.riveState).toBe(semantic);
    }
});
