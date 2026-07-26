# Kubidm UI Lab

The Kubidm UI Lab is a development-only, Storybook-style harness for the mascot-guided identity experience. It deliberately uses the same Askama, HTMX, Bootstrap, CSS, vanilla-JavaScript, routing, and CSP environment as the real Kubidm UI instead of introducing a second frontend framework.

## Safety

The lab route has two independent gates:

1. it is compiled only when Rust `debug_assertions` are enabled; and
2. it is registered only when `KUBIDM_UI_LAB` is present.

Release builds do not contain the route. The lab uses fixture identities and simulated outcomes; it is not an authentication endpoint and must not be enabled in production.

## Run the UI Lab

```bash
KUBIDM_UI_LAB=1 make run
```

Open:

```text
https://localhost:8443/ui/_lab
```

The development certificate may need to be accepted by the browser.

## Run the guided product UI

The product experience has an independent deployment-level presentation switch:

```bash
KUBIDM_GUIDED_UI=full make run
```

Supported values are:

- `off` — legacy product presentation, no guide integration;
- `subtle` — compact mascot/state feedback with teaching surfaces reduced;
- `full` — proactive teaching, stories, recommendations, and mascot presence.

For backward compatibility, `1`, `true`, `yes`, and `on` map to `full`; `0`, `false`, and `no` map to `off`. Unknown values fail closed to `off`.

The switch changes presentation and guide lifecycle integration only. It does **not** change authentication policy, mechanism ordering, credential validation, session issuance, endpoint behavior, or form payloads.

Both development modes may run together:

```bash
KUBIDM_GUIDED_UI=full KUBIDM_UI_LAB=1 make run
```

## What the lab covers

The lab contains deterministic stories for:

- first encounter and account identification;
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
- Crab Dialog variants;
- recommendation taxonomy; and
- authoritative security notices.

The canonical Scenario A-E journeys live in `static/modules/guide_scenarios.mjs`. Scenario A now follows the real product order:

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

Theme, viewport, motion, and story remain linkable through the URL fragment. Guide presentation mode is an additional local review control.

A runtime panel records semantic `kubidm:guide-state` and WebAuthn lifecycle events. Renderer integration and browser automation can therefore assert product semantics without inspecting animation internals.

## Semantic contract

Every story exposes:

```text
product state
recommendation
mascot state
severity
motion level
journey stage
```

The bounded vocabulary lives in `static/modules/guide_contract.mjs`.

Stories and product code describe meaning, not animation clips. Prefer names such as `webauthn_cancelled`, `credential_policy_conflict`, and `applications_arrival`; never make policy depend on names such as `bounce_success` or `claw_left_variant`.

## Renderer boundary

`static/modules/guide_renderer.mjs` owns mascot rendering.

The v1 renderer is a self-hosted native SVG renderer with CSS/SMIL motion:

```text
product / policy state
        -> guidance semantics
        -> guide controller
        -> native SVG renderer
```

Canonical still poses are used for idle, welcome, guide, protect, working, success, warning, and goodbye. Full-motion `travel` uses an animated `crab-travel.svg` with a leg cycle, body movement, and Identity Band tail secondary motion. Reduced/static travel uses the still idle pose, so unavoidable motion is never embedded in accessibility modes.

Rive remains an enhancement boundary rather than a v1 dependency. A future self-hosted Rive renderer must consume the same semantic state and fall back to the native renderer without changing authentication or product rules.

In the UI Lab, missing artwork may use a labelled development placeholder. Real product surfaces hide a missing asset completely, preserving forms and navigation.

## Canonical mascot assets

```text
server/core/static/img/guide/
├── kubidm-identity-glyph.svg
├── crab-idle.svg
├── crab-welcome.svg
├── crab-guide.svg
├── crab-protect.svg
├── crab-working.svg
├── crab-success.svg
├── crab-warning.svg
├── crab-goodbye.svg
└── crab-travel.svg
```

The v1 character is the locked shell-less B1-derived crab: coral body, six walking legs, asymmetric Guide/Guardian claws, teal Identity Band, and the Kubidm identity glyph. Do not reintroduce a secondary/back shell.

## Real product integration

### Authentication

`static/modules/guide_controller.mjs` consumes semantic `data-guide-*` hooks and data-free WebAuthn lifecycle events. It is HTMX-safe and can attach/detach as scenes are swapped.

The server-provided mechanism order remains authoritative. The first strongest-ranked allowed mechanism is presented as **Recommended**; remaining allowed mechanisms are **Works OK** rather than errors.

Guided WebAuthn teaches before opening native browser/device UI. Once the native UI is active, the crab becomes quiet. A browser assertion is never treated as authentication success.

