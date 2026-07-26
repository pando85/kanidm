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
            text: "Your device proves it has the right credential without sending the private key to Kubidm. The credential is also designed to work with the correct site, which makes phishing much harder.",
        },
        content: "passkey-story",
        next: "passkey-working",
    },
    "passkey-working": {
        title: "Passkey working",
        productState: "webauthn_pending",
        recommendation: "recommended",
        mascotState: "working",
        severity: "neutral",
        heading: "Use your passkey",
        subtitle: "Follow your browser or device prompt.",
        dialog: {
            variant: "orient",
            text: "I’ll stay quiet while your device handles this part.",
        },
        content: "working",
        next: "success",
    },
    success: {
        title: "Confirmed success",
        productState: "authentication_confirmed",
        recommendation: "none",
        mascotState: "success",
        severity: "positive",
        heading: "You’re signed in",
        subtitle: "Your identity was confirmed.",
        dialog: {
            variant: "celebrate",
            text: "Nice. You’re in. I’ll keep the celebrations small so this still feels good on login number five hundred.",
        },
        content: "success",
        next: "resilience",
    },
    "password-ok": {
        title: "Password — Works OK",
        productState: "password_selected",
        recommendation: "works_ok",
        mascotState: "guide",
        severity: "neutral",
        heading: "Sign in with your password",
        subtitle: "This is a valid option for your account.",
        dialog: {
            variant: "orient",
            text: "That works. If you want, we can add a passkey later for faster, phishing-resistant sign-in.",
        },
        content: "password",
    },
    "webauthn-cancel": {
        title: "WebAuthn cancelled",
        productState: "webauthn_cancelled",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "Passkey prompt closed",
        subtitle: "Nothing changed.",
        dialog: {
            variant: "orient",
            text: "No problem. Try again when you’re ready, or choose another allowed sign-in method.",
        },
        content: "cancelled",
    },
    oauth: {
        title: "OAuth destination",
        productState: "oauth_login",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "Sign in to continue",
        subtitle: "Grafana is asking Acme Corp to confirm your identity.",
        dialog: {
            variant: "orient",
            text: "You’re signing in through Acme Corp so you can continue to Grafana.",
        },
        content: "oauth",
    },
    reauth: {
        title: "Reauthentication",
        productState: "reauthentication",
        recommendation: "required",
        mascotState: "protect",
        severity: "neutral",
        heading: "Confirm it’s you",
        subtitle: "Before changing your credentials, Kubidm needs to verify your identity again.",
        dialog: {
            variant: "orient",
            text: "This extra check protects a sensitive change. Use one of the methods allowed for reauthentication.",
        },
        content: "method-choice",
    },
    "policy-required": {
        title: "Policy-required action",
        productState: "policy_required",
        recommendation: "required",
        mascotState: "protect",
        severity: "caution",
        heading: "A security step is required",
        subtitle: "Your organisation requires a stronger sign-in method before you continue.",
        dialog: null,
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
        heading: "Want a backup way in?",
        subtitle: "Your primary sign-in works. A backup can help if your normal device is unavailable.",
        dialog: {
            variant: "suggest",
            text: "You’re in good shape. I’d add a backup method next so one lost device doesn’t become a support ticket.",
        },
        content: "resilience",
        next: "credentials-progress",
    },
    "credentials-progress": {
        title: "Credential progress",
        productState: "credential_setup",
        recommendation: "optional",
        mascotState: "idle",
        severity: "neutral",
        heading: "Your identity journey",
        subtitle: "Progress, not a security score.",
        dialog: {
            variant: "orient",
            text: "You can already sign in. These next steps improve resilience without turning security into a points game.",
        },
        content: "progress",
        next: "complete",
    },
    complete: {
        title: "Journey complete",
        productState: "recommended_setup_complete",
        recommendation: "none",
        mascotState: "success",
        severity: "positive",
        heading: "You’re ready",
        subtitle: "Your recommended identity setup is complete.",
        dialog: {
            variant: "celebrate",
            text: "All set. I’ll stay out of the way now and show up when something actually needs your attention.",
        },
        content: "complete",
    },
    "component-dialog": {
        title: "Primitive: Crab Dialog",
        productState: "component_preview",
        recommendation: "none",
        mascotState: "guide",
        severity: "neutral",
        heading: "Crab Dialog variants",
        subtitle: "Teaching is accessible HTML, not text inside animation.",
        dialog: null,
        content: "component-dialog",
    },
    "component-options": {
        title: "Primitive: Recommendation options",
        productState: "component_preview",
        recommendation: "recommended",
        mascotState: "guide",
        severity: "neutral",
        heading: "Recommendation taxonomy",
        subtitle: "Required, Recommended, Works OK, and Optional are contextual product semantics.",
        dialog: null,
        content: "component-options",
    },
    "component-notice": {
        title: "Primitive: Security notice",
        productState: "component_preview",
        recommendation: "required",
        mascotState: "warning",
        severity: "caution",
        heading: "Authoritative security UI",
        subtitle: "The mascot may support this state, but never replaces the notice.",
        dialog: null,
        content: "component-notice",
    },
};

