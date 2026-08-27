import { MotionLevel, Severity } from "./guide_contract.mjs";
import { guideRiveBindingValues, guideRiveTriggers } from "./guide_rive_binding.mjs";
import {
    loadGuideRiveContract,
    loadGuideRiveFile,
    loadGuideRiveRuntime,
    validateGuideRiveContract,
} from "./guide_rive_runtime.mjs";

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
    constructor(slot, { onReady = null, onFailure = null } = {}) {
        if (!(slot instanceof HTMLElement)) {
            throw new TypeError("RiveGuideRenderer requires an HTMLElement slot");
        }
        this.slot = slot;
        this.onReady = onReady;
        this.onFailure = onFailure;
        this.canvas = document.createElement("canvas");
        this.canvas.dataset.guideRiveCanvas = "";
        this.canvas.setAttribute("aria-hidden", "true");
        this.canvas.hidden = true;
        this.slot.append(this.canvas);

        this.rive = null;
        this.pendingRive = null;
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
        if (this.destroyed || this.failed || state.motionLevel !== MotionLevel.FULL) return;

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
        const [runtime, contract] = await Promise.all([loadGuideRiveRuntime(), loadGuideRiveContract()]);
        if (this.destroyed) return;

        const riveFile = await loadGuideRiveFile(runtime);
        if (this.destroyed) return;
        this.contract = contract;

        const options = {
            riveFile,
            canvas: this.canvas,
            artboard: contract.artboard,
            stateMachines: contract.stateMachine,
            autoplay: true,
            autoBind: true,
            enableRiveAssetCDN: false,
        };
        if (runtime.Layout && runtime.Fit && runtime.Alignment) {
            options.layout = new runtime.Layout({
                fit: runtime.Fit.Contain,
                alignment: runtime.Alignment.Center,
            });
        }

        await new Promise((resolve, reject) => {
            let instance;
            const cleanupPending = () => {
                instance?.cleanup?.();
                if (this.pendingRive === instance) this.pendingRive = null;
            };

            options.onLoad = () => {
                if (this.destroyed) {
                    cleanupPending();
                    resolve();
                    return;
                }
                try {
                    const validated = validateGuideRiveContract(instance, contract);
                    this.viewModelInstance = instance.viewModelInstance || validated.instance;
                    if (!instance.viewModelInstance) instance.bindViewModelInstance(this.viewModelInstance);
                    this.pendingRive = null;
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
                        mockRuntime: runtime.__kubidmMock === true,
                        cachedRiveFile: true,
                        fallbackActive: false,
                        lastError: null,
                    });
                    if (this.lastState) this.applyState(this.lastState);
                    this.onReady?.();
                    resolve();
                } catch (error) {
                    cleanupPending();
                    reject(error);
                }
            };
            options.onLoadError = (error) => {
                cleanupPending();
                reject(error instanceof Error ? error : new Error(String(error)));
            };
            instance = new runtime.Rive(options);
            this.pendingRive = instance;
            if (this.destroyed) cleanupPending();
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
        const values = guideRiveBindingValues(state);

        enumSet(instance, "state", values.state);
        enumSet(instance, "motion", values.motion);
        enumSet(instance, "severity", values.severity);
        enumSet(instance, "travelDirection", values.travelDirection);
        numberSet(instance, "lookX", values.lookX);
        numberSet(instance, "lookY", values.lookY);

        for (const trigger of guideRiveTriggers(this.previousMascotState, state)) {
            fire(instance, trigger);
        }
        this.previousMascotState = state.mascotState;

        setDiagnostic({
            semanticState: state.mascotState,
            productState: state.productState,
            motion: state.motionLevel,
            severity: state.severity,
            riveState: values.state,
            fallbackActive: false,
        });

        if (state.severity === Severity.CRITICAL || state.motionLevel !== MotionLevel.FULL) {
            // The View Model owns the stillness. The state machine still advances once
            // so the bound critical/static pose is actually applied.
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
        this.pendingRive?.cleanup?.();
        this.pendingRive = null;
        this.rive?.cleanup?.();
        this.rive = null;
        this.viewModelInstance = null;
        this.canvas.remove();
    }
}
