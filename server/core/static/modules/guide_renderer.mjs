import { MascotState, MotionLevel, assertGuideValue } from "./guide_contract.mjs";
import { RiveGuideRenderer } from "./guide_rive_renderer.mjs";

const STATIC_ASSET_ROOT = "/pkg/img/guide";

export const GuideFallback = Object.freeze({
    HIDE: "hide",
    LABEL: "label",
});

function assetState(mascotState) {
    if (mascotState !== MascotState.TRAVEL) return mascotState;
    // Static/reduced/failure rendering uses the same canonical idle artwork.
    // A real travel gait exists only inside the production Rive rig.
    return MascotState.IDLE;
}

export class StaticGuideRenderer {
    constructor(slot, { assetRoot = STATIC_ASSET_ROOT, fallback = GuideFallback.HIDE } = {}) {
        if (!(slot instanceof HTMLElement)) {
            throw new TypeError("StaticGuideRenderer requires an HTMLElement slot");
        }
        if (!Object.values(GuideFallback).includes(fallback)) {
            throw new TypeError(`Unsupported Kubidm guide fallback: ${fallback}`);
        }

        this.slot = slot;
        this.assetRoot = assetRoot.replace(/\/$/, "");
        this.fallbackMode = fallback;
        this.suppressed = false;
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

        this.suppressed = false;
        this.slot.dataset.mascotState = mascotState;
        this.slot.dataset.motion = motionLevel;
        this.image.alt = `Kubidm guide: ${mascotState}`;

        const nextState = assetState(mascotState);
        const nextSrc = `${this.assetRoot}/crab-${nextState}.webp`;
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
        if (this.suppressed) {
            this.image.hidden = true;
            this.fallback.hidden = true;
            return;
        }
        this.slot.hidden = false;
        this.image.hidden = false;
        this.fallback.hidden = true;
    }

    hideImage() {
        this.suppressed = true;
        this.image.hidden = true;
        this.fallback.hidden = true;
    }

    showFallback() {
        if (this.suppressed) {
            this.image.hidden = true;
            this.fallback.hidden = true;
            return;
        }

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
        this.suppressed = true;
    }
}

export class GuideRendererController {
    constructor(slot, { renderer = "auto", ...rendererOptions } = {}) {
        if (!new Set(["auto", "static"]).has(renderer)) {
            throw new Error(`Unsupported Kubidm guide renderer: ${renderer}`);
        }
        this.slot = slot;
        this.rendererName = renderer;
        this.rendererOptions = rendererOptions;
        this.staticRenderer = new StaticGuideRenderer(slot, rendererOptions);
        this.riveRenderer = null;
        this.riveReady = false;
        this.lastState = null;
    }

    ensureRiveRenderer() {
        if (this.rendererName === "static") return null;
        if (!this.riveRenderer) {
            this.riveRenderer = new RiveGuideRenderer(this.slot, {
                onReady: () => {
                    this.riveReady = true;
                    if (this.lastState) {
                        // Startup intentionally renders a still static fallback while
                        // Rive loads. Restore the requested motion marker before hiding
                        // that fallback or guide_motion.css will keep the canvas hidden.
                        this.slot.dataset.mascotState = this.lastState.mascotState;
                        this.slot.dataset.motion = this.lastState.motionLevel;
                    }
                    this.staticRenderer.hideImage();
                },
                onFailure: () => {
                    this.riveReady = false;
                    if (this.lastState) {
                        this.staticRenderer.setState({
                            ...this.lastState,
                            motionLevel: MotionLevel.STATIC,
                        });
                    }
                },
            });
        }
        return this.riveRenderer;
    }

    setState(state) {
        this.lastState = state;
        if (state.motionLevel === MotionLevel.FULL && this.rendererName !== "static") {
            // A still fallback is allowed during startup/failure, but once Rive is
            // ready it remains the only visible full-motion character renderer.
            if (this.riveReady) this.staticRenderer.hideImage();
            else this.staticRenderer.setState({ ...state, motionLevel: MotionLevel.STATIC });
            this.ensureRiveRenderer()?.setState(state);
            return;
        }

        this.riveRenderer?.destroy();
        this.riveRenderer = null;
        this.riveReady = false;
        this.staticRenderer.setState(state);
    }

    destroy() {
        this.riveRenderer?.destroy();
        this.riveRenderer = null;
        this.riveReady = false;
        this.staticRenderer.destroy();
    }
}

export function createGuideRenderer(slot, options) {
    return new GuideRendererController(slot, options);
}
