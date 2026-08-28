# Kubidm Authentication and Credential-Setup UI

- **Status:** UI design baseline
- **Date:** 2026-07-26
- **Related architecture:** [Mascot-Guided Product Experience](mascot_guided_product_experience.md)
- **Related mascot system:** [Mascot and Motion Design System](mascot_design_system.md)
- **Related product journey:** [Guided Identity Journey](guided_identity_journey.md)
- **Scope:** canonical authentication shell, credential-setup surfaces, responsive behaviour, teaching surfaces, recommendation presentation, and the first interactive prototype scenarios

## Purpose

This document turns the mascot, motion, and guided-journey decisions into a concrete Kubidm UI system.

The goal is not to build a standalone onboarding application. The goal is to redesign the existing Kubidm authentication and credential-management surfaces so that the crab can proactively teach, recommend, guide, celebrate confirmed progress, and become quiet when the user no longer needs help.

The UI must preserve the current server-authoritative authentication model and remain usable without mascot animation, without teaching content, and without JavaScript animation support.

The canonical experience is:

```text
Meet
  -> identify account
  -> choose authentication method
  -> learn when useful
  -> authenticate/configure
  -> confirmed success
  -> resilience recommendation
  -> optional extras/recovery
  -> complete
```

For returning configured users, most of this journey collapses to normal fast authentication.

## Existing-flow grounding

The UI design is derived from the current Kubidm flows rather than replacing them with a client-side wizard.

Existing concepts that remain valid include:

- the shared login base shell;
- tenant/domain image and display name;
- normal login, reauthentication, and OAuth2 login contexts;
- initial username/account identification;
- server-rendered authentication-mechanism selection;
- password authentication;
- passkey/security-key WebAuthn authentication;
- TOTP authentication;
- backup-code authentication;
- server-rendered validation and error states;
- credential reset/update pages; and
- the existing credential-update partial as the authoritative credential-management surface.

The redesign changes hierarchy, presentation, guidance, and progressive teaching. It does not require authentication protocol or route changes as a prerequisite.

## Design objective

The experience should make an ordinary user able to answer, without already understanding IAM terminology:

1. Where am I signing in?
2. Why am I being asked to authenticate?
3. What does Kubidm recommend?
4. What other options are valid?
5. Why is the recommended option useful?
6. What is happening while the browser/device is authenticating me?
7. Am I finished?
8. Is there anything else worth configuring?

The crab helps answer these questions, but the normal interface remains authoritative.

## Canonical authentication shell

### Identity hierarchy

The authentication UI distinguishes three identities.

1. **Kubidm product identity** answers: "What system is guiding this identity flow?"
2. **Tenant/domain identity** answers: "Which organisation or identity domain am I using?"
3. **Application identity** answers: "Where am I going after authentication?" when OAuth2 context exists.

These identities must not be visually conflated.

A tenant logo does not replace the Kubidm product identity. An OAuth application logo does not replace the tenant identity.

### Desktop composition

Desktop uses two conceptual zones:

```text
+--------------------------------------------------------------+
|                                                              |
|  KUBIDM PRODUCT ZONE            AUTHENTICATION TASK ZONE     |
|                                                              |
|  Kubidm mark                    Tenant identity               |
|  restrained brand line          Context heading               |
|                                 Authentication controls       |
|             mascot safe zone    Guidance / teaching           |
|                                                              |
+--------------------------------------------------------------+
```

Baseline proportion:

```text
product zone: approximately 35%
task zone:    approximately 65%
```

This is a composition guideline rather than a hard layout constraint.

The product zone uses the deep-navy brand foundation. The task zone is the focused interaction surface and may be light or dark according to the active theme.

The product zone must not become a marketing page. Repeated authentication is a utility workflow.

Acceptable persistent copy is short, for example:

> Identity that guides every step.

Long feature checklists such as "Passwordless / Privacy-first / Open source" are not part of the normal daily login surface.

### Tablet composition

At intermediate widths:

- remove non-essential product copy;
- reduce the product zone significantly or collapse it into a header band;
- preserve Kubidm product identity;
- preserve tenant/application context;
- preserve a predictable mascot safe zone; and
- keep the authentication task visually dominant.

### Mobile composition

Mobile uses a single-column task surface.

```text
+------------------------------+
| Kubidm                       |
|                              |
| Tenant / destination         |
| Context heading              |
|                              |
| Authentication controls      |
|                              |
| Crab dialog when relevant    |
|                       mascot |
+------------------------------+
```

