import { MotionLevel, normaliseGuideState } from "./guide_contract.mjs";

const applicationStories = Object.freeze({
    "applications-arrival": {
        title: "Applications arrival — right",
        productState: "applications_arrival",
        recommendation: "none",
        mascotState: "travel",
        severity: "neutral",
        heading: "Applications",
        subtitle: "The same guide travels into the authenticated destination.",
        travelDirection: "right",
        lookX: 0.85,
    },
    "applications-arrival-left": {
        title: "Applications arrival — left",
        productState: "applications_arrival",
        recommendation: "none",
        mascotState: "travel",
        severity: "neutral",
        heading: "Applications",
        subtitle: "Mirrored travel verifies the same lateral gait in the opposite direction.",
        travelDirection: "left",
        lookX: -0.85,
    },
    applications: {
        title: "Applications settled",
        productState: "applications",
        recommendation: "none",
        mascotState: "idle",
        severity: "neutral",
        heading: "Applications",
        subtitle: "Routine use is quiet after the confirmed arrival.",
        travelDirection: "right",
        lookX: 0.15,
    },
});

const ui = {
    canvas: document.querySelector("#ui-lab-canvas"),
    title: document.querySelector("#ui-lab-story-title"),
    productState: document.querySelector("#ui-lab-product-state"),
    recommendation: document.querySelector("#ui-lab-recommendation"),
    mascotState: document.querySelector("#ui-lab-mascot-state"),
    severity: document.querySelector("#ui-lab-severity"),
    motion: document.querySelector("#ui-lab-motion"),
};

function storyFromHash(hash = location.hash) {
    return new URLSearchParams(hash.slice(1)).get("story");
}

function writeStoryToHash(story) {
    const params = new URLSearchParams(location.hash.slice(1));
    params.set("story", story);
    history.replaceState(null, "", `#${params.toString()}`);
}

function renderApplicationsStory(name, { updateHash = true } = {}) {
    const story = applicationStories[name];
    if (!story || !ui.canvas) return false;

    const motionLevel = Object.values(MotionLevel).includes(ui.motion?.value) ? ui.motion.value : MotionLevel.STATIC;
    const assetState = story.mascotState === "travel" ? "idle" : story.mascotState;

    ui.title.textContent = story.title;
    ui.productState.textContent = story.productState;
    ui.recommendation.textContent = story.recommendation;
    ui.mascotState.textContent = story.mascotState;
    ui.severity.textContent = story.severity;

    ui.canvas.innerHTML = `<section class="ui-lab-applications"
        data-guide-scene="applications"
        data-guide-state="${story.mascotState}"
        data-guide-action="${story.productState}"
        data-guide-severity="${story.severity}"
        data-guide-travel-direction="${story.travelDirection}"
        data-guide-look-x="${story.lookX}">
        <header class="ui-lab-applications-header">
            <div><span class="ui-lab-kicker">Authenticated destination</span><h2>${story.heading}</h2><p>${story.subtitle}</p></div>
            <div class="ui-lab-avatar" aria-hidden="true">A</div>
        </header>
        <div class="ui-lab-app-grid">
            <article><div class="ui-lab-app-icon">G</div><strong>Grafana</strong><small>Observability</small></article>
            <article><div class="ui-lab-app-icon">F</div><strong>Forgejo</strong><small>Source control</small></article>
            <article><div class="ui-lab-app-icon">K</div><strong>Kubernetes</strong><small>Platform</small></article>
        </div>
        <div class="ui-lab-app-guide-slot"
             data-lab-mascot
             data-mascot-state="${story.mascotState}"
             data-travel-direction="${story.travelDirection}"
             data-look-x="${story.lookX}"
             data-motion="${motionLevel}">
            <img data-lab-mascot-image src="/pkg/img/guide/crab-${assetState}.webp" alt="Kubidm guide: ${story.mascotState}" />
        </div>
    </section>`;

    document.querySelectorAll("[data-story]").forEach((button) => {
        button.setAttribute("aria-current", button.dataset.story === name ? "true" : "false");
    });

    const state = normaliseGuideState({
        productState: story.productState,
        recommendation: story.recommendation,
        mascotState: story.mascotState,
        severity: story.severity,
        motionLevel,
        travelDirection: story.travelDirection,
        lookX: story.lookX,
    });
    window.dispatchEvent(new CustomEvent("kubidm:guide-state", { detail: state }));
    if (updateHash) writeStoryToHash(name);
    return true;
}

document.addEventListener(
    "click",
    (event) => {
        const button = event.target.closest("[data-story], [data-go-story]");
        const story = button?.dataset.story || button?.dataset.goStory;
        if (!applicationStories[story]) return;
        event.preventDefault();
        renderApplicationsStory(story);
    },
    true,
);

[document.querySelector("#ui-lab-theme"), document.querySelector("#ui-lab-viewport"), ui.motion]
    .filter(Boolean)
    .forEach((control) => {
        control.addEventListener("change", () => {
            const story = storyFromHash();
            if (applicationStories[story]) {
                queueMicrotask(() => renderApplicationsStory(story, { updateHash: false }));
            }
        });
    });

window.addEventListener("hashchange", () => {
    const story = storyFromHash();
    if (applicationStories[story]) renderApplicationsStory(story, { updateHash: false });
});

const initialHash = globalThis.__kubidmUiLabInitialHash || location.hash;
const initial = storyFromHash(initialHash);
if (applicationStories[initial]) renderApplicationsStory(initial);
delete globalThis.__kubidmUiLabInitialHash;