### Confirmed Auth -> Applications

Normal login may create a one-shot `sessionStorage` handoff containing only a timestamp. It expires after two minutes and contains no username, credential, token, mechanism secret, or server result.

OAuth and reauthentication do not create this handoff. Denial, errors, interruption, and return to identify clear it.

Only an already-authenticated Applications render may consume the marker and emit:

```text
success -> travel -> idle
```

This preserves the server as the success authority without modifying authentication protocol behavior.

### OAuth and reauthentication

Full mode explains the context in short accessible HTML. OAuth identifies the tenant as identity verifier and the OAuth client as destination. Reauthentication explains that an already-signed-in user is being verified again for a sensitive action. The normal UI remains authoritative.

### Applications

The guide has reserved non-overlapping space. A successful normal login gets the one-shot travel/arrival sequence; direct visits are quiet. An empty application list explains that authentication succeeded and that there are simply no linked applications.

### Profile

Profile editing has semantic read-only/edit/review states. Full mode explains why edit may require reauthentication and tells the user to review the server-rendered difference before confirmation. The crab remains in a reserved settings safe zone.

### Credentials

The existing credential editor is still the source of truth. `static/modules/credential_guide.mjs` translates already-rendered policy state into guide posture:

- blocking `alert-danger` -> critical warning posture;
- `alert-warning` requirement -> caution/protect;
- active passkey enrollment -> protect;
- active TOTP/password setup -> guide;
- pending edits -> guide/review;
- otherwise -> quiet idle.

Credential progress is deliberately non-scoring and limited to facts the current editor can establish: visible sign-in method, visible passkey, unresolved warnings, and pending/saved changes.

Passkey, TOTP, and password creation each have short contextual teaching. A missing passkey may produce a dismissible recommendation only when no authoritative policy warning is present. The copy explicitly states that existing policy-valid methods remain valid.

### Settings and logout

Profile/Credentials share a reserved guide safe zone so the crab remains physically present across HTMX navigation. Logout switches an active scene to `goodbye` as soon as the confirmation modal appears; navigation is never delayed for animation. The modal also has the static canonical goodbye asset.

## Teaching decay and privacy

Teaching familiarity is presentation state, not identity/security state.

`static/modules/guide_preferences.mjs` stores only a small local browser preference object:

- story identifiers already seen;
- optional-suggestion dismissal counts/timestamps; and
- whether onboarding teaching has been completed.

It stores no username, account identifier, credential information, token, policy result, or authentication result.

Stories decay after being seen; explicit Learn More stories can remain available. Optional suggestions stop after repeated dismissal and are spaced by a minimum reminder interval. This state can never satisfy policy or mark recovery/security configuration complete.

## Reusable Askama primitives

Typed components live in `src/https/views/guide.rs` with templates under `templates/guide/`:

- `CrabDialogView`;
- `RecommendationOptionView`;
- `SecurityNoticeView`; and
- `JourneyProgressView`.

The rollout parser and guide primitives have Rust render/unit tests. Shared styles live in `static/guide.css`; surface-specific layout is separated into auth, applications, settings, and UI-Lab stylesheets.

## Development workflow

For a new state:

1. identify the authoritative product/policy state;
2. define renderer-independent semantics;
3. create/update a deterministic UI Lab story;
4. add it to a canonical journey if applicable;
5. test light/dark;
6. test desktop/tablet/mobile;
7. test full/reduced/static motion;
8. test full/subtle/off presentation;
9. inspect the semantic event trace;
10. verify that all critical information remains available with no mascot/runtime; and
11. reuse/extract the production Askama primitive rather than embedding security rules in animation code.

## Acceptance checklist

A guided state is ready when:

- the primary task is obvious without animation;
- recommendation labels match authoritative server/product semantics;
- valid alternatives are not styled or narrated as failures;
- mascot/dialog content is supplementary and accessible;
- security warnings remain authoritative normal UI;
- serious states reduce motion and personality;
- mobile does not require the mascot to understand or complete the task;
- reduced/static preserve all information;
- full/subtle/off preserve workflow behavior;
- the mascot never covers actionable controls;
- no action waits for animation to finish; and
- success appears only after a confirmed product/server outcome.

## Validation

- Askama UI Lab has a render smoke test in `src/https/views/ui_lab.rs`.
- Typed guide primitives and rollout parsing have tests in `src/https/views/guide.rs`.
- JavaScript is covered by the repository ESLint workflow.
- Static assets and styles are exercised by the normal pre-commit/build/container workflows.
- The UI Lab semantic trace is the contract surface for future browser/E2E automation and a Rive renderer.