There is no permanent split-screen product panel on narrow mobile layouts.

The mascot is smaller and more episodic on mobile. It should appear for teaching, meaningful state changes, and relevant recommendations rather than permanently consuming a large percentage of the viewport.

## Mascot safe zone

The crab does not live inside form controls or validation content.

The layout reserves a mascot safe zone adjacent to the task rather than overlaying interactive UI.

The safe zone must guarantee that the mascot does not cover:

- focused inputs;
- primary or secondary buttons;
- WebAuthn/native browser dialogs;
- validation text;
- policy warnings;
- account-lock messages;
- OAuth destination information; or
- credential-removal confirmation controls.

The mascot may visually point or look toward a control without occupying the control's hit area.

If insufficient space exists, the mascot yields space before the form does.

## Canonical UI primitives

The first prototype should treat the following as reusable product components even if the initial implementation uses Askama partials and Bootstrap classes rather than a component framework.

### `AuthenticationShell`

Responsibilities:

- product identity;
- tenant/domain identity;
- optional OAuth application identity;
- reauthentication purpose;
- task area;
- mascot stage/safe zone; and
- theme/responsive layout.

### `IdentityContext`

Presents the current context without relying on an informational alert for normal states.

Normal login example:

```text
Welcome back
Sign in to Acme
```

OAuth example:

```text
Sign in to continue
Grafana
via Acme
```

Reauthentication example:

```text
Confirm it's you
alex@example.com
Before changing your credentials
```

Critical warnings remain alerts. Normal context is page hierarchy, not alert styling.

### `CrabDialog`

Variants are defined by the guided-journey specification:

```text
orient
teach
suggest
celebrate
```

Baseline anatomy:

```text
[ crab ]  +------------------------------------+
          | concise message                    |
          | optional second sentence           |
          |                                    |
          | [primary action]                   |
          | secondary / learn-more action      |
          +------------------------------------+
```

Rules:

- one primary idea per dialog;
- normally no more than two short sentences;
- one primary action at most;
- a valid optional step exposes a clear skip/dismiss path;
- teaching copy explains why rather than merely assigning a label;
- security errors are not delivered only through Crab Dialog;
- the dialog may disappear entirely for experienced users.

### `RecommendationOption`

Used in mechanism selection and post-authentication recommendations.

Each option may contain:

- mechanism/action name;
- short plain-language description;
- one recommendation category;
- optional reason; and
- optional "Learn why" action.

The four categories are:

```text
Required
Recommended
Works OK
Optional
```

`Works OK` must look valid and neutral. It must not look like a warning.

`Required` is authoritative policy presentation and must not be expressed solely through mascot personality.

### `JourneyProgress`

Progress is milestone based rather than score based.

Canonical conceptual stages:

```text
Access
Primary sign-in
Resilience
Recovery
Ready
```

Possible state example:

```text
[check] You can sign in
[check] Recommended sign-in configured
[dot]   Backup/resilience available
[dot]   Recovery ready
```

The exact milestones shown depend on policy and available capabilities. A user must not be shown an impossible target as though it were incomplete work.

### `StoryCard`

A micro-story is a short optional teaching sequence.

It consists of:

- a title framed as a user question;
- 2-3 short frames;
- simple visual metaphor;
- concise technical truth;
- `Got it` or equivalent completion action; and
- skip/close when the story is optional.

Typical duration is approximately 10-20 seconds if the user reads all frames.

### `SecurityNotice`

This is normal authoritative UI, not mascot speech.

Used for:

- policy requirements;
- authentication denial;
- lockout;
- credential-policy conflict;
- serious credential state;
- server failures; and
- destructive actions.

The crab may become Guardian/Warning/Security mode next to a `SecurityNotice`, but the notice stands on its own without the mascot.

## Colour semantics

The UI extends the mascot colour system with explicit product roles.

```text
Deep Navy / Navy
  product structure, dark surfaces, serious security context

Teal
  user action and recommendation emphasis

Cyan
  Kubidm/system identity activity

Coral Orange
  mascot identity and limited brand warmth

Existing semantic warning/danger/success colours
  authoritative product state
```

Important distinction:

> **Teal communicates user action. Cyan communicates Kubidm/system activity.**

Examples:

