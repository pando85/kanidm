# Kubidm Guide — Production Rive Execution Plan

- **Status:** Required before the mascot animation system is considered production-complete
- **Scope:** Rive authoring, runtime integration, verification, LLM feedback loop, accessibility, performance, and rollout
- **Related documents:**
  - [Mascot and Motion Design System](mascot_design_system.md)
  - [Guided Identity Journey](guided_identity_journey.md)
  - [Authentication and Credential-Setup UI](authentication_credential_ui.md)
  - [Mascot-Guided Identity Experience v1](mascot_guided_v1_implementation.md)

## 1. Purpose

The existing semantic guide architecture is correct, but whole-image CSS transforms are not the final production animation system. The production target is a single canonical Rive character whose internal parts are rigged and animated while the product continues to communicate only renderer-independent semantic states.

The required architecture is:

```text
server/account policy
        -> product state
        -> guide semantics
        -> GuideRendererController
             -> RiveGuideRenderer        full motion
             -> StaticGuideRenderer      reduced/static/runtime failure
```

The Rive layer must never decide authentication policy, recommendation level, credential validity, authentication success, or navigation.

This plan is intentionally executable by coding agents and visual-review agents. Every phase ends with a verification gate. A phase is not complete until its gate produces evidence that can be reviewed by both deterministic tests and a visual reviewer.

## 2. Non-negotiable character baseline

The Rive character must reproduce the approved shell-less B1-derived Kubidm crab, not reinterpret it.

### Construction invariants

- broad compact coral/orange body;
- no secondary, back, mint, decorative, or implied shell;
- six visible simplified walking legs;
- two short eye stalks;
- restrained eye/brow/mouth expression system;
- asymmetric claws:
  - Guide claw: articulate, point, present, wave, inspect;
  - Guardian claw: slightly broader, protect, stop, warning, security;
- permanent dark-teal Kubidm Identity Band;
- pale/light band stripe;
- side knot/tail used for restrained secondary motion;
- centered Kubidm identity badge;
- no extra torso eyes, chest face, or decorative facial elements;
- mature, clean vector construction: friendly but not toy-like.

### Behavior invariant

> Curious when guiding. Calm when protecting. Quiet when security is serious.

The more serious the security state, the less character motion is allowed.

## 3. Required source and runtime assets

The final branch must contain:

```text
server/core/static/img/guide/
├── kubidm-guide.riv
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

The WebP files remain production fallbacks for `reduced`, `static`, Rive load failure, WASM failure, and unsupported browsers.

There must be no independently drawn `crab-travel` pose. Travel is a state of the same rigged character.

## 4. Rive authoring contract

### 4.1 Artboard

```text
Artboard: KubidmGuide
Default State Machine: ProductGuide
Default View Model: GuideState
```

The artboard background is transparent. The character must fit inside a normalized safe area with enough space for claw gestures and the band tail without clipping.

### 4.2 Rig hierarchy

Recommended logical hierarchy:

```text
KubidmGuide
├── body
├── face
│   ├── eye_left
│   │   └── pupil_left
│   ├── eye_right
│   │   └── pupil_right
│   ├── brow_left
│   ├── brow_right
│   └── mouth
├── claw_guide
│   ├── upper
│   └── pincer
├── claw_guardian
│   ├── upper
│   └── pincer
├── legs
│   ├── leg_guide_front
│   ├── leg_guide_mid
│   ├── leg_guide_back
│   ├── leg_guardian_front
│   ├── leg_guardian_mid
│   └── leg_guardian_back
├── identity_band
│   ├── band_main
│   ├── band_stripe
│   ├── badge
│   └── tail
└── activity_signal
```

Names may change only if the machine-readable contract and runtime tests are updated in the same commit.

### 4.3 Production control surface: Data Binding

Do not build a new implementation around legacy State Machine Inputs. Use a View Model and Data Binding.

Required View Model:

```text
GuideState
```

Required properties:

```text
state             enum
motion            enum
severity          enum
travelDirection   enum
lookX             number  [-1, 1]
lookY             number  [-1, 1]
attention         trigger
successSmall      trigger
successMajor      trigger
goodbye           trigger
```

Enum values:

```text
state:
  idle
  welcome
  guide
  protect
  working
  success
  warning
  goodbye
  travel

