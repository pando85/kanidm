# Kubidm Guided Identity Journey

- **Status:** Product experience baseline
- **Date:** 2026-07-26
- **Related architecture:** [Mascot-Guided Product Experience](mascot_guided_product_experience.md)
- **Related visual system:** [Mascot and Motion Design System](mascot_design_system.md)
- **Scope:** proactive guidance, authentication education, recommendation semantics, configuration progression, micro-stories, gamification boundaries, dialog content, and guidance decay

## Purpose

Kubidm should make identity configuration understandable rather than merely exposing credential controls.

The crab is therefore more than a state indicator. It is a **proactive identity coach** that teaches users what the available authentication methods mean, recommends a good default, explains valid alternatives, guides configuration, and turns an otherwise dry security setup into a short narrative journey.

The experience must remain technically honest and security-preserving. The guide may explain and recommend, but server-side policy and normal product UI remain authoritative.

The intended progression is:

```text
teacher -> guide -> companion -> guardian when needed
```

A new user receives more explanation. A configured or experienced user sees much less. Security-critical situations always prioritise clarity and authoritative UI over personality.

## Product promise

The guided journey should make users feel:

> I understand what I am choosing, I know what Kubidm recommends, and I can finish without becoming an identity expert.

The experience is intentionally opinionated, but not coercive.

Kubidm should be willing to say both:

> I recommend a passkey here.

and:

> A password works too. We can improve this later if you want.

when both statements are true under the active account and domain policy.

## Core principles

### Teach before asking for unfamiliar choices

A user should not need to understand terms such as WebAuthn, TOTP, recovery code, or attestation before completing a workflow.

When a concept is likely to be unfamiliar, Kubidm gives a short explanation in normal language and offers a deeper explanation only on demand.

### Recommend, do not shame

A supported alternative is not represented as failure simply because Kubidm prefers another option.

If password authentication is valid under policy, choosing it must not produce a disappointed mascot, red warning state, negative score, or guilt-oriented copy.

### Explain why

Recommendations need a reason.

Poor:

> Passkeys are recommended.

Better:

> I recommend a passkey. It is quick to use and designed to resist phishing.

### Policy is authoritative

The guide never decides whether a credential is allowed, sufficient, required, or successfully configured.

These facts come from Kubidm's server-side account, authentication, and policy state.

The guide translates authoritative state into teaching and presentation.

### Progress, not pressure

The configuration journey may show milestones and completion, but it does not use:

- XP;
- points;
- streaks;
- leaderboards;
- arbitrary security scores;
- fake urgency;
- loss aversion; or
- punishments for skipping an optional step.

The desired feeling is completion of a useful journey, not optimisation of a game mechanic.

### Security seriousness overrides personality

The mascot may be expressive while teaching or celebrating a safe, confirmed outcome.

Policy denial, account lockout, serious credential problems, and other critical states remain calm and minimally animated. The authoritative warning or error is normal product UI.

## Recommendation taxonomy

Every user-facing authentication or security option exposed through the guided journey should be classified into one of four presentation categories.

| Category | Meaning | Presentation |
| --- | --- | --- |
| **Required** | Active policy requires this action before the workflow can proceed | Authoritative product requirement; Guardian mode may support it |
| **Recommended** | Preferred choice for the current context when policy permits alternatives | Primary recommendation with concise reason |
| **Works OK** | Valid and supported, but Kubidm has a preferred choice for this context | Neutral choice; no warning styling or negative reaction |
| **Optional** | Additional convenience, resilience, or protection that is not necessary to complete the current journey | Secondary choice; safe to skip when policy permits |

These categories are not a global ranking of authentication technologies.

Classification is contextual. It depends on:

- domain policy;
- account policy;
- authentication mechanism availability;
- device/browser capability;
- whether the flow is normal authentication, reauthentication, recovery, or credential configuration;
- what the account already has configured; and
- whether an additional mechanism improves resilience without weakening the intended authentication policy.

The browser must not independently infer `Required` or `Recommended` from visible controls.

## Baseline recommendation policy

Where the active policy allows multiple suitable choices and the client supports them, Kubidm should generally present passkeys as the preferred normal sign-in experience.