- `Use passkey` primary button: teal action emphasis;
- badge/network pulse while verifying: cyan;
- selected recommended mechanism: teal treatment;
- policy error: semantic warning/danger treatment, not cyan mascot glow.

Final values remain subject to accessibility and theme validation.

## Canonical screen sequence

The first high-fidelity prototype should implement the following sequence using real Kubidm concepts.

### Screen 1: Meet and identify

Purpose:

- establish product, tenant, and optional destination identity;
- collect the account identifier when required;
- introduce the crab only for new/learning users.

New-user guidance example:

> Hi. I'll help you get signed in, and I'll explain anything unfamiliar along the way.

Primary UI remains the account/username field and normal recovery affordance.

The crab should not turn username entry into a tutorial unless additional explanation is genuinely useful.

Returning experienced user:

- no introductory dialog;
- mascot may remain ambient or absent until useful;
- fastest available authentication path should remain fast.

### Screen 2: Choose authentication method

This screen evolves the current mechanism chooser from an unranked button list into contextual options.

When passkey is available, permitted, and appropriate as the preferred mechanism:

```text
Use a passkey            Recommended
Fast and phishing-resistant.

Use a password           Works OK
A familiar option when your policy allows it.

Other options
```

The exact available choices continue to come from authoritative Kubidm state.

The recommendation engine must never advertise a method that is unavailable for this account, browser, flow, or policy.

Crab suggestion example:

> I'd use a passkey here. It's quick, and it's designed to resist phishing.

Secondary action:

> Why?

### Screen 3: Learn the why

Teaching is contextual and optional unless understanding is necessary for informed consent or safe completion.

Passkey story baseline:

#### Frame 1: Passwords can be handed over

> A password is something you can type into a site, which also means a convincing fake site can try to ask for it.

#### Frame 2: A passkey uses cryptographic keys

> With a passkey, the private key is not sent to Kubidm when you sign in.

#### Frame 3: The site matters

> Passkeys are designed to work with the correct site, which makes phishing much harder.

Avoid the inaccurate blanket statement that every passkey permanently stays on one physical device. Syncable passkeys may be securely synchronized by a platform credential provider.

Story completion returns directly to the user's intended action.

### Screen 4: Authenticate or configure

For WebAuthn/passkey:

- switch crab from Guide to Protect;
- invoke native browser/platform UI without delay;
- reduce mascot movement while native UI is active;
- never visually compete with the browser prompt;
- show normal textual pending state when useful; and
- do not imply success until server confirmation.

Crab copy before native UI may be:

> Your device will take it from here. I'll wait here while it verifies you.

For password:

- render the normal password field;
- preserve password-manager compatibility;
- do not show a disappointment reaction because password was selected;
- optionally provide a subtle future passkey recommendation outside the active password task.

For TOTP:

- present the one-time-code field plainly;
- explain TOTP only when the user appears to need onboarding or asks to learn more;
- invalid syntax/error remains authoritative product feedback.

For backup code:

- communicate that it is a recovery/backup path rather than a normal preferred daily method when that is accurate for the current flow;
- keep the input and submission path straightforward.

### Screen 5: Confirmed success

Only server-confirmed success triggers celebration.

Typical successful authentication:

- short badge pulse;
- happy/soft expression;
- small celebration;
- immediate navigation is never held for the animation.

First-time credential configuration may use a slightly stronger success moment than routine login.

Crab copy:

> All set. You're in.

or, after credential creation:

> Nice. Your passkey is ready.

No points, XP, streaks, or arbitrary score changes are shown.

### Screen 6: Resilience recommendation

After the primary task is complete, the product may recommend a resilience step when it is genuinely useful and policy/capability data supports it.

Example:

> You can sign in now. Want to set up a backup way in?

Possible actions:

```text
[Add backup method]   Recommended
[Not now]
```

This must not be shown as a warning simply because the user chooses `Not now` when the step is optional.

Recommendation logic must avoid introducing a weaker fallback that undermines the intended security policy.

### Screen 7: Credential setup

Credential configuration reuses the same product language inside the existing credentials area.

The canonical layout is:

```text
+---------------------------------------------------------+
| Credentials                                             |
|                                                         |
| Your setup                         mascot / dialog       |
| [journey progress]                                      |
|                                                         |
| Primary sign-in                                        |
| [existing credential controls]                         |
|                                                         |
| Backup and resilience                                  |
| [existing credential controls]                         |
|                                                         |
| Recovery                                               |
| [existing recovery controls where supported]           |
+---------------------------------------------------------+
```

