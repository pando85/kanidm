import {
    MascotState,
    MotionLevel,
    assertGuideValue,
} from "./guide_contract.mjs";

const STATIC_ASSET_ROOT = "/pkg/img/guide";

export const GuideFallback = Object.freeze({
    HIDE: "hide",
    LABEL: "label",
});

export class StaticGuideRenderer {
    constructor(
        slot,
        { assetRoot = STATIC_ASSET_ROOT, fallback = GuideFallback.HIDE } = {},
    ) {
        if (!(slot instanceof HTMLElement)) {
            throw new TypeError("StaticGuideRenderer requires an HTMLElement slot");
        }
        if (!Object.values(GuideFallback).includes(fallback)) {
            throw new TypeError(`Unsupported Kubidm guide fallback: ${fallback}`);
        }

        this.slot = slot;
        this.assetRoot = assetRoot.replace(/\/$/, "");
        this.fallbackMode = fallback;
        this.image = slot.querySelector("[data-guide-image], [data-lab-mascot-image]");
        this.fallback = slot.querySelector("[data-guide-fallback], [data-lab-mascot-fallback]");

        if (!this.image) {
            this.image = document.createElement("img");
            this.image.dataset.guideImage = "";
            this.slot.append(this.image);
        }

        if (!this.fallback) {
            this.fallback = document.createElement("div");
            this.fallback.dataset.guideFallback = "";
            this.fallback.className = "ui-lab-mascot-fallback";
            this.fallback.hidden = true;
            this.slot.append(this.fallback);
        }

        this.image.addEventListener("error", () => this.showFallback());
        this.image.addEventListener("load", () => this.showImage());
    }

    setState({ mascotState = MascotState.IDLE, motionLevel = MotionLevel.STATIC } = {}) {
        assertGuideValue("mascotState", mascotState);
        assertGuideValue("motionLevel", motionLevel);

        this.slot.dataset.mascotState = mascotState;
        this.slot.dataset.motion = motionLevel;
        this.image.alt = `Kubidm guide: ${mascotState}`;

        const nextSrc = `${this.assetRoot}/crab-${mascotState}.svg`;
        if (this.image.getAttribute("src") !== nextSrc) {
            if (this.fallbackMode === GuideFallback.HIDE) this.slot.hidden = true;
            this.image.hidden = false;
            this.fallback.hidden = true;
            this.image.src = nextSrc;
        } else if (this.image.complete && this.image.naturalWidth === 0) {
            this.showFallback();
        } else if (this.image.complete) {
            this.showImage();
        }
    }

    showImage() {
        this.slot.hidden = false;
        this.image.hidden = false;
        this.fallback.hidden = true;
    }

    showFallback() {
        const state = this.slot.dataset.mascotState || MascotState.IDLE;
        this.image.hidden = true;

        if (this.fallbackMode === GuideFallback.HIDE) {
            this.fallback.hidden = true;
            this.slot.hidden = true;
            return;
        }

        this.slot.hidden = false;
        this.fallback.hidden = false;
        this.fallback.innerHTML = "";

        const label = document.createElement("span");
        label.append("Mascot asset slot", document.createElement("br"));
        const strong = document.createElement("strong");
        strong.textContent = state;
        label.append(strong);
        this.fallback.append(label);
    }

    destroy() {
        // Static SVG has no runtime resources. This method exists so callers can
        // use the same lifecycle contract for the future Rive renderer.
    }
}

export class GuideRendererController {
    constructor(slot, { renderer = "static", ...rendererOptions } = {}) {
        if (renderer !== "static") {
            throw new Error(`Unsupported Kubidm guide renderer: ${renderer}`);
        }
        this.slot = slot;
        this.rendererName = renderer;
        this.renderer = new StaticGuideRenderer(slot, rendererOptions);
    }

    setState(state) {
        this.renderer.setState(state);
    }

    destroy() {
        this.renderer.destroy();
    }
}

export function createGuideRenderer(slot, options) {
    return new GuideRendererController(slot, options);
}
