import { expect, test } from "@playwright/test";

test("guided login remains interactive while the Rive asset is still pending", async ({ page }) => {
    let releaseAsset;
    const assetMayFinish = new Promise((resolve) => {
        releaseAsset = resolve;
    });

    await page.route("**/pkg/img/guide/kubidm-guide.riv", async (route) => {
        await assetMayFinish;
        await route.fulfill({ status: 404, contentType: "application/octet-stream", body: "" });
    });

    try {
        await page.goto("/ui/login", { waitUntil: "domcontentloaded" });
        await expect(page.locator("#username")).toBeEditable({ timeout: 1_500 });
        await expect(page.locator('form[action="/ui/login/begin"] button[type="submit"]')).toBeEnabled({
            timeout: 1_500,
        });
    } finally {
        releaseAsset();
    }
});
