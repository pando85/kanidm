import { MotionLevel, normaliseGuideState } from "./guide_contract.mjs";

const stories = {
    "first-login": {
        title: "Meet / identify",
        productState: "identify",
        recommendation: "none",
        mascotState: "welcome",
        severity: "neutral",
        heading: "Welcome",
        subtitle: "Sign in to Acme Corp",
        dialog: {
            variant: "orient",
            text: "Hi. I’ll help you sign in and explain the security choices when they matter.",
        },
        content: "identify",
        next: "method-choice",
    },
    "method-choice": {
        title: "Choose authentication method",
        productState: "choose_method",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "How would you like to sign in?",
        subtitle: "Choose any method allowed for this account.",
        dialog: {
            variant: "suggest",
            text: "I recommend a passkey here. It’s quick to use and designed to resist phishing. A password works too if you prefer it.",
        },
        content: "method-choice",
    },
    "passkey-story": {
        title: "Passkey micro-story",
        productState: "teach_passkey",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "Why a passkey?",
        subtitle: "A short explanation before you choose.",
        dialog: {
            variant: "teach",
            text: "A passkey proves it’s you without sending its private key to Kubidm. It is also designed to work with the correct site, which makes phishing much harder.",
        },
        content: "passkey-story",
    },
    "passkey-working": {
        title: "Native WebAuthn active",
        productState: "webauthn_pending",
        recommendation: "recommended",
        mascotState: "working",
        severity: "neutral",
        heading: "Check your device",
        subtitle: "Your browser or device is handling the next step.",
        dialog: null,
        content: "working",
    },
    success: {
        title: "Confirmed authentication",
        productState: "authentication_confirmed",
        recommendation: "none",
        mascotState: "success",
        severity: "positive",
        heading: "You’re signed in",
        subtitle: "Kubidm confirmed your identity.",
        dialog: {
            variant: "celebrate",
            text: "Done. I’ll take you to your applications.",
        },
        content: "success",
    },
    "password-ok": {
        title: "Password — Works OK",
        productState: "password_selected",
        recommendation: "works_ok",
        mascotState: "guide",
        severity: "neutral",
        heading: "Use your password",
        subtitle: "This is a valid option for the current account policy.",
        dialog: {
            variant: "orient",
            text: "That works. If you want, we can add a passkey later for a faster, phishing-resistant sign-in.",
        },
        content: "password",
    },
    "webauthn-cancel": {
        title: "WebAuthn interrupted",
        productState: "webauthn_interrupted",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "No problem",
        subtitle: "The WebAuthn request did not complete.",
        dialog: {
            variant: "orient",
            text: "You can try again when you’re ready, or choose another sign-in method if one is available.",
        },
        content: "cancelled",
    },
    oauth: {
        title: "OAuth destination",
        productState: "oauth_context",
        recommendation: "none",
        mascotState: "guide",
        severity: "neutral",
        heading: "Sign in to continue",
        subtitle: "Grafana wants you to authenticate with Acme Corp.",
        dialog: {
            variant: "orient",
            text: "I’ll verify your Acme identity, then send you back to Grafana.",
        },
        content: "oauth",
    },
    reauth: {
        title: "Reauthentication",
        productState: "reauthentication",
        recommendation: "required",
        mascotState: "protect",
        severity: "caution",
        heading: "Confirm it’s you",
        subtitle: "Kubidm needs another check before this sensitive action.",
        dialog: {
            variant: "orient",
            text: "This extra check protects a security-sensitive change. Use one of the methods your account allows.",
        },
        content: "reauth",
    },
    "policy-required": {
        title: "Policy-required action",
        productState: "policy_required",
        recommendation: "required",
        mascotState: "protect",
        severity: "caution",
        heading: "One security step is required",
        subtitle: "Your organisation’s policy requires this before you can continue.",
        dialog: {
            variant: "orient",
            text: "This one isn’t optional. I’ll show you what the policy requires and why the normal UI is blocking progress.",
        },
        content: "policy-required",
    },
    returning: {
        title: "Returning configured user",
        productState: "normal_login",
        recommendation: "recommended",
        mascotState: "idle",
        severity: "neutral",
        heading: "Welcome back",
        subtitle: "Sign in to Acme Corp",
        dialog: null,
        content: "returning",
    },
    resilience: {
        title: "Resilience suggestion",
        productState: "resilience_available",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "You can sign in now",
        subtitle: "There’s one more thing worth considering.",
        dialog: {
            variant: "suggest",
            text: "Your primary sign-in is ready. If policy and your setup support it, a backup path can help if your usual device is unavailable.",
        },
        content: "resilience",
    },
    "credentials-progress": {
        title: "Credential progress",
        productState: "credential_setup",
        recommendation: "optional",
        mascotState: "idle",
        severity: "neutral",
        heading: "Your sign-in setup",
        subtitle: "Progress without a security score.",
        dialog: {
            variant: "orient",
            text: "I’ll show what is configured, what policy still requires, and what is simply optional.",
        },
        content: "credentials-progress",
    },
    complete: {
        title: "Journey complete",
        productState: "recommended_setup_complete",
        recommendation: "none",
        mascotState: "success",
        severity: "positive",
        heading: "You’re ready",
        subtitle: "The recommended setup for this fixture is complete.",
        dialog: {
            variant: "celebrate",
            text: "All set. I’ll stay out of the way unless something changes or you ask for help.",
        },
        content: "complete",
    },
    goodbye: {
        title: "Goodbye",
        productState: "logout",
        recommendation: "none",
        mascotState: "goodbye",
        severity: "neutral",
        heading: "See you next time",
        subtitle: "Logout is never delayed for mascot motion.",
        dialog: {
            variant: "orient",
            text: "I’ll be here next time you sign in.",
        },
        content: "goodbye",
    },
    "component-dialog": {
        title: "Crab Dialog variants",
        productState: "component_preview",
        recommendation: "none",
        mascotState: "idle",
        severity: "neutral",
        heading: "Crab Dialog",
        subtitle: "Accessible HTML content, separate from animation.",
        dialog: null,
        content: "component-dialog",
    },
    "component-options": {
        title: "Recommendation taxonomy",
        productState: "component_preview",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "Recommendation options",
        subtitle: "Required, Recommended, Works OK, and Optional are contextual presentation states.",
        dialog: null,
        content: "component-options",
    },
    "component-notice": {
        title: "Security notices",
        productState: "component_preview",
        recommendation: "none",
        mascotState: "warning",
        severity: "caution",
        heading: "Authoritative security notices",
        subtitle: "The normal UI owns warnings and errors.",
        dialog: null,
        content: "component-notice",
    },
};