This is a product recommendation, not a protocol rule.

A valid password remains a `Works OK` option when permitted by policy. Hardware security keys may be recommended or optional depending on the deployment and required assurance. TOTP may be required as part of a multi-factor mechanism, or presented as an available alternative where appropriate. Backup codes are emergency material and should not be presented as an everyday sign-in preference.

The exact ordering must be derived from the mechanisms that the server actually offers. The current login flow already exposes a mechanism-choice step rather than assuming one global method; the guided experience builds on that model.

## Journey model

The initial guided identity journey consists of eight conceptual stages.

```text
1. Meet Kubidm
       |
2. Choose how to authenticate
       |
3. Learn why
       |
4. Configure
       |
5. Confirmed success
       |
6. Build resilience
       |
7. Optional extras / recovery
       |
8. Complete
```

The stages are conceptual, not mandatory pages. A deployment may skip stages when they are irrelevant or already satisfied.

For example, an account with a policy-required hardware authenticator may enter directly into explanation and configuration rather than showing a meaningless choice screen.

## Progress semantics

The journey uses milestones rather than a numerical security score.

Suggested milestones:

```text
ACCESS
Can authenticate under current policy

PRIMARY
Preferred normal sign-in method configured, where applicable

RESILIENCE
An approved independent fallback or backup path exists, where applicable

RECOVERY
Available recovery path understood/configured, where supported

COMPLETE
Current recommended setup reached
```

A user may remain in a valid intermediate state.

For example:

```text
Can sign in
    -> Good setup
        -> Resilient setup
```

`Complete` means the current recommendation set is satisfied, not that the account is permanently secure or that every possible credential has been added.

## Canonical first-run journey

### Stage 1: Meet Kubidm

**Goal:** establish the crab's role and explain that the setup will be guided.

**Crab mode:** Welcome / Guide.

**Default dialog:**

> Hi. I'm Kubidm. I'll help you set up how you sign in, and I'll explain the choices as we go.

**Primary action:** `Continue`

**Secondary action:** `Skip introduction`, when there is no required explanatory or configuration step.

The introduction should be shown once, not on every login.

### Stage 2: Choose authentication method

**Goal:** present only methods currently allowed and available, while making the recommended path obvious.

**Crab mode:** Guide / Suggest.

When a passkey is available and is the recommended option:

> If you can, I'd use a passkey. It's quick to use and designed to resist phishing.

Example options:

```text
Set up / Use a passkey        Recommended
Use a password                Works OK
Other available methods       Contextual classification
```

If the user chooses password:

> That works. Use a unique password; a password manager can make that much easier. We can add a passkey later if your policy allows it.

The crab accepts the choice immediately. There is no negative animation.

If only one mechanism is allowed, Kubidm does not pretend the user has a choice. The screen explains the required method and proceeds.

### Stage 3: Learn why

**Goal:** explain the recommended method without forcing a documentation detour.

**Crab mode:** Teach.

Teaching is a short micro-story. The user can skip it unless an administrator has a separate non-mascot acknowledgement requirement.

For passkeys, the default teaching story is:

**Panel 1 — the problem**

> Passwords can be typed into the wrong site or stolen after they are reused or exposed.

**Panel 2 — what changes**

> With a passkey, your private key is not sent to Kubidm during sign-in, and the credential is designed to work with the correct site.

**Panel 3 — practical result**

> That gives you fast sign-in with strong phishing resistance, without another password to remember.

Do not say that all passkeys "stay on one device". Some passkeys can be synchronised by credential providers. Teaching copy should describe the security property that matters to Kubidm rather than make assumptions about credential storage.

**Primary action:** `Set up a passkey`

**Secondary action:** `Maybe later`, when policy permits.

### Stage 4: Configure

**Goal:** make a security-sensitive setup feel supervised while keeping native/browser UI primary.

**Crab mode:** Protect, then Working.

Before browser or OS WebAuthn UI takes focus:

> Your browser or device will take over for a moment. I'll wait here while you finish.

While the external prompt is active:

- the crab becomes quiet;
- the guardian claw is active;
- the badge may show restrained cyan activity;
- no repeated dialog is shown; and
- native/browser instructions remain visually dominant.

The guide must never claim that a credential has been created until Kubidm has confirmed it.

### Stage 5: Confirmed success

**Goal:** reward successful completion without excessive celebration.

**Crab mode:** Success.

For a major milestone such as first passkey registration:

> Done. Your passkey is ready.

Optional second line:

> You can use it the next time Kubidm offers this sign-in method.

This uses `success.major`, but remains short enough that the user can continue immediately.

For a routine credential update, use `success.small` and shorter copy.

### Stage 6: Build resilience

**Goal:** explain why losing one device or method should not necessarily become an account-recovery incident.

**Crab mode:** Suggest / Guardian-light.

If the account and policy support an approved independent backup path:

> You can sign in now. I'd also set up a backup path in case your usual device is unavailable.

The system must not blindly recommend "more methods". A weaker fallback can undermine the benefit of a stronger primary method if policy treats both as equivalent authentication paths.

The recommendation engine must therefore expose only backup/resilience options approved for this account and policy.

**Primary action:** context-specific, for example `Add backup method`.

**Secondary action:** `Not now`, when optional.

### Stage 7: Optional extras and recovery

**Goal:** distinguish useful additional setup from requirements.

**Crab mode:** Guide.

Default framing:

> You're already set up. These are optional ways to make recovery or sign-in fit you better.

Possible items depend entirely on server capabilities and policy. Examples may include:

- an approved backup authentication method;
- hardware security key registration;
- TOTP where supported by the current account policy;
- backup codes where supported;
- account recovery configuration; and
- future Kubidm-supported recovery mechanisms.

No unavailable or disallowed method should be shown merely for educational completeness.

### Stage 8: Complete

**Goal:** explicitly end onboarding and reduce future mascot activity.

**Crab mode:** Celebrate, then Idle.

Default dialog:

> You're ready. I'll stay quiet unless something changes or you ask for help.

Alternative when the account is valid but has skipped optional recommendations:

> You're ready to sign in. There are a couple of optional improvements left, and I'll remind you gently later if they're still useful.

Completion should feel like relief, not the start of an engagement loop.

## Alternative method paths

### Password

When password is allowed but a stronger/easier method is recommended, classify it as `Works OK`, not as a warning.

Teaching copy:

> A password works here. Make it unique to this account; a password manager is the easiest way to avoid reusing passwords.

If another factor is required by policy, the guide should explain that the password is only one part of the sign-in path.

### Hardware security key

Teaching copy:

> A security key is a strong choice when you want a separate physical authenticator. Like passkeys, WebAuthn security keys are designed to resist phishing.

The UI should not imply that a security key is universally better than a passkey. The recommendation depends on deployment requirements and user context.

### TOTP

Teaching copy:

> This code gives Kubidm another proof that it's you. It is useful when your policy asks for a second factor, but the code itself can still be entered into a convincing fake site, so follow the sign-in page carefully.

This explanation is deliberately honest about the difference between TOTP and phishing-resistant WebAuthn authentication.

### Backup code

Teaching copy:

> A backup code is for emergencies, not everyday sign-in. Keep it private and somewhere you can reach if your normal authenticator is unavailable.

If codes are single-use, the authoritative product UI should state that property based on the actual implementation rather than relying on mascot copy.

### Account recovery

Teaching copy:

> Recovery is what we use when normal sign-in is no longer available. It should be set up carefully because it becomes another path back into your account.

Recovery must never be presented as equivalent to normal authentication merely because both can restore access.

## Micro-story system

Teaching content should normally take 10-20 seconds and contain no more than three short panels.

Each micro-story has:

```text
problem -> mechanism -> practical consequence
```

The user may skip or dismiss stories unless product policy independently requires acknowledgement.

### Story: Why passkeys resist phishing

**Problem**

> A fake sign-in page can ask you to type a password or one-time code.

**Mechanism**

> A passkey is cryptographically tied to the site it was created for, and its private key is not sent to Kubidm.

**Consequence**

> A look-alike site cannot simply ask you to hand over the same secret.

