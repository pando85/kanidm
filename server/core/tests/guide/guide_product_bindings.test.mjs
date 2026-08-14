import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

async function source(relativePath) {
    return readFile(new URL(`../../${relativePath}`, import.meta.url), "utf8");
}

function includesAll(text, path, expected) {
    for (const fragment of expected) {
        assert.ok(text.includes(fragment), `${path} must contain ${JSON.stringify(fragment)}`);
    }
}

test("guided authentication product bindings remain present", async () => {
    const files = {
        "templates/base.html": await source("templates/base.html"),
        "templates/login_base.html": await source("templates/login_base.html"),
        "templates/login.html": await source("templates/login.html"),
        "templates/login_mech_choose.html": await source("templates/login_mech_choose.html"),
        "templates/login_password.html": await source("templates/login_password.html"),
        "templates/login_totp.html": await source("templates/login_totp.html"),
        "templates/login_backupcode.html": await source("templates/login_backupcode.html"),
        "templates/login_webauthn.html": await source("templates/login_webauthn.html"),
        "templates/login_denied.html": await source("templates/login_denied.html"),
        "static/pkhtml.js": await source("static/pkhtml.js"),
    };

    includesAll(files["templates/base.html"], "templates/base.html", ["data-guide-motion-config"]);
    includesAll(files["templates/login_base.html"], "templates/login_base.html", [
        'data-guide-scene="auth"',
        "/pkg/modules/guide_controller.mjs",
        "data-guide-auth-arrival",
        "data-guide-slot",
    ]);
    includesAll(files["templates/login.html"], "templates/login.html", [
        "identify_error",
        "welcome-orientation",
        "data-guide-new-only",
    ]);
    includesAll(files["templates/login_mech_choose.html"], "templates/login_mech_choose.html", [
        'data-guide-action="choose_method"',
        "data-guide-recommendation",
        'data-kind="(% if mech.autofocus %)recommended(% else %)works_ok(% endif %)"',
    ]);
    includesAll(files["templates/login_password.html"], "templates/login_password.html", [
        'data-guide-state="guide"',
        'data-guide-action="password"',
    ]);
    includesAll(files["templates/login_totp.html"], "templates/login_totp.html", [
        'data-guide-state="protect"',
        'data-guide-action="totp"',
        'data-guide-severity="caution"',
    ]);
    includesAll(files["templates/login_backupcode.html"], "templates/login_backupcode.html", [
        'data-guide-state="protect"',
        'data-guide-action="backup_code"',
    ]);
    includesAll(files["templates/login_webauthn.html"], "templates/login_webauthn.html", [
        'data-guide-action="webauthn_ready"',
        'data-guide-target="primary-action"',
        "data-guide-status",
    ]);
    assert.match(
        files["templates/login_webauthn.html"],
        /action="\/ui\/login\/seckey"[\s\S]*?data-challenge="\(\( challenge \)\)"/,
        "security-key WebAuthn form must carry the server challenge",
    );
    includesAll(files["templates/login_denied.html"], "templates/login_denied.html", [
        'data-guide-action="authentication_denied"',
        'data-guide-severity="critical"',
    ]);

    const webauthn = files["static/pkhtml.js"];
    includesAll(webauthn, "static/pkhtml.js", [
        "window.dispatchEvent(new CustomEvent(`kubidm:webauthn-${name}`",
        'if (!document.querySelector("[data-guide-scene]"))',
        'dispatchWebauthnEvent("submit")',
        'dispatchWebauthnEvent("cancelled"',
    ]);

    assert.doesNotMatch(
        webauthn,
        /addEventListener\("load",\s*\(\) => \{\s*passkey_login\(\);\s*\}\)/s,
        "guided WebAuthn must not be triggered unconditionally on page load",
    );
});