const ui = {
    canvas: document.querySelector("#ui-lab-canvas"),
    preview: document.querySelector("#ui-lab-preview"),
    title: document.querySelector("#ui-lab-story-title"),
    productState: document.querySelector("#ui-lab-product-state"),
    recommendation: document.querySelector("#ui-lab-recommendation"),
    mascotState: document.querySelector("#ui-lab-mascot-state"),
    severity: document.querySelector("#ui-lab-severity"),
    theme: document.querySelector("#ui-lab-theme"),
    viewport: document.querySelector("#ui-lab-viewport"),
    motion: document.querySelector("#ui-lab-motion"),
};

function escapeHtml(value) {
    return String(value)
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#039;");
}

function dialogMarkup(dialog) {
    if (!dialog) return "";
    return `<section class="ui-lab-dialog" data-variant="${escapeHtml(dialog.variant)}">
        <span class="ui-lab-dialog-label">${escapeHtml(dialog.variant)}</span>
        <p>${escapeHtml(dialog.text)}</p>
    </section>`;
}

function optionMarkup(title, reason, recommendation, primary = false) {
    return `<button type="button" class="ui-lab-option${primary ? " ui-lab-option-primary" : ""}">
        <span>
            <strong>${escapeHtml(title)}</strong>
            <small>${escapeHtml(reason)}</small>
        </span>
        <span class="ui-lab-chip" data-kind="${escapeHtml(recommendation)}">${escapeHtml(recommendation)}</span>
    </button>`;
}

