import { expect, test } from "@playwright/test";

const profiles = [
    ["desktop", { width: 1440, height: 900 }],
    ["tablet", { width: 820, height: 1180 }],
    ["mobile", { width: 390, height: 844 }],
];

function intersects(a, b) {
    return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}

for (const [name, viewport] of profiles) {
    test(`${name} guide layout has no horizontal overflow or actionable overlap`, async ({ page }) => {
        await page.setViewportSize(viewport);
        await page.goto(`/ui/_lab?rive=mock#story=first-login&theme=light&viewport=${name}&motion=full`);
        await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true);

        const overflow = await page.evaluate(() => ({
            document: document.documentElement.scrollWidth - document.documentElement.clientWidth,
            body: document.body.scrollWidth - document.body.clientWidth,
        }));
        expect(overflow.document).toBeLessThanOrEqual(1);
        expect(overflow.body).toBeLessThanOrEqual(1);

        const mascot = await page.locator(".ui-lab-mascot-slot").boundingBox();
        expect(mascot).not.toBeNull();
        const actions = page.locator(
            "#ui-lab-canvas button, #ui-lab-canvas input, #ui-lab-canvas select, #ui-lab-canvas a[href]",
        );
        for (let index = 0; index < (await actions.count()); index += 1) {
            const action = actions.nth(index);
            if (!(await action.isVisible())) continue;
            const box = await action.boundingBox();
            if (!box) continue;
            expect(intersects(mascot, box), `mascot overlaps actionable control ${index} at ${name}`).toBe(false);
        }

        const riveCanvas = page.locator("[data-guide-rive-canvas]");
        await expect(riveCanvas).toHaveAttribute("aria-hidden", "true");
        await expect(riveCanvas).not.toHaveAttribute("tabindex", /.+/);
    });
}
