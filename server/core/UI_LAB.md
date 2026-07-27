# Kubidm UI Lab

The Kubidm UI Lab is a development-only, Storybook-style harness for the mascot-guided identity experience. It uses the same Askama, HTMX, Bootstrap, CSS, vanilla JavaScript, routing, CSP, and production guide modules as Kubidm rather than introducing a second frontend framework.

## Safety

The route has two independent gates:

1. it is compiled only when Rust `debug_assertions` are enabled; and
2. it is registered only when `KUBIDM_UI_LAB` is present.

Release builds do not contain the route. The lab uses fixture identities and simulated outcomes; it is not an authentication endpoint and must not be enabled in production.

## Run the UI Lab

```bash
KUBIDM_UI_LAB=1 KUBIDM_GUIDED_UI=full make run
```

Open:

```text
https://localhost:8443/ui/_lab
```

The development certificate may need to be accepted by the browser.

The product experience has an independent deployment-level presentation switch:

```text
KUBIDM_GUIDED_UI=off      legacy presentation
KUBIDM_GUIDED_UI=subtle   compact feedback, proactive teaching reduced
KUBIDM_GUIDED_UI=full     proactive teaching/recommendations/mascot presence
```

The switch changes presentation only. It does not alter authentication policy, mechanism ordering, credential validation, session issuance, endpoint behavior, or form payloads.

## What the lab covers

The lab contains deterministic stories for:

- first encounter/account identification;
- method recommendation and valid alternatives;
- passkey teaching;
- native WebAuthn pending state;
- confirmed authentication;
- Applications travel and settled arrival;
- password as **Works OK**;
- WebAuthn cancellation;
- OAuth destination context;
- reauthentication;
- policy-required action;
- returning experienced user;
- resilience suggestion;
- credential progress;
- journey completion;
- logout/goodbye;
- Crab Dialog variants;
- recommendation taxonomy; and
- authoritative security notices.

Scenario A follows the real product order:

```text
identify
  -> choose
  -> teach passkey
  -> native WebAuthn
  -> server-confirmed success
  -> Applications travel
  -> Applications idle
  -> optional credential improvement
```

## Review controls

The lab separates four independent concerns:

```text
theme:       light | dark
viewport:    desktop | tablet | mobile
motion:      full | reduced | static
guide mode:  full | subtle | off
```

Theme, viewport, motion, and story are linkable through the URL fragment. The operating-system `prefers-reduced-motion` setting always overrides a Full motion selection.

A runtime panel records semantic `kubidm:guide-state`, `kubidm:guide-diagnostics`, and WebAuthn lifecycle events.

## Semantic contract

Every story exposes renderer-independent product meaning such as:

```text
product state
recommendation
mascot state
severity
motion level
travel direction / gaze where relevant
journey stage
```

The bounded vocabulary lives in `static/modules/guide_contract.mjs`. Stories and product code describe meaning, not animation clip or rig names.

## Production renderer boundary

`static/modules/guide_renderer.mjs` owns renderer selection:

```text
product / policy state
        -> guidance semantics
        -> GuideRendererController
             -> RiveGuideRenderer       full
             -> StaticGuideRenderer     reduced/static/failure
```

### Full motion

Full internal character animation is Rive-only. The production runtime path is:

```text
static/rive/rive.js
static/rive/rive.wasm
static/img/guide/kubidm-guide.riv
```

The web runtime is pinned/self-hosted and the public WASM fallback URL is explicitly disabled. No CDN is part of the production renderer.

The real `.riv` contract is defined in:

```text
static/guide_rive_contract.json
```

Required identities:

```text
Artboard:      KubidmGuide
State Machine: ProductGuide
View Model:    GuideState
```

The product communicates through View Models/Data Binding. It does not know animation/bone/transition names.

### Reduced/static/failure

Canonical transparent WebP poses are used for deterministic fallback:

```text
static/img/guide/
├── kubidm-identity-glyph.svg
├── crab-idle.webp
├── crab-welcome.webp
├── crab-guide.webp
├── crab-protect.webp
├── crab-working.webp
├── crab-success.webp
├── crab-warning.webp
└── crab-goodbye.webp
```

There are intentionally no `crab-*.svg` poses and no separate travel artwork. Reduced/static modes never instantiate the full Rive renderer. Rive/runtime/WASM/asset failure falls back to the same WebPs.

CSS does not implement internal crab motion. Character geometry, claws, legs, gaze, band tail, success motion, and travel gait belong to the Rive source.

## Mock Rive modes

Before the real `.riv` exists, the exact production renderer/lifecycle can be tested with an in-memory Data Binding runtime:

