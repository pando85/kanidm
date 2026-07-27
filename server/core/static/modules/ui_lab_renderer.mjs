import { MotionLevel } from "./guide_contract.mjs";
import {
    GuideFallback,
    createGuideRenderer,
} from "./guide_renderer.mjs";

const query = new URLSearchParams(window.location.search);
if (query.get("rive") === "mock" || query.get("rive") === "mock-fail") {
    const { installMockRiveRuntime } = await import("./guide_rive_mock.mjs");
    installMockRiveRuntime({ failLoad: query.get("rive") === "mock-fail" });
}

const canvas = document.querySelector("#ui-lab-canvas");
const preview = document.querySelector("#ui-lab-preview");

if (!canvas || !preview) {
    throw new Error("Kubidm UI Lab renderer requires the preview canvas");
}

const renderers = new WeakMap();
const activeSlots = new Set();

function motionLevel() {
    const value = preview.dataset.motion || MotionLevel.STATIC;
    return Object.values(MotionLevel).includes(value) ? value : MotionLevel.STATIC;
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

    renderer.setState({
        mascotState: slot.dataset.mascotState || "idle",
        motionLevel: motionLevel(),
        productState: slot.closest("[data-guide-action]")?.dataset.guideAction || "ui_lab",
        severity: slot.closest("[data-guide-severity]")?.dataset.guideSeverity || "neutral",
    });
}

function sync() {
    const currentSlots = new Set(canvas.querySelectorAll(".ui-lab-mascot-slot"));
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

window.addEventListener("pagehide", () => {
    for (const slot of activeSlots) renderers.get(slot)?.destroy();
    activeSlots.clear();
});

sync();
