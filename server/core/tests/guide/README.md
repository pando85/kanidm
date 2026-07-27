# Kubidm Rive verification harness

This directory is the executable verification boundary for the production mascot renderer.

The product/application side can be developed and tested without Rive Editor by using the deterministic mock runtime. The **only external authoring deliverable** is the real `kubidm-guide.riv` plus its visual-review evidence.

## Prerequisites

From `server/core`:

```bash
pnpm install --frozen-lockfile
pnpm exec playwright install chromium firefox webkit
```

Run Kubidm with the debug-only UI Lab and guided UI enabled:

```bash
KUBIDM_UI_LAB=1 KUBIDM_GUIDED_UI=full make run
```

The default test URL is `https://localhost:8443`. Override it with `KUBIDM_UI_BASE_URL`.

## Fast repo-side gates

These require no `.riv` asset and no Rive Editor:

```bash
pnpm test:guide
pnpm test:guide:e2e
```

The browser suite uses `?rive=mock` by default and verifies:

- the production `RiveGuideRenderer` contract;
- exact artboard/state-machine/View-Model identities;
- reduced/static modes do not instantiate full Rive motion;
- deterministic renderer failure activates static fallback;
- product controls remain usable after Rive failure;
- 100 story transitions do not accumulate active Rive instances;
- semantic state and renderer state remain aligned;
- local runtime/WASM/contract assets are served with expected MIME types.

## UI Lab mock modes

```text
/ui/_lab?rive=mock
/ui/_lab?rive=mock-fail
```

`mock` exercises the real Kubidm renderer/lifecycle with an in-memory Data Binding implementation.

`mock-fail` injects an initialization failure and must leave the normal UI usable with the static WebP fallback.

The UI Lab exposes read-only diagnostics at:

```js
window.__kubidmGuideDiagnostics
```

Mock lifecycle counters are available at:

```js
window.__kubidmMockRiveStats
```

These exist only for development/test instrumentation and are not product security state.

## External Rive authoring handoff

A Rive-capable environment must start from:

- `book/src/developers/designs/rive_production_execution_plan.md`;
- `server/core/static/guide_rive_contract.json`;
- the approved canonical model/pose references;
- `tests/guide/visual_review_prompt.md`.

It must return:

```text
server/core/static/img/guide/kubidm-guide.riv
artifacts/guide-review/<commit>/...
visual-review.json
```

The `.riv` must contain:

```text
Artboard:      KubidmGuide
State Machine: ProductGuide
View Model:    GuideState
```

and satisfy every property in `guide_rive_contract.json` through View Models/Data Binding.

## Real `.riv` contract gate

After `kubidm-guide.riv` has been exported, run the browser suite against the actual vendored Rive runtime:

```bash
KUBIDM_EXPECT_REAL_RIVE=1 pnpm test:guide:e2e
```

The opt-in test fails if the real asset cannot load, falls back to static rendering, uses the wrong artboard/state machine/View Model, or does not expose the required Data Binding contract.

## Generate visual-review evidence

Smoke evidence:

```bash
pnpm guide:evidence
```

Full production matrix:

```bash
KUBIDM_GUIDE_FULL_MATRIX=1 pnpm guide:evidence
```

Mock pipeline validation before the real asset exists:

```bash
KUBIDM_RIVE_TEST_MODE=mock pnpm guide:evidence
```

Evidence is written under:

```text
artifacts/guide-review/<commit>/
```

and includes:

- manifest with commit, browser, `.riv` SHA-256 and vendored runtime hashes/version;
- semantic and Rive diagnostic trace;
- console warnings/errors;
- failed network requests;
- deterministic start/mid/end screenshots;
- video for selected motion-heavy states;
- a list of external network requests.

Real-mode evidence **fails** if any external HTTP(S) request is observed. Production Rive is self-hosted.

## Independent LLM visual review

Give an independent vision-capable agent:

1. the approved canonical model and pose references;
2. `visual_review_prompt.md`;
3. the generated evidence directory.

The reviewer returns JSON matching `visual_review.schema.json`.

Validate it with:

```bash
pnpm guide:review:validate visual-review.json
```

The validator fails when:

- `pass` is false;
- any blocking defect exists;
- any required category is missing;
- any score is outside 0..5;
- **any production score is below 4**.

The animation-authoring agent may not be the only visual reviewer. The canonical silhouette and travel gait additionally require human approval.

## Feedback loop

Every Rive change follows:

```text
AUTHOR IN RIVE
  -> export kubidm-guide.riv
  -> pnpm test:guide
  -> KUBIDM_EXPECT_REAL_RIVE=1 pnpm test:guide:e2e
  -> KUBIDM_GUIDE_FULL_MATRIX=1 pnpm guide:evidence
  -> independent LLM visual review
  -> pnpm guide:review:validate visual-review.json
  -> human silhouette/travel approval when applicable
  -> fix Rive geometry/motion defects in Rive
  -> fix runtime/layout defects in Kubidm
  -> repeat affected gates
```

Do not patch character geometry or bad gait with CSS. CSS is for page layout; Rive owns internal character geometry and full-motion animation.

## Runtime vendoring

The production web runtime is pinned and self-hosted. To reproduce/update it deliberately:

```bash
pnpm vendor:rive
```

This writes:

```text
static/rive/rive.js
static/rive/rive.wasm
static/rive/VERSION.json
```

`VERSION.json` records the exact package version and SHA-256 of both vendored files. Runtime upgrades require normal code review plus the full Rive verification matrix.