function progressMarkup(items) {
    return `<ol class="ui-lab-progress">
        ${items
            .map(
                (item) => `<li data-complete="${item.complete}">
                    <span class="ui-lab-progress-marker"></span>
                    <span><strong>${escapeHtml(item.title)}</strong><small>${escapeHtml(item.detail)}</small></span>
                </li>`,
            )
            .join("")}
    </ol>`;
}

function noticeMarkup(title, text, severity) {
    return `<section class="ui-lab-notice" data-severity="${escapeHtml(severity)}" role="${severity === "critical" ? "alert" : "status"}">
        <strong>${escapeHtml(title)}</strong>
        <p>${escapeHtml(text)}</p>
    </section>`;
}

function mascotMarkup(state) {
    const label = escapeHtml(state);
    return `<div class="ui-lab-mascot-slot" data-mascot-state="${label}">
        <img data-lab-mascot-image src="/pkg/img/guide/crab-${label}.webp" alt="Kubidm guide: ${label}" />
        <div data-lab-mascot-fallback class="ui-lab-mascot-fallback" hidden>
            <span>Mascot asset slot<br><strong>${label}</strong></span>
        </div>
    </div>`;
}

function storyContent(story) {
    switch (story.content) {
        case "identify":
            return `<form class="ui-lab-form" onsubmit="return false">
                <label>Username<input class="form-control" value="alex" /></label>
                <label class="form-check"><input type="checkbox" class="form-check-input" checked /> Remember my username</label>
                <button type="button" class="btn ui-lab-primary-action" data-go-story="method-choice">Continue</button>
            </form>`;
        case "method-choice":
            return `<div class="ui-lab-option-list">
                ${optionMarkup("Use a passkey", "Quick and phishing-resistant", "Recommended", true)}
                ${optionMarkup("Use a password", "Valid for this account", "Works OK")}
                ${optionMarkup("Other methods", "Show other policy-allowed choices", "Optional")}
            </div>`;
        case "passkey-story":
            return `<div class="ui-lab-story-card">
                <div class="ui-lab-story-step"><span>1</span><p>Your device creates or holds a cryptographic credential.</p></div>
                <div class="ui-lab-story-step"><span>2</span><p>The private key is not sent to Kubidm during authentication.</p></div>
                <div class="ui-lab-story-step"><span>3</span><p>The credential is designed to work with the correct site, helping resist phishing.</p></div>
                <button type="button" class="btn ui-lab-primary-action" data-go-story="passkey-working">Use a passkey</button>
            </div>`;
        case "working":
            return `<div class="ui-lab-native-placeholder">
                <span class="ui-lab-activity-dot"></span>
                <strong>Browser / OS passkey UI is active</strong>
                <p>The mascot becomes quieter while the native interface has focus.</p>
            </div>`;
        case "success":
            return `${noticeMarkup("Identity verified", "Authentication was confirmed by Kubidm.", "positive")}
                <button type="button" class="btn ui-lab-primary-action" data-go-story="applications-arrival">Continue</button>`;
        case "password":
            return `<form class="ui-lab-form" onsubmit="return false">
                <label>Password<input type="password" class="form-control" value="example-password" /></label>
                <button type="button" class="btn ui-lab-primary-action" data-go-story="success">Sign in</button>
            </form>`;
        case "cancelled":
            return `<div class="ui-lab-option-list">
                ${optionMarkup("Try passkey again", "Open the native passkey flow again", "Recommended", true)}
                ${optionMarkup("Choose another method", "Return to available sign-in methods", "Works OK")}
            </div>`;
        case "oauth":
            return `<div class="ui-lab-destination-card"><div class="ui-lab-app-icon">G</div><div><strong>Grafana</strong><small>Application destination</small></div></div>
                ${optionMarkup("Use a passkey", "Authenticate to Acme, then return to Grafana", "Recommended", true)}`;
        case "reauth":
            return `${noticeMarkup("Security-sensitive action", "This reauthentication is required before changing credentials.", "caution")}
                ${optionMarkup("Use a passkey", "Confirm your identity", "Required", true)}`;
        case "policy-required":
            return `${noticeMarkup("Passkey required", "The fixture policy requires a passkey before credential changes can be saved.", "caution")}
                ${optionMarkup("Set up a passkey", "Required by the current policy", "Required", true)}`;
        case "returning":
            return `<div class="ui-lab-option-list">
                ${optionMarkup("Use a passkey", "Your usual sign-in method", "Recommended", true)}
                <button type="button" class="btn btn-link text-body-secondary">Other methods</button>
            </div>`;
        case "resilience":
            return `${progressMarkup([
                { title: "You can sign in", detail: "Primary authentication is ready", complete: true },
                { title: "Recommended method", detail: "Passkey configured", complete: true },
                { title: "Backup path", detail: "Not configured in this fixture", complete: false },
            ])}
            <div class="ui-lab-option-list">
                ${optionMarkup("Review backup options", "See what this policy supports", "Recommended", true)}
                ${optionMarkup("Not now", "You can revisit this later", "Optional")}
            </div>`;
        case "credentials-progress":
            return progressMarkup([
                { title: "Can sign in", detail: "At least one policy-valid method", complete: true },
                { title: "Passkey", detail: "Recommended primary method", complete: true },
                { title: "Backup", detail: "Additional resilience", complete: false },
                { title: "Recovery", detail: "Depends on domain configuration", complete: false },
            ]);
        case "complete":
            return `${progressMarkup([
                { title: "Can sign in", detail: "Ready", complete: true },
                { title: "Recommended method", detail: "Ready", complete: true },
                { title: "Resilience", detail: "Ready for this fixture", complete: true },
            ])}
            <p class="text-body-secondary">Routine sign-ins now become quiet. The guide returns when context changes or a security action needs attention.</p>`;
        case "goodbye":
            return `<div class="ui-lab-native-placeholder"><strong>Logout continues immediately</strong><p>The guide may wave once, but navigation never waits for animation.</p></div>`;
        case "component-dialog":
            return `<div class="ui-lab-component-stack">
                ${dialogMarkup({ variant: "orient", text: "I’ll help you understand what comes next." })}
                ${dialogMarkup({ variant: "teach", text: "A passkey uses cryptography instead of a reusable secret you type into a page." })}
                ${dialogMarkup({ variant: "suggest", text: "I recommend a passkey here, but the password option is valid too." })}
                ${dialogMarkup({ variant: "celebrate", text: "Your passkey is ready." })}
            </div>`;
        case "component-options":
            return `<div class="ui-lab-option-list">
                ${optionMarkup("Required action", "Policy prevents progress without this", "Required")}
                ${optionMarkup("Recommended action", "Preferred for this context", "Recommended", true)}
                ${optionMarkup("Valid alternative", "Supported without negative treatment", "Works OK")}
                ${optionMarkup("Extra resilience", "Safe to skip when policy permits", "Optional")}
            </div>`;
        case "component-notice":
            return `<div class="ui-lab-component-stack">
                ${noticeMarkup("Informational", "Normal product information remains normal UI.", "neutral")}
                ${noticeMarkup("Attention", "Policy requires another action.", "caution")}
                ${noticeMarkup("Account locked", "The authoritative UI communicates the critical state. The mascot stays almost still.", "critical")}
            </div>`;
        default:
            return "";
    }
}

