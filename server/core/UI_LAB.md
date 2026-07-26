# Kubidm UI Lab

The Kubidm UI Lab is a development-only, Storybook-style harness for the mascot-guided authentication and credential experience.

It deliberately uses the same server, Askama base template, Bootstrap bundle, Kubidm CSS, and vanilla JavaScript environment as the real UI. It does not introduce React, Vue, Web Components, or a second production frontend model.

## Safety

The route has two independent gates:

1. it is compiled only when Rust `debug_assertions` are enabled; and
2. the route is registered only when `KUBIDM_UI_LAB` is present in the environment.

Release builds do not contain the route.

The lab contains fixture identities and simulated outcomes. It is not an authentication endpoint and must not be enabled on a production deployment.

## Run the UI Lab

From the repository root:

```bash
KUBIDM_UI_LAB=1 make run
```

The insecure development configuration listens on `https://localhost:8443`.

Open:

```text
https://localhost:8443/ui/_lab
```

The development server uses a locally generated certificate, so the browser may require accepting the development certificate before the lab is reachable.

## Run the guided real authentication prototype

The real authentication templates have an independent experimental presentation switch:

```bash
KUBIDM_GUIDED_UI=1 make run
```

Accepted enabled values are `1`, `true`, `yes`, and `on`, case-insensitively. With the variable unset or set to any other value, Kubidm keeps the legacy login presentation.

This switch changes presentation and guide lifecycle integration only. It does not change authentication policy, mechanism ordering, credential validation, session issuance, or form endpoints.

For side-by-side development of the real flow and the fixture catalogue, both switches may be enabled:

```bash
KUBIDM_GUIDED_UI=1 KUBIDM_UI_LAB=1 make run
```

## What the lab is for

Use the lab to iterate on states that are slow, awkward, or unsafe to reproduce through a real authentication session:

- first encounter / account identification;
- authentication-method recommendation;
- passkey teaching;
- WebAuthn pending state;
- confirmed success;
- valid password alternative;
- WebAuthn cancellation;
- OAuth destination context;
- reauthentication;
- policy-required action;
- returning configured user;
- resilience recommendation;
- credential-journey progress;
- journey completion;
- Crab Dialog variants;
- recommendation taxonomy; and
- authoritative security notices.

The toolbar can switch:

```text
theme:    light | dark
viewport: desktop | tablet | mobile
motion:   full | reduced | static
```

The current story and controls are encoded in the URL fragment, so a particular state can be linked directly during review.

The lab also records the last semantic `kubidm:guide-state` and WebAuthn lifecycle events in a visible runtime-contract panel. This makes renderer integration and browser automation reviewable without inspecting animation internals.

## Canonical scenarios

Stories can be reviewed individually or as complete canonical journeys through the scenario navigator.

The current scenario catalogue is defined in `static/modules/guide_scenarios.mjs`:

- **Scenario A — new user, passkey recommended**;
- **Scenario B — valid password alternative**;
- **Scenario C — returning configured user**;
- **Scenario D — WebAuthn cancellation**; and
- **Scenario E — policy-required action**.

The navigator exposes Previous, Restart, and Next controls and shows the current semantic journey stage. Scenario definitions contain product meaning and expected guide state; they do not contain renderer animation names.

## Fixture semantics

Every story exposes the semantic state under the preview:

```text
product state
recommendation
mascot state
severity
```

The shared semantic vocabulary is defined in `static/modules/guide_contract.mjs` and includes:

```text
recommendation
severity
mascot state
motion level
journey stage
```

`travel` is a semantic mascot state even though static rendering uses the idle pose for that phase. The future Rive renderer owns the actual walking cycle.

These values intentionally match the renderer-independent contracts in the mascot ADR and Guided Identity Journey.

Stories should describe product meaning first. Do not create stories named after Rive animation clips.

Good:

```text
webauthn-cancel
policy-required
resilience
```

Bad:

```text
crab_animation_07
bounce_success
claw_left_variant
```

## Renderer boundary

`static/modules/guide_renderer.mjs` owns mascot rendering.

The initial implementation provides a static-SVG renderer and lifecycle-compatible controller. The lab adapter in `static/modules/ui_lab_renderer.mjs` translates story state and selected motion level into this renderer contract.

The intended layering is:

```text
product / policy state
        -> guidance semantics
        -> renderer controller
        -> static SVG | Rive
```

Adding Rive must not require changing story definitions or embedding authentication policy in animation files.

In the UI Lab, a missing canonical SVG is represented by a labelled development placeholder. In real guided product surfaces, missing artwork is hidden entirely so forms and controls remain clean and usable.

## Real authentication integration

The production-facing prototype uses semantic `data-guide-*` hooks on existing login states. `static/modules/guide_controller.mjs` translates those hooks and WebAuthn lifecycle events into renderer-independent guide state.

