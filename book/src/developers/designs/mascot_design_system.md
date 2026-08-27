# Kubidm Mascot and Motion Design System

- **Status:** Design baseline
- **Date:** 2026-07-26
- **Related ADR:** [Mascot-Guided Product Experience](mascot_guided_product_experience.md)
- **Scope:** Kubidm mascot identity, character construction, pose language, motion grammar, authentication storyboard, and Rive-facing motion contract

## Purpose

This document records the visual and interaction decisions made during the first Kubidm mascot design phase. The related ADR defines the architectural boundary and implementation strategy; this document defines what the approved character and motion system are intended to look and behave like.

The mascot is not decorative artwork placed on top of the product. It is a state-aware visual representation of Kubidm that accompanies users through identity workflows while preserving the authority of the normal UI.

This document intentionally distinguishes between **locked design decisions** and **open design work**. Implementation should not silently change locked characteristics without updating this design specification.

## Brand concept

Kubidm uses a mascot-led product experience in which the crab represents Kubidm itself.

The character is:

- the visual embodiment of Kubidm;
- an identity guide;
- a guardian during security-sensitive operations;
- a workflow companion; and
- a visual representation of system state.

The character is not:

- a chatbot;
- a pet;
- an animated cursor follower;
- a replacement for alerts, headings, validation, or status text;
- a source of security decisions; or
- a comedic reaction layer for authentication failures.

The core behavioural rule is:

> **Curious when guiding. Calm when protecting. Quiet when security is serious.**

## Personality balance

The canonical personality balance is:

```text
Guide     70%
Guardian  30%
```

The character should feel competent before it feels charming.

Target qualities:

| Quality | Direction |
| --- | --- |
| Trustworthy | Very high |
| Friendly | High |
| Technical | Moderately high |
| Playful | Moderate |
| Cute | Controlled |
| Serious | Available when context requires it |
| Corporate stiffness | Low |

The intended impression is closer to a friendly and competent infrastructure engineer than a cartoon assistant.

## Canonical character direction

### Base model

The approved direction derives from the first **Sailor Band** concept, refined into a cleaner vector-friendly character suitable for rigging and repeated daily use.

The canonical model is the refined B1 direction.

### Character construction

The following characteristics are locked:

- compact, broad orange/coral crab body;
- smooth rounded body/carapace with **no additional secondary shell layer**;
- two short eye stalks;
- expressive but controlled eyes;
- simple mouth;
- six simplified visible walking legs;
- upright claws;
- slightly asymmetric claw roles;
- permanent teal Kubidm Identity Band;
- light/white stripe through the band;
- knotted trailing band tail;
- central identity badge; and
- clean vector construction suitable for small-size rendering and rigging.

The mascot should remain recognisable primarily by silhouette, band, eyes, claws, and movement rather than by surface detail.

### No secondary shell

Explorations with a mint/teal external back shell were rejected.

The canonical model uses the orange rounded body as the crab's carapace/body shape. There is no separate decorative shell covering the back or upper body.

This decision keeps the silhouette cleaner, preserves the friendly B1 character, reduces rigging complexity, and prevents the character from appearing armoured or turtle-like.

### Claw roles

The claws are functionally asymmetric even when the overall character remains visually balanced.

#### Guide claw

The guide claw is the primary interaction claw. It is used for:

- pointing;
- presenting;
- selecting;
- greeting;
- waving;
- indicating the next step; and
- drawing attention to UI elements.

#### Guardian claw

The guardian claw is the primary protection/security claw. It is used for:

- protective poses;
- credential operations;
- locks and identity symbols;
- warning posture;
- stop/attention posture; and
- security-sensitive confirmation.

A small size difference is acceptable, but the asymmetry should remain subtle rather than making the character look anatomically distorted.

### Eyes and face

The face remains deliberately simple.

The eyes carry most of the expression. The final construction should avoid exaggerated toy-like proportions while remaining friendly and legible at UI sizes.

The mouth uses a small vocabulary rather than many cartoon expressions.

The character should not depend on facial detail alone to communicate system state; posture, gaze, claws, stillness, and the identity badge are equally important.

## Kubidm Identity Band