const ui = {
    canvas: document.querySelector("#ui-lab-canvas"),
    preview: document.querySelector("#ui-lab-preview"),
    theme: document.querySelector("#ui-lab-theme"),
    viewport: document.querySelector("#ui-lab-viewport"),
    motion: document.querySelector("#ui-lab-motion"),
    title: document.querySelector("#ui-lab-story-title"),
    productState: document.querySelector("#ui-lab-product-state"),
    recommendation: document.querySelector("#ui-lab-recommendation"),
    mascotState: document.querySelector("#ui-lab-mascot-state"),
    severity: document.querySelector("#ui-lab-severity"),
};

if (!ui.canvas) {
    throw new Error("Kubidm UI Lab canvas is missing");
}

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
    return `<section class="crab-dialog" data-variant="${escapeHtml(dialog.variant)}">
        <p>${escapeHtml(dialog.text)}</p>
    </section>`;
}

function optionMarkup({ title, reason, recommendation, story }) {
    const chip = recommendation
        ? `<span class="ui-lab-chip" data-kind="${escapeHtml(recommendation)}">${escapeHtml(labelForRecommendation(recommendation))}</span>`
        : "";
    const storyAttr = story ? ` data-go-story="${escapeHtml(story)}"` : "";
    return `<button type="button" class="ui-lab-option" data-recommendation="${escapeHtml(recommendation || "none")}"${storyAttr}>
        <span>
            <span class="ui-lab-option-title">${escapeHtml(title)}</span>
            <span class="ui-lab-option-reason">${escapeHtml(reason)}</span>
        </span>
        ${chip}
    </button>`;
}

function labelForRecommendation(value) {
    return {
        required: "Required",
        recommended: "Recommended",
        works_ok: "Works OK",
        optional: "Optional",
    }[value] || value;
}

