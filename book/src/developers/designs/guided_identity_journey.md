# Kubidm Guided Identity Journey

- **Status:** Product experience baseline
- **Date:** 2026-07-26
- **Related architecture:** [Mascot-Guided Product Experience](mascot_guided_product_experience.md)
- **Related visual system:** [Mascot and Motion Design System](mascot_design_system.md)
- **Related UI system:** [Authentication and Credential-Setup UI](authentication_credential_ui.md)
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

## Journey stages

The canonical first-run journey is:

```text
Meet
-> Choose
-> Learn
-> Configure
-> Confirmed Success
-> Resilience
-> Optional Extras / Recovery
-> Complete
```

These are product-experience stages, not route names. A deployment or policy may skip stages that are irrelevant.

### Meet

Goal: orient a new user without creating a mandatory onboarding ceremony.

Example:

> Hi. I'll help you get your identity set up, and I'll explain anything unfamiliar along the way.

The user should be able to proceed immediately.

### Choose

Goal: make the available mechanisms understandable and visibly distinguish the recommended option from alternatives.

Example:

> I'd use a passkey here. It's quick, and it's designed to resist phishing.

The recommendation must correspond to an option the product actually allows.

### Learn

Goal: explain the reason behind an unfamiliar choice in a short micro-story.

Teaching is optional by default and must never delay authentication.

### Configure

Goal: help the user complete the actual mechanism setup or authentication task.

The crab becomes calmer and more protective while the device/browser performs sensitive work.

### Confirmed Success

Goal: acknowledge a server-confirmed result without over-celebrating routine operations.

Example:

> Nice. Your passkey is ready.

### Resilience

Goal: recommend a genuinely useful backup or recovery path when it improves the current configuration.

Example:

> You can sign in now. Want to set up a backup way in?

The user may skip this when policy permits.

### Optional Extras / Recovery

Goal: expose additional capabilities without turning the journey into a checklist of every feature Kubidm supports.

Only relevant, safe options should be recommended.

### Complete

Goal: communicate that the user's current setup has reached the applicable recommended state.

Example:

> You're ready. I'll stay out of the way unless you need me.

Completion is not a claim that the account has achieved a universal maximum-security configuration.

## Crab Dialog

Crab Dialog is an accessible UI primitive, not text embedded inside the animation asset.

### Variants

```text
orient
teach
suggest
celebrate
```

### Orient

Explains the current identity context.

Example:

> You're signing in with Acme to continue to Grafana.

### Teach

Explains a concept or trade-off.

Example:

> A passkey uses cryptographic keys, so there isn't a password for a fake site to trick you into typing.

### Suggest

Recommends a next step.

Example:

> You're already set up to sign in. A backup method could help if your usual device isn't available.

### Celebrate

Acknowledges confirmed progress.

Example:

> All set. You're in.

### Dialog rules

- one primary idea per dialog;
- normally no more than two short sentences;
- recommendations include a reason;
- optional dialogs are dismissible;
- the same optional recommendation is not repeated indefinitely;
- no critical security fact exists only in a Crab Dialog;
- dialog content must be localisable independently of the Rive asset.

## Security micro-stories

Micro-stories are short teaching sequences, normally 2-3 frames and approximately 10-20 seconds if read fully.

They are not mandatory animation sequences and can be rendered statically.

### Passkeys and phishing

Frame 1:

> A password is something you can type into a site, so a convincing fake site can try to ask for it.

Frame 2:

> With a passkey, the private key isn't sent to Kubidm when you sign in.

Frame 3:

> Passkeys are designed to work with the correct site, which makes phishing much harder.

Do not state that every passkey permanently stays on one physical device. Syncable passkeys may be securely synchronised by a platform credential provider.

### Passwords

Goal: explain the trade-off without describing passwords as inherently invalid.

> Passwords are familiar and work in many places. The trade-off is that they're secrets you can type, reuse, forget, or accidentally give to the wrong site.

### Hardware security keys

> A security key proves you have a physical authenticator. It can be a strong option, especially when your organisation requires a dedicated device.

The exact recommendation depends on policy and context.

### TOTP

> An authenticator code adds another proof after your password. It helps protect a stolen password, but a fake site can still try to ask you for the code too.

### Backup codes

> Backup codes are for getting back in when your normal method isn't available. Store them somewhere safe rather than using them for everyday sign-in.

### Recovery

> Recovery is your route back when normal authentication isn't available. It is different from the method you use every day.

## Method-choice baseline

When the product can safely make this recommendation, the baseline normal-login presentation is:

```text
Passkey        Recommended
Password       Works OK
Other methods  contextual
```

This is not a hard-coded global ordering. It is a product-experience baseline that must yield to:

- policy requirements;
- browser capability;
- account state;
- available authenticators;
- reauthentication requirements;
- recovery context; and
- deployment-specific restrictions.

## Choosing a valid alternative

If the user chooses a valid non-recommended method, the crab accepts the choice immediately.

Password example:

> That works. If you want, we can add a passkey later for quicker, phishing-resistant sign-in.

After this acknowledgement:

- do not immediately ask again;
- do not use warning styling;
- do not lower a score;
- do not show disappointment;
- continue the selected authentication path.

## Proactivity rules

The crab is proactive when the guidance can materially help the current decision.

### Proactive by default

- first meaningful encounter;
- first mechanism choice;
- an unfamiliar recommended mechanism;
- newly completed primary setup with a relevant resilience recommendation;
- a materially changed security capability or policy;
- a required action that benefits from orientation.

### Quiet by default

- routine login for an experienced user;
- a recommendation already satisfied;
- a repeatedly dismissed optional suggestion;
- while native WebAuthn/browser UI is active;
- while a serious warning/error needs attention;
- when guidance would delay a common task.

## Guidance decay

The guidance layer tracks a conceptual experience state:

```text
new
learning
configured
experienced
```

### New

- proactive orientation;
- recommendation reason visible;
- teaching offered directly.

### Learning

- contextual suggestions;
- previously completed stories suppressed;
- progress visible during configuration.

### Configured

- recommendations limited to meaningful incomplete resilience/recovery work;
- routine authentication is quiet.

### Experienced

- state feedback and Guardian behaviour only unless context materially changes;
- teaching remains available on demand.

## Guidance memory

Guidance memory is presentation state, not authentication state.

It may eventually track values such as:

```text
completed_story_ids
dismissed_recommendation_ids
last_recommendation_time
experience_state
```

It must not store:

- passwords;
- WebAuthn challenges/assertions;
- TOTP values;
- backup codes;
- recovery secrets; or
- other credential material.

The persistence mechanism remains open. Server-side account preferences, browser-local state, or a combination may be evaluated later.

## Reminder suppression

Optional recommendations need bounded repetition.

Baseline behaviour:

1. show when newly relevant;
2. allow immediate dismissal;
3. one later reminder may be shown if still relevant;
4. repeated dismissal suppresses proactive reminders for a substantial period or until circumstances change;
5. completion suppresses the recommendation permanently while the condition remains satisfied.

The exact timing is an implementation decision and should be testable/configurable rather than embedded into Rive.

## Authentication flow behaviour

### Passkey/WebAuthn starts

Crab:

```text
Guide -> Protect -> Working
```

While native browser/platform UI is active:

- mascot motion becomes quiet;
- no story/dialog competes for focus;
- no overlay covers the native UI;
- pending UI remains understandable without the mascot.

### WebAuthn success

Only after Kubidm confirms success:

```text
Working -> Success
```

### WebAuthn cancellation

Cancellation is a recoverable user choice, not a security failure.

Example:

> No problem. Nothing changed.

Return to the available methods according to current policy.

Repeated cancellation should not produce increasingly dramatic reactions.

### Password authentication

If password is allowed, it remains a normal supported task.

The crab may later suggest a passkey, but it does not interrupt active password entry to campaign for another method.

### TOTP

The crab may teach TOTP during initial setup. Routine code entry becomes quiet once the user is experienced.

### Backup code

The UI should make it clear that backup code use is exceptional/recovery-oriented when that is the intended product semantics.

## Configuration progression

Progress is expressed as useful milestones, not scores.

Conceptual progression:

```text
Access
Primary sign-in
Resilience
Recovery
Ready
```

Example:

```text
[check] You can sign in
[check] Passkey configured
[dot]   Backup method
[dot]   Recovery ready
```

The actual labels depend on account/policy state.

A deployment must not show a milestone that the user cannot achieve or that the product does not support in the current context.

## Resilience recommendations

A backup recommendation is made only when it improves the user's ability to regain access without undermining the intended policy.

The guide must not assume that "more authentication methods" always means "better security".

A weaker fallback can weaken the effective security of a stronger primary method.

Therefore recommendation selection belongs to product/policy logic, not content or animation.

## OAuth context

OAuth2 authentication should explain the relationship among product, tenant, and destination.

Example:

> You're signing in with Acme to continue to Grafana.

The guide does not imply Kubidm operates or endorses the destination application.

Routine OAuth login for experienced users should not replay onboarding education.

## Reauthentication context

Reauthentication is more Guardian-like than ordinary login.

Example:

> Quick check before this security change. Confirm it's you and we'll continue.

The actual purpose comes from the authoritative reauthentication context.

## Recovery context