test("guided Applications, settings and logout bindings remain present", async () => {
    const files = {
        "templates/apps.html": await source("templates/apps.html"),
        "templates/apps_partial.html": await source("templates/apps_partial.html"),
        "templates/base_htmx_with_nav.html": await source("templates/base_htmx_with_nav.html"),
        "templates/signout_modal.html": await source("templates/signout_modal.html"),
        "templates/user_settings.html": await source("templates/user_settings.html"),
        "templates/user_settings_partial_base.html": await source("templates/user_settings_partial_base.html"),
        "templates/user_settings_profile_partial.html": await source("templates/user_settings_profile_partial.html"),
        "templates/user_settings/profile_changes_partial.html": await source(
            "templates/user_settings/profile_changes_partial.html",
        ),
        "templates/credentials_status.html": await source("templates/credentials_status.html"),
        "templates/credentials_reset.html": await source("templates/credentials_reset.html"),
    };

    includesAll(files["templates/apps.html"], "templates/apps.html", [
        "/pkg/modules/guide_controller.mjs",
        "/pkg/modules/guide_apps.mjs",
    ]);
    includesAll(files["templates/apps_partial.html"], "templates/apps_partial.html", [
        'data-guide-scene="applications"',
        "applications_empty",
        "data-guide-slot",
    ]);
    includesAll(files["templates/base_htmx_with_nav.html"], "templates/base_htmx_with_nav.html", [
        "/pkg/modules/guide_logout.mjs",
    ]);
    includesAll(files["templates/signout_modal.html"], "templates/signout_modal.html", [
        "/pkg/img/guide/crab-goodbye.webp",
    ]);
    includesAll(files["templates/user_settings.html"], "templates/user_settings.html", [
        "/pkg/modules/guide_controller.mjs",
        "/pkg/modules/credential_guide.mjs",
    ]);
    includesAll(files["templates/user_settings_partial_base.html"], "templates/user_settings_partial_base.html", [
        'data-guide-scene="settings"',
        "data-guide-slot",
    ]);
    includesAll(files["templates/user_settings_profile_partial.html"], "templates/user_settings_profile_partial.html", [
        'data-guide-scene="profile"',
        "profile_edit",
        "profile_readonly",
        'data-guide-target="primary-action"',
    ]);
    includesAll(
        files["templates/user_settings/profile_changes_partial.html"],
        "templates/user_settings/profile_changes_partial.html",
        [
            'data-guide-scene="profile"',
            'data-guide-action="profile_review"',
            'data-guide-story-id="profile-review"',
            'data-guide-target="primary-action"',
        ],
    );
    includesAll(files["templates/credentials_status.html"], "templates/credentials_status.html", [
        'data-guide-scene="credentials"',
        "data-guide-credential-state",
        'data-guide-action="credential_setup"',
    ]);
    includesAll(files["templates/credentials_reset.html"], "templates/credentials_reset.html", [
        'data-guide-scene="credentials"',
        "/pkg/modules/guide_controller.mjs",
        "/pkg/modules/credential_guide.mjs",
        "data-guide-slot",
    ]);
});

test("guided credential-enrolment subflows keep semantic steps and primary actions", async () => {
    const passkey = await source("templates/credential_update_add_passkey_partial.html");
    const password = await source("templates/credential_update_add_password_partial.html");
    const totp = await source("templates/credential_update_add_totp_partial.html");

    includesAll(passkey, "templates/credential_update_add_passkey_partial.html", [
        'data-guide-credential-step="passkey_enrolment"',
        'data-guide-story-id="passkey-enrolment-name"',
        'data-guide-target="primary-action"',
    ]);
    includesAll(password, "templates/credential_update_add_password_partial.html", [
        'data-guide-credential-step="password_setup"',
        'data-guide-story-id="password-alternative"',
        'data-guide-target="primary-action"',
        'hx-post="/ui/api/check_password_strength"',
    ]);
    includesAll(totp, "templates/credential_update_add_totp_partial.html", [
        'data-guide-credential-step="totp_enrolment"',
        'data-guide-story-id="totp-basics"',
        'data-guide-target="primary-action"',
    ]);
});