function storyContent(story) {
    switch (story.content) {
        case "identify":
            return `<form data-lab-form>
                <label class="form-label" for="lab-username">Username</label>
                <input id="lab-username" class="form-control" autocomplete="username" value="alex@example.com" />
                <div class="form-check form-switch mt-3">
                    <input id="lab-remember" class="form-check-input" type="checkbox" checked />
                    <label class="form-check-label" for="lab-remember">Remember my username</label>
                </div>
                <div class="ui-lab-actions">
                    <button type="button" class="btn ui-lab-primary-action" data-go-story="method-choice">Continue</button>
                    <button type="button" class="btn btn-link">Recover account</button>
                </div>
            </form>`;
        case "method-choice":
            return `<div class="ui-lab-options">
                ${optionMarkup({ title: "Use a passkey", reason: "Fast and designed to resist phishing.", recommendation: story.recommendation === "required" ? "required" : "recommended", story: "passkey-story" })}
                ${optionMarkup({ title: "Use a password", reason: "Valid for this account and policy.", recommendation: "works_ok", story: "password-ok" })}
                ${optionMarkup({ title: "Other sign-in options", reason: "Show other mechanisms available to this account.", recommendation: "optional" })}
            </div>`;
        case "passkey-story":
            return `<div class="ui-lab-story-card">
                <div class="ui-lab-story-flow">
                    <div class="ui-lab-story-icon">Device</div>
                    <div aria-hidden="true">→</div>
                    <div class="ui-lab-story-icon">Site-bound<br />proof</div>
                </div>
            </div>
            <p class="text-body-secondary">The story is deliberately short. Deeper technical detail belongs behind a Learn more action.</p>
            <div class="ui-lab-actions">
                <button type="button" class="btn ui-lab-primary-action" data-go-story="passkey-working">Use a passkey</button>
                <button type="button" class="btn btn-outline-secondary" data-go-story="password-ok">Choose another method</button>
            </div>`;
        case "working":
            return `<div class="ui-lab-system-indicator" aria-label="Authentication in progress">ID</div>
                <p class="text-center text-body-secondary">Waiting for the browser/device passkey prompt…</p>
                <div class="ui-lab-actions justify-content-center">
                    <button type="button" class="btn btn-outline-secondary" data-go-story="webauthn-cancel">Simulate cancel</button>
                    <button type="button" class="btn ui-lab-primary-action" data-go-story="success">Simulate confirmed success</button>
                </div>`;
        case "success":
            return `<div class="ui-lab-notice"><strong>Authentication confirmed.</strong> This success state is only shown after product/server confirmation.</div>
                <div class="ui-lab-actions">
                    <button type="button" class="btn ui-lab-primary-action" data-go-story="resilience">Continue</button>
                </div>`;
        case "password":
            return `<form data-lab-form>
                <label class="form-label" for="lab-password">Password</label>
                <input id="lab-password" class="form-control" type="password" autocomplete="current-password" value="not-a-real-password" />
                <div class="ui-lab-actions">
                    <button type="button" class="btn ui-lab-primary-action" data-go-story="success">Sign in</button>
                    <button type="button" class="btn btn-link" data-go-story="method-choice">Other methods</button>
                </div>
            </form>`;
        case "cancelled":
            return `<div class="ui-lab-notice">The passkey prompt was closed. No credential or account state changed.</div>
                <div class="ui-lab-actions">
                    <button type="button" class="btn ui-lab-primary-action" data-go-story="passkey-working">Try again</button>
                    <button type="button" class="btn btn-outline-secondary" data-go-story="method-choice">Other methods</button>
                </div>`;
        case "oauth":
            return `<div class="ui-lab-story-card">
                <strong>Destination</strong>
                <p class="mb-0 mt-2">Grafana</p>
                <small class="text-body-secondary">Authentication is provided by Acme Corp through Kubidm.</small>
            </div>
            <div class="ui-lab-options">
                ${optionMarkup({ title: "Use a passkey", reason: "Recommended for this sign-in.", recommendation: "recommended", story: "passkey-working" })}
                ${optionMarkup({ title: "Other methods", reason: "Use another mechanism allowed for this account.", recommendation: "optional", story: "method-choice" })}
            </div>`;
        case "policy-required":
            return `<div class="ui-lab-notice" data-severity="caution" role="alert">
                <strong>Action required.</strong> Your organisation requires a stronger authentication method before this workflow can continue.
            </div>
            <div class="ui-lab-options">
                ${optionMarkup({ title: "Set up a passkey", reason: "Required by the active account/domain policy.", recommendation: "required", story: "passkey-story" })}
            </div>`;
        case "returning":
            return `<div class="ui-lab-options">
                ${optionMarkup({ title: "Use a passkey", reason: "Your normal sign-in method.", recommendation: "recommended", story: "passkey-working" })}
                ${optionMarkup({ title: "Other sign-in options", reason: "Available if you need them.", recommendation: "optional", story: "method-choice" })}
            </div>
            <p class="small text-body-secondary">No teaching dialog: configured/experienced users get the shortest path.</p>`;
        case "resilience":
            return `<div class="ui-lab-options">
                ${optionMarkup({ title: "Add a backup method", reason: "Helps when your normal device is unavailable.", recommendation: "recommended", story: "credentials-progress" })}
                ${optionMarkup({ title: "Not now", reason: "You can return to this later if policy permits.", recommendation: "optional", story: "credentials-progress" })}
            </div>`;
        case "progress":
            return progressMarkup([true, true, true, false]);
        case "complete":
            return `${progressMarkup([true, true, true, true])}
                <div class="ui-lab-notice"><strong>Recommended setup complete.</strong> The guide can now decay to companion mode.</div>`;
        case "component-dialog":
            return `<div class="crab-dialog" data-variant="orient"><p>Orient: explain where the user is or what happens next.</p></div>
                <div class="crab-dialog" data-variant="teach"><p>Teach: explain one security idea in normal language.</p></div>
                <div class="crab-dialog" data-variant="suggest"><p>Suggest: recommend an optional next step and explain why.</p></div>
                <div class="crab-dialog" data-variant="celebrate"><p>Celebrate: acknowledge a confirmed milestone without overdoing it.</p></div>`;
        case "component-options":
            return `<div class="ui-lab-options">
                ${optionMarkup({ title: "Policy action", reason: "The workflow cannot proceed without this.", recommendation: "required" })}
                ${optionMarkup({ title: "Passkey", reason: "Preferred for this context.", recommendation: "recommended" })}
                ${optionMarkup({ title: "Password", reason: "Valid supported alternative.", recommendation: "works_ok" })}
                ${optionMarkup({ title: "Backup method", reason: "Useful extra resilience.", recommendation: "optional" })}
            </div>`;
        case "component-notice":
            return `<div class="ui-lab-notice">Neutral product information.</div>
                <div class="ui-lab-notice" data-severity="caution" role="alert">Caution: authoritative UI takes priority over mascot personality.</div>
                <div class="ui-lab-notice" data-severity="critical" role="alert">Critical: mascot movement becomes minimal or static.</div>`;
        default:
            return "";
    }
}