### Story: Why passwords need to be unique

**Problem**

> If the same password is used in several places, one breach can create problems somewhere else.

**Mechanism**

> A password manager can generate and remember a different password for each account.

**Consequence**

> One exposed password does not automatically reveal the others.

### Story: Why a backup path matters

**Problem**

> Phones, laptops, and hardware keys can be lost, replaced, or unavailable.

**Mechanism**

> An independent recovery or backup path gives you another approved way back in.

**Consequence**

> Losing one authenticator is less likely to become an administrator-assisted recovery incident.

### Story: More methods are not automatically stronger

**Problem**

> A strong primary method can be undermined if the account also accepts an unnecessarily weak fallback.

**Mechanism**

> Kubidm recommends only backup paths that fit the active policy and assurance requirements.

**Consequence**

> Resilience should add another safe path, not create an easy bypass.

### Story: Recovery is different from sign-in

**Problem**

> Sometimes none of your normal authenticators are available.

**Mechanism**

> Recovery uses a separate, deliberately controlled process to restore access.

**Consequence**

> It can get you back in, but because it is powerful, it deserves the same careful treatment as credentials.

## Crab dialog component

The teaching experience introduces a specific UI primitive: **Crab Dialog**.

It is not an alert, toast, error message, or chat interface.

Conceptual model:

```text
CrabDialog
  variant: orient | teach | suggest | celebrate
  title: optional
  body: required
  primary_action: optional
  secondary_action: optional
  learn_more: optional
  dismissible: boolean
```

### Orient

Used to establish context or the next step.

Example:

> Let's choose how you'll sign in.

### Teach

Used for one concise concept or micro-story.

Example:

> Passkeys are designed to resist phishing because the credential is tied to the real site.

### Suggest

Used for an opinionated but non-required recommendation.

Example:

> You're set up. I'd add a backup path too, if your policy offers one.

### Celebrate

Used after server-confirmed success.

Example:

> Nice. Your passkey is ready.

### Dialog content rules

- normally no more than two short sentences on a single dialog;
- one concept per dialog;
- explain the reason for recommendations;
- use ordinary language first and technical terminology second;
- do not mimic a human conversation or imply sentience;
- do not ask open-ended questions that suggest a chatbot capability;
- do not use mascot dialog as the only source of instructions required to complete a task;
- do not place credential material, usernames, secrets, challenges, or raw policy errors in mascot dialog;
- do not use dialog to soften or reinterpret an authoritative security denial; and
- do not block navigation merely to finish a line of mascot copy.

## Proactivity model

The crab is proactive only when the intervention has immediate value.

### High-proactivity moments

The guide may initiate teaching or suggestions during:

- first encounter;
- initial authentication-method selection;
- first configuration of an unfamiliar credential type;
- a transition from minimally valid setup to recommended setup;
- a newly relevant resilience/recovery opportunity;
- a major capability becoming available after an upgrade or policy change; and
- a required security action whose reason may not be obvious.

### Low-proactivity moments

The guide should normally remain ambient during:

- routine returning-user login;
- repeated application selection;
- ordinary profile viewing;
- credential pages where the user has already dismissed the same optional recommendation; and
- repeated successful actions that require no new learning.

### Never interrupt for

- decorative trivia;
- unrelated release notes;
- repeated reminders during the same session;
- an optional recommendation immediately after the user explicitly rejected it;
- marketing copy; or
- mascot engagement for its own sake.

## Guidance decay

Guidance becomes quieter as the user gains experience.

| Experience state | Default behaviour |
| --- | --- |
| **New** | Proactive orientation and teaching |
| **Learning** | Contextual teaching and recommendations |
| **Configured** | Mostly ambient; recommendations only when materially useful |
| **Experienced** | State feedback only by default |
| **Security event** | Guardian behaviour regardless of experience level |

The system should not infer experience solely from account age. A long-lived account may still encounter a credential type for the first time.

Guidance is better tracked per concept and journey than through one global "expert" flag.

## Reminder and suppression rules

Optional recommendations need bounded repetition.

Initial baseline:

1. show the recommendation contextually when it first becomes relevant;
2. if dismissed, do not show it again during the same session;
3. a later subtle reminder is permitted only when the user returns to a directly relevant security/configuration surface;
4. repeated dismissal progressively suppresses the recommendation;
5. once the recommendation is satisfied, remove it immediately; and
6. a material policy or capability change may make an old recommendation relevant again.

Routine login should not become a reminder channel for every incomplete optional security step.

## Guidance memory

Security state and policy remain server-authoritative.

The guide may additionally need non-security-critical memory such as:

```text
introduction_seen
story_passkeys_seen
story_totp_seen
backup_recommendation_dismissed
recovery_story_seen
```

These values affect presentation only. They must never affect whether Kubidm permits authentication, authorisation, recovery, or credential changes.

The persistence mechanism is intentionally left open for implementation design. Cross-device account preference is preferable when a suitable server-side user preference mechanism exists; local browser storage may be acceptable for purely local presentation hints but must not become a security dependency.

## Failure and cancellation behaviour

### User cancels WebAuthn/native prompt

Crab returns calmly from Protect/Working to Guide or Idle.

Suggested copy only when clarification is useful:

> No problem. Nothing was added. You can try again or choose another available option.

Do not classify cancellation as an account failure.

### Recoverable input error

Normal product validation is authoritative.

The crab may enter Warning and direct gaze toward the error, but should not repeat the full validation message.

### Policy denial

Guardian/security mode.

The authoritative policy explanation remains normal UI. Mascot motion becomes restrained or static.

### Account lockout or critical security state

No gamification or teaching story.

The guide becomes quiet and serious. Only concise orientation is permitted if it helps the user find the authoritative next action.

### Network/server error

Do not invent a security interpretation.

The normal error component explains the transport or server problem. The crab may use a concerned neutral posture and stop active progress animation.

## Returning-user experience

The normal returning-user login should be much quieter than onboarding.

Typical flow:

```text
Welcome / Idle
    -> Guide primary allowed method when useful
    -> Protect / Working during authentication
    -> Success
    -> Travel to applications
```

No passkey lesson is repeated simply because a passkey button is visible.

If the user requests help, `Learn more`, or opens a security configuration journey, teaching becomes available again.

## Reauthentication and OAuth

### Reauthentication

The crab explains why the product is asking again without implying that the existing session is invalid.

Example:

> Kubidm needs to confirm it's you before this sensitive action continues.

The authoritative purpose is supplied by the server-rendered reauthentication context.

### OAuth application login

The guide should distinguish the identity provider from the destination application.

Example:

> You're signing in through Kubidm to continue to Grafana.

The application name/logo comes from the authoritative OAuth client context. The crab may orient the user, but must not imply that Kubidm endorses the application.

## Semantic product state

The guided journey adds presentation semantics above the renderer-level mascot state defined in the architecture ADR.

Conceptual fields:

```text
journey_stage:
  none | introduction | access | primary_auth | teaching | configure |
  resilience | recovery | complete

recommendation:
  none | optional | works_ok | recommended | required

guidance_mode:
  ambient | orient | teach | suggest | celebrate | guardian

teaching_state:
  none | available | active | dismissed | completed

experience_state:
  new | learning | configured | experienced
```

These do not replace the existing renderer contract:

```text
scene + action + status + severity + motion mode
```

Instead, product logic chooses the teaching/recommendation state, and the guide adapter maps the resulting experience onto the visual state machine.

Example:

```text
journey_stage=primary_auth
recommendation=recommended
guidance_mode=suggest
teaching_state=available
experience_state=new

->

auth + point + idle + neutral + full
```

The Rive asset should not contain business rules such as "passkeys are recommended" or "this user needs a backup method".

## Implementation boundary

The first implementation should keep three layers separate.

### Product/policy layer

Determines:

- allowed authentication mechanisms;
- required mechanisms;
- credential state;
- policy satisfaction;
- server-confirmed success/failure; and
- available recovery/resilience actions.

### Guidance/content layer

Determines:

- recommendation category;
- whether a story or suggestion is relevant;
- which approved dialog content to show;
- progression milestone presentation; and
- whether optional guidance is suppressed.

