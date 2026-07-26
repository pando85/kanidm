# Kubidm UI Lab

The Kubidm UI Lab is a development-only, Storybook-style harness for the mascot-guided authentication and credential experience.

It deliberately uses the same server, Askama base template, Bootstrap bundle, Kubidm CSS, and vanilla JavaScript environment as the real UI. It does not introduce React, Vue, Web Components, or a second production frontend model.

## Safety

The route has two independent gates:

1. it is compiled only when Rust `debug_assertions` are enabled; and
2. the route is registered only when `KUBIDM_UI_LAB` is present in the environment.

Release builds do not contain the route.

The lab contains fixture identities and simulated outcomes. It is not an authentication endpoint and must not be enabled on a production deployment.

## Run

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

## Fixture semantics

Every story exposes the semantic state under the preview:

```text
product state
recommendation
mascot state
severity
```

These are intentionally close to the renderer-independent contracts in the mascot ADR and Guided Identity Journey.

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

Until an asset exists, the lab renders a labelled mascot-state placeholder. This allows layout, copy, recommendation, responsive, and accessibility work to proceed before the final artwork is integrated.

When the Rive runtime is introduced, the same stories should gain a renderer switch rather than replacing the static fixture model.

## Development workflow

For a new UI state:

1. identify the authoritative product state and policy meaning;
2. add or update the semantic story fixture in `static/modules/ui_lab.mjs`;
3. validate the story in light and dark themes;
4. validate desktop, tablet, and mobile layouts;
5. validate full, reduced, and static motion modes;
6. verify all required text and controls remain understandable without a mascot asset;
7. once the design is approved, extract/reuse the corresponding Askama partial in the production route; and
8. keep the lab fixture as the regression/demo state for that component.

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

The Askama view has a render smoke test in `src/https/views/ui_lab.rs`.

JavaScript is covered by the repository's existing ESLint workflow because the story module lives below `server/core/static/` and is not vendored under `external/`.
