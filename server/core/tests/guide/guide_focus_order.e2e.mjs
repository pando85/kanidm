import { expect, test } from "@playwright/test";

async function taskTabOrder(page, url, waitForReady) {
    await page.goto(url);
    await waitForReady(page);

    const username = page.locator("#ui-lab-canvas input").first();
    await username.focus();
    const order = [];
    for (let index = 0; index < 3; index += 1) {
        order.push(
            await page.evaluate(() => {
                const active = document.activeElement;
                return {
                    tag: active?.tagName || null,
                    type: active?.getAttribute?.("type") || null,
                    text: active?.textContent?.trim() || null,
                };
            }),
        );
        await page.keyboard.press("Tab");
    }
    return order;
}

test("task keyboard focus order is identical for Rive, failure fallback and static mode", async ({ page }) => {
    const base = "/ui/_lab";
    const rive = await taskTabOrder(
        page,
        `${base}?rive=mock#story=first-login&theme=light&viewport=desktop&motion=full`,
        (current) => current.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true),
    );
    const failure = await taskTabOrder(
        page,
        `${base}?rive=mock-fail#story=first-login&theme=light&viewport=desktop&motion=full`,
        (current) => current.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.fallbackActive === true),
    );
    const staticMode = await taskTabOrder(
        page,
        `${base}?rive=mock#story=first-login&theme=light&viewport=desktop&motion=static`,
        async (current) => {
            await expect(current.locator("[data-lab-mascot-image]")).toBeVisible();
        },
    );

    expect(failure).toEqual(rive);
    expect(staticMode).toEqual(rive);
    expect(rive.map((entry) => entry.tag)).toEqual(["INPUT", "INPUT", "BUTTON"]);
});
