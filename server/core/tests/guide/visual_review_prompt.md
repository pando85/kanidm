# Kubidm production mascot visual review prompt

You are the independent visual reviewer for the Kubidm production Rive mascot.

Your task is **design compliance and motion-quality verification**, not subjective cuteness or style preference.

Review the supplied `artifacts/guide-review/<commit>/` evidence bundle together with the approved canonical mascot references. Read `manifest.json` first. Your result must copy its `commit` into `evidence_commit` and its `rivSha256` into `riv_sha256` exactly; this binds the approval to the evidence and mascot binary you actually reviewed. Set `reviewer` to the independent model/process identity used for this review and `reviewed_at` to the review completion time in ISO 8601 form.

## Locked character invariants

The character must remain:

- a shell-less broad compact coral/orange crab;
- exactly six visible compact walking legs;
- two short eye stalks;
- asymmetric claws:
  - Guide claw: more articulate and used for pointing/presenting/waving;
  - Guardian claw: slightly broader and used for protection/warning/security;
- a permanent dark-teal Kubidm Identity Band;
- a pale/light stripe on the Identity Band;
- a centered identity badge;
- a side band knot/tail;
- restrained, mature and friendly rather than toy-like.

Reject immediately if you see:

- a back, secondary or mint shell;
- chest/torso eyes or another face;
- more or fewer than six visible walking legs in a state where the legs should be readable;
- clothing or ornament not present in the approved character;
- claw-role reversal;
- a substantially different silhouette/proportion language.

## Motion rule

> Curious when guiding. Calm when protecting. Quiet when security is serious.

Serious security states must reduce motion. `critical` must contain no idle-personality loop.

## State checks

Verify that each visual state clearly matches the semantic trace:

- `idle`: quiet and settled;
- `welcome`: friendly invitation, not goodbye;
- `guide`: gaze leads, then body, then Guide claw;
- `protect`: Guardian claw reads as protective, expression focused rather than angry;
- `working`: stable guardian posture, restrained activity indication;
- `success`: one restrained acknowledgement, not repeated bouncing/confetti;
- `warning`: mostly still and calm, no comic fear;
- `goodbye`: one wave then optional departure;
- `travel`: a real lateral crab gait, never a frozen image sliding.

## Travel-specific checks

Travel is a signature animation. Verify all of these:

1. gaze leads toward the destination;
2. body anticipation is readable but restrained;
3. all six legs participate in a believable lateral gait;
4. feet do not visibly skate across the ground plane;
5. claws stay comparatively stable and retain Guide/Guardian identity;
6. body vertical oscillation stays subtle (target <= 3% of character height);
7. body rotation is restrained (target <= 2 degrees in steady travel);
8. band tail visibly lags body movement and settles afterward;
9. deceleration and a final step are readable;
10. final gaze/settle reads as arrival;
11. left and right travel do not mirror the semantic identity of the claws incorrectly.

## Accessibility and product-state checks

Verify that:

- reduced motion has no lateral gait, bounce or continuous tail movement;
- static mode is fully still;
- critical states are nearly static;
- mascot/canvas never obscures actionable controls;
- the mascot never contradicts authoritative UI state;
- success only appears after the semantic trace reports a confirmed outcome;
- no animation is necessary to understand the task.

## Scoring

Score every category as an integer from 0 to 5:

- `silhouette_fidelity`
- `proportion_fidelity`
- `band_badge_fidelity`
- `face_fidelity`
- `claw_role_readability`
- `pose_semantic_readability`
- `motion_smoothness`
- `lack_of_clipping_deformation`
- `accessibility_appropriateness`
- `product_state_consistency`

A score below 4 in **any** category fails the production gate.

Any locked-invariant violation is a blocking defect regardless of numeric score.

## Output

Return **JSON only**, conforming to `visual_review.schema.json`:

```json
{
  "reviewer": "independent-review-model-or-process",
  "reviewed_at": "2026-08-14T21:00:00Z",
  "evidence_commit": "copy manifest.commit exactly",
  "riv_sha256": "copy manifest.rivSha256 exactly",
  "pass": true,
  "scores": {
    "silhouette_fidelity": 5,
    "proportion_fidelity": 5,
    "band_badge_fidelity": 5,
    "face_fidelity": 5,
    "claw_role_readability": 5,
    "pose_semantic_readability": 5,
    "motion_smoothness": 5,
    "lack_of_clipping_deformation": 5,
    "accessibility_appropriateness": 5,
    "product_state_consistency": 5
  },
  "blocking_defects": [],
  "non_blocking_defects": [],
  "recommended_changes": []
}
```

Set `pass` to `false` if any category is below 4 or any blocking defect exists.

The authoring agent must not be the sole reviewer. Canonical silhouette and travel gait also require human approval before the PR can be marked ready.