The teal band is a permanent part of the character identity. It is not treated as an optional sailor costume.

The design term is **Kubidm Identity Band**.

The band:

- wraps across the front of the body;
- uses a dark teal base;
- carries a light/white accent stripe;
- supports the central badge;
- terminates in a tied tail; and
- remains present across canonical poses.

The tied tail is an important motion element. It provides restrained secondary movement when the character travels, settles, or changes behavioural mode.

### Tail behaviour by personality mode

**Guide mode**

- loose;
- small secondary movement;
- slight natural lag behind body movement.

**Guardian mode**

- restrained;
- reduced amplitude;
- posture appears more stable.

**Security-critical mode**

- still or nearly still.

## Identity badge and glyph

The central badge is locked as part of the character construction.

The current lock/key-like symbol inside the badge is **provisional**.

A future design phase must define a canonical Kubidm identity glyph that can work consistently as:

- the mascot badge mark;
- a compact product mark;
- a favicon or small app icon;
- an identity-system activity indicator;
- a security confirmation mark; and
- a documentation/brand symbol.

The full crab mascot and the compact product glyph should be related but must not be forced to serve the same size/use cases.

## Working colour palette

The existing Kubidm palette remains the foundation. The current design exploration adds teal and cyan roles around that base.

Working tokens:

```text
Deep Navy   #071228
Navy        #0d1a30
Coral       #cb4f32
Teal        #0ea39a
Cyan        #29c3d3
Warm Gray   #ccaba5
White       #ffffff
```

The exact teal and cyan values remain subject to final accessibility, dark-theme, and contrast validation before becoming application-wide CSS tokens.

### Colour semantics

**Orange/coral**

- mascot body;
- warmth;
- recognisable crab identity.

**Deep navy/navy**

- structural details;
- brand foundation;
- serious/security context.

**Teal**

- Identity Band;
- persistent character identity.

**Cyan**

- active identity/system energy;
- authentication processing;
- credential/security feedback;
- confirmed identity-system activity.

Cyan is not a general decorative glow. It should indicate that Kubidm is actively doing something identity-related.

## Core pose vocabulary

Eight core poses form the canonical pose language.

| Pose | Meaning | Typical use |
| --- | --- | --- |
| Idle | Kubidm is present and available | Applications, profile, waiting |
| Welcome | Start of a journey | Login, onboarding entry |
| Guide / Point | A next action or target deserves attention | Passkey CTA, app selection |
| Working / Inspect | Kubidm is processing or checking | Authentication, validation |
| Protect | A security-sensitive operation is active | Passkeys, credentials, reauthentication |
| Success | A confirmed operation completed | Login, save, passkey registration |
| Warning | User attention is required | Policy issue, unsaved state, recoverable problem |
| Goodbye | The journey is ending | Logout |

These poses should be reusable across workflows rather than duplicated as route-specific artwork.

## Pose definitions

### Idle

Idle is the default long-lived pose and therefore should be the least demanding visually.

Characteristics:

- stable body;
- relaxed claws;
- guide claw slightly more open than the guardian claw;
- attentive eyes;
- small smile;
- relaxed band tail.

The intended message is:

> I am here if you need me.

It should not demand attention.

### Welcome

Welcome indicates invitation rather than farewell.

Characteristics:

- guide claw opens outward;
- guardian claw remains closer to the body;
- slight upward body movement;
- friendly gaze;
- small secondary tail motion.

Do not use the goodbye wave as the welcome gesture.

### Guide / Point

The guide pose directs attention to a real UI target.

The claw should not grow a literal pointing finger. The natural claw rotates and opens toward the target.

Canonical attention order:

```text
eyes -> body -> guide claw
```

The eyes and guide claw must agree about the target.

The character should point once and then relax rather than repeatedly calling for attention.

### Working / Inspect

Working indicates processing or waiting.

The mascot remains calm. Activity is represented primarily through a restrained cyan identity/system signal while the character watches or inspects it.

Avoid generic frantic spinner behaviour.

During native WebAuthn or OS dialogs, the mascot becomes quieter because the operating-system interface is the user's primary focus.

### Protect

