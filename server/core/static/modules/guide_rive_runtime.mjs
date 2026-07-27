const DEFAULT_RUNTIME_URL = "/pkg/rive/rive.js";
const DEFAULT_WASM_URL = "/pkg/rive/rive.wasm";
const DEFAULT_ASSET_URL = "/pkg/img/guide/kubidm-guide.riv";
const DEFAULT_CONTRACT_URL = "/pkg/guide_rive_contract.json";

let runtimePromise = null;
let contractPromise = null;

function loadScript(src) {
    return new Promise((resolve, reject) => {
        const existing = document.querySelector(`script[data-kubidm-rive-runtime][src="${src}"]`);
        if (existing) {
            if (globalThis.rive?.Rive) {
                resolve();
                return;
            }
            existing.addEventListener("load", resolve, { once: true });
            existing.addEventListener("error", () => reject(new Error(`Failed to load Rive runtime: ${src}`)), {
                once: true,
            });
            return;
        }

        const script = document.createElement("script");
        script.src = src;
        script.async = true;
        script.dataset.kubidmRiveRuntime = "";
        script.addEventListener("load", resolve, { once: true });
        script.addEventListener(
            "error",
            () => reject(new Error(`Failed to load Rive runtime: ${src}`)),
            { once: true },
        );
        document.head.append(script);
    });
}

export async function loadGuideRiveContract(url = DEFAULT_CONTRACT_URL) {
    if (!contractPromise) {
        contractPromise = fetch(url, { credentials: "same-origin", cache: "force-cache" }).then(async (response) => {
            if (!response.ok) {
                throw new Error(`Failed to load Kubidm Rive contract (${response.status})`);
            }
            const contract = await response.json();
            if (!contract?.artboard || !contract?.stateMachine || !contract?.viewModel) {
                throw new Error("Kubidm Rive contract is incomplete");
            }
            return Object.freeze(contract);
        });
    }
    return contractPromise;
}

export async function loadGuideRiveRuntime({
    runtimeUrl = DEFAULT_RUNTIME_URL,
    wasmUrl = DEFAULT_WASM_URL,
} = {}) {
    if (!runtimePromise) {
        runtimePromise = (async () => {
            const override = globalThis.__kubidmRiveRuntimeOverride;
            const runtime =
                override?.Rive && override?.RuntimeLoader
                    ? override
                    : await (async () => {
                          if (!globalThis.rive?.Rive) await loadScript(runtimeUrl);
                          return globalThis.rive;
                      })();

            if (!runtime?.Rive || !runtime?.RuntimeLoader) {
                throw new Error("Self-hosted Rive runtime did not expose Rive and RuntimeLoader");
            }

            runtime.RuntimeLoader.setWasmUrl(wasmUrl);
            // canvas-lite has an optional WASM fallback URL. Its upstream default can
            // point at a public CDN. Kubidm is intentionally self-hosted, so disable
            // that path explicitly: local WASM failure must reach our static fallback.
            runtime.RuntimeLoader.setWasmFallbackUrl?.(null);
            return runtime;
        })().catch((error) => {
            runtimePromise = null;
            throw error;
        });
    }
    return runtimePromise;
}

export function guideRiveAssetUrl() {
    return DEFAULT_ASSET_URL;
}

function propertyNames(viewModel) {
    return new Set((viewModel?.properties || []).map((property) => property.name));
}

function enumValues(runtime, enumName) {
    const item = (runtime.enums?.() || []).find((entry) => entry.name === enumName);
    return new Set(item?.values || item?.enumerants || item?.entries || []);
}

export function validateGuideRiveContract(runtimeInstance, contract) {
    const viewModel = runtimeInstance.viewModelByName?.(contract.viewModel);
    if (!viewModel) throw new Error(`Missing Rive View Model: ${contract.viewModel}`);

    const names = propertyNames(viewModel);
    for (const propertyName of Object.keys(contract.properties || {})) {
        if (!names.has(propertyName)) {
            throw new Error(`Missing Rive View Model property: ${propertyName}`);
        }
    }

    const instance = runtimeInstance.viewModelInstance || viewModel.defaultInstance?.();
    if (!instance) throw new Error(`Missing default View Model instance for ${contract.viewModel}`);

    for (const [name, definition] of Object.entries(contract.properties || {})) {
        let property;
        if (definition.type === "enum") property = instance.enum?.(name);
        else if (definition.type === "number") property = instance.number?.(name);
        else if (definition.type === "trigger") property = instance.trigger?.(name);
        else throw new Error(`Unsupported Kubidm Rive contract type: ${definition.type}`);
        if (!property) throw new Error(`Rive property ${name} is not accessible as ${definition.type}`);
    }

    // Some runtime versions expose enum metadata through r.enums(). Validate it when
    // available, but do not reject runtimes that only expose enum values through VMI.
    for (const [name, definition] of Object.entries(contract.properties || {})) {
        if (definition.type !== "enum" || !definition.values?.length) continue;
        const values = enumValues(runtimeInstance, name);
        if (values.size === 0) continue;
        for (const expected of definition.values) {
            if (!values.has(expected)) throw new Error(`Rive enum ${name} is missing value ${expected}`);
        }
    }

    return { viewModel, instance };
}

export function resetGuideRiveRuntimeForTests() {
    runtimePromise = null;
    contractPromise = null;
}

export const GuideRivePaths = Object.freeze({
    runtime: DEFAULT_RUNTIME_URL,
    wasm: DEFAULT_WASM_URL,
    asset: DEFAULT_ASSET_URL,
    contract: DEFAULT_CONTRACT_URL,
});
