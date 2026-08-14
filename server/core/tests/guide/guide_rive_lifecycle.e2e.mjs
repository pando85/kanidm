import { expect, test } from "@playwright/test";

async function stats(page) {
    return page.evaluate(() => globalThis.__kubidmMockRiveStats);
}

async function waitForActiveCount(page, active) {
    await page.waitForFunction((expected) => globalThis.__kubidmMockRiveStats?.active === expected, active);
}

test("full to static to full cleans and recreates Rive from the cached file", async ({ page }) => {
    await page.goto("/ui/_lab?rive=mock#story=method-choice&theme=light&viewport=desktop&motion=full");
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true);
    await expect(page.locator("[data-guide-rive-canvas]")).toBeVisible();

    const initial = await stats(page);
    expect(initial.created).toBe(1);
    expect(initial.active).toBe(1);
    expect(initial.fileCreated).toBe(1);
    expect(initial.fileInits).toBe(1);
    expect(initial.resizes).toBeGreaterThan(0);

    await page.locator("#ui-lab-motion").selectOption("static");
    await waitForActiveCount(page, 0);
    await expect(page.locator("[data-guide-rive-canvas]")).toHaveCount(0);
    await expect(page.locator("[data-lab-mascot-image]")).toBeVisible();

    const stopped = await stats(page);
    expect(stopped.cleaned).toBe(1);
    expect(stopped.fileCreated).toBe(1);
    expect(stopped.fileInits).toBe(1);

    await page.locator("#ui-lab-motion").selectOption("full");
    await waitForActiveCount(page, 1);
    await expect(page.locator("[data-guide-rive-canvas]")).toBeVisible();
    await expect(page.locator("[data-lab-mascot-image]")).toBeHidden();

    const restarted = await stats(page);
    expect(restarted.created).toBe(2);
    expect(restarted.cleaned).toBe(1);
    expect(restarted.active).toBe(1);
    expect(restarted.fileCreated).toBe(1);
    expect(restarted.fileInits).toBe(1);
    expect(restarted.usedRiveFile).toBe(true);
    expect(restarted.resizes).toBeGreaterThan(initial.resizes);
});