Protect exposes the 30% Guardian side of the personality.

Characteristics:

- guardian claw moves toward the front/centre;
- focused eyes;
- slightly more stable/wide stance;
- reduced tail movement;
- cyan identity/security feedback may appear;
- smile becomes neutral or restrained where appropriate.

Typical uses include:

- passkey operations;
- credential changes;
- MFA;
- reauthentication; and
- security-sensitive confirmation.

### Success

Success is confirmation, not celebration by default.

Common characteristics:

- short cyan badge pulse;
- eyes soften or become happy;
- guide claw opens;
- small upward body movement and settle.

The animation must only begin after product/server confirmation of success.

Two levels exist:

```text
success.small
success.major
```

`success.small` is used for common operations such as profile saves.

`success.major` is reserved for meaningful milestones such as completed authentication or first passkey enrolment.

### Warning

Warning communicates that attention is required without panic.

Characteristics:

- body largely still;
- smile neutralises;
- gaze moves toward the warning;
- claws reduce expressive movement;
- guardian claw may move slightly toward centre.

Do not use slapstick, crying, violent shaking, or exaggerated fear.

### Goodbye

Goodbye is the canonical farewell sequence.

The guide claw performs one short wave, then the character looks toward the exit and travels sideways out of the scene.

The band tail should be the last part of the character to settle/leave, reinforcing secondary-motion continuity.

## Expression system

The expression system remains intentionally small.

### Eye states

```text
open
soft
focused
looking-left
looking-right
closed-happy
concerned
```

### Brow states

```text
neutral
curious
focused
concerned
```

### Mouth states

```text
neutral
small-smile
happy
serious
```

These states can be combined, but implementations should avoid inventing large numbers of bespoke emotions.

## Personality modes

### Guide mode

- curious;
- energetic but controlled;
- smooth movement;
- normal secondary band-tail motion;
- attention-oriented gaze.

### Guardian mode

- calm;
- confident;
- protective;
- more stable posture;
- controlled movement;
- reduced secondary motion.

### Security mode

- serious;
- minimal;
- focused eyes;
- neutral/serious mouth;
- reduced or zero idle motion;
- still tail.

A global design principle follows:

> **The more serious the security state, the less the mascot moves.**

## Signature motion language

### Sideways movement means journey

The crab moves laterally between workflows.

This is a signature brand behaviour, not merely anatomically appropriate movement.

Examples:

```text
Login -> Applications -> Profile -> Credentials -> Logout
```

Sideways travel visually communicates progression from one identity context to another.

Cross-page scenes should be choreographed so an exit on one page and entry on the next feel like the same character continuing its journey.

### Gaze means attention

Gaze is a first-class interaction primitive.

The mascot uses eyes before stronger body gestures whenever it needs to direct the user toward an action or object.

Typical ordering:

```text
eyes -> body -> guide claw
```

This allows the mascot to guide without constantly pointing or displaying instructional text.

### Cyan illumination means system activity

Cyan indicates active Kubidm identity-system behaviour.

Examples:

- authentication pending;
- passkey operation active;
- identity verification processing;
- server-confirmed identity result.

The glow should be restrained and semantic.

### Guardian claw forward means protection

The guardian claw moving toward centre/front represents a security/protection posture.

### Stillness means importance

Reduced movement communicates seriousness. Warning and security-critical states deliberately remove animation rather than adding more dramatic motion.

## Motion timing system

Five timing classes are defined.

| Token | Duration | Intended use |
| --- | ---: | --- |
| `instant` | 120 ms | Eye direction, tiny reactions |
| `quick` | 220 ms | State acknowledgements |
| `normal` | 350 ms | Pose changes |
| `expressive` | 600 ms | Welcome, success |
| `journey` | 800-1200 ms | Enter, exit, travel |

These durations are initial production targets and may be tuned during the Rive prototype, but changes should preserve their relative hierarchy.

### Easing families

Initial motion curves:

```text
enter      cubic-bezier(.20,.80,.30,1)
settle     cubic-bezier(.20,.70,.20,1)
attention  cubic-bezier(.30,0,.20,1)
```

Core workflow motion should avoid elastic/springy cartoon physics.

