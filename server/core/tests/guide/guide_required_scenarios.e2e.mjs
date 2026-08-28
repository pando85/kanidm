import { expect, test } from "@playwright/test";
import { readFile } from "node:fs/promises";

const contract = JSON.parse(await readFile(new URL("../../static/guide_rive_contract.json", import.meta.url), "utf8"));
const required = [
    ["first_encounter", "first-login", "identify", "welcome"],
    ["method_choice", "method-choice", "choose_method", "guide"],
    ["passkey_teaching", "passkey-story", "teach_passkey", "guide"],
    ["webauthn_pending", "passkey-working", "webauthn_pending", "working"],
    ["confirmed_success", "success", "authentication_confirmed", "success"],
    ["applications_travel", "applications-arrival", "applications_arrival", "travel"],
    ["applications_idle", "applications", "applications", "idle"],
    ["password_works_ok", "password-ok", "password_selected", "guide"],
    ["webauthn_cancelled", "webauthn-cancel", "webauthn_interrupted", "guide"],
    ["oauth_context", "oauth", "oauth_context", "guide"],
    ["reauthentication", "reauth", "reauthentication", "protect"],
    ["policy_required", "policy-required", "policy_required", "protect"],
    ["returning_user", "returning", "normal_login", "idle"],
    ["credential_progress", "credentials-progress", "credential_setup", "idle"],
    ["logout_goodbye", "goodbye", "logout", "goodbye"],
];

test("browser scenario matrix exactly matches the machine contract", () => {
    expect(required.map(([scenario]) => scenario)).toEqual(contract.requiredScenarios);
});

test("every required product scenario reaches the declared semantic and Rive state", async ({ page }) => {
    test.setTimeout(60_000);

    for (const [contractScenario, story, productState, mascotState] of required) {
        await page.goto(`/ui/_lab?rive=mock#story=${story}&theme=light&viewport=desktop&motion=full`);
        await expect(page.locator("#ui-lab-product-state"), contractScenario).toHaveText(productState);
        await expect(page.locator("#ui-lab-mascot-state"), contractScenario).toHaveText(mascotState);
        await page.waitForFunction(
            ([expectedProduct, expectedMascot]) => {
                const diagnostic = globalThis.__kubidmGuideDiagnostics;
                return (
                    diagnostic?.loaded === true &&
                    diagnostic?.productState === expectedProduct &&
                    diagnostic?.semanticState === expectedMascot &&
                    diagnostic?.riveState === expectedMascot
                );
            },
            [productState, mascotState],
        );
    }
});