motion:
  full
  reduced
  static

severity:
  neutral
  positive
  caution
  critical

travelDirection:
  left
  right
```

`state`, `severity`, and `motion` are semantic values. Animation names, clip names, bone names, or transition implementation details must not leak into application code.

## 5. Animation catalogue

### 5.1 Enter -> Welcome

Target duration: **600–850 ms**.

Full motion:

1. crab enters laterally;
2. approximately 1.5–2 gait cycles;
3. body vertical oscillation <= 3%;
4. gaze faces destination;
5. band tail lags motion slightly;
6. crab decelerates and settles;
7. Guide claw opens into Welcome;
8. eyes settle toward the user.

Reduced:

- no walking cycle;
- soft opacity/pose transition only;
- no tail oscillation.

Static:

- directly render `welcome` still state.

### 5.2 Welcome -> Guide

Target duration: **350–450 ms**.

Ordering is mandatory:

```text
eyes -> body -> Guide claw
```

- gaze reaches the target first;
- body rotates no more than approximately 3–5 degrees;
- Guide claw moves once toward the target;
- gesture holds briefly and relaxes;
- no repeated pointing loop.

### 5.3 Guide -> Protect

Target duration: **450–550 ms**.

Ordering:

```text
guide claw retracts
-> eyes focus
-> Guardian claw moves forward
-> body lowers slightly
-> badge/activity signal settles
```

- stance becomes wider/stabler;
- tail secondary motion drops sharply;
- expression becomes focused, not angry.

### 5.4 Protect -> Working

Working may last from hundreds of milliseconds to many seconds.

Required behavior:

- Guardian posture remains stable;
- eyes focus on operation, not user;
- body breathing <= 1% vertical scale/translation equivalent;
- activity signal may pulse slowly;
- tail is nearly static;
- no frantic spinner behavior;
- state should settle/pause whenever no visible animation is needed.

### 5.5 Working -> Success

Small success target: **300–450 ms**.

Major success target: **600–750 ms**.

Ordering:

```text
activity signal -> eyes -> body -> Guide claw
```

Major success may include:

- one badge/activity pulse;
- happy/soft eyes;
- body lift <= 4%;
- Guide claw opening;
- no confetti;
- no repeated bouncing.

### 5.6 Warning

- mostly still;
- smile removed;
- gaze moves to warning target;
- Guide claw lowers;
- Guardian claw becomes central;
- no red flashing;
- no fear animation;
- no comic reaction.

### 5.7 Critical security

Critical mode is the minimum-motion state.

- tail still;
- body still;
- eyes focused;
- mouth neutral;
- no continuous activity pulse unless the operation itself requires a static indication;
- no idle animation;
- no celebration/attention motion.

### 5.8 Goodbye

Target duration: **650–1000 ms** before optional exit, but logout must never wait for it.

1. gaze user;
2. Guide claw rises;
3. one wave only;
4. gaze shifts to exit direction;
5. lateral gait begins if the page remains visible long enough.

### 5.9 Travel

Travel is a signature brand animation and must not look like a frozen image translating across the page.

Mandatory sequence:

```text
look toward destination
-> body anticipates
-> legs begin lateral gait
-> body travels
-> band tail lags
-> decelerate
-> final step
-> tail settles
-> eyes inspect destination
```

Rules:

- six-leg gait remains readable;
- 2.5–3.5 leg cycles/second is the target range;
- body vertical oscillation <= 3%;
- body rotation <= 2 degrees during steady travel;
- claws remain comparatively stable;
- band tail lag about 100–150 ms perceptually;
- no sliding feet;
- no teleporting between phases;
- travel direction must work in both left and right directions without producing anatomically inconsistent claw semantics.

## 6. Idle behavior

Idle must not be a deterministic repeating cartoon loop.

Allowed micro-events:

- blink;
- small eye glance;
- tiny claw adjustment;
- small band-tail settle;
- minimal body weight shift.

Constraints:

- one micro-event at a time;
- roughly 8–15 seconds between optional micro-events;
- no constant breathing loop if the Rive runtime can settle instead;
- no idle behavior in `critical` severity;
- runtime should pause or naturally settle when idle/static.

## 7. Authoring workflow for an LLM-assisted Rive session

Rive Editor is the source of truth for the `.riv` binary. The built-in Rive Agent may be used to create vector geometry, scripts, data models, and animation, but every generated change must pass the same visual and runtime gates as manual work.

### Phase A — canonical vector model

Give the Rive authoring agent the approved model sheet and pose sheet as reference images.

Suggested authoring prompt:

```text
Recreate this exact Kubidm mascot as clean editable vector geometry.
Do not redesign it.
The character is a shell-less coral/orange crab with six visible compact legs,
short eye stalks, asymmetric Guide and Guardian claws, and a permanent dark-teal
Identity Band with a pale stripe, centered badge, and side knot/tail.