## Choreography contract

### `enter -> welcome`

Purpose: Kubidm joins the user.

Sequence:

1. character travels sideways into its safe area;
2. legs complete approximately 1.5-2 walking cycles;
3. body vertical movement remains subtle, around 3% or less;
4. eyes already look toward the destination;
5. band tail lags slightly behind body motion;
6. character decelerates and settles;
7. guide claw opens into the Welcome pose.

Target total duration: approximately **850 ms**.

The UI must be interactive from the start; the entrance never blocks the page.

Reduced-motion mode replaces walking with a short opacity/scale transition.

Static mode renders the approved Welcome SVG immediately.

### `welcome -> guide`

Purpose: direct attention to an actionable UI target.

Sequence:

1. eyes move toward the target, approximately 0-120 ms;
2. body rotates slightly toward target, approximately 120-300 ms;
3. guide claw follows and opens, approximately 180-350 ms;
4. band tail settles by approximately 450 ms.

Target total duration: approximately **450 ms**.

The pose may hold for approximately 700-1000 ms before partially relaxing. It should not repeat the pointing gesture continuously.

### `guide -> protect`

Purpose: show that a user has entered a security-sensitive operation.

Sequence:

1. guide claw retracts;
2. eyes become focused;
3. guardian claw moves toward centre/front;
4. badge/identity signal ramps into restrained cyan illumination;
5. body lowers slightly and stance becomes more stable;
6. tail motion decreases.

Target total duration: approximately **500 ms**.

### `protect -> working`

Purpose: show that Kubidm is processing or waiting securely.

This state must support indefinite duration.

Characteristics:

- guardian stance remains;
- cyan activity pulse is slow, approximately every 1.8-2.4 seconds;
- eyes observe the identity/system indicator;
- body breathing is extremely subtle, around 0.5-1% vertical scale variation;
- tail is almost static;
- no obvious short loop.

During native WebAuthn dialogs, motion should be especially restrained.

### `working -> success`

Purpose: represent a confirmed successful result.

The transition starts only after the application/server confirms success.

Sequence:

1. working indicator converges toward the identity badge;
2. badge emits one short cyan pulse;
3. eyes soften/change to happy state;
4. guardian claw returns from protection position;
5. guide claw opens;
6. body rises approximately 3-4% and settles.

Target total duration for major success: approximately **650 ms**.

Optional cyan accents are limited to a small number of subtle sparks. Do not use confetti for standard identity operations.

### `success.small`

For frequent confirmations such as profile save:

- badge pulse;
- softened eyes;
- small nod/settle.

Target duration: approximately **300-450 ms**.

### `success.major`

For meaningful milestones such as completed authentication:

- badge pulse;
- happy eyes;
- controlled body rise and settle;
- guide claw opens.

Target duration: approximately **600-750 ms**.

### `success -> travel`

Purpose: carry the user into the next product context.

Sequence:

1. eyes look toward travel direction;
2. body subtly prepares for lateral movement;
3. first leg initiates;
4. sideways walking begins;
5. body oscillation remains at or below approximately 3%;
6. claws remain comparatively stable;
7. band tail follows with approximately 100-150 ms of natural lag.

Target cadence: approximately **2.5-3.5 leg cycles per second**.

Typical local travel: **600-900 ms**.

Typical scene exit: **700-1000 ms**.

### Cross-page travel illusion

The outgoing page and incoming page should match travel direction and approximate velocity.

Conceptual sequence:

```text
outgoing page: target position -> off-screen edge
incoming page: opposite edge -> target position
```

The page may change, but the user should perceive that the same crab continued travelling.

This continuity is a signature Kubidm experience.

### `travel -> idle`

Purpose: arrive naturally without snapping to a stop.

Sequence:

1. horizontal velocity reduces during the final ~200 ms;
2. final step completes;
3. body may overshoot target by approximately 1-2%;
4. tail catches up approximately 100 ms later;
5. eyes briefly inspect the new context;
6. character settles into Idle.

Settling should complete approximately **400 ms** after travel ends.

### Idle micro-behaviour

Idle is not a continuously animated loop.

Possible bounded micro-events include:

- blink;
- small gaze movement;
- tiny guide-claw adjustment;
- subtle tail twitch;
- minimal body shift.

A single micro-event may occur at a random interval of approximately **8-15 seconds**.

Only one idle event should be active at a time.

### `idle -> goodbye -> exit`

Purpose: end the session journey.

Sequence:

1. eyes look toward user;
2. guide claw rises;
3. character performs one wave;
4. eyes look toward exit direction;
5. sideways walking begins;
6. body leaves;
7. band tail is the final secondary element to disappear.

The wave should not loop repeatedly.

### Warning transition

A normal state may enter Warning quickly but without drama.

Sequence:

1. eyes move to warning target;
2. smile neutralises;
3. guide claw lowers;
4. guardian claw moves slightly toward centre;
5. character becomes mostly still.

Do not use red flashing, shaking, crying, or exaggerated fear.

### Security-critical mode

Security-critical states deliberately remove motion.

```text
idle motion     off
tail            still
eyes            focused
mouth           neutral/serious
body movement   none
cyan pulse      none or static
```

Examples include account lock, serious policy conflict, attestation failure, or a security-sensitive session state.

## Motion hierarchy by semantic purpose

### Attention

```text
eyes -> body -> guide claw
```

### Security

```text
eyes -> guardian claw -> body -> badge
```

### Success

```text
badge -> eyes -> body -> guide claw
```

### Travel

```text
eyes -> body -> legs -> band tail
```

This hierarchy should guide both rig authoring and animation review.

## Authentication-to-applications storyboard

The first approved end-to-end journey is the passkey authentication happy path.

### Frame 1: Arrival

State: **Welcome**.

The login UI appears and remains immediately usable. Kubidm enters laterally and settles beside the authentication area.

Optional supporting copy belongs in the normal UI, not in a speech bubble.

### Frame 2: Guide to action

State: **Guide / Point**.

The crab looks at the primary passkey action, then uses the guide claw to reinforce the target.

The gesture occurs once and relaxes.

### Frame 3: Authentication started

State: **Protect**.

After the user starts the passkey operation:

- the guide claw retracts;
- guardian posture becomes active;
- cyan identity energy appears;
- the native passkey/WebAuthn interface remains the visual priority.

### Frame 4: Authenticating

State: **Working + Guardian**.

The mascot waits calmly while authentication is pending.

The badge/system signal may pulse slowly. Body and tail motion remain minimal.

### Frame 5: Confirmed success

State: **Success Major**.

After confirmed authentication:

- identity signal converges into the badge;
- badge pulses once;
- expression becomes happy;
- guide claw opens;
- body performs a small controlled rise and settle.

### Frame 6: Travel to Applications

State: **Travel -> Idle**.

The crab looks toward the direction of the next context, walks sideways out of the authentication scene, enters the application portal with matching movement, looks briefly toward the application grid, and settles into Idle.

The intended illusion is that the character travelled with the user rather than disappearing and being replaced by another animation.

## Rive state-machine design baseline

The approved conceptual graph is:

```text
                         +-----------+
                         |  WARNING  |
                         +-----^-----+
                               |
ENTER                          |
  |                            |
  v                            |
WELCOME                        |
  |                            |
  v                            |
 IDLE <------------------------+
  |
  +----> GUIDE
  |        |
  |        v
  |     PROTECT
  |        |
  |        v
  |     WORKING
  |       /   \
  |      /     \
  | WARNING   SUCCESS
  |             |
  |             v
  |           TRAVEL ---------> IDLE
  |
  +----> GOODBYE
             |
             v
            EXIT
```

Global transitions also exist conceptually for:

```text
ANY -> SECURITY_CRITICAL
ANY -> STATIC
ANY -> EXIT
```

The exact Rive implementation may use nested state machines or blend logic, but the browser-facing semantic contract should preserve these meanings.

## Proposed Rive-facing inputs

The implementation should expose semantic values rather than raw animation clip names.

### Scene

```text
auth
applications
profile
credentials
logout
```

### Mode

```text
guide
guardian
security
```

### State

```text
enter
welcome
idle
guide
protect
working
success
warning
travel
goodbye
exit
```

