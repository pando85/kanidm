import { expect, test } from "@playwright/test";

const fixtureUsername = process.env.KUBIDM_E2E_TEST_USERNAME || "guide_e2e_user";
const fixturePassword = process.env.KUBIDM_E2E_TEST_PASSWORD;
// Each settled HTMX response may expose any credential state that the server policy derives.
const credentialAction = /^credential_(setup|attention_required|policy_conflict)$/;

test.describe.configure({ retries: 0 });

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

async function clickSettings(page, label) {
    await page.locator('main[data-guide-scene="settings"]').getByRole("link", { name: label, exact: true }).click();
}

async function assertStableGuide(page, expectedAction) {
    await expect(page.locator('main[data-guide-scene="settings"]')).toHaveCount(1);
    await expect(page.locator("[data-guide-slot] ")).toHaveCount(1);
    await expect(page.locator('main[data-guide-scene="settings"] [data-guide-state]').first()).toHaveAttribute(
        "data-guide-action",
        expectedAction,
    );

    // The destination semantic marker above is emitted by the settled HTMX response.
    // At that point replacement must have destroyed any previous renderer instance.
    expect(await page.locator("[data-guide-rive-canvas]").count()).toBeLessThanOrEqual(1);
}

test("Profile and Credentials survive 20 HTMX cycles without guide DOM leaks", async ({ page, browserName }) => {
    test.skip(browserName !== "chromium", "the authenticated HTMX fixture is intentionally exercised once in Chromium");
    test.skip(!fixturePassword, "requires the deterministic normal-person password provisioned by CI");
    test.setTimeout(120_000);

    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error.message));

    await loginAsFixture(page);
    await page.goto("/ui/profile");
    await assertStableGuide(page, /profile_(readonly|edit)/);

    for (let cycle = 0; cycle < 20; cycle += 1) {
        await clickSettings(page, "Credentials");
        await assertStableGuide(page, credentialAction);

        await clickSettings(page, "Profile");
        await assertStableGuide(page, /profile_(readonly|edit)/);
    }

    expect(pageErrors, `unexpected page errors after HTMX churn: ${pageErrors.join(" | ")}`).toEqual([]);
});