Preserve the reference proportions, silhouette, eye size, claw asymmetry, band
placement, and restrained expression. Do not add a back shell, mint shell, chest
face, extra eyes, clothing, shadows that change the silhouette, or additional
ornament.

Separate body, eyes/pupils, brows, mouth, each claw, all six legs, band main,
band stripe, badge, and band tail into independently animatable components.
Use a low/medium vertex count suitable for web runtime animation.
Transparent background.
```

**Gate A:** export a static reference frame from the artboard and compare it against the approved reference at desktop and mobile display sizes. Reject if the silhouette or proportions read as a different character.

### Phase B — rig

Suggested prompt:

```text
Rig the approved Kubidm crab without changing its visible resting silhouette.
Create independent control for gaze, brows, mouth, Guide claw, Guardian claw,
six-leg lateral gait, body translation/rotation, badge/activity signal, and band
tail secondary motion. Keep deformations clean and prevent band/body clipping.
The resting idle frame must remain visually equivalent to the approved static model.
```

**Gate B:** test extreme but valid controls. There must be no detached geometry, band clipping, inverted legs, eye escape, or claw/body intersections.

### Phase C — View Model and state machine

Suggested prompt:

```text
Create View Model GuideState and State Machine ProductGuide according to the
repository guide_rive_contract.json. Use Data Binding, not legacy state-machine
inputs. State changes must be interruptible and must respect motion and severity.
Critical severity disables idle personality and secondary movement. Static motion
must reach deterministic still poses.
```

**Gate C:** runtime contract validator must discover every required View Model property and enum value. Missing/renamed properties fail the build.

### Phase D — animation catalogue

Author animations in the order:

1. idle static base;
2. welcome;
3. guide;
4. protect;
5. working;
6. success small;
7. success major;
8. warning;
9. goodbye;
10. lateral travel left/right;
11. reduced/static behavior;
12. interruption transitions.

**Gate D:** each animation is exercised independently in the UI Lab before scenario testing.

### Phase E — product scenario integration

Exercise canonical scenarios, especially:

```text
Scenario A:
identify
-> choose
-> teach passkey
-> native WebAuthn
-> server-confirmed success
-> travel
-> Applications idle
-> optional credential improvement
```

**Gate E:** the semantic event trace and visible Rive state must agree at every step.

## 8. Web runtime implementation

### 8.1 Runtime choice

Use a pinned, self-hosted `@rive-app/canvas-lite` runtime unless the final `.riv` uses a feature unavailable in lite.

Reasons:

- this mascot does not require Rive text, layout, audio, or scripting;
- canvas-lite has a substantially smaller WASM footprint than the full Canvas or WebGL2 variants;
- the current character is simple enough that Canvas is appropriate;
- WebGL2 should only be selected if measured performance or required features justify the larger runtime.

Do not use deprecated `@rive-app/webgl`.

### 8.2 Self-hosting

No CDN dependency is allowed in production.

Pin the Rive runtime version in the repository and self-host:

```text
/pkg/rive/rive.js
/pkg/rive/rive.wasm
/pkg/img/guide/kubidm-guide.riv
```

Configure the runtime WASM URL explicitly before creating a Rive instance.

The application must still boot and authenticate if JS, WASM, or `.riv` loading fails.

### 8.3 RiveGuideRenderer

Implement alongside the current static renderer:

```text
GuideRendererController
├── RiveGuideRenderer
└── StaticGuideRenderer
```

Selection policy:

```text
full + Rive supported + asset/runtime load succeeds
    -> RiveGuideRenderer