The existing credential-update UI remains authoritative for actual operations.

The guided layer may:

- group capabilities conceptually;
- explain terminology;
- surface one relevant recommendation;
- show progress/milestones;
- celebrate confirmed additions; and
- suppress completed teaching.

It must not hide important existing policy state merely to make the journey look simpler.

### Screen 8: Complete

Completion means the user has reached the applicable recommended configuration for the current policy/capability context, not that they achieved a universal maximum-security score.

Example:

> You're ready. I'll stay out of the way unless you need me.

After this point, normal experience should become quiet.

## Normal login after onboarding

For a configured/experienced user, login should not replay the guided journey.

Expected flow:

```text
context
  -> preferred authentication action
  -> native/browser authentication
  -> confirmed success
  -> destination
```

Teaching surfaces remain available on demand but are not proactive by default.

The mascot behaves primarily as:

- ambient companion;
- state feedback;
- contextual guide when the flow changes; or
- guardian when security context requires it.

## Canonical prototype scenarios

The first interactive prototype must cover five scenarios end to end.

### Scenario A: New user, passkey recommended

```text
Meet
-> identify account
-> method chooser
-> passkey marked Recommended
-> optional passkey story
-> native WebAuthn
-> confirmed success
-> backup/resilience recommendation
-> complete or continue configuration
```

Acceptance points:

- recommendation reason is visible;
- user can understand passkey basics without IAM jargon;
- native UI is not obstructed;
- success requires confirmation;
- optional resilience can be declined safely.

### Scenario B: New user chooses password

```text
Meet
-> identify account
-> method chooser
-> password marked Works OK where accurate
-> password form
-> confirmed success
-> one non-coercive future recommendation if useful
```

Crab example:

> That works. If you want, we can add a passkey later for quicker, phishing-resistant sign-in.

Acceptance points:

- no negative mascot reaction;
- no warning colour because of the valid choice;
- no repeated immediate attempt to change the user's mind;
- normal password-manager behaviour remains intact.

### Scenario C: Returning configured user

```text
context
-> preferred action
-> authenticate
-> success
-> destination
```

Acceptance points:

- no onboarding story;
- no progress checklist unless the user opens configuration;
- no recurring recommendation already dismissed/satisfied;
- mascot does not slow down daily login.

### Scenario D: WebAuthn cancellation

```text
passkey selected
-> Protect
-> native prompt
-> user cancels
-> return to chooser/task
```

Crab copy may be:

> No problem. Nothing changed.

Acceptance points:

- cancellation is not an authentication-security failure;
- no warning animation;
- alternatives remain available according to policy;
- repeated cancellation does not produce nagging.

### Scenario E: Policy-required action

```text
policy requires action
-> SecurityNotice
-> Guardian mode
-> required task
-> confirmation
```

Crab may support orientation:

> This one is required by your organisation. I'll help you through it.

Acceptance points:

- `Required` meaning comes from authoritative product policy;
- requirement is clear without mascot presence;
- no skip affordance when policy forbids skipping;
- serious state uses restrained motion.

## OAuth context

OAuth login should make the application destination part of the page hierarchy rather than a generic informational banner.

Example:

```text
Sign in to continue

[Grafana icon] Grafana
via Acme

[authentication method]
```

The crab can orient the user once:

> You're signing in with Acme to continue to Grafana.

It must not imply that the destination application is operated by Kubidm.

Cross-origin navigation must not wait for mascot exit animation.

## Reauthentication context

Reauthentication should communicate continuity of identity and reason.

Example:

```text
Confirm it's you
alex@example.com
Before changing your credentials
```

The crab becomes more Guardian-like than during ordinary login.

Example copy:

> Quick check before this security change. Confirm it's you and we'll continue.

The reauthentication reason must come from the product context rather than be invented by the mascot layer.

## Recovery context

Recovery is conceptually different from normal authentication.

The UI should not describe recovery methods as equivalent daily sign-in mechanisms when they are not.

Teaching goal:

> Recovery is the route you use when your normal sign-in methods are unavailable.

Recovery workflows should use a calmer Guardian tone and reduced gamification because the user may already be in a stressful situation.

## Failure and error behaviour

### Invalid user input

- authoritative validation remains attached to the relevant field;
- mascot may look toward the field but does not repeat the entire error;
- no comedy or exaggerated concern.

### Authentication failure