Recovery is treated as a separate journey with lower playfulness.

The user may already be stressed or locked out, so:

- orientation is concise;
- Guardian personality is stronger;
- celebrations are minimal;
- no game-like progress pressure is used;
- authoritative recovery instructions remain primary.

## Failure and warning behaviour

### Recoverable input error

- normal field error is primary;
- crab may direct attention once;
- no slapstick or exaggerated worry.

### Authentication rejection

- product message remains authoritative;
- mascot response depends on severity;
- the guide does not reveal additional security information beyond what the product intentionally exposes.

### Policy denial

- Guardian/Security mode;
- teaching suppressed unless it helps explain a safe next action;
- no optional framing for a genuinely required condition.

### Account lockout

- Security mode;
- mascot nearly/static;
- normal product UI provides the status and available next steps.

### Server/transport failure

- do not blame the user;
- do not present the failure as an invalid credential unless Kubidm knows that is the cause;
- mascot remains restrained.

## Gamification boundaries

Permitted:

- journey progression;
- milestones;
- micro-stories;
- small character reactions;
- confirmed-success celebrations;
- visual transition from teacher to quiet companion.

Not permitted:

- XP;
- security scores with arbitrary numeric precision;
- competitive ranking;
- streaks;
- artificial scarcity;
- fake urgency;
- punishment for skipping valid optional work;
- animation that encourages users to make security decisions merely to please the mascot.

## Content tone

The crab speaks in short, confident, normal language.

Preferred:

> I'd use a passkey here. It's quick, and it's designed to resist phishing.

Avoid:

> For optimal zero-trust cryptographic posture, enrol a WebAuthn credential.

Preferred:

> That works. We can add a passkey later if you want.

Avoid:

> Password chosen. Your account is less secure.

The technical details remain available through deeper documentation or Learn more content.

## Security content review

Teaching copy is product security content.

Claims about mechanisms must be reviewed for technical accuracy and updated as platform behaviour changes.

In particular:

- do not imply passkeys cannot sync;
- do not imply TOTP is phishing-resistant;
- do not imply all hardware-backed credentials have identical properties;
- do not label every additional fallback as stronger security;
- do not make absolute guarantees such as "cannot be hacked".

## Accessibility

Teaching content must remain available in all motion modes.

Reduced/static mode changes animation, not the explanation or choice set.

Crab Dialog is accessible normal UI content when present.

Micro-stories must support:

- keyboard navigation;
- screen-reader reading order;
- static rendering;
- immediate exit/skip;
- no required timed interaction.

## Product / guidance / renderer separation

Three layers remain separate.

### Product/policy

Determines:

- allowed mechanisms;
- required mechanisms;
- credential state;
- policy satisfaction;
- server-confirmed outcomes;
- safe recovery/resilience capabilities.

### Guidance/content

Determines:

- recommendation category;
- recommendation explanation;
- Crab Dialog content;
- micro-story selection;
- milestone language;
- reminder suppression;
- teaching/experience state.

### Renderer

Determines:

- pose;
- gaze;
- claw gesture;
- expression;
- transition;
- full/reduced/static visual rendering.

The Rive asset never decides product policy or recommendation logic.

## Conceptual semantic fields

The guided journey adds presentation semantics such as:

```text
journey_stage:
  meet | primary_auth | resilience | recovery | complete

recommendation:
  none | optional | works_ok | recommended | required

guidance_mode:
  quiet | orient | teach | suggest | celebrate | guardian

teaching_state:
  none | available | active | completed | dismissed | suppressed

experience_state:
  new | learning | configured | experienced
```

These complement rather than replace the renderer contract:

```text
scene + action + status + severity + motion mode
```

## Canonical prototype scenarios

The first interactive product prototype must cover:

1. new user with passkey recommended and available;
2. new user who chooses password instead;
3. returning configured user;
4. WebAuthn cancellation; and
5. a policy-required security action.

The concrete screen hierarchy for these scenarios is defined in [Authentication and Credential-Setup UI](authentication_credential_ui.md).

## Open questions

- exact mechanism recommendation mapping for all current policies;
- exact resilience/recovery recommendation catalogue;
- guidance-memory persistence;
- reminder timing;
- localisation workflow for Crab Dialog and story content;
- whether administrators may tune guidance intensity independently of mascot visibility;
- whether deployments can substitute organisation-authored teaching copy without changing security meaning;
- how first-run state is determined for existing migrated accounts.

## Next milestone

Build the canonical authentication and credential-setup prototype defined in [Authentication and Credential-Setup UI](authentication_credential_ui.md), starting with the new-user/passkey-recommended scenario and validating the same structure against password choice, returning-user login, WebAuthn cancellation, and required policy action.