reduced
    -> Rive with reduced property OR StaticGuideRenderer,
       whichever produces the more deterministic accessible result

static
    -> StaticGuideRenderer

Rive/WASM/load/runtime error
    -> StaticGuideRenderer
```

The product controller must never know the internal Rive animation names.

### 8.4 Lifecycle

The Rive renderer must:

- initialize lazily only when a visible guide slot exists;
- reuse/cache parsed `.riv` data where multiple instances are created;
- resize canvas for device pixel ratio and container changes;
- pause when offscreen;
- pause/settle when the graphic is static;
- clean up observers/runtime instances on HTMX scene removal;
- recreate safely on HTMX scene insertion;
- never leak one user's browser presentation state into another account/security decision.

## 9. Machine-readable contract

The repository file:

```text
server/core/static/guide_rive_contract.json
```

is normative for renderer integration and automated tests.

The `.riv` file, runtime adapter, UI Lab diagnostics, tests, and this document must agree with it.

Changes to the contract require:

1. contract change;
2. `.riv` change;
3. runtime change;
4. tests;
5. regenerated visual evidence;
6. reviewer approval.

No silent editor rename is allowed.

## 10. Automated verification harness

### 10.1 Contract smoke test

Add a development test using the pinned Rive runtime that loads `kubidm-guide.riv` and fails if:

- the file fails to parse;
- artboard `KubidmGuide` cannot be selected;
- state machine `ProductGuide` cannot play;
- default/bound View Model `GuideState` is missing;
- any required property is missing;
- any required enum value is missing;
- setting each property causes an exception;
- the Rive instance emits a load error.

Use public Rive runtime APIs. Do not parse the `.riv` binary manually.

### 10.2 UI Lab diagnostic bridge

In debug UI Lab only, expose a read-only diagnostic object:

```js
window.__kubidmGuideDiagnostics = {
  renderer: "rive",
  loaded: true,
  artboard: "KubidmGuide",
  stateMachine: "ProductGuide",
  semanticState: "travel",
  motion: "full",
  severity: "neutral",
  riveState: "travel",
  fallbackActive: false,
  lastError: null,
};
```

This is test instrumentation, not production security state.

Browser tests must assert both semantic events and this diagnostic bridge.

### 10.3 Browser scenario test

Add Playwright (or the repository-standard browser automation if one exists by implementation time) for debug UI Lab only.

For each canonical story/state:

1. open deterministic story URL;
2. wait for `loaded=true`;
3. set viewport/theme/motion/presentation controls;
4. assert expected semantic state;
5. assert Rive renderer active in full mode;
6. assert static renderer active in static mode;
7. take deterministic screenshots at specified checkpoints;
8. record any console error, failed network request, or Rive load error;
9. fail on unexpected fallback in full mode.

### 10.4 Screenshot matrix

Minimum matrix:

```text
states:
  idle, welcome, guide, protect, working,
  success, warning, goodbye, travel

viewports:
  desktop 1440x900
  tablet  820x1180
  mobile  390x844

themes:
  light, dark

motion:
  full, reduced, static
```

Do not capture every Cartesian product on every commit if CI cost is too high. Use:

- full matrix in nightly/release/explicit visual job;
- representative smoke matrix on each PR.

## 11. LLM visual-review loop

Every animation change must produce review artifacts that an LLM can inspect.

### Evidence bundle

For each changed state include:

```text
artifacts/guide-review/<commit>/
├── manifest.json
├── semantic-trace.json
├── console.json
├── network.json
├── desktop-light/
│   ├── start.png
│   ├── mid.png
│   └── end.png
├── desktop-dark/...
├── mobile-light/...
└── motion-preview.webm   # optional but preferred for travel/transition changes
```

`manifest.json` records:

- commit SHA;
- `.riv` SHA-256;
- Rive runtime version;
- browser/version;
- story ID;
- semantic state;
- motion mode;
- viewport;
- reference image version.

### Visual reviewer prompt

Use this prompt for a vision-capable review agent:

```text
You are reviewing the Kubidm production mascot against the locked design.
Do not judge whether the image is generally cute or attractive. Judge design
compliance and animation quality.

