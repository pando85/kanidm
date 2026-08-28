import { MotionLevel } from "./guide_contract.mjs";
import { GuideFallback, createGuideRenderer } from "./guide_renderer.mjs";

const query = new URLSearchParams(window.location.search);
if (query.get("rive") === "mock" || query.get("rive") === "mock-fail") {
    const { installMockRiveRuntime } = await import("./guide_rive_mock.mjs");
    installMockRiveRuntime({ failLoad: query.get("rive") === "mock-fail" });
}

const canvas = document.querySelector("#ui-lab-canvas");
const preview = document.querySelector("#ui-lab-preview");
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

if (!canvas || !preview) {
    throw new Error("Kubidm UI Lab renderer requires the preview canvas");
}

const renderers = new WeakMap();
const activeSlots = new Set();

function motionLevel() {
    const value = preview.dataset.motion || MotionLevel.STATIC;
    if (value === MotionLevel.STATIC) return MotionLevel.STATIC;
    if (reducedMotion.matches) return MotionLevel.REDUCED;
    return Object.values(MotionLevel).includes(value) ? value : MotionLevel.STATIC;
}

function numericDataset(value) {
    const number = Number(value ?? 0);
    return Number.isFinite(number) ? number : 0;
}

function syncSlot(slot) {
    let renderer = renderers.get(slot);
    if (!renderer) {
        renderer = createGuideRenderer(slot, {
            renderer: "auto",
            fallback: GuideFallback.LABEL,
        });
        renderers.set(slot, renderer);
    }
    activeSlots.add(slot);

    const semanticRoot = slot.closest("[data-guide-action]");
    renderer.setState({
        mascotState: slot.dataset.mascotState || semanticRoot?.dataset.guideState || "idle",
        motionLevel: motionLevel(),
        productState: semanticRoot?.dataset.guideAction || "ui_lab",
        severity: semanticRoot?.dataset.guideSeverity || "neutral",
        travelDirection: slot.dataset.travelDirection || semanticRoot?.dataset.guideTravelDirection || "right",
        lookX: numericDataset(slot.dataset.lookX ?? semanticRoot?.dataset.guideLookX),
        lookY: numericDataset(slot.dataset.lookY ?? semanticRoot?.dataset.guideLookY),
    });
}

function sync() {
    const currentSlots = new Set(canvas.querySelectorAll(".ui-lab-mascot-slot, [data-lab-mascot]"));
    for (const slot of activeSlots) {
        if (currentSlots.has(slot)) continue;
        renderers.get(slot)?.destroy();
        activeSlots.delete(slot);
    }
    currentSlots.forEach(syncSlot);
}

new MutationObserver(sync).observe(canvas, {
    childList: true,
    subtree: true,
});

new MutationObserver(sync).observe(preview, {
    attributes: true,
    attributeFilter: ["data-motion"],
});

reducedMotion.addEventListener("change", sync);

window.addEventListener("pagehide", () => {
    for (const slot of activeSlots) renderers.get(slot)?.destroy();
    activeSlots.clear();
});

sync();