function renderStory(name, { updateHash = true } = {}) {
    const story = stories[name] || stories["first-login"];
    ui.title.textContent = story.title;
    ui.productState.textContent = story.productState;
    ui.recommendation.textContent = story.recommendation;
    ui.mascotState.textContent = story.mascotState;
    ui.severity.textContent = story.severity;

    ui.canvas.innerHTML = `<section class="ui-lab-auth-shell" data-story-name="${escapeHtml(name)}" data-guide-action="${escapeHtml(story.productState)}" data-guide-severity="${escapeHtml(story.severity)}">
        <aside class="ui-lab-product-zone">
            <div class="ui-lab-product-mark">kubi<span>dm</span></div>
            <div class="ui-lab-product-copy">
                <h2>Identity that guides every step.</h2>
                <p>Secure identity for cloud-native infrastructure.</p>
            </div>
            ${mascotMarkup(story.mascotState)}
        </aside>
        <section class="ui-lab-task-zone">
            <div class="ui-lab-auth-card">
                <div class="ui-lab-tenant">
                    <div class="ui-lab-tenant-logo" aria-hidden="true">A</div>
                    <div>
                        <strong>Acme Corp</strong>
                        <div class="small text-body-secondary">Identity domain</div>
                    </div>
                </div>
                <h2>${escapeHtml(story.heading)}</h2>
                <p class="ui-lab-subtitle">${escapeHtml(story.subtitle)}</p>
                ${dialogMarkup(story.dialog)}
                ${storyContent(story)}
            </div>
        </section>
    </section>`;

    document.querySelectorAll("[data-story]").forEach((button) => {
        const active = button.dataset.story === name;
        button.setAttribute("aria-current", active ? "true" : "false");
    });

    const image = ui.canvas.querySelector("[data-lab-mascot-image]");
    const fallback = ui.canvas.querySelector("[data-lab-mascot-fallback]");
    if (image && fallback) {
        const showFallback = () => {
            image.hidden = true;
            fallback.hidden = false;
        };
        image.addEventListener("error", showFallback, { once: true });
        if (image.complete && image.naturalWidth === 0) showFallback();
    }

    const state = normaliseGuideState({
        productState: story.productState,
        recommendation: story.recommendation,
        mascotState: story.mascotState,
        severity: story.severity,
        motionLevel: Object.values(MotionLevel).includes(ui.motion.value)
            ? ui.motion.value
            : MotionLevel.STATIC,
    });
    window.dispatchEvent(new CustomEvent("kubidm:guide-state", { detail: state }));

    if (updateHash) writeHash(name);
}