### Triggers

```text
attention
success_small
success_major
warning
start_travel
goodbye
```

### Continuous values

```text
look_x: -1.0 .. 1.0
look_y: -1.0 .. 1.0
```

### Travel direction

```text
left
right
```

### Motion level

```text
full
reduced
static
```

The application-facing API remains renderer-independent as defined in the related ADR.

## Motion accessibility modes

Every major scene must support three presentation levels.

### Full

Uses the complete approved motion language.

### Reduced

Keeps only low-motion semantic feedback such as:

- gaze changes;
- expression changes;
- restrained badge illumination; and
- short cross-fades.

Removes:

- sideways walking;
- body bounce;
- tail secondary motion;
- continuous idle movement; and
- expressive celebration.

### Static

Uses canonical SVG poses with no animation.

Expected fallback set:

```text
idle.svg
welcome.svg
guide.svg
protect.svg
working.svg
success.svg
warning.svg
goodbye.svg
```

The normal HTML UI always communicates the actual workflow state regardless of motion mode.

## Security tone

Security tone is a visual system, not just a copy-writing rule.

### Neutral

Permitted:

- walking;
- pointing;
- gaze guidance;
- presenting;
- restrained idle behaviour.

### Positive

Permitted:

- short badge pulse;
- happy eyes;
- small controlled body rise;
- single wave where contextually appropriate.

### Caution

Behaviour:

- mostly static;
- focused gaze;
- reduced smile;
- guardian claw closer to centre;
- normal UI warning remains primary.

### Critical

Behaviour:

- static or nearly static;
- focused eyes;
- neutral/serious mouth;
- no idle gestures;
- no playful cyan pulse;
- no joke, celebration, crying, or slapstick reaction.

## Repetition and familiarity

The mascot should remain pleasant after repeated daily use.

Design rules:

- do not point repeatedly;
- do not celebrate every successful action;
- use small success for frequent operations;
- keep idle activity infrequent and bounded;
- allow high-frequency users to experience mostly ambient feedback;
- do not delay real actions to complete animation.

A future product decision may introduce familiarity levels, but the underlying motion system should already work when most expressive animations are suppressed.

## Placement principles already established

The mascot is supplementary and must live in a safe visual area.

It must never:

- cover primary controls;
- cover validation messages;
- cover authoritative security alerts;
- change content layout while travelling;
- sit above native browser/WebAuthn dialogs;
- make a form inaccessible at narrow widths; or
- create meaningful layout shift when the runtime loads.

Exact desktop, tablet, and mobile placement remains part of the forthcoming product UI design phase.

## Brand distinctness

The character may share the high-level idea of a friendly crab with Rust community culture, but the Kubidm mascot must remain an original design.

Distinctive Kubidm elements include:

- Kubidm Identity Band;
- central identity badge;
- guide/guardian claw roles;
- 70/30 Guide/Guardian personality model;
- cyan identity-energy semantics;
- sideways workflow-travel language;
- gaze-driven attention language; and
- security seriousness represented through stillness.

Do not reproduce Ferris artwork, exact proportions, silhouette, facial construction, claw geometry, or branding.

## Decisions still open

The following items are intentionally not locked by this document:

- final Kubidm identity glyph inside the badge;
- mascot name, if any;
- final application-wide teal/cyan token values after accessibility validation;
- canonical desktop/tablet/mobile mascot placement;
- final authentication UI visual design;
- complete failure-path storyboard catalogue;
- exact prominence/familiarity behaviour for frequent users;
- final Rive rig topology and bone structure;
- final compressed `.riv` and runtime performance budgets; and
- tenant-visible mascot configuration UI.

## Next design phase

The next design phase is the **Kubidm Product UI Design System**, beginning with the authentication interface.

The first UI design set should cover:

- desktop, tablet, and mobile;
- light and dark themes;
- passkey-first authentication;
- password or other sign-in alternatives;
- reauthentication;
- OAuth application authentication context;
- full-motion mascot placement;
- reduced-motion placement; and
- mascot-disabled/static presentation.

This phase must design the actual product around the mascot rather than altering the mascot again without a specific product requirement.