- normal product error explains what is safe to reveal;
- mascot uses Warning or restrained Guide mode depending on severity;
- alternatives may be presented if server policy permits them.

### Account lockout or policy denial

- SecurityNotice is primary;
- mascot enters static/minimal Security mode;
- teaching and celebration are suspended;
- no attempt to make the state playful.

### Server/transport error

- distinguish service failure from credential rejection when the server can do so safely;
- do not blame the user;
- do not convert generic HTTP failure directly into a credential-specific mascot reaction.

## Teaching and reminder placement

Proactive teaching is allowed when:

- the user is new to the journey;
- the concept is unfamiliar and relevant now;
- a recommendation materially improves the user's next decision;
- a new capability changes the user's available options; or
- configuration is incomplete and the reminder policy permits another prompt.

Proactive teaching is suppressed when:

- the user has completed the relevant story;
- the user has repeatedly dismissed the same optional recommendation;
- the user is experienced and no material context changed;
- a serious warning/error requires attention;
- native authentication UI is active; or
- the guidance would delay a routine task.

## Responsive Crab Dialog behaviour

### Desktop

- dialog may sit beside the mascot in the safe zone;
- it may visually align with the related control through gaze/pointing;
- it does not cover the authentication card.

### Tablet

- dialog may appear above or below the task card;
- mascot remains secondary to the form;
- long horizontal speech bubbles are avoided.

### Mobile

- dialog becomes a normal in-flow card associated with the mascot;
- no floating bubble may cover the keyboard, input, or browser authentication UI;
- optional stories may open as an in-flow step or sheet that remains keyboard/screen-reader accessible.

## Light and dark themes

Both themes are first-class.

### Light

- task surface uses light neutral background/surface;
- deep navy is used for product structure and text hierarchy;
- teal and cyan retain distinct action/activity roles;
- mascot remains coral/orange.

### Dark

- deep navy/navy surfaces become primary structural backgrounds;
- task cards must maintain sufficient separation from the page background;
- cyan glow must be restrained and must not reduce text readability;
- teal action controls require contrast validation;
- semantic warnings/errors must remain visually distinct from orange mascot colour.

The mascot artwork may need theme-specific outline/shadow treatment, but its identity colours should not fundamentally change between themes.

## Motion modes

### Full

Uses the approved motion grammar:

- entrance;
- gaze;
- pointing;
- Protect/Working;
- success;
- sideways travel;
- secondary tail motion; and
- bounded idle gestures.

### Reduced

Keeps information and state changes while removing most translation and secondary motion.

Permitted examples:

- expression change;
- gaze change;
- single badge pulse;
- cross-fade between poses.

### Static

Uses canonical SVG poses with all guidance text and controls unchanged.

No product decision may depend on the motion mode.

## Accessibility requirements

The prototype must support:

- keyboard-only operation;
- visible focus states;
- screen-reader-accessible labels and errors;
- logical heading hierarchy;
- live status where current Kubidm flows require it;
- 200% zoom without mascot overlap;
- reduced motion;
- forced colours/high contrast where supported;
- touch targets appropriate for mobile; and
- authentication completion without the mascot runtime.

The animated crab is normally decorative from an accessibility-tree perspective when its information is duplicated by normal UI.

Crab Dialog content is real UI content and therefore must be accessible when present.

Animation should not produce inaccessible duplicate announcements.

## Template-aligned implementation strategy

The first prototype should be achievable by evolving current templates rather than introducing a new frontend framework.

Conceptual mapping:

```text
login_base.html
  -> AuthenticationShell + IdentityContext + mascot stage

login.html
  -> account identification state

login_mech_choose.html
  -> RecommendationOption list + optional Crab Dialog

login_webauthn.html
  -> Protect/Working integration around existing WebAuthn buttons

login_password.html
  -> password task with neutral Works OK presentation where applicable

login_totp.html
  -> TOTP task + optional teach affordance

login_backupcode.html
  -> backup/recovery task semantics

credentials_reset.html / credentials_status.html
  -> credential-setup shell + JourneyProgress + guidance area

credentials_update_partial
  -> remains authoritative operation content
```

The exact partial boundaries may differ during implementation.

## Semantic presentation contract

The UI consumes product semantics and derives guidance presentation.

Conceptual state:

```text
scene
journey_stage
operation
action
status
severity
recommendation
guidance_mode
teaching_state
experience_state
motion_mode
```

Example: passkey recommendation