function writeHash(story) {
    const params = new URLSearchParams();
    params.set("story", story);
    params.set("theme", ui.theme.value);
    params.set("viewport", ui.viewport.value);
    params.set("motion", ui.motion.value);
    history.replaceState(null, "", `#${params.toString()}`);
}

function applyControls({ updateHash = true } = {}) {
    document.documentElement.setAttribute("data-bs-theme", ui.theme.value);
    ui.preview.dataset.viewport = ui.viewport.value;
    ui.preview.dataset.motion = ui.motion.value;
    if (updateHash) {
        const current = new URLSearchParams(location.hash.slice(1)).get("story") || "first-login";
        writeHash(current);
        renderStory(current, { updateHash: false });
    }
}

function initialiseFromHash() {
    const params = new URLSearchParams(location.hash.slice(1));
    const story = params.get("story") || "first-login";
    const theme = params.get("theme");
    const viewport = params.get("viewport");
    const motion = params.get("motion");

    if (["light", "dark"].includes(theme)) ui.theme.value = theme;
    if (["desktop", "tablet", "mobile"].includes(viewport)) ui.viewport.value = viewport;
    if (["full", "reduced", "static"].includes(motion)) ui.motion.value = motion;

    applyControls({ updateHash: false });
    renderStory(story, { updateHash: false });
    writeHash(stories[story] ? story : "first-login");
}

document.addEventListener("click", (event) => {
    const target = event.target.closest("[data-story], [data-go-story]");
    if (!target) return;
    const story = target.dataset.story || target.dataset.goStory;
    if (stories[story]) renderStory(story);
});

[ui.theme, ui.viewport, ui.motion].forEach((control) => {
    control.addEventListener("change", () => applyControls());
});

window.addEventListener("hashchange", initialiseFromHash);

initialiseFromHash();
