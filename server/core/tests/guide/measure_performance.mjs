import { chromium } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const baseURL = process.env.KUBIDM_UI_BASE_URL || "https://localhost:8443";
const baseOrigin = new URL(baseURL).origin;
const mode = process.env.KUBIDM_RIVE_TEST_MODE || "real";
const maxInitMs = Number(process.env.KUBIDM_RIVE_MAX_INIT_MS || 2000);
const maxFrameP95Ms = Number(process.env.KUBIDM_RIVE_MAX_FRAME_P95_MS || 50);
const maxHeapGrowthBytes = Number(process.env.KUBIDM_RIVE_MAX_HEAP_GROWTH_BYTES || 8 * 1024 * 1024);
const stressTransitions = Number(process.env.KUBIDM_RIVE_STRESS_TRANSITIONS || 100);
const output = path.resolve("artifacts", "guide-performance.json");

function percentile(values, percentileValue) {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * percentileValue) - 1);
    return sorted[index];
}

function labUrl(story) {
    const query = mode === "mock" ? "?rive=mock" : "";
    return `${baseURL}/ui/_lab${query}#story=${story}&theme=light&viewport=desktop&motion=full`;
}

async function heapUsed(session) {
    await session.send("HeapProfiler.collectGarbage");
    const response = await session.send("Performance.getMetrics");
    return response.metrics.find((metric) => metric.name === "JSHeapUsedSize")?.value || 0;
}

const browser = await chromium.launch();
const context = await browser.newContext({ ignoreHTTPSErrors: true, viewport: { width: 1440, height: 900 } });
const page = await context.newPage();
const consoleErrors = [];
const externalRequests = [];
page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
});
page.on("request", (request) => {
    const url = request.url();
    if (/^https?:/.test(url) && new URL(url).origin !== baseOrigin) externalRequests.push(url);
});

await page.addInitScript(() => {
    globalThis.__kubidmRivePerf = { loadedAt: null, firstDiagnosticAt: null };
    window.addEventListener("kubidm:guide-diagnostics", (event) => {
        const now = performance.now();
        if (globalThis.__kubidmRivePerf.firstDiagnosticAt === null) {
            globalThis.__kubidmRivePerf.firstDiagnosticAt = now;
        }
        if (event.detail?.loaded === true && globalThis.__kubidmRivePerf.loadedAt === null) {
            globalThis.__kubidmRivePerf.loadedAt = now;
        }
    });
});

await page.goto(labUrl("applications-arrival"), { waitUntil: "networkidle" });
await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true, null, {
    timeout: maxInitMs + 5000,
});
const init = await page.evaluate(() => globalThis.__kubidmRivePerf);
const initMs = init.loadedAt;

const frameIntervals = await page.evaluate(async () => {
    const intervals = [];
    await new Promise((resolve) => {
        let first = null;
        let previous = null;
        const tick = (timestamp) => {
            if (first === null) first = timestamp;
            if (previous !== null) intervals.push(timestamp - previous);
            previous = timestamp;
            if (timestamp - first >= 1200) resolve();
            else requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
    });
    return intervals;
});
const frameP95Ms = percentile(frameIntervals, 0.95);

const session = await context.newCDPSession(page);
await session.send("Performance.enable");
const sequence = ["first-login", "method-choice", "passkey-working", "success", "policy-required", "goodbye"];

async function churn(count) {
    for (let index = 0; index < count; index += 1) {
        const story = sequence[index % sequence.length];
        await page.locator(`[data-story="${story}"]`).click();
        await page.waitForFunction(
            (expected) => document.querySelector("#ui-lab-story-title")?.textContent?.length > 0 &&
                document.querySelector(`[data-story="${expected}"]`)?.getAttribute("aria-current") === "true",
            story,
        );
    }
}

await churn(20);
const heapBefore = await heapUsed(session);
await churn(stressTransitions);
const heapAfter = await heapUsed(session);
const heapGrowthBytes = heapAfter - heapBefore;
const canvasCount = await page.locator("[data-guide-rive-canvas]").count();
const diagnostics = await page.evaluate(() => globalThis.__kubidmGuideDiagnostics);

const result = {
    mode,
    thresholds: {
        maxInitMs,
        maxFrameP95Ms,
        maxHeapGrowthBytes,
        stressTransitions,
    },
    measurements: {
        initMs,
        frameP95Ms,
        frameSampleCount: frameIntervals.length,
        heapBefore,
        heapAfter,
        heapGrowthBytes,
        canvasCount,
    },
    diagnostics,
    consoleErrors,
    externalRequests,
};

await mkdir(path.dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(result, null, 2)}\n`);
await browser.close();

const failures = [];
if (!Number.isFinite(initMs) || initMs > maxInitMs) failures.push(`Rive init ${initMs}ms > ${maxInitMs}ms`);
if (frameP95Ms > maxFrameP95Ms) failures.push(`frame p95 ${frameP95Ms}ms > ${maxFrameP95Ms}ms`);
if (heapGrowthBytes > maxHeapGrowthBytes) {
    failures.push(`post-GC JS heap growth ${heapGrowthBytes} > ${maxHeapGrowthBytes}`);
}
if (canvasCount > 1) failures.push(`active Rive canvases ${canvasCount} > 1`);
if (diagnostics?.fallbackActive) failures.push("Rive unexpectedly fell back to static renderer");
if (consoleErrors.length > 0) failures.push(`console errors: ${consoleErrors.join("; ")}`);
if (mode === "real" && externalRequests.length > 0) {
    failures.push(`external requests: ${externalRequests.join("; ")}`);
}

if (failures.length > 0) {
    console.error(`Kubidm Rive performance gate failed:\n- ${failures.join("\n- ")}`);
    process.exit(1);
}

console.log(output);
