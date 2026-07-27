# Mascot-Guided Identity Experience — v1 Implementation Baseline

This document records the implementation decisions locked for the first production-capable Kubidm Guide System. It supplements the architecture, mascot, guided-journey, authentication UI, and [Production Rive Execution Plan](rive_production_execution_plan.md).

Where an earlier document describes one of these items as provisional or future work, this baseline and the Rive execution plan are the newer decisions.

## Status

The product/guidance integration and production web-renderer infrastructure are implemented behind deployment controls. The existing authentication, credential-policy, session, OAuth/OIDC, and protocol behavior remains authoritative.

The production rendering architecture is:

```text
server/account policy
        -> product state
        -> guidance semantics
        -> GuideRendererController
             -> RiveGuideRenderer       full motion
             -> StaticGuideRenderer     reduced/static/load failure
```

Full-motion character animation belongs exclusively to the Rive rig. WebP artwork is a deterministic accessibility/failure fallback; CSS does not implement internal crab animation.

The repository side is complete enough to accept and verify the final Rive-authored `kubidm-guide.riv`. The Rive animation system is not production-complete until that real asset passes the contract, browser, performance, accessibility, visual-review, and human-approval gates defined in the execution plan.

Animation never decides policy, authentication success, credential validity, or navigation.

## Canonical identity badge

The badge is no longer provisional for v1.

`server/core/static/img/guide/kubidm-identity-glyph.svg` is the compact vector representation of the dark badge worn on the Kubidm Identity Band. Its geometry follows the approved mascot artwork rather than introducing a generic lock or unrelated identity icon.

## Canonical mascot assets

The deterministic fallback pack is:

