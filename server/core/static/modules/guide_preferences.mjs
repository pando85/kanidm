const STORAGE_KEY = "kubidm.guide.v1";
const MAX_DISMISSALS = 2;
const MIN_REMINDER_INTERVAL_MS = 3 * 24 * 60 * 60 * 1000;

function defaultState() {
    return {
        version: 1,
        storiesSeen: [],
        suggestions: {},
        onboardingComplete: false,
    };
}

function safeParse(value) {
    if (!value) return defaultState();
    try {
        const parsed = JSON.parse(value);
        if (!parsed || parsed.version !== 1) return defaultState();
        return {
            version: 1,
            storiesSeen: Array.isArray(parsed.storiesSeen)
                ? parsed.storiesSeen.filter((item) => typeof item === "string")
                : [],
            suggestions: parsed.suggestions && typeof parsed.suggestions === "object" ? parsed.suggestions : {},
            // Accept the short-lived prototype field for forward compatibility
            // with browsers that may already have exercised this branch.
            onboardingComplete: parsed.onboardingComplete === true || parsed.journeyComplete === true,
        };
    } catch {
        return defaultState();
    }
}

export function readGuidePreferences() {
    try {
        return safeParse(localStorage.getItem(STORAGE_KEY));
    } catch {
        return defaultState();
    }
}

function writeGuidePreferences(state) {
    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
        // Storage can be disabled or unavailable. Guidance simply becomes
        // session-like; authentication must never depend on this state.
    }
    return state;
}

export function markStorySeen(storyId) {
    if (!storyId) return readGuidePreferences();
    const state = readGuidePreferences();
    if (!state.storiesSeen.includes(storyId)) state.storiesSeen.push(storyId);
    return writeGuidePreferences(state);
}

export function shouldTeachStory(storyId) {
    const state = readGuidePreferences();
    return !state.storiesSeen.includes(storyId);
}

export function recordSuggestionDismissal(suggestionId, now = Date.now()) {
    if (!suggestionId) return readGuidePreferences();
    const state = readGuidePreferences();
    const previous = state.suggestions[suggestionId] || { dismissals: 0, lastDismissedAt: 0 };
    state.suggestions[suggestionId] = {
        dismissals: Math.min(MAX_DISMISSALS, Number(previous.dismissals || 0) + 1),
        lastDismissedAt: now,
    };
    return writeGuidePreferences(state);
}

export function shouldShowSuggestion(suggestionId, now = Date.now()) {
    if (!suggestionId) return true;
    const state = readGuidePreferences();
    const item = state.suggestions[suggestionId];
    if (!item) return true;
    if (Number(item.dismissals || 0) >= MAX_DISMISSALS) return false;
    return now - Number(item.lastDismissedAt || 0) >= MIN_REMINDER_INTERVAL_MS;
}

export function markGuideOnboardingComplete() {
    const state = readGuidePreferences();
    state.onboardingComplete = true;
    return writeGuidePreferences(state);
}

export function guideExperienceLevel() {
    const state = readGuidePreferences();
    if (state.onboardingComplete) return "experienced";
    if (state.storiesSeen.length > 0 || Object.keys(state.suggestions).length > 0) return "learning";
    return "new";
}

export function resetGuidePreferences() {
    try {
        localStorage.removeItem(STORAGE_KEY);
    } catch {
        // Development helper only; no-op when storage is unavailable.
    }
}
