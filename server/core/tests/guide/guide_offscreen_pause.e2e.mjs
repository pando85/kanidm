import { expect, test } from "@playwright/test";

test("Rive pauses offscreen and resumes when the guide becomes visible", async ({ page }) => {
    await page.goto("/ui/_lab?rive=mock#story=returning&theme=light&viewport=desktop&motion=full");
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true);
    await expect(page.locator("[data-guide-rive-canvas]")).toBeVisible();

    const before = await page.evaluate(() => ({
        plays: globalThis.__kubidmMockRiveStats.plays,
        pauses: globalThis.__kubidmMockRiveStats.pauses,
    }));

    await page.locator(".ui-lab-preview-stage").evaluate((node) => {
        node.style.display = "none";
    });
    await page.waitForFunction((baseline) => globalThis.__kubidmMockRiveStats.pauses > baseline, before.pauses);

    await page.locator(".ui-lab-preview-stage").evaluate((node) => {
        node.style.display = "";
    });
    await expect(page.locator("[data-guide-rive-canvas]")).toBeVisible();
    await page.waitForFunction((baseline) => globalThis.__kubidmMockRiveStats.plays > baseline, before.plays);
});
