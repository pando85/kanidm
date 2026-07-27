import { MascotState, MotionLevel, Severity } from "./guide_contract.mjs";
import {
    guideRiveAssetUrl,
    loadGuideRiveContract,
    loadGuideRiveRuntime,
    validateGuideRiveContract,
} from "./guide_rive_runtime.mjs";

const MAJOR_SUCCESS_STATES = new Set([
    "authentication_confirmed",
    "recommended_setup_complete",
    "credential_update_complete",
]);

function clampLook(value) {
    const number = Number(value ?? 0);
    return Number.isFinite(number) ? Math.max(-1, Math.min(1, number)) : 0;
}

function setDiagnostic(patch) {
    if (!document.querySelector("[data-ui-lab]")) return;
    const current = globalThis.__kubidmGuideDiagnostics || {};
    globalThis.__kubidmGuideDiagnostics = Object.freeze({ ...current, ...patch });
    window.dispatchEvent(
        new CustomEvent("kubidm:guide-diagnostics", {
            detail: globalThis.__kubidmGuideDiagnostics,
        }),
    );
}

function enumSet(instance, name, value) {
    const property = instance.enum(name);
    if (property.value !== value) property.value = value;
}

function numberSet(instance, name, value) {
    const property = instance.number(name);
    if (property.value !== value) property.value = value;
}

function fire(instance, name) {
    instance.trigger(name).trigger();
}

export class RiveGuideRenderer {
    constructor(slot, { onFailure = null } = {}) {
        if (!(slot instanceof HTMLElement)) {
            throw new TypeError("RiveGuideRenderer requires an HTMLElement slot");
        }
        this.slot = slot;
        this.onFailure = onFailure;
        this.canvas = document.createElement("canvas");
        this.canvas.dataset.guideRiveCanvas = "";
        this.canvas.setAttribute("aria-hidden", "true");
        this.canvas.hidden = true;
        this.slot.append(this.canvas);

        this.rive = null;
        this.viewModelInstance = null;
        this.contract = null;
        this.lastState = null;
        this.initPromise = null;
        this.failed = false;
        this.destroyed = false;
        this.resizeObserver = null;
        this.intersectionObserver = null;

        setDiagnostic({
            renderer: "rive",
            loaded: false,
            fallbackActive: false,
            lastError: null,
        });
    }

    setState(state) {
        this.lastState = state;
        if (this.destroyed || this.failed) return;
        if (state.motionLevel !== MotionLevel.FULL) return;

        if (this.rive && this.viewModelInstance) {
            this.applyState(state);
            return;
        }
        this.ensureLoaded().catch((error) => this.fail(error));
    }

    async ensureLoaded() {
        if (this.initPromise) return this.initPromise;
        this.initPromise = this.initialize().catch((error) => {
            this.initPromise = null;
            throw error;
        });
        return this.initPromise;
    }

    async initialize() {
        const [runtime, contract] = await Promise.all([
            loadGuideRiveRuntime(),
            loadGuideRiveContract(),
        ]);
        if (this.destroyed) return;
        this.contract = contract;

        const options = {
            src: guideRiveAssetUrl(),
            canvas: this.canvas,
            artboard: contract.artboard,
            stateMachines: contract.stateMachine,
            autoplay: true,
            autoBind: true,
        };
        if (runtime.Layout && runtime.Fit && runtime.Alignment) {
            options.layout = new runtime.Layout({
                fit: runtime.Fit.Contain,
                alignment: runtime.Alignment.Center,
            });
        }

        await new Promise((resolve, reject) => {
            let instance;
            options.onLoad = () => {
                try {
                    const validated = validateGuideRiveContract(instance, contract);
                    this.viewModelInstance = instance.viewModelInstance || validated.instance;
                    if (!instance.viewModelInstance) instance.bindViewModelInstance(this.viewModelInstance);
                    this.rive = instance;
                    this.installLifecycle();
                    this.resize();
                    this.canvas.hidden = false;
                    this.slot.hidden = false;
                    setDiagnostic({
                        renderer: "rive",
                        loaded: true,
                        artboard: contract.artboard,
                        stateMachine: contract.stateMachine,
                        viewModel: contract.viewModel,
                        fallbackActive: false,
                        lastError: null,
                    });
                    if (this.lastState) this.applyState(this.lastState);
                    resolve();
                } catch (error) {
                    instance?.cleanup?.();
                    reject(error);
                }
            };
            options.onLoadError = (error) => reject(error instanceof Error ? error : new Error(String(error)));
            instance = new runtime.Rive(options);
        });
    }

    installLifecycle() {
        this.resizeObserver?.disconnect();
        this.intersectionObserver?.disconnect();

        this.resizeObserver = new ResizeObserver(() => this.resize());
        this.resizeObserver.observe(this.slot);

        this.intersectionObserver = new IntersectionObserver((entries) => {
            const visible = entries.some((entry) => entry.isIntersecting);
            if (!this.rive) return;
            if (visible && this.lastState?.motionLevel === MotionLevel.FULL) this.rive.play?.();
            else this.rive.pause?.();
        });
        this.intersectionObserver.observe(this.slot);
    }

    resize() {
        this.rive?.resizeDrawingSurfaceToCanvas?.();
    }

    applyState(state) {
        if (!this.viewModelInstance || this.destroyed) return;
        const instance = this.viewModelInstance;

        enumSet(instance, "state", state.mascotState);
        enumSet(instance, "motion", state.motionLevel);
        enumSet(instance, "severity", state.severity);
        enumSet(instance, "travelDirection", state.travelDirection || "right");
        numberSet(instance, "lookX", clampLook(state.lookX));
        numberSet(instance, "lookY", clampLook(state.lookY));

        const previous = this.previousMascotState;
        if (previous !== state.mascotState) {
            if (state.mascotState === MascotState.GUIDE) fire(instance, "attention");
            if (state.mascotState === MascotState.SUCCESS) {
                fire(instance, MAJOR_SUCCESS_STATES.has(state.productState) ? "successMajor" : "successSmall");
            }
            if (state.mascotState === MascotState.GOODBYE) fire(instance, "goodbye");
        }
        this.previousMascotState = state.mascotState;

        setDiagnostic({
            semanticState: state.mascotState,
            productState: state.productState,
            motion: state.motionLevel,
            severity: state.severity,
            riveState: state.mascotState,
            fallbackActive: false,
        });

        if (state.severity === Severity.CRITICAL || state.motionLevel !== MotionLevel.FULL) {
            // The View Model owns the visual stillness. pause() is intentionally not
            // used here because the state machine must advance once to apply bindings.
            return;
        }
    }

    fail(error) {
        if (this.failed || this.destroyed) return;
        this.failed = true;
        const message = error instanceof Error ? error.message : String(error);
        this.canvas.hidden = true;
        setDiagnostic({
            renderer: "static",
            loaded: false,
            fallbackActive: true,
            lastError: message,
        });
        this.onFailure?.(error);
    }

    destroy() {
        this.destroyed = true;
        this.resizeObserver?.disconnect();
        this.intersectionObserver?.disconnect();
        this.resizeObserver = null;
        this.intersectionObserver = null;
        this.rive?.cleanup?.();
        this.rive = null;
        this.viewModelInstance = null;
        this.canvas.remove();
    }
}
