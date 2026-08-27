# Kubidm Rive verification harness

This directory is the executable verification boundary for the production mascot renderer.

The product/application side can be developed and tested without Rive Editor by using the deterministic mock runtime. The **only external authoring deliverable** is the real `kubidm-guide.riv` plus its generated visual-review evidence and independent approvals.

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

The unit gate validates semantic/Data-Binding vocabulary, product template bindings, runtime-version drift, vendored runtime/license SHA-256 values, fallback image integrity, absence of pose SVGs/CSS character keyframes, and the local-only WASM configuration.

The browser suite uses `?rive=mock` by default and verifies:

- the production `RiveGuideRenderer` contract;
- exact artboard/state-machine/View-Model identities;
- `prefers-reduced-motion` overrides a Full selection;
- reduced/static modes do not instantiate full Rive motion;
- deterministic renderer failure activates static fallback;
- product controls remain usable after Rive failure;
- 100 story transitions do not accumulate active Rive instances;
- 20 authenticated Profile <-> Credentials HTMX cycles do not duplicate guide slots/canvases;
- semantic state and renderer state remain aligned;
- local runtime/WASM/contract assets are served with expected MIME types; and
- the Rive runtime's public WASM fallback URL is explicitly disabled.

CI provisions a disposable normal-person account, creates a one-use credential reset token, sets a masked random password through the real credential UI, signs in through the real login UI, and performs the 20-cycle authenticated HTMX stress test. Credentials and reset tokens are masked and are not persisted as review artifacts.

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
human-approval.json
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

The normal PR browser workflow automatically detects a committed non-empty `kubidm-guide.riv` and enables the real-runtime browser, performance and full evidence gates in addition to the mock gates.

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
desktop browsers                  Chromium, Firefox, WebKit
mobile profile                    Chromium 390x844, 4x CPU throttle
post-GC JS heap growth            <= 8 MiB
stress transitions                100
active Rive canvases after churn  <= 1
console errors                    0
external requests in real mode    0
```

The harness writes `artifacts/guide-performance.json`. A later change may tighten these thresholds once the final `.riv` is measured on target hardware; they must not be relaxed simply to make a failing asset pass.

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

The reviewer returns JSON matching `visual_review.schema.json`. It must copy `manifest.commit` and `manifest.rivSha256` into `evidence_commit` and `riv_sha256`, identify the independent reviewer/process, and record the review time. This prevents a review result for an older binary/evidence set from being reused accidentally.

Validate it with:

```bash
pnpm guide:review:validate visual-review.json
```

The validator fails when:

- review provenance is missing or malformed;
- `pass` is false;
- any blocking defect exists;
- any required category is missing;
- any score is outside 0..5; or
- **any production score is below 4**.

The animation-authoring agent may not be the only visual reviewer. The canonical silhouette and travel gait additionally require human approval.

## Human silhouette and travel approval

Record the two mandatory high-impact approvals in a JSON file tied to the same evidence commit and `.riv` SHA-256:

```json
{
  "evidence_commit": "copy manifest.commit exactly",
  "riv_sha256": "copy manifest.rivSha256 exactly",
  "silhouette": {
    "approved": true,
    "reviewer": "reviewer identity",
    "reviewed_at": "2026-08-14T21:00:00Z"
  },
  "travel_gait": {
    "approved": true,
    "reviewer": "reviewer identity",
    "reviewed_at": "2026-08-14T21:00:00Z"
  }
}
```

Do not synthesize or pre-fill approval. The named reviewer must actually inspect the full production evidence, especially desktop/mobile canonical silhouette and both travel directions.

## Final release-readiness gate

After the real asset passes the automated suite, full evidence has been generated, the independent visual review passes, and both human approvals exist, run:

```bash
pnpm guide:release:validate \
  artifacts/guide-review/<commit>/manifest.json \
  visual-review.json \
  human-approval.json
```

This gate fails unless:

- the real `.riv` exists and is non-empty;
- the evidence is real-mode and full-matrix;
- the evidence `.riv` hash matches the currently checked-out binary;
- desktop/tablet/mobile × light/dark × full/reduced/static coverage exists;
- no full-motion capture fell back to static;
- no external production request was recorded;
- the independent visual review itself passes and references the same evidence commit and `.riv` SHA-256; and
- human silhouette and travel approvals reference the same evidence commit and `.riv` SHA-256.

A passing result is the machine-checkable release handoff. The PR must remain draft until this gate and normal repository CI are both green.

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
  -> human silhouette/travel approval
  -> pnpm guide:release:validate manifest.json visual-review.json human-approval.json
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
