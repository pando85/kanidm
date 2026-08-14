import { chromium, firefox, webkit } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const baseURL = process.env.KUBIDM_UI_BASE_URL || "https://localhost:8443";
const baseOrigin = new URL(baseURL).origin;
const mode = process.env.KUBIDM_RIVE_TEST_MODE || "real";
const maxInitMs = Number(process.env.KUBIDM_RIVE_MAX_INIT_MS || 2000);
const maxFrameP95Ms = Number(process.env.KUBIDM_RIVE_MAX_FRAME_P95_MS || 50);
const maxMobileInitMs = Number(process.env.KUBIDM_RIVE_MAX_MOBILE_INIT_MS || 4000);
const maxMobileFrameP95Ms = Number(process.env.KUBIDM_RIVE_MAX_MOBILE_FRAME_P95_MS || 80);
const maxHeapGrowthBytes = Number(process.env.KUBIDM_RIVE_MAX_HEAP_GROWTH_BYTES || 8 * 1024 * 1024);
const stressTransitions = Number(process.env.KUBIDM_RIVE_STRESS_TRANSITIONS || 100);
const mobileCpuRate = Number(process.env.KUBIDM_RIVE_MOBILE_CPU_RATE || 4);
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

async function collectFrameIntervals(page) {
    return page.evaluate(async () => {
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
}

async function churn(page, count) {
    const sequence = ["first-login", "method-choice", "passkey-working", "success", "policy-required", "goodbye"];
    for (let index = 0; index < count; index += 1) {
        const story = sequence[index % sequence.length];
        await page.locator(`[data-story="${story}"]`).click();
        await page.waitForFunction(
            (expected) =>
                document.querySelector("#ui-lab-story-title")?.textContent?.length > 0 &&
                document.querySelector(`[data-story="${expected}"]`)?.getAttribute("aria-current") === "true",
            story,
        );
    }
}

async function measureProfile({
    browserType,
    name,
    viewport,
    initLimit,
    frameLimit,
    cpuRate = 1,
    measureHeap = false,
}) {
    const browser = await browserType.launch();
    const context = await browser.newContext({ ignoreHTTPSErrors: true, viewport });
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

    let cdp = null;
    if (browserType === chromium) {
        cdp = await context.newCDPSession(page);
        if (cpuRate > 1) await cdp.send("Emulation.setCPUThrottlingRate", { rate: cpuRate });
    }

    await page.goto(labUrl("applications-arrival"), { waitUntil: "networkidle" });
    await page.waitForFunction(() => globalThis.__kubidmGuideDiagnostics?.loaded === true, null, {
        timeout: initLimit + 5000,
    });

    const init = await page.evaluate(() => globalThis.__kubidmRivePerf);
    const initMs = init.loadedAt;
    const frameIntervals = await collectFrameIntervals(page);
    const frameP95Ms = percentile(frameIntervals, 0.95);

    let heapBefore = null;
    let heapAfter = null;
    let heapGrowthBytes = null;
    if (measureHeap && cdp) {
        await cdp.send("Performance.enable");
        await churn(page, 20);
        heapBefore = await heapUsed(cdp);
        await churn(page, stressTransitions);
        heapAfter = await heapUsed(cdp);
        heapGrowthBytes = heapAfter - heapBefore;
    }

    const canvasCount = await page.locator("[data-guide-rive-canvas]").count();
    const diagnostics = await page.evaluate(() => globalThis.__kubidmGuideDiagnostics);
    const browserVersion = browser.version();
    await browser.close();

    const failures = [];
    if (!Number.isFinite(initMs) || initMs > initLimit) failures.push(`Rive init ${initMs}ms > ${initLimit}ms`);
    if (frameP95Ms > frameLimit) failures.push(`frame p95 ${frameP95Ms}ms > ${frameLimit}ms`);
    if (heapGrowthBytes !== null && heapGrowthBytes > maxHeapGrowthBytes) {
        failures.push(`post-GC JS heap growth ${heapGrowthBytes} > ${maxHeapGrowthBytes}`);
    }
    if (canvasCount > 1) failures.push(`active Rive canvases ${canvasCount} > 1`);
    if (diagnostics?.fallbackActive) failures.push("Rive unexpectedly fell back to static renderer");
    if (consoleErrors.length > 0) failures.push(`console errors: ${consoleErrors.join("; ")}`);
    if (mode === "real" && externalRequests.length > 0) {
        failures.push(`external requests: ${externalRequests.join("; ")}`);
    }

    return {
        name,
        browserVersion,
        viewport,
        cpuRate,
        thresholds: {
            maxInitMs: initLimit,
            maxFrameP95Ms: frameLimit,
            maxHeapGrowthBytes: measureHeap ? maxHeapGrowthBytes : null,
            stressTransitions: measureHeap ? stressTransitions : null,
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
        failures,
    };
}

const profiles = [
    {
        browserType: chromium,
        name: "chromium-desktop",
        viewport: { width: 1440, height: 900 },
        initLimit: maxInitMs,
        frameLimit: maxFrameP95Ms,
        measureHeap: true,
    },
    {
        browserType: firefox,
        name: "firefox-desktop",
        viewport: { width: 1440, height: 900 },
        initLimit: maxInitMs,
        frameLimit: maxFrameP95Ms,
    },
    {
        browserType: webkit,
        name: "webkit-desktop",
        viewport: { width: 1440, height: 900 },
        initLimit: maxInitMs,
        frameLimit: maxFrameP95Ms,
    },
    {
        browserType: chromium,
        name: `chromium-mobile-${mobileCpuRate}x-cpu`,
        viewport: { width: 390, height: 844 },
        initLimit: maxMobileInitMs,
        frameLimit: maxMobileFrameP95Ms,
        cpuRate: mobileCpuRate,
    },
];

const results = [];
for (const profile of profiles) results.push(await measureProfile(profile));

const result = {
    mode,
    thresholds: {
        maxInitMs,
        maxFrameP95Ms,
        maxMobileInitMs,
        maxMobileFrameP95Ms,
        maxHeapGrowthBytes,
        stressTransitions,
        mobileCpuRate,
    },
    profiles: results,
};

await mkdir(path.dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(result, null, 2)}\n`);

const failures = results.flatMap((profile) => profile.failures.map((failure) => `${profile.name}: ${failure}`));
if (failures.length > 0) {
    console.error(`Kubidm Rive performance gate failed:\n- ${failures.join("\n- ")}`);
    process.exit(1);
}

console.log(output);
