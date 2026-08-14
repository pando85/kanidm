import { expect, test } from "@playwright/test";

const fixtureUsername = process.env.KUBIDM_E2E_TEST_USERNAME || "guide_e2e_user";
const fixturePassword = process.env.KUBIDM_E2E_TEST_PASSWORD;
const fixtureResetToken = process.env.KUBIDM_E2E_TEST_RESET_TOKEN;

test.describe.configure({ retries: 0 });

async function waitForHtmxSettled(page) {
    await expect(page.locator(".htmx-settling, .htmx-request")).toHaveCount(0);
}

async function onboardPassword(page) {
    await page.goto(`/ui/reset?token=${encodeURIComponent(fixtureResetToken)}`);
    await expect(page.getByRole("heading", { name: "Updating Credentials" })).toBeVisible();

    const passwordFormResponse = page.waitForResponse(
        (response) =>
            response.request().method() === "POST" && new URL(response.url()).pathname === "/ui/reset/add_password",
    );
    await page.getByRole("button", { name: "Add Password" }).click();
    const response = await passwordFormResponse;
    expect(response.ok()).toBe(true);
    await page.waitForFunction(() => document.querySelector("#new-password") instanceof HTMLInputElement);

    const passwordInput = page.locator("#new-password");
    await waitForHtmxSettled(page);
    await passwordInput.fill(fixturePassword);
    await page.locator("#new-password-check").fill(fixturePassword);

    const passwordSubmitResponse = page.waitForResponse(
        (candidate) =>
            candidate.request().method() === "POST" && new URL(candidate.url()).pathname === "/ui/reset/add_password",
    );
    await page.locator("#password-submit").click();
    expect((await passwordSubmitResponse).ok()).toBe(true);

    const saveChanges = page.getByRole("button", { name: "Save Changes" });
    await page.waitForFunction(() => !document.querySelector("#new-password"));
    await waitForHtmxSettled(page);
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

async function clickSettings(page, label) {
    await page.getByRole("link", { name: label, exact: true }).click();
}

async function assertStableGuide(page, expectedAction) {
    await expect(page.locator('main[data-guide-scene="settings"]')).toHaveCount(1);
    await expect(page.locator("[data-guide-slot]")).toHaveCount(1);
    await expect(page.locator('main[data-guide-scene="settings"] [data-guide-state]').first()).toHaveAttribute(
        "data-guide-action",
        expectedAction,
    );
    await waitForHtmxSettled(page);

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
        await clickSettings(page, "Credentials");
        await assertStableGuide(page, "credential_setup");

        await clickSettings(page, "Profile");
        await assertStableGuide(page, /profile_(readonly|edit)/);
    }

    expect(pageErrors, `unexpected page errors after HTMX churn: ${pageErrors.join(" | ")}`).toEqual([]);
});