The controller is HTMX-safe: it can attach to scenes that appear after a partial swap, release a scene when it disappears, and attach again without reloading the page.

The multi-mechanism chooser continues to use the order returned by the authentication server. Kubidm already sorts available mechanisms strongest-first; the guided presentation exposes the first server-ranked mechanism as **Recommended** and presents the remaining allowed choices as **Works OK** rather than inventing a client-side security ranking.

`static/pkhtml.js` emits data-free WebAuthn lifecycle events for start, assertion submission, interruption, and error. Obtaining a browser assertion never produces a success state: authentication success remains server-authoritative.

Guided normal login uses a privacy-minimal handoff marker in `sessionStorage`. The marker contains only a timestamp, expires after two minutes, and is created only for a normal login whose intended destination is Applications. OAuth and reauthentication explicitly clear/exclude this marker.

The Applications page may consume the marker only after it has rendered as an authenticated destination. That arrival can therefore emit:

```text
success -> travel -> idle
```

without changing `login.rs`, session issuance, or authentication protocol behavior. The marker is one-shot and is cleared on authentication denial, interrupted WebAuthn, errors, or a return to the identify step.

## Credential guidance

Guided credential surfaces use the existing credential editor rather than a separate wizard.

`static/modules/credential_guide.mjs` reads already-rendered authoritative editor state and translates it into guide posture and copy:

- blocking `.alert-danger` policy conflict -> critical / warning posture;
- `.alert-warning` requirement -> caution / protect posture;
- enabled existing Discard Changes control -> pending-change guidance;
- otherwise -> calm idle orientation.

The adapter is also HTMX-safe, so Profile -> Credentials navigation works without a full reload.

The setup summary is deliberately non-scoring and limited to facts visible in the current editor:

- a visible sign-in method;
- a visible configured passkey;
- whether the editor currently reports unresolved policy warnings; and
- whether changes are currently pending.

A missing passkey is described as optional unless the authoritative policy warning says otherwise. Recovery/resilience completion is not inferred until Kubidm exposes authoritative state for it.

## Reusable Askama primitives

The first production-oriented guide primitives live in `src/https/views/guide.rs` with templates under `templates/guide/`:

- `CrabDialogView`;
- `RecommendationOptionView`;
- `SecurityNoticeView`; and
- `JourneyProgressView`.

These primitives are typed on the Rust side and have render tests. Their reusable styles live in `static/guide.css` rather than in the UI Lab chrome stylesheet.

The lab can continue using fixture markup while these components stabilise, but approved production surfaces should progressively move to the shared Askama views.

## Mascot assets

The lab expects canonical static fallback assets at:

```text
server/core/static/img/guide/
├── crab-idle.svg
├── crab-welcome.svg
├── crab-guide.svg
├── crab-protect.svg
├── crab-working.svg
├── crab-success.svg
├── crab-warning.svg
└── crab-goodbye.svg
```

`travel` intentionally reuses `crab-idle.svg` in the static renderer; it becomes a true walking animation only in the motion renderer.

Until an asset exists, the lab renders a labelled mascot-state placeholder. This allows layout, copy, recommendation, responsive, and accessibility work to proceed before the final artwork is integrated.

When the Rive runtime is introduced, the same stories should gain a renderer switch rather than replacing the static fixture model.

## Development workflow

For a new UI state:

1. identify the authoritative product state and policy meaning;
2. add or update the semantic story fixture in `static/modules/ui_lab.mjs`;
3. add it to a canonical scenario when it belongs to a repeatable journey;
4. validate the story in light and dark themes;
5. validate desktop, tablet, and mobile layouts;
6. validate full, reduced, and static motion modes;
7. inspect the semantic event trace;
8. verify all required text and controls remain understandable without a mascot asset;
9. once the design is approved, extract/reuse the corresponding Askama primitive in the production route; and
10. keep the lab fixture as the regression/demo state for that component.

The long-term target is for production Askama partials and the UI Lab to share the same component markup. The initial harness keeps fixture rendering isolated so the first prototype does not require a broad authentication-template refactor.

## Story acceptance checklist

A story is ready to become production UI when:

- the primary task is obvious without animation;
- recommendation labels match authoritative policy semantics;
- a valid alternative is not styled as an error;
- mascot/dialog content is supplementary and accessible;
- security warnings remain authoritative normal UI;
- mobile does not require the mascot to understand or complete the task;
- reduced/static modes preserve all information;
- no action waits for animation to finish; and
- success appears only after a simulated or real confirmed product result.

## Validation

The Askama lab view has a render smoke test in `src/https/views/ui_lab.rs`.

The reusable Askama guide primitives have render tests in `src/https/views/guide.rs`.

JavaScript is covered by the repository's existing ESLint workflow because the lab and guide modules live below `server/core/static/` and are not vendored under `external/`.
