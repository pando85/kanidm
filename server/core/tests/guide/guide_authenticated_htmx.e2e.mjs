import { expect, test } from "@playwright/test";

const fixtureUsername = process.env.KUBIDM_E2E_TEST_USERNAME || "guide_e2e_user";
const fixturePassword = process.env.KUBIDM_E2E_TEST_PASSWORD;
const fixtureResetToken = process.env.KUBIDM_E2E_TEST_RESET_TOKEN;

async function onboardPassword(page) {
    await page.goto(`/ui/reset?token=${encodeURIComponent(fixtureResetToken)}`);
    await expect(page.getByRole("heading", { name: "Updating Credentials" })).toBeVisible();

    await page.getByRole("button", { name: "Add Password" }).click();
    await expect(page.locator("#new-password")).toBeVisible();
    await page.locator("#new-password").fill(fixturePassword);
    await page.locator("#new-password-check").fill(fixturePassword);
    await page.locator("#password-submit").click();

    const saveChanges = page.getByRole("button", { name: "Save Changes" });
    await expect(saveChanges).toBeEnabled();
    await saveChanges.click();
    await page.waitForURL((url) => url.pathname.startsWith("/ui/login"), { timeout: 15_000 });
}

async function loginAsFixture(page) {
    await page.goto("/ui/login");
    await expect(page.locator("#username")).toBeVisible();
    await page.locator("#username").fill(fixtureUsername);
    await page.locator('form[action="/ui/login/begin"] button[type="submit"]').click();

    const passwordChoice = page.locator(
        'form[action="/ui/login/mech_choose"]:has(input[name="mech"][value="password"]) button[type="submit"]',
    );
    if (await passwordChoice.isVisible().catch(() => false)) {
        await passwordChoice.click();
    }

    const passwordInput = page.locator('form[action="/ui/login/pw"] #password');
    await expect(passwordInput).toBeVisible();
    await passwordInput.fill(fixturePassword);

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

test("Profile and Credentials survive 20 HTMX cycles without guide DOM leaks", async ({ page, browserName }) => {
    test.skip(browserName !== "chromium", "one-use onboarding fixture is intentionally exercised once in Chromium");
    test.skip(
        !fixturePassword || !fixtureResetToken,
        "requires the deterministic normal-person onboarding fixture from CI",
    );
    test.setTimeout(120_000);

    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));

    await onboardPassword(page);
    await loginAsFixture(page);
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
