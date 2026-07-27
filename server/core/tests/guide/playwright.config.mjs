import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.KUBIDM_UI_BASE_URL || "https://localhost:8443";

export default defineConfig({
    testDir: ".",
    testMatch: /guide_rive\.e2e\.mjs$/,
    timeout: 30_000,
    expect: { timeout: 5_000 },
    fullyParallel: false,
    retries: process.env.CI ? 1 : 0,
    reporter: process.env.CI ? [["line"], ["html", { open: "never", outputFolder: "playwright-report" }]] : "line",
    use: {
        baseURL,
        ignoreHTTPSErrors: true,
        trace: "retain-on-failure",
        screenshot: "only-on-failure",
    },
    projects: [
        { name: "chromium", use: { ...devices["Desktop Chrome"] } },
        { name: "firefox", use: { ...devices["Desktop Firefox"] } },
        { name: "webkit", use: { ...devices["Desktop Safari"] } },
    ],
});