Required character invariants:
- shell-less broad coral/orange crab;
- six visible compact legs;
- two short eye stalks;
- asymmetric Guide and Guardian claws;
- dark-teal Identity Band with pale stripe, centered badge and side tail;
- restrained mature expression;
- no added shell, clothing, chest face, extra eyes, or ornament.

Motion rule:
Curious when guiding. Calm when protecting. Quiet when security is serious.

For each artifact, score 0-5:
1. silhouette fidelity;
2. proportion fidelity;
3. band/badge fidelity;
4. face fidelity;
5. claw-role readability;
6. pose semantic readability;
7. motion smoothness;
8. lack of clipping/deformation;
9. accessibility appropriateness;
10. product-state consistency.

For travel additionally inspect:
- real six-leg lateral gait rather than sliding;
- feet do not visibly skate;
- gaze leads movement;
- band tail lags and settles;
- body oscillation remains restrained;
- final step and settle are readable.

Return JSON only:
{
  "pass": true|false,
  "scores": {...},
  "blocking_defects": [],
  "non_blocking_defects": [],
  "recommended_changes": []
}

Fail if any invariant is violated, any blocking defect exists, or any fidelity,
clipping, accessibility, or product-state score is below 4.
```

### Reviewer independence

The agent that authored the animation must not be the only reviewer. At least one separate visual-review model/process must inspect the evidence bundle.

For high-impact changes to the canonical silhouette or travel gait, require human approval as well as LLM approval.

## 12. Numeric verification gates

These numbers are initial engineering gates and may only be relaxed with measured evidence.

### Asset/runtime

- `.riv` should target <= 250 KB compressed unless visual fidelity demonstrably requires more;
- use `canvas-lite` unless another renderer is justified by a measured requirement;
- no production CDN request for runtime/WASM/artwork;
- no Rive request may block form interactivity or authentication submission.

### Layout

- mascot may not overlap actionable UI in canonical viewports;
- no artboard clipping during any accepted pose;
- no horizontal page scroll introduced by the guide;
- canvas backing resolution must track layout/device-pixel-ratio without visibly blurry output.

### Motion

- no continuous animation in static mode;
- reduced mode contains no lateral gait, bouncing, or continuous tail motion;
- critical severity contains no idle personality loop;
- travel body oscillation <= 3% of character height;
- steady travel rotation <= 2 degrees;
- no repeated success celebration;
- no repeated pointing loop.

### Reliability

- 100 consecutive state changes in the UI Lab must produce no JS exception;
- 20 HTMX Profile <-> Credentials navigation cycles must not multiply Rive instances/listeners;
- simulated `.riv` 404 must fall back to static UI;
- simulated WASM failure must fall back to static UI;
- authentication controls remain usable under both failures.

## 13. Performance verification

Test at least:

- current desktop Chromium;
- Firefox;
- WebKit/Safari-equivalent browser automation where available;
- a low-end/mobile CPU profile.

Collect:

- Rive/WASM transfer size;
- `.riv` transfer size;
- initialization time;
- first visible mascot frame;
- animation frame stability during travel;
- CPU time while idle;
- memory before/after repeated HTMX navigation.

Acceptance intent:

- idle/static character should settle or pause and approach negligible ongoing CPU use;
- no accumulating Rive instance/memory trend after navigation cycles;
- travel should remain visually smooth under realistic mobile CPU throttling;
- runtime initialization must not delay the auth form becoming interactive.

## 14. Accessibility verification

Tests must verify:

- `prefers-reduced-motion: reduce` prevents full-motion choreography;
- deployment `KUBIDM_GUIDED_MOTION=static` never starts Rive full animation;
- all critical information remains HTML/UI, never artwork-only;
- canvas is decorative/supplementary and has an appropriate accessible fallback/label strategy;
- keyboard navigation and focus order are identical with Rive success, Rive failure, and static fallback;
- native WebAuthn browser UI visually and functionally takes priority.

## 15. Security/CSP verification

Before enabling Rive in production:

1. pin exact runtime version;
2. self-host JS/WASM/`.riv`;
3. verify CSP requires no external origin;
4. verify any WASM CSP requirement against actual supported browser behavior;
5. do not relax `worker-src` unless the chosen runtime path demonstrably needs a worker;
6. confirm `.riv` and WASM are served with correct MIME types;
7. ensure asset failure cannot block auth/navigation;
8. no secrets, usernames, credential material, tokens, or policy internals are passed into Rive properties.

## 16. Feedback loop — mandatory for every animation change

The development loop is:

```text
AUTHOR
  -> export .riv
  -> contract smoke test
  -> UI Lab scenario tests
  -> capture evidence bundle
  -> deterministic assertions
  -> independent LLM visual review
  -> human review when required
  -> classify defects
  -> iterate
  -> rerun the complete affected matrix
  -> merge only when all gates pass