```text
server/core/static/img/guide/
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

The final full-motion branch additionally requires:

```text
server/core/static/img/guide/kubidm-guide.riv
```

The character remains the locked shell-less B1-derived model:

- broad coral/orange body;
- no secondary, back, mint, decorative, or implied shell;
- six compact visible walking legs;
- asymmetric Guide and Guardian claws;
- short eye stalks and restrained expression vocabulary;
- permanent dark-teal Kubidm Identity Band with pale stripe and side knot/tail; and
- dark identity badge centered on the band.

All fallback pose artwork comes from the same approved canonical pose sheet. There are intentionally no `crab-*.svg` pose files and no separate `crab-travel` asset. Travel is a state of the same Rive character, not a ninth drawing.

## Production Rive contract

The normative machine-readable ABI is:

```text
server/core/static/guide_rive_contract.json
```

Required identities:

```text
Artboard:      KubidmGuide
State Machine: ProductGuide
View Model:    GuideState
```

The production control surface uses Rive View Models/Data Binding, not legacy state-machine inputs.

Required semantic properties include:

```text
state             idle | welcome | guide | protect | working | success | warning | goodbye | travel
motion            full | reduced | static
severity          neutral | positive | caution | critical
travelDirection   left | right
lookX / lookY     -1 .. 1
attention         trigger
successSmall      trigger
successMajor      trigger
goodbye           trigger
```

Animation clip names, bone names, transition names, or rig implementation details must not leak into product code.

## Runtime decision

The web runtime is pinned and self-hosted:

```text
@rive-app/canvas-lite 2.39.1
/pkg/rive/rive.js
/pkg/rive/rive.wasm
```

The repository also carries:

```text
server/core/static/rive/LICENSE
server/core/static/rive/VERSION.json
```

`VERSION.json` records package/version, immutable upstream `gitHead`, license source, and SHA-256 values for JS/WASM/LICENSE. `pnpm vendor:rive` reproduces the vendored runtime.

No production CDN request is allowed. Kubidm explicitly configures the local WASM URL and disables the runtime's public WASM fallback URL. Runtime/WASM/`.riv` failure degrades to the static guide rather than blocking identity workflows.

## Renderer policy

```text
full       RiveGuideRenderer
reduced    StaticGuideRenderer
static     StaticGuideRenderer
Rive error StaticGuideRenderer
```

`RiveGuideRenderer` owns:

- lazy runtime/contract loading;
- Data Binding and one-shot triggers;
- directional gaze/travel inputs;
- DPR/canvas resizing;
- offscreen pause behavior;
- HTMX-safe teardown/recreation;
- cleanup of both loaded and still-loading instances; and
- development diagnostics.

`StaticGuideRenderer` owns only canonical still WebP fallback poses.

## Presentation rollout

`KUBIDM_GUIDED_UI` selects guidance amount:

```text
off      legacy presentation; no guide integration
subtle   compact state/mascot feedback; proactive teaching reduced
full     proactive coach, teaching, stories, recommendations, mascot presence
```

Backward-compatible truthy values map to `full`; false-like values map to `off`; unknown values fail closed to `off`.

The switch affects presentation only. It does not alter server policy, allowed mechanisms, credential validation, session issuance, or endpoint behavior.

## Motion rollout

`KUBIDM_GUIDED_MOTION` selects the deployment motion ceiling:

```text
auto      full motion unless the user's OS requests reduced motion
full      permit Rive full motion, still overridden by prefers-reduced-motion
reduced   deterministic still fallback
static    deterministic still fallback
```

`prefers-reduced-motion: reduce` always prevents the full Rive renderer even when deployment configuration permits it. Motion is never required to understand or complete a workflow.

## Teaching familiarity and decay

Teaching familiarity is local presentation state, not account/security state. The browser may remember only story identifiers seen, optional-suggestion dismissal counts/timestamps, and whether onboarding teaching has completed.

It stores no usernames, account identifiers, credentials, tokens, policy decisions, or authentication results. This state may make Kubidm quieter; it can never satisfy security policy or change an authentication decision.

## Production journey coverage

The v1 product integration covers the primary experience surfaces.

### Authentication

- first encounter and remembered-user variants;
- server-ranked method choice;
- passkey and hardware-security-key teaching;
- password, TOTP, and backup-code states;
- native WebAuthn pending/cancellation/error lifecycle;
- authentication denial and login/session errors;
- OAuth destination context; and
- reauthentication context.

### Confirmed authentication -> Applications

Normal login records only a short-lived pending-attempt timestamp in `sessionStorage`; OAuth and reauthentication are excluded. Only an authenticated Applications render may consume it and emit:

```text
success -> travel -> idle
```

Travel carries explicit direction/gaze semantics to Rive. Client-side WebAuthn assertion is never treated as successful authentication.

### Applications

- confirmed arrival and signature travel state;
- quiet settled state;
- non-overlapping mascot safe zone; and
- explicit empty-applications explanation.

### Profile

- read-only/protected posture;
- edit-unlock/reauth explanation;
- edit state; and
- authoritative change-review guidance.

### Credentials

- authoritative policy warning/protection states;
- non-scoring factual setup progress;
- pending-change review;
- passkey/TOTP/password enrollment guidance; and
- optional passkey recommendation with reminder decay.

### Logout

- goodbye state when confirmation opens;
- restored state when logout is cancelled; and
- no delay before navigation when logout is confirmed.

## UI Lab and verification harness

The debug-only `/ui/_lab` is the deterministic review harness. It supports canonical journey stories, all mascot semantic states, Applications travel/settle, light/dark, desktop/tablet/mobile, full/reduced/static motion, full/subtle/off presentation, semantic event trace, and Rive diagnostics.

Two deterministic runtime modes support development before the real `.riv` exists:

```text
/ui/_lab?rive=mock
/ui/_lab?rive=mock-fail
```

The repository provides:

```text
pnpm test:guide
pnpm test:guide:e2e
pnpm guide:performance
pnpm guide:evidence
pnpm guide:review:validate <review.json>
pnpm vendor:rive
```

The harness verifies runtime/contract drift, vendored hashes/license, WebP fallback integrity, no pose SVGs/CSS character keyframes, local-only WASM behavior, semantic bindings, reduced/static behavior, load failure, 100 transitions, real `.riv`/WASM failure gates, performance/memory, visual evidence, and independent review thresholds.

The final staging pass additionally requires 20 authenticated Profile <-> Credentials HTMX cycles because UI Lab deliberately has no real authenticated session fixture.

See `server/core/tests/guide/README.md` for executable commands.

## Security invariants

The v1 implementation preserves these non-negotiable rules:

1. Normal UI and server state remain authoritative.
2. A recommendation never changes what policy allows.
3. Valid alternatives are not rendered as warnings or failures.
4. Critical security states use less motion and personality, not more.
5. Native browser/WebAuthn UI takes visual priority while active.
6. Authentication, form submission, and navigation never wait for mascot animation.
7. Success is shown only after a confirmed product/server outcome.
8. Mascot placement reserves space and does not cover actionable controls.
9. Reduced/static/no-runtime behavior remains fully usable.
10. Renderer failures degrade to normal Kubidm UI rather than blocking identity workflows.
11. Rive receives no username, credential, secret, token, or policy-internal data.
12. Production Rive makes no external runtime/WASM/artwork request.

## Remaining external deliverable

The only irreplaceable authoring work outside this repository environment is creation of the real `kubidm-guide.riv` in a Rive-capable environment, followed by the mandatory feedback loop in the Rive execution plan.

The external author must not rewrite product semantics or runtime integration. It must produce the canonical vector/rig/state machine/View Model/animations, especially the real six-leg lateral gait, then return the `.riv` for the repository's existing gates.

The PR may be marked ready only after the real asset passes automated contract/browser/performance/accessibility/security gates, independent LLM visual review, and human approval for canonical silhouette and travel gait.
