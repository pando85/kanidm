import { expect, test } from "@playwright/test";

const idmAdminPassword = process.env.KUBIDM_E2E_IDM_ADMIN_PASSWORD;

async function loginAsIdmAdmin(page) {
    await page.goto("/ui/login");
    await expect(page.locator("#username")).toBeVisible();
    await page.locator("#username").fill("idm_admin");
    await page.locator('form[action="/ui/login/begin"] button[type="submit"]').click();

    const passwordChoice = page.locator(
        'form[action="/ui/login/mech_choose"]:has(input[name="mech"][value="password"]) button[type="submit"]',
    );
    if (await passwordChoice.isVisible().catch(() => false)) {
        await passwordChoice.click();
    }

    const passwordInput = page.locator('form[action="/ui/login/pw"] #password');
    await expect(passwordInput).toBeVisible();
    await passwordInput.fill(idmAdminPassword);

    const signedIn = page.waitForURL((url) => !url.pathname.startsWith("/ui/login"), { timeout: 15_000 });
    await page.locator('form[action="/ui/login/pw"] button[type="submit"]').click();
    await signedIn;
}

async function clickSettingsAndWaitForHtmx(page, label) {
    const settled = page.evaluate(
        () =>
            new Promise((resolve) => {
                document.body.addEventListener("htmx:afterSettle", () => resolve(true), { once: true });
            }),
    );
    await page.getByRole("link", { name: label, exact: true }).click();
    await settled;
}

async function assertStableGuide(page, expectedAction) {
    await expect(page.locator('main[data-guide-scene="settings"]')).toHaveCount(1);
    await expect(page.locator("[data-guide-slot]")).toHaveCount(1);
    await expect(page.locator('main[data-guide-scene="settings"] [data-guide-state]').first()).toHaveAttribute(
        "data-guide-action",
        expectedAction,
    );

    // Full motion may still be waiting on or falling back from the production Rive asset,
    // but HTMX replacement must never leave multiple live canvases in the document.
    expect(await page.locator("[data-guide-rive-canvas]").count()).toBeLessThanOrEqual(1);
}

test("Profile and Credentials survive 20 HTMX cycles without guide DOM leaks", async ({ page }) => {
    test.skip(!idmAdminPassword, "requires the deterministic CI idm_admin fixture");
    test.setTimeout(120_000);

    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));

    await loginAsIdmAdmin(page);
    await page.goto("/ui/profile");
    await assertStableGuide(page, /profile_(readonly|edit)/);

    for (let cycle = 0; cycle < 20; cycle += 1) {
        await clickSettingsAndWaitForHtmx(page, "Credentials");
        await assertStableGuide(page, "credential_setup");

        await clickSettingsAndWaitForHtmx(page, "Profile");
        await assertStableGuide(page, /profile_(readonly|edit)/);
    }

    expect(pageErrors, `unexpected page errors after HTMX churn: ${pageErrors.join(" | ")}`).toEqual([]);
});