```text
/ui/_lab?rive=mock
```

Failure behavior is deterministic with:

```text
/ui/_lab?rive=mock-fail
```

Development diagnostics are exposed at:

```js
window.__kubidmGuideDiagnostics
window.__kubidmMockRiveStats
```

They are test instrumentation only, never security/account state.

## Real product integration

### Authentication

`static/modules/guide_controller.mjs` consumes semantic `data-guide-*` hooks and data-free WebAuthn lifecycle events. It is HTMX-safe and attaches/detaches the renderer as scenes change.

The server-provided mechanism order remains authoritative. The strongest-ranked allowed mechanism may be **Recommended**; other allowed mechanisms remain **Works OK**.

Guided WebAuthn teaches before opening native browser/device UI. Once native UI is active, the crab becomes quiet. A browser assertion is never treated as authentication success.

### Confirmed Auth -> Applications

Normal login may create a one-shot `sessionStorage` handoff containing only a timestamp. OAuth and reauthentication do not create it.

Only an authenticated Applications render may consume it and emit:

```text
success -> travel -> idle
```

Applications travel publishes direction/gaze semantics; the Rive rig supplies the actual six-leg lateral gait.

### Applications

The guide has reserved non-overlapping space. Confirmed login gets the one-shot travel/arrival sequence; direct visits remain quiet. Empty application state distinguishes “authenticated, no assigned apps” from auth failure.

### Profile and Credentials

Profile and Credentials share a reserved guide safe zone across HTMX navigation. Existing server-rendered policy/warnings remain authoritative.

Credential guidance can react to blocking warnings, requirements, passkey enrollment, TOTP/password setup, pending edits, and quiet settled state. Progress is factual/non-scoring only.

### Logout

Opening sign-out confirmation emits `goodbye`; cancelling restores the previous state. Confirming logout never waits for animation. Static fallback uses `crab-goodbye.webp`.

## Teaching decay and privacy

Teaching familiarity is presentation state, not identity/security state. Local state may store story IDs seen, optional-suggestion dismissal metadata, and whether onboarding teaching has completed.

It stores no username/account identifier, credential information, token, policy result, or authentication result and can never satisfy policy.

## Reusable Askama primitives

Typed guide components live in `src/https/views/guide.rs` with templates under `templates/guide/`:

- `CrabDialogView`;
- `RecommendationOptionView`;
- `SecurityNoticeView`; and
- `JourneyProgressView`.

Shared styles live in `static/guide.css`; surface-specific layout is separated into auth, Applications, settings, and UI-Lab stylesheets.

## Verification workflow

Repo-side deterministic gates:

```bash
cd server/core
pnpm test:guide
pnpm test:guide:e2e
KUBIDM_RIVE_TEST_MODE=mock pnpm guide:performance
KUBIDM_RIVE_TEST_MODE=mock pnpm guide:evidence
```

After the real `kubidm-guide.riv` is committed:

```bash
KUBIDM_EXPECT_REAL_RIVE=1 pnpm test:guide:e2e
pnpm guide:performance
KUBIDM_GUIDE_FULL_MATRIX=1 pnpm guide:evidence
pnpm guide:review:validate visual-review.json
```

The complete executable runbook is `tests/guide/README.md`. The authoring/visual quality contract is `book/src/developers/designs/rive_production_execution_plan.md`.

The final production gate also includes an authenticated staging check of 20 Profile <-> Credentials HTMX navigation cycles because the UI Lab deliberately has no real account/session fixture.

## Development workflow

For every new or changed state:

1. identify authoritative product/policy state;
2. define renderer-independent semantics;
3. create/update deterministic UI Lab story;
4. test light/dark and desktop/tablet/mobile;
5. test full/reduced/static and OS reduced motion;
6. inspect semantic + Rive diagnostics;
7. verify Rive failure leaves the task usable;
8. run deterministic contract/browser/performance gates;
9. generate evidence;
10. run independent visual LLM review;
11. get human approval for canonical silhouette/travel changes; and
12. fix character geometry/motion in Rive, not with CSS.

## Acceptance checklist

A guided state is ready when:

- the primary task is obvious without animation;
- recommendation labels match authoritative server/product semantics;
- valid alternatives are not treated as failures;
- mascot/dialog content is supplementary and accessible;
- security warnings remain authoritative normal UI;
- serious states reduce motion/personality;
- mobile does not require the mascot to complete the task;
- reduced/static preserve all information;
- the mascot never covers controls;
- no action waits for animation; and
- success appears only after a confirmed product/server outcome.