```text
scene: auth
journey_stage: primary_auth
operation: passkey
action: present
status: idle
severity: neutral
recommendation: recommended
guidance_mode: teach
teaching_state: available
experience_state: new
motion_mode: full
```

Example: optional password chosen

```text
scene: auth
journey_stage: primary_auth
operation: password
action: present
status: pending
severity: neutral
recommendation: works_ok
guidance_mode: guide
teaching_state: suppressed
experience_state: learning
motion_mode: full
```

Example: required policy action

```text
scene: credentials
journey_stage: resilience
operation: policy-required-action
action: protect
status: warning
severity: caution
recommendation: required
guidance_mode: guardian
teaching_state: suppressed
experience_state: configured
motion_mode: reduced
```

The renderer does not decide `recommendation`, policy, or allowed alternatives.

## Prototype implementation order

### Phase UI-1: Authentication shell

Implement static/high-fidelity versions of:

- normal login;
- OAuth login;
- reauthentication;
- desktop/mobile;
- light/dark.

No Rive dependency is required to validate hierarchy.

### Phase UI-2: Method chooser and Crab Dialog

Implement:

- recommendation categories;
- passkey Recommended example;
- password Works OK example;
- `teach` and `suggest` Crab Dialog variants;
- optional micro-story entry.

### Phase UI-3: Mechanism tasks

Integrate:

- passkey/security key;
- password;
- TOTP;
- backup code;
- authoritative errors.

### Phase UI-4: Credential setup and progress

Integrate:

- credential management;
- journey milestones;
- one resilience recommendation;
- success and completion states.

### Phase UI-5: Motion prototype

Attach the approved mascot rig/state system to the already-valid static UI.

Validate:

- full;
- reduced;
- static;
- native WebAuthn priority;
- no layout shift;
- no blocked action.

## Prototype acceptance criteria

The canonical prototype is successful when:

1. a new non-expert user can distinguish the recommended method from valid alternatives;
2. the recommendation includes a plain-language reason;
3. a user can choose a valid non-recommended option without negative visual treatment;
4. routine returning-user login remains fast and quiet;
5. WebAuthn native UI remains visually and functionally primary while active;
6. a server-confirmed result is required before celebration;
7. optional resilience can be skipped without fake warning state;
8. required policy actions are understandable without mascot presence;
9. the same interaction hierarchy works on desktop and mobile;
10. light, dark, reduced-motion, and static modes preserve all necessary information;
11. authentication still works if the guide runtime fails; and
12. the UI can be implemented incrementally in the existing Askama/HTMX architecture.

## Design decisions locked by this baseline

- Authentication uses a reusable product/tenant/task hierarchy rather than treating tenant branding as the product identity.
- Desktop provides a Kubidm product zone and a focused authentication task zone.
- Mobile collapses to a single-column task-first layout.
- The mascot has a non-overlapping safe zone and yields space before essential UI.
- Crab Dialog is an accessible UI primitive, not text baked into animation.
- Normal authentication context is page hierarchy, not informational-alert styling.
- Method selection exposes contextual recommendation categories.
- Teal represents user action/recommendation emphasis; cyan represents Kubidm/system activity.
- Teaching is proactive for new users and decays for configured/experienced users.
- Valid non-recommended methods are accepted without guilt or negative mascot reaction.
- Credential setup uses journey milestones rather than scores or XP.
- Native WebAuthn UI takes priority over mascot animation.
- Credential and policy operations remain authoritative in normal Kubidm UI.

## Open questions

The following remain implementation/design follow-ups:

- exact CSS spacing, radius, typography, and breakpoint tokens;
- final theme-adjusted teal/cyan values after contrast testing;
- final identity glyph;
- exact mechanism recommendation mappings for each policy combination;
- how guidance memory is persisted;
- which resilience/recovery combinations are safe to recommend automatically;
- whether the product zone remains visible on every desktop reauthentication flow;
- how tenant custom branding interacts with dark/light task surfaces;
- whether micro-stories render inline, in a sheet, or both depending on viewport;
- final wording after UX/security review and localisation planning; and
- final Rive rig integration details.

## Next milestone

Build the first interactive prototype for the five canonical scenarios, beginning with **Scenario A: new user with passkey recommended**.

The first prototype should prove the product interaction and information hierarchy with static mascot states before full animation is required. Once the static flow is correct, attach the Rive state machine and validate the same journey in full, reduced, and static modes.
