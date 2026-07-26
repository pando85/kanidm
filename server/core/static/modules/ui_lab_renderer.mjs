import { MotionLevel } from "./guide_contract.mjs";
import { createGuideRenderer } from "./guide_renderer.mjs";

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
        renderer = createGuideRenderer(slot, { renderer: "static" });
        renderers.set(slot, renderer);
    }

    renderer.setState({
        mascotState: slot.dataset.mascotState || "idle",
        motionLevel: motionLevel(),
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