function progressMarkup(completed) {
    const labels = ["You can sign in", "Recommended primary method", "Backup method available", "Recovery/resilience ready"];
    return `<div class="ui-lab-progress">
        <ol>${labels.map((label, index) => `<li data-complete="${completed[index] ? "true" : "false"}">${escapeHtml(label)}</li>`).join("")}</ol>
    </div>`;
}

function mascotMarkup(state) {
    const safeState = ["idle", "welcome", "guide", "protect", "working", "success", "warning", "goodbye"].includes(state)
        ? state
        : "idle";
    return `<div class="ui-lab-mascot-slot" data-mascot-state="${safeState}">
        <img src="/pkg/img/guide/crab-${safeState}.svg" alt="Kubidm guide: ${safeState}" data-lab-mascot-image />
        <div class="ui-lab-mascot-fallback" data-lab-mascot-fallback hidden>
            <span>Mascot asset slot<br /><strong>${escapeHtml(safeState)}</strong></span>
        </div>
    </div>`;
}

function renderStory(name, { updateHash = true } = {}) {
    const story = stories[name] || stories["first-login"];

    ui.title.textContent = story.title;
    ui.productState.textContent = story.productState;
    ui.recommendation.textContent = story.recommendation;
    ui.mascotState.textContent = story.mascotState;
    ui.severity.textContent = story.severity;

    ui.canvas.innerHTML = `<section class="ui-lab-auth-shell" data-story-name="${escapeHtml(name)}">
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
