# Kubidm Rive verification harness

This directory is the executable verification boundary for the production mascot renderer.

The product/application side can be developed and tested without Rive Editor by using the deterministic mock runtime. The **only external authoring deliverable** is the real `kubidm-guide.riv` plus its generated visual-review evidence and approval result.

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

The unit gate validates semantic/Data-Binding vocabulary, runtime-version drift, vendored runtime/license SHA-256 values, fallback image integrity, absence of pose SVGs/CSS character keyframes, and the local-only WASM configuration.

The browser suite uses `?rive=mock` by default and verifies:

- the production `RiveGuideRenderer` contract;
- exact artboard/state-machine/View-Model identities;
- `prefers-reduced-motion` overrides a Full selection;
- reduced/static modes do not instantiate full Rive motion;
- deterministic renderer failure activates static fallback;
- product controls remain usable after Rive failure;
- 100 story transitions do not accumulate active Rive instances;
- semantic state and renderer state remain aligned;
- local runtime/WASM/contract assets are served with expected MIME types; and
- the Rive runtime's public WASM fallback URL is explicitly disabled.

## UI Lab mock modes

```text
/ui/_lab?rive=mock
/ui/_lab?rive=mock-fail
```

`mock` exercises the real Kubidm renderer/lifecycle with an in-memory Data Binding implementation.

`mock-fail` injects an initialization failure and must leave the normal UI usable with the static WebP fallback.

The UI Lab exposes read-only diagnostics at:

```js
window.__kubidmGuideDiagnostics;
```

Mock lifecycle counters are available at:

```js
window.__kubidmMockRiveStats;
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

The Rive environment must **not** vendor or change the web runtime. Kubidm already pins and self-hosts `@rive-app/canvas-lite@2.39.1`; the authored binary must work against that exact runtime or the runtime upgrade must be a separate reviewed change.

## Real `.riv` contract and failure gates

After `kubidm-guide.riv` has been exported, run the browser suite against the actual vendored Rive runtime:

```bash
KUBIDM_EXPECT_REAL_RIVE=1 pnpm test:guide:e2e
```

The real-asset gates fail if:

- the asset cannot load or unexpectedly falls back;
- artboard/state-machine/View-Model identities are wrong;
- required Data Binding properties are missing;
- a `.riv` 404 prevents the task from remaining usable;
- a local WASM failure prevents the task from remaining usable;
- the runtime makes an external HTTP(S) request; or
- the server does not serve the local runtime/WASM/contract correctly.

## Performance and leak verification

Run the quantitative harness with the real asset:

```bash
pnpm guide:performance
```

Validate the pipeline itself before the real asset exists:

```bash
KUBIDM_RIVE_TEST_MODE=mock pnpm guide:performance
```

The current engineering gates are intentionally conservative and can be overridden only for measured investigation:

```text
initialization                    <= 2000 ms
animation frame interval p95     <= 50 ms
post-GC JS heap growth            <= 8 MiB
stress transitions                100
active Rive canvases after churn  <= 1
console errors                    0
external requests in real mode    0
```

The harness writes `artifacts/guide-performance.json`. A later change may tighten these thresholds once the final `.riv` is measured on target hardware; they must not be relaxed simply to make a failing asset pass.

The product-level requirement for 20 authenticated Profile <-> Credentials HTMX cycles remains a staging/real-session gate because UI Lab intentionally contains no authenticated account/session fixture. The renderer's generic DOM-removal lifecycle is stress-tested independently in UI Lab; the authenticated navigation cycle must still be run in the final staging validation.

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
- a list of external network requests; and
- the exact independent-review prompt/schema/validator paths.

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
  -> pnpm guide:performance
  -> KUBIDM_GUIDE_FULL_MATRIX=1 pnpm guide:evidence
  -> independent LLM visual review
  -> pnpm guide:review:validate visual-review.json
  -> authenticated 20-cycle Profile <-> Credentials staging check
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
static/rive/LICENSE
static/rive/VERSION.json
```

`VERSION.json` records the exact package version, immutable upstream `gitHead`, license source, and SHA-256 values for JS/WASM/LICENSE. Runtime upgrades require normal code review plus the full Rive verification matrix.
