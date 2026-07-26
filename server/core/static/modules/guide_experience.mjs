import {
    guideExperienceLevel,
    markGuideOnboardingComplete,
    markStorySeen,
    recordSuggestionDismissal,
    shouldShowSuggestion,
    shouldTeachStory,
} from "./guide_preferences.mjs";

const boundDetails = new WeakSet();

function bindStory(node) {
    const storyId = node.dataset.guideStoryId;
    if (!storyId) return;

    const shouldShow = shouldTeachStory(storyId) || node.dataset.guideRepeat === "always";
    node.hidden = !shouldShow;
    if (!shouldShow) return;

    if (node instanceof HTMLDetailsElement) {
        if (boundDetails.has(node)) return;
        boundDetails.add(node);
        node.addEventListener("toggle", () => {
            if (node.open) markStorySeen(storyId);
        });
        return;
    }

    markStorySeen(storyId);
}

function bindSuggestion(node) {
    const suggestionId = node.dataset.guideSuggestionId;
    if (!suggestionId) return;
    const eligible = node.dataset.guideSuggestionEligible !== "false";
    node.hidden = !eligible || !shouldShowSuggestion(suggestionId);
}

export function syncGuideExperience(scene = document.querySelector("[data-guide-scene]")) {
    if (!(scene instanceof HTMLElement)) return;

    const experience = guideExperienceLevel();
    scene.dataset.guideExperience = experience;

    scene.querySelectorAll("[data-guide-new-only]").forEach((node) => {
        node.hidden = experience !== "new";
    });

    scene.querySelectorAll("[data-guide-story-id]").forEach(bindStory);
    scene.querySelectorAll("[data-guide-suggestion-id]").forEach(bindSuggestion);
}

export function completeGuideOnboarding() {
    markGuideOnboardingComplete();
    syncGuideExperience();
}

document.addEventListener("click", (event) => {
    const button = event.target.closest("[data-guide-dismiss-suggestion]");
    if (!button) return;
    const container = button.closest("[data-guide-suggestion-id]");
    if (!container) return;
    recordSuggestionDismissal(container.dataset.guideSuggestionId);
    container.hidden = true;
});

window.addEventListener("kubidm:guide-state", (event) => {
    const state = event.detail;
    if (state?.productState === "authentication_confirmed") {
        completeGuideOnboarding();
    }
});

document.body.addEventListener("htmx:afterSettle", () => syncGuideExperience());
syncGuideExperience();
