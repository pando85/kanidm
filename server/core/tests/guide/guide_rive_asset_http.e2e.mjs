import { expect, test } from "@playwright/test";

test("production .riv is served as a non-empty same-origin binary asset", async ({ request }) => {
    test.skip(process.env.KUBIDM_EXPECT_REAL_RIVE !== "1", "Run after kubidm-guide.riv is exported");

    const response = await request.get("/pkg/img/guide/kubidm-guide.riv");
    expect(response.ok()).toBe(true);

    const contentType = response.headers()["content-type"] || "";
    expect(contentType).toMatch(/^application\//i);
    expect(contentType).not.toContain("text/html");

    const bytes = await response.body();
    expect(bytes.length).toBeGreaterThan(0);
});
