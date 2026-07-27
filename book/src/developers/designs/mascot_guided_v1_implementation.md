# Mascot-Guided Identity Experience — v1 Implementation Baseline

This document records the implementation decisions that are now locked for the first production-capable version of the Kubidm Guide System. It supplements the earlier architecture, mascot, guided-journey, and authentication UI design documents.

Where an earlier design document describes one of the items below as provisional or future work, this v1 baseline is the newer decision.

## Status

The v1 experience is implemented behind deployment controls and retains the existing authentication, credential-policy, session, and protocol behavior as the authoritative product layer.

The implementation follows:

```text
server/account policy
        -> product state
        -> guidance semantics
        -> guide controller
        -> canonical static artwork
        -> CSS choreography
```

Animation does not decide policy, authentication success, credential validity, or navigation.

## Canonical identity badge

The badge is no longer provisional for v1.

`server/core/static/img/guide/kubidm-identity-glyph.svg` is the compact vector representation of the dark badge worn on the Kubidm Identity Band. Its geometry follows the approved mascot artwork rather than introducing a separate abstract branching/K mark.

The SVG may be used wherever a compact product identity mark is needed. Future brand refinement may evolve the mark, but v1 product implementation must not substitute a generic padlock or an unrelated identity symbol for the badge worn by the canonical crab.

## Canonical mascot assets

The production pose pack is:

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

The character remains the locked shell-less B1-derived model:

- broad coral/orange body;
- no secondary, back, mint, or decorative shell;
- six compact walking legs;
- asymmetric Guide and Guardian claws;
- short eye stalks and restrained expression vocabulary;
- permanent dark-teal Kubidm Identity Band with a pale stripe and side knot/tail; and
- dark identity badge centered on the band.

All pose artwork comes from the same approved canonical pose sheet. A state must not be independently redrawn into a different crab model.

There are intentionally **no `crab-*.svg` pose files** and no separate `crab-travel` asset. Earlier simplified vectors and raster-in-SVG wrappers were removed because they did not reproduce the approved mascot consistently and could render blank under SVG sanitization.

`travel` is a semantic/motion state of the same crab. The renderer reuses the canonical idle artwork and CSS performs the lateral movement. This guarantees that travel cannot introduce a ninth character model.

## Renderer decision

Rive remains a supported future enhancement boundary, but it is not a dependency of the production-capable v1.

The v1 renderer is fully self-hosted and uses:

- canonical transparent WebP pose artwork from the approved pose sheet; and
- CSS choreography for pose/state and lateral travel motion.

The artwork itself contains no required animation. Reduced and static modes can therefore use the same canonical poses without unavoidable embedded motion.

This gives Kubidm a complete renderer with no CDN, no additional JavaScript framework, no worker requirement, and no new authentication dependency.

A future Rive renderer must implement the same renderer-controller contract and consume the same semantic states. It must not require product, policy, route, or story definitions to be rewritten, and its character model must remain visually faithful to the approved canonical crab.

## Presentation rollout

`KUBIDM_GUIDED_UI` selects how much guidance is shown:

```text
off      legacy presentation; no guide integration
subtle   compact mascot/state feedback; proactive teaching reduced
full     proactive coach, teaching, stories, recommendations, mascot presence
```

Backward-compatible values:

- `1`, `true`, `yes`, `on` -> `full`;
- `0`, `false`, `no` -> `off`;
- unknown values -> `off`.

The switch affects presentation only. It does not alter server policy, allowed mechanisms, credential validation, session issuance, or endpoint behavior.

## Motion rollout

`KUBIDM_GUIDED_MOTION` selects the deployment motion ceiling:

```text
auto      full motion unless the user's OS requests reduced motion
full      permit full motion, still overridden by prefers-reduced-motion
reduced   remove continuous/large movement while retaining state/expression changes
static    use still poses only
```

Unknown values fall back to `auto`.

`prefers-reduced-motion: reduce` always prevents full motion even when the deployment permits it. Travel in reduced/static modes remains the same canonical still crab without lateral movement.

Motion is never required to understand or complete a workflow.

## Teaching familiarity and decay

Teaching familiarity is local presentation state, not account/security state.

The browser may locally remember only:

- which teaching story identifiers have been seen;
- optional-suggestion dismissal counts and timestamps; and
- whether onboarding teaching has already been completed.

It does not store usernames, account identifiers, credentials, tokens, policy decisions, or authentication results.

This local state may make Kubidm quieter. It can never satisfy security policy, mark recovery complete, or change an authentication decision.

## Production journey coverage

The v1 integration covers the primary experience surfaces:

### Authentication

- first encounter and remembered-user variants;
- server-ranked authentication method choice;
- passkey and hardware-security-key teaching;
- password, TOTP, and backup-code states;
- native WebAuthn pending/cancellation/error lifecycle;
- authentication denial and login/session errors;
- OAuth destination context; and
- reauthentication context.

### Confirmed authentication -> Applications

Normal login records only a short-lived pending-attempt timestamp in `sessionStorage`. OAuth and reauthentication are excluded.

Only an authenticated Applications render can consume that marker and emit:

```text
success -> travel -> idle
```

A client-side WebAuthn assertion is never treated as successful authentication.

### Applications

- confirmed arrival and signature lateral travel;
- quiet settled state;
- non-overlapping mascot safe zone; and
- explicit empty-applications explanation that distinguishes no assigned apps from an authentication failure.

### Profile

- read-only/protected posture;
- explanation of edit unlock / reauthentication;
- edit state; and
- authoritative change-review confirmation guidance.

### Credentials

- authoritative policy warning/protection states;
- non-scoring factual setup progress;
- pending-change review;
- passkey enrollment;
- TOTP enrollment;
- password setup as a policy-valid alternative when allowed; and
- optional passkey recommendation with reminder decay.

### Logout

- goodbye state when the confirmation modal opens;
- restored state when logout is cancelled; and
- no delay before navigation when logout is confirmed.

## UI Lab

The development-only `/ui/_lab` surface remains the canonical deterministic review harness.

It supports:

- Scenario A-E journeys;
- authentication and credential stories;
- Applications travel/settled stories;
- light/dark theme;
- desktop/tablet/mobile viewport;
- full/reduced/static motion;
- full/subtle/off guide presentation; and
- a visible semantic event trace.

The lab is debug-build-only and additionally requires `KUBIDM_UI_LAB` to register its route.

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

## Future enhancement boundary

The production-capable v1 is complete without Rive. A future motion/art pass may introduce a Rive rig for richer gaze, claw articulation, walk phase continuity, and Identity Band secondary motion.

Such a change is renderer work, not a redesign of policy, workflow, recommendation, teaching, or product-state semantics. The Rive character must preserve the same canonical body, face, claws, legs, band, knot/tail, and badge language as the approved pose sheet.
