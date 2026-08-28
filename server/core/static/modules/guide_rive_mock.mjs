const PROPERTY_DEFINITIONS = Object.freeze([
    ["state", "enum"],
    ["motion", "enum"],
    ["severity", "enum"],
    ["travelDirection", "enum"],
    ["lookX", "number"],
    ["lookY", "number"],
    ["attention", "trigger"],
    ["successSmall", "trigger"],
    ["successMajor", "trigger"],
    ["goodbye", "trigger"],
]);

const ENUMS = Object.freeze([
    {
        name: "state",
        values: ["idle", "welcome", "guide", "protect", "working", "success", "warning", "goodbye", "travel"],
    },
    { name: "motion", values: ["full", "reduced", "static"] },
    { name: "severity", values: ["neutral", "positive", "caution", "critical"] },
    { name: "travelDirection", values: ["left", "right"] },
]);

class MockValueProperty {
    constructor(value) {
        this.value = value;
    }
}

class MockTriggerProperty {
    constructor(name, stats) {
        this.name = name;
        this.stats = stats;
    }

    trigger() {
        this.stats.triggers.push(this.name);
    }
}

class MockViewModelInstance {
    constructor(stats) {
        this.stats = stats;
        this.values = new Map([
            ["state", new MockValueProperty("idle")],
            ["motion", new MockValueProperty("static")],
            ["severity", new MockValueProperty("neutral")],
            ["travelDirection", new MockValueProperty("right")],
            ["lookX", new MockValueProperty(0)],
            ["lookY", new MockValueProperty(0)],
        ]);
        this.triggers = new Map([
            ["attention", new MockTriggerProperty("attention", stats)],
            ["successSmall", new MockTriggerProperty("successSmall", stats)],
            ["successMajor", new MockTriggerProperty("successMajor", stats)],
            ["goodbye", new MockTriggerProperty("goodbye", stats)],
        ]);
        this.viewModelName = "GuideState";
    }

    enum(name) {
        return this.values.get(name) || null;
    }

    number(name) {
        return this.values.get(name) || null;
    }

    trigger(name) {
        return this.triggers.get(name) || null;
    }
}

class MockViewModel {
    constructor(instance) {
        this.boundInstance = instance;
        this.properties = PROPERTY_DEFINITIONS.map(([name, type]) => ({ name, type }));
    }

    defaultInstance() {
        return this.boundInstance;
    }
}

export function createMockRiveRuntime({ failLoad = false } = {}) {
    const stats = globalThis.__kubidmMockRiveStats || {
        created: 0,
        cleaned: 0,
        active: 0,
        fileCreated: 0,
        fileCleaned: 0,
        fileInits: 0,
        plays: 0,
        pauses: 0,
        resizes: 0,
        wasmUrl: null,
        wasmFallbackUrl: undefined,
        usedRiveFile: false,
        riveAssetCdnEnabled: undefined,
        fileAssetCdnEnabled: undefined,
        triggers: [],
    };
    globalThis.__kubidmMockRiveStats = stats;

    class MockRiveFile {
        constructor(options) {
            this.options = options;
            stats.fileCreated += 1;
            stats.fileAssetCdnEnabled = options.enableRiveAssetCDN;
        }

        async init() {
            stats.fileInits += 1;
            if (failLoad) throw new Error("Injected mock Rive load failure");
            return this;
        }

        cleanup() {
            if (this.cleaned) return;
            this.cleaned = true;
            stats.fileCleaned += 1;
        }
    }

    class MockRive {
        constructor(options) {
            this.options = options;
            this.viewModelInstance = new MockViewModelInstance(stats);
            this.viewModel = new MockViewModel(this.viewModelInstance);
            stats.created += 1;
            stats.active += 1;
            stats.usedRiveFile = Boolean(options.riveFile);
            stats.riveAssetCdnEnabled = options.enableRiveAssetCDN;
            queueMicrotask(() => options.onLoad?.());
        }

        viewModelByName(name) {
            return name === "GuideState" ? this.viewModel : null;
        }

        bindViewModelInstance(instance) {
            this.viewModelInstance = instance;
        }

        enums() {
            return ENUMS;
        }

        resizeDrawingSurfaceToCanvas() {
            stats.resizes += 1;
        }

        play() {
            stats.plays += 1;
        }

        pause() {
            stats.pauses += 1;
        }

        cleanup() {
            if (this.cleaned) return;
            this.cleaned = true;
            stats.cleaned += 1;
            stats.active -= 1;
        }
    }

    return Object.freeze({
        __kubidmMock: true,
        Rive: MockRive,
        RiveFile: MockRiveFile,
        RuntimeLoader: {
            setWasmUrl(url) {
                stats.wasmUrl = url;
            },
            setWasmFallbackUrl(url) {
                stats.wasmFallbackUrl = url;
            },
        },
        Layout: class MockLayout {
            constructor(options) {
                Object.assign(this, options);
            }
        },
        Fit: { Contain: "contain" },
        Alignment: { Center: "center" },
    });
}

export function installMockRiveRuntime(options) {
    const runtime = createMockRiveRuntime(options);
    globalThis.__kubidmRiveRuntimeOverride = runtime;
    return runtime;
}
