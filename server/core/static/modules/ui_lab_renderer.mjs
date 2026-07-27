import { MotionLevel } from "./guide_contract.mjs";
import {
    GuideFallback,
    createGuideRenderer,
} from "./guide_renderer.mjs";

const canvas = document.querySelector("#ui-lab-canvas");
const preview = document.querySelector("#ui-lab-preview");

if (!canvas || !preview) {
    throw new Error("Kubidm UI Lab renderer requires the preview canvas");
}

const renderers = new WeakMap();

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

    renderer.setState({
        mascotState: slot.dataset.mascotState || "idle",
        motionLevel: motionLevel(),
        productState: slot.closest("[data-guide-action]")?.dataset.guideAction || "ui_lab",
        severity: slot.closest("[data-guide-severity]")?.dataset.guideSeverity || "neutral",
    });
}

function sync() {
    canvas.querySelectorAll(".ui-lab-mascot-slot").forEach(syncSlot);
}

new MutationObserver(sync).observe(canvas, {
    childList: true,
    subtree: true,
});

new MutationObserver(sync).observe(preview, {
    attributes: true,
    attributeFilter: ["data-motion"],
});

sync();