### Renderer layer

Determines:

- pose;
- gaze;
- claw movement;
- transition;
- motion intensity;
- expression; and
- static/reduced/full rendering.

This separation prevents animation or copy code from becoming an authentication-policy engine.

## Accessibility

Teaching must remain usable without animation.

- Crab Dialog content is normal semantic HTML.
- Dialog controls are ordinary keyboard-accessible controls.
- Animation is supplementary and normally `aria-hidden`.
- Progress milestones expose textual status rather than relying on colour or mascot pose.
- Micro-stories can be advanced without waiting for animation.
- `prefers-reduced-motion` changes movement, not information availability.
- Static mode uses the same text and choices.
- Dismiss and skip controls have clear accessible names.

## Privacy

The guide should not need sensitive information to teach authentication concepts.

Do not interpolate into mascot dialog:

- passwords;
- TOTP values;
- backup codes;
- WebAuthn challenges/assertions;
- recovery secrets;
- credential labels unless explicitly approved for normal visible UI;
- raw server error payloads; or
- application secrets.

Using an already-visible application name, domain display name, or reauthentication purpose is acceptable when it comes from the same authoritative UI context and does not introduce a new disclosure.

## Content governance

Security teaching text is product/security content and should be reviewed like security-sensitive UI, not treated as ad-hoc mascot copy.

Changes that alter claims about authentication properties should receive appropriate security review.

The content catalogue should ultimately be centralised rather than copied into route-specific templates.

Translations must preserve technical meaning rather than translating only tone.

## Initial acceptance criteria

Before this journey is considered ready for implementation:

- recommendation categories are available from authoritative product state or an explicit mapping layer;
- passkey, password, hardware-key, TOTP, backup-code, and recovery teaching text has security review;
- optional choices can be skipped without mascot pressure;
- required actions are visually distinguishable from recommendations;
- WebAuthn cancellation does not appear as failure;
- success is never shown before server confirmation;
- guidance decay prevents repeated onboarding during routine login;
- the complete journey works in static/reduced-motion mode;
- no security decision depends on guidance-memory state; and
- at least one full first-run path and one returning-user path are covered by browser integration tests.

## Canonical v1 scenarios to prototype

The first interactive prototype should cover these scenarios before expanding the catalogue:

1. **New user, passkey recommended and available**
   - introduction;
   - mechanism recommendation;
   - passkey micro-story;
   - native WebAuthn setup;
   - confirmed success;
   - optional resilience recommendation;
   - completion.

2. **New user chooses password instead**
   - recommendation shown once;
   - password accepted as `Works OK` when policy permits;
   - no negative reaction;
   - contextual future passkey suggestion permitted.

3. **Returning configured user**
   - no onboarding lesson;
   - primary method guidance only when necessary;
   - normal Protect/Working/Success flow.

4. **WebAuthn cancellation**
   - calm return to choice;
   - no failure celebration or warning escalation;
   - retry and alternatives remain available.

5. **Required policy action**
   - requirement comes from normal product UI;
   - Guardian mode supports the explanation;
   - no skip control when policy does not permit skipping.

## Open questions

- Which exact account/domain policy states should drive `Recommended` versus `Works OK` for every existing authentication mechanism?
- What user-preference mechanism should persist teaching/dismissal state across devices?
- Should administrators be able to suppress all proactive teaching while retaining state animation?
- Which resilience/recovery configurations can safely be recommended without lowering authentication assurance?
- How should guidance behave when passkey support is available on the account but not the current client device/browser?
- Should a user be able to manually switch between compact and teaching-oriented guide modes?
- Which pieces of the journey should appear during administrator-driven credential reset versus self-service setup?
- How should localisation review verify that security claims remain technically accurate?

## Next design step

With recommendation semantics, teaching, progression, and proactivity defined, the next design phase is the **canonical authentication and credential-setup UI**.

The first UI prototype should implement the v1 scenarios in this document across desktop and mobile, with light/dark themes and full/reduced/static mascot modes. The UI should be derived from the real Kubidm login and credential flows rather than from a standalone onboarding application.
