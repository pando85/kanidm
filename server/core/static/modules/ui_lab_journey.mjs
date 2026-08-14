import { ScenarioId, scenarios, scenarioById, stepIndexForStory } from "./guide_scenarios.mjs";

const toolbar = document.querySelector(".ui-lab-toolbar");
const storyTitle = document.querySelector("#ui-lab-story-title");

if (!toolbar || !storyTitle) {
    throw new Error("Kubidm UI Lab journey controls require the lab toolbar");
}

const scenarioControl = document.createElement("section");
scenarioControl.className = "ui-lab-scenario-control";
scenarioControl.setAttribute("aria-label", "Canonical scenario navigator");
scenarioControl.innerHTML = `
    <label class="ui-lab-scenario-select">
        <span>Scenario</span>
        <select data-lab-scenario></select>
    </label>
    <div class="ui-lab-scenario-progress" aria-live="polite">
        <strong data-lab-scenario-title></strong>
        <span data-lab-scenario-step></span>
    </div>
    <div class="ui-lab-scenario-actions">
        <button type="button" class="btn btn-sm btn-outline-secondary" data-lab-scenario-prev>Previous</button>
        <button type="button" class="btn btn-sm btn-outline-secondary" data-lab-scenario-reset>Restart</button>
        <button type="button" class="btn btn-sm ui-lab-primary-action" data-lab-scenario-next>Next</button>
    </div>
`;

toolbar.insertAdjacentElement("afterend", scenarioControl);

const ui = {
    scenario: scenarioControl.querySelector("[data-lab-scenario]"),
    title: scenarioControl.querySelector("[data-lab-scenario-title]"),
    step: scenarioControl.querySelector("[data-lab-scenario-step]"),
    previous: scenarioControl.querySelector("[data-lab-scenario-prev]"),
    restart: scenarioControl.querySelector("[data-lab-scenario-reset]"),
    next: scenarioControl.querySelector("[data-lab-scenario-next]"),
};

for (const [id, scenario] of Object.entries(scenarios)) {
    const option = document.createElement("option");
    option.value = id;
    option.textContent = scenario.title;
    ui.scenario.append(option);
}

function storyFromHash() {
    return new URLSearchParams(location.hash.slice(1)).get("story") || "first-login";
}

function activeScenario() {
    return scenarioById(ui.scenario.value);
}

function navigateToStory(story) {
    const button = [...document.querySelectorAll("[data-story]")].find(
        (candidate) => candidate.dataset.story === story,
    );
    if (!button) {
        throw new Error(`UI Lab story is missing from the catalogue: ${story}`);
    }
    button.click();
}

function updateScenarioUi() {
    const scenario = activeScenario();
    const story = storyFromHash();
    const index = stepIndexForStory(scenario, story);

    ui.title.textContent = scenario.title;

    if (index < 0) {
        ui.step.textContent = "Current story is outside this scenario";
        ui.previous.disabled = true;
        ui.next.disabled = true;
        ui.restart.disabled = false;
        return;
    }

    const step = scenario.steps[index];
    ui.step.textContent = `Step ${index + 1} of ${scenario.steps.length} · ${step.stage}`;
    ui.previous.disabled = index === 0;
    ui.next.disabled = index === scenario.steps.length - 1;
    ui.restart.disabled = index === 0;
}

function move(delta) {
    const scenario = activeScenario();
    const index = stepIndexForStory(scenario, storyFromHash());
    if (index < 0) return;

    const target = scenario.steps[index + delta];
    if (target) navigateToStory(target.story);
}

ui.scenario.value = ScenarioId.PASSKEY_FIRST_RUN;
ui.scenario.addEventListener("change", () => {
    navigateToStory(activeScenario().steps[0].story);
});
ui.previous.addEventListener("click", () => move(-1));
ui.next.addEventListener("click", () => move(1));
ui.restart.addEventListener("click", () => navigateToStory(activeScenario().steps[0].story));

// The core lab uses history.replaceState, which intentionally does not emit a
// hashchange event. Observe the visible story title as the stable review signal
// and also listen for direct URL changes.
new MutationObserver(updateScenarioUi).observe(storyTitle, {
    childList: true,
    characterData: true,
    subtree: true,
});
window.addEventListener("hashchange", updateScenarioUi);
window.addEventListener("popstate", updateScenarioUi);

updateScenarioUi();
