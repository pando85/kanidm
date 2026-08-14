import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const baseURL = process.env.KUBIDM_UI_BASE_URL || "https://localhost:8443";
const baseOrigin = new URL(baseURL).origin;
const mode = process.env.KUBIDM_RIVE_TEST_MODE || "real";
const fullMatrix = process.env.KUBIDM_GUIDE_FULL_MATRIX === "1";
const commit =
    process.env.GUIDE_REVIEW_COMMIT || execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const outputRoot = path.resolve("artifacts", "guide-review", commit);

const stories = [
    "returning",
    "first-login",
    "method-choice",
    "reauth",
    "passkey-working",
    "success",
    "component-notice",
    "goodbye",
    "applications-arrival",
];
const themes = fullMatrix ? ["light", "dark"] : ["light"];
const viewports = fullMatrix
    ? [
          ["desktop", { width: 1440, height: 900 }],
          ["tablet", { width: 820, height: 1180 }],
          ["mobile", { width: 390, height: 844 }],
      ]
    : [
          ["desktop", { width: 1440, height: 900 }],
          ["mobile", { width: 390, height: 844 }],
      ];
const motions = fullMatrix ? ["full", "reduced", "static"] : ["full"];

async function sha256IfPresent(filename) {
    try {
        const bytes = await readFile(filename);
        return createHash("sha256").update(bytes).digest("hex");
    } catch {
        return null;
    }
}

async function jsonIfPresent(filename) {
    try {
        return JSON.parse(await readFile(filename, "utf8"));
    } catch {
        return null;
    }
}

function urlFor(story, theme, viewport, motion) {
    const rive = mode === "mock" ? "?rive=mock" : "";
    return `${baseURL}/ui/_lab${rive}#story=${story}&theme=${theme}&viewport=${viewport}&motion=${motion}`;
}

await mkdir(outputRoot, { recursive: true });
const browser = await chromium.launch();
const runtimeVersion = await jsonIfPresent(path.resolve("static", "rive", "VERSION.json"));
const manifest = {
    schemaVersion: 1,
    commit,
    generatedAt: new Date().toISOString(),
    baseURL,
    mode,
    fullMatrix,
    browser: {
        name: "chromium",
        version: browser.version(),
    },
    rivSha256: await sha256IfPresent(path.resolve("static", "img", "guide", "kubidm-guide.riv")),
    runtime: runtimeVersion,
    review: {
        prompt: "tests/guide/visual_review_prompt.md",
        schema: "tests/guide/visual_review.schema.json",
        validator: "tests/guide/validate_visual_review.mjs",
    },
    captures: [],
};
const allSemantic = [];
const allConsole = [];
const allNetwork = [];
const externalRequests = [];

for (const [viewportName, viewportSize] of viewports) {
    for (const theme of themes) {
        for (const motion of motions) {
            for (const story of stories) {
                const artifactDir = path.join(outputRoot, `${story}-${viewportName}-${theme}-${motion}`);
                await mkdir(artifactDir, { recursive: true });
                const context = await browser.newContext({
                    ignoreHTTPSErrors: true,
                    viewport: viewportSize,
                    recordVideo:
                        ["method-choice", "success", "applications-arrival"].includes(story) && motion === "full"
                            ? { dir: artifactDir }
                            : undefined,
                });
                const page = await context.newPage();
                await page.addInitScript(() => {
                    globalThis.__kubidmEvidenceTrace = [];
                    window.addEventListener("kubidm:guide-state", (event) => {
                        globalThis.__kubidmEvidenceTrace.push({
                            type: "semantic",
                            at: performance.now(),
                            detail: event.detail,
                        });
                    });
                    window.addEventListener("kubidm:guide-diagnostics", (event) => {
                        globalThis.__kubidmEvidenceTrace.push({
                            type: "diagnostic",
                            at: performance.now(),
                            detail: event.detail,
                        });
                    });
                });

                const consoleEntries = [];
                const networkEntries = [];
                page.on("console", (message) => {
                    if (["error", "warning"].includes(message.type())) {
                        consoleEntries.push({ type: message.type(), text: message.text() });
                    }
                });
                page.on("request", (request) => {
                    const url = request.url();
                    if (!/^https?:/.test(url)) return;
                    if (new URL(url).origin !== baseOrigin) {
                        externalRequests.push({ story, url });
                    }
                });
                page.on("requestfailed", (request) => {
                    networkEntries.push({
                        url: request.url(),
                        error: request.failure()?.errorText || "request failed",
                    });
                });

                await page.goto(urlFor(story, theme, viewportName, motion), {
                    waitUntil: "networkidle",
                });
                if (motion === "full") {
                    await page.waitForFunction(
                        (expectedMode) => {
                            const diagnostic = globalThis.__kubidmGuideDiagnostics;
                            return expectedMode === "mock"
                                ? diagnostic?.loaded === true && diagnostic?.mockRuntime === true
                                : diagnostic?.loaded === true &&
                                      diagnostic?.renderer === "rive" &&
                                      diagnostic?.mockRuntime !== true;
                        },
                        mode,
                        { timeout: 10_000 },
                    );
                }

                await page.screenshot({
                    path: path.join(artifactDir, "start.png"),
                    fullPage: true,
                });
                await page.waitForTimeout(350);
                await page.screenshot({
                    path: path.join(artifactDir, "mid.png"),
                    fullPage: true,
                });
                await page.waitForTimeout(750);
                await page.screenshot({
                    path: path.join(artifactDir, "end.png"),
                    fullPage: true,
                });

                const diagnostic = await page.evaluate(() => globalThis.__kubidmGuideDiagnostics || null);
                const trace = await page.evaluate(() => globalThis.__kubidmEvidenceTrace || []);
                const semanticState = await page.locator("#ui-lab-mascot-state").textContent();
                if (motion === "full" && diagnostic?.fallbackActive) {
                    throw new Error(`Unexpected full-mode fallback for ${story}/${viewportName}/${theme}`);
                }
                if (motion === "full" && diagnostic?.riveState !== semanticState) {
                    throw new Error(`Rive/semantic mismatch: expected ${semanticState}, got ${diagnostic?.riveState}`);
                }

                manifest.captures.push({
                    story,
                    semanticState,
                    theme,
                    viewport: viewportName,
                    viewportSize,
                    motion,
                    diagnostic,
                    artifactDir: path.relative(outputRoot, artifactDir),
                });
                allSemantic.push({ story, viewport: viewportName, theme, motion, trace });
                allConsole.push({
                    story,
                    viewport: viewportName,
                    theme,
                    motion,
                    entries: consoleEntries,
                });
                allNetwork.push({
                    story,
                    viewport: viewportName,
                    theme,
                    motion,
                    entries: networkEntries,
                });
                await context.close();
            }
        }
    }
}

await browser.close();
manifest.externalRequests = externalRequests;
await writeFile(path.join(outputRoot, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(path.join(outputRoot, "semantic-trace.json"), `${JSON.stringify(allSemantic, null, 2)}\n`);
await writeFile(path.join(outputRoot, "console.json"), `${JSON.stringify(allConsole, null, 2)}\n`);
await writeFile(path.join(outputRoot, "network.json"), `${JSON.stringify(allNetwork, null, 2)}\n`);

if (mode === "real" && externalRequests.length > 0) {
    throw new Error(`Production Rive evidence made external requests: ${JSON.stringify(externalRequests)}`);
}

console.log(outputRoot);