```

A failed gate returns to authoring. Do not patch over a visual defect with CSS unless the defect is layout outside the Rive character. Character geometry/motion defects are fixed in the Rive source.

### Defect classes

**P0 — reject immediately**

- wrong mascot model/shell;
- authentication is blocked by renderer failure;
- critical security state is playful/high-motion;
- reduced/static preference ignored;
- Rive state contradicts authoritative product state.

**P1 — must fix before merge**

- gait visibly slides;
- body/band/claw clipping;
- wrong pose/state;
- repeated celebration/pointing;
- severe performance regression;
- memory/listener leak;
- mobile overlap.

**P2 — normally fix before merge**

- timing/easing mismatch;
- secondary motion too strong;
- minor visual fidelity deviation;
- insufficient settle quality.

**P3 — backlog permissible**

- optional micro-expression polish;
- additional nonessential idle variation.

## 17. Suggested implementation commits

Keep changes reviewable:

```text
1. build canonical Rive vector model and commit kubidm-guide.riv
2. add GuideState data-binding contract and state machine
3. add core poses and reduced/static behavior
4. add lateral gait/travel choreography
5. vendor pinned canvas-lite runtime + WASM
6. implement RiveGuideRenderer with static fallback
7. add UI Lab Rive diagnostics
8. add contract/browser tests and evidence capture
9. performance/accessibility/CSP hardening
10. remove CSS character-motion fallback from full mode
11. final visual verification and documentation
```

Do not combine the initial `.riv` authoring, runtime integration, and removal of fallback behavior into one unreviewable commit.

## 18. Definition of done

The Rive migration is complete only when all are true:

- `kubidm-guide.riv` exists in the branch;
- the character passes the locked visual-invariant review;
- `GuideState` Data Binding contract passes automated introspection;
- `RiveGuideRenderer` is the only renderer used for **full** motion;
- static WebP remains the deterministic static/failure fallback;
- CSS no longer pretends to animate internal character articulation in full mode;
- travel contains a real lateral gait;
- reduced/static behavior is verified;
- UI Lab exposes deterministic Rive diagnostics;
- browser scenario tests pass;
- visual evidence bundle passes independent LLM review;
- CSP/self-hosting tests pass;
- performance and leak tests pass;
- normal repository CI is green;
- a human reviewer approves the canonical silhouette and travel gait.

Until those gates are met, PR #285 may contain a production-capable **fallback guide experience**, but it must not claim that the final production Rive animation system is complete.

## 19. External implementation notes

Current Rive guidance relevant to this plan:

- use View Models/Data Binding for new runtime control surfaces rather than legacy State Machine Inputs;
- self-hosting the WASM runtime is supported and recommended when reliability/control matter;
- `canvas-lite` is the smallest current Canvas runtime variant and omits text/layout/audio/scripting engines;
- Rive state machines can settle when there is no animation work, reducing ongoing resource use;
- runtime export of `.riv` files requires a Rive plan that supports runtime export;
- continuously test animation on target devices and pause/offload graphics when offscreen or static.

Re-check Rive runtime documentation and pin versions at implementation time rather than copying an unpinned CDN version from examples.
