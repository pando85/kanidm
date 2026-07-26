# Kubidm Mascot and Motion Design System

- **Status:** Design baseline
- **Date:** 2026-07-26
- **Related ADR:** [Mascot-Guided Product Experience](mascot_guided_product_experience.md)
- **Related product journey:** [Guided Identity Journey](guided_identity_journey.md)
- **Related UI system:** [Authentication and Credential-Setup UI](authentication_credential_ui.md)
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

The body is stable, claws relaxed, eyes attentive, and mouth in a small neutral-friendly expression. The band tail hangs naturally.

Idle communicates:

> I am here if you need me.

It must not communicate:

> Look at me.

### Welcome

Welcome is an invitation into a journey rather than a repeated wave.

- guide claw opens outward;
- guardian claw stays closer to the body;
- eyes look toward the user/task;
- body may rise slightly;
- band tail follows softly.

The wave gesture is reserved primarily for Goodbye.

### Guide / Point

Guide/Point is a signature pose.

The guide claw rotates or narrows toward a UI target without becoming a literal finger.

The gaze and guide claw must agree about the target.

Motion order:

```text
eyes -> body -> guide claw
```

### Working / Inspect

Working does not use frantic spinner behaviour.

The crab focuses on a small cyan identity/system signal while the body remains relatively calm.

The activity indicator may animate more than the mascot.

### Protect

Protect reveals the Guardian side.

- guardian claw moves forward/central;
- eyes become focused;
- stance becomes more stable;
- mouth becomes neutral;
- band tail movement reduces;
- identity badge may illuminate cyan when Kubidm is actively processing identity state.

### Success

Success is brief and proportional to the importance of the operation.

Two levels are defined:

```text
success.small
success.major
```

`success.small` is suitable for ordinary saved changes.

`success.major` is suitable for authentication completion, first passkey setup, or other meaningful milestones.

Success must only follow an authoritative product confirmation.

### Warning

Warning is controlled attention rather than panic.

- eyes focus on the relevant warning;
- smile disappears;
- guide claw lowers;
- guardian claw moves closer to centre;
- body remains mostly stable.

### Goodbye

Goodbye uses the signature wave and sideways exit.

Sequence:

```text
look at user
-> raise guide claw
-> one wave
-> look toward exit
-> sideways travel
-> band tail exits last
```

## Expression vocabulary

### Eyes

```text
open
soft
focused
looking-left
looking-right
closed-happy
concerned
```

### Brows

```text
neutral
curious
focused
concerned
```

### Mouth

```text
neutral
small-smile
happy
serious
```

The vocabulary is intentionally bounded. Kubidm is not an animated-film character.

## Signature interaction principles

### Sideways movement communicates journey

The crab's natural lateral walk becomes Kubidm's progress metaphor.

It may appear to travel between workflow contexts, including across page navigation, by matching exit and entrance direction/velocity.

### Gaze communicates attention

Gaze is a first-class interaction primitive.

The crab can visually connect itself to a real UI target without speech or repeated pointing.

### Cyan communicates identity-system activity

Cyan appears when Kubidm is actively performing identity-related work.

### Stillness communicates seriousness

The more serious the security state, the less the mascot moves.

This is a locked design rule.

## Timing system

Motion uses a small timing vocabulary.

| Token | Duration | Typical use |
| --- | ---: | --- |
| `instant` | 120 ms | eye direction, tiny reaction |
| `quick` | 220 ms | acknowledgement |
| `normal` | 350 ms | pose transition |
| `expressive` | 600 ms | welcome or significant success |
| `journey` | 800-1200 ms | enter, exit, or travel |

Animation must never block an operation until a timing token completes.

## Easing families

Conceptual easing families:

```text
enter      cubic-bezier(.20,.80,.30,1)
settle     cubic-bezier(.20,.70,.20,1)
attention  cubic-bezier(.30,0,.20,1)
```

These may be tuned in the final Rive rig while preserving the intended character: responsive starts, soft landing, controlled security motion.

## Core transition choreography

### Enter -> Welcome

Approximate total: 850 ms.

1. sideways walk into the mascot safe zone;
2. approximately 1.5-2 walking cycles;
3. small vertical oscillation only;
4. tail trails body movement;
5. stop and settle;
6. guide claw opens;
7. eyes orient toward the task/user.

The UI is usable from the start of the animation.

### Welcome -> Guide

Approximate total: 450 ms.

Order:

```text
eyes
-> small body orientation
-> guide claw
-> tail settle
```

The guide gesture happens once, then relaxes while gaze may remain on the target.

### Guide -> Protect

Approximate total: 500 ms.

1. guide claw retracts;
2. eyes become focused;
3. guardian claw moves forward;
4. badge receives controlled cyan illumination;
5. stance becomes slightly lower/stabler;
6. tail movement reduces.

No bouncing.

### Protect -> Working

Working is an indefinite-capable state.

- guardian posture remains;
- badge/activity pulse is slow;
- body breathing is minimal;
- eyes focus on the active system signal;
- tail nearly still.

The loop must look natural whether the operation takes 300 ms or 20 seconds.

### Working -> Success

Approximate total: 650 ms for major success.

1. activity signal resolves into the badge;
2. one cyan pulse;
3. eyes soften;
4. guardian claw relaxes;
5. body rises slightly and settles;
6. guide claw opens.

### Success -> Travel

1. eyes look toward travel direction;
2. body prepares laterally;
3. legs enter walking cycle;
4. body travels sideways;
5. tail lags approximately 100-150 ms.

Walking should feel competent and energetic, not like cartoon running.

### Travel -> Idle

1. horizontal movement decelerates;
2. complete the final step;
3. body settles;
4. tail catches up;
5. brief gaze scan of the new context;
6. enter Idle.

### Idle -> Goodbye -> Exit

Approximate goodbye sequence: around 1 second before/while non-blocking exit begins.

1. look at user;
2. raise guide claw;
3. one wave;
4. look toward exit;
5. walk sideways out;
6. tail disappears last.

Logout/navigation does not wait for the animation.

## Idle behaviour

Idle is mostly static.

Occasional micro-events may occur on a randomised bounded interval, approximately 8-15 seconds in the full-motion baseline:

- blink;
- slight eye glance;
- small claw adjustment;
- subtle tail twitch;
- minimal body shift.

Only one idle event should occur at a time.

Deterministic repetitive loops are discouraged because they feel robotic and become distracting during daily use.

## Warning and critical security motion

### Warning

- fast gaze toward the relevant UI;
- expression becomes neutral/concerned;
- guide claw lowers;
- guardian claw comes closer to centre;
- movement stops quickly.

### Security-critical

```text
idle motion:   off
tail:          still
eyes:          focused
mouth:         serious/neutral
body motion:   none
cyan pulse:    none or static when semantically useful
```

The authoritative product message remains the primary communication.

## Body-part motion hierarchy

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

These hierarchies guide rig choreography and transition blending.

## Rive state-machine baseline

Conceptual states:

```text
ENTER
  -> WELCOME
  -> IDLE
      -> GUIDE
      -> PROTECT
          -> WORKING
              -> SUCCESS
              -> WARNING
      -> GOODBYE
          -> EXIT

ANY -> WARNING
ANY -> SECURITY_CRITICAL
ANY -> STATIC/FALLBACK
```

Product-specific semantics remain outside Rive.

## Rive-facing inputs

Conceptual inputs include:

```text
scene:
  auth | applications | profile | credentials | logout

mode:
  guide | guardian | security

state:
  enter | welcome | idle | guide | protect | working |
  success | warning | travel | goodbye | exit

look_x: -1.0 .. 1.0
look_y: -1.0 .. 1.0

travel_direction:
  left | right

motion_level:
  full | reduced | static
```

Possible triggers:

```text
attention
success_small
success_major
warning
start_travel
goodbye
```

Names may change during implementation, but the separation between semantic product state and renderer input is mandatory.

## Motion modes

### Full

Full character motion as defined above.

### Reduced

Keep:

- gaze changes;
- expression changes;
- controlled badge illumination;
- cross-fades or minimal pose transitions.

Remove or minimise:

- walking;
- bounce;
- tail physics;
- repeated idle gestures;
- decorative travel.

### Static

Use canonical SVG states such as:

```text
idle
guide
protect
working
success
warning
goodbye
```

All essential information remains in normal UI.

## First authentication storyboard

The first approved happy-path storyboard is:

```text
Arrival
-> Guide to action
-> Authentication started
-> Working / native WebAuthn pending
-> Confirmed success
-> Sideways travel
-> Applications idle
```

The complete product and teaching hierarchy for this journey is defined in [Authentication and Credential-Setup UI](authentication_credential_ui.md).

## Design-system relationship

The documents have distinct responsibilities:

- [Mascot-Guided Product Experience](mascot_guided_product_experience.md) defines architecture and integration boundaries;
- this document defines canonical character and motion behaviour;
- [Guided Identity Journey](guided_identity_journey.md) defines proactive teaching, recommendations, progression, and guidance decay;
- [Authentication and Credential-Setup UI](authentication_credential_ui.md) defines the concrete screen/component hierarchy for the first implementation prototype.

## Open design work

- final identity glyph;
- final vector construction/model sheet source;
- final Rive rig topology and control names;
- precise leg cycle and deformation limits after rig testing;
- final eye/brow shapes at small sizes;
- final theme-specific edge/shadow treatment;
- measured CPU/main-thread behaviour of idle animation;
- final static SVG production assets;
- whether the mascot receives a formal name.
