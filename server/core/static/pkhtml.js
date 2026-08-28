/**
 * Initiates the passkey login process by requesting credentials from the user.
 *
 * This function retrieves the credential request options from the DOM, converts
 * necessary fields from Base64 to Uint8Array, and then uses the Web Authentication API
 * to get the user's credentials. Upon successful retrieval, it encodes the assertion
 * response back to Base64 and submits the form with the credential data.
 *
 * Guide lifecycle events deliberately contain no credential material. An assertion
 * being obtained is not authentication success: only the server may determine that.
 *
 * @function passkey_login
 * @throws {Error} If the passkey authentication process fails.
 */

function dispatchWebauthnEvent(name, detail = {}) {
    window.dispatchEvent(new CustomEvent(`kubidm:webauthn-${name}`, { detail }));
}

function passkey_login() {
    let form = document.getElementById("cred-form");
    let credentialRequestOptions = JSON.parse(atob(form.dataset.challenge));
    credentialRequestOptions.publicKey.challenge = Base64.toUint8Array(credentialRequestOptions.publicKey.challenge);
    credentialRequestOptions.publicKey.allowCredentials?.forEach(function (listItem) {
        listItem.id = Base64.toUint8Array(listItem.id);
    });

    dispatchWebauthnEvent("start");

    navigator.credentials
        .get({ publicKey: credentialRequestOptions.publicKey })
        .then((assertion) => {
            document.getElementById("cred").value = JSON.stringify({
                id: assertion.id,
                rawId: Base64.fromUint8Array(new Uint8Array(assertion.rawId), true),
                type: assertion.type,
                response: {
                    authenticatorData: Base64.fromUint8Array(
                        new Uint8Array(assertion.response.authenticatorData),
                        true,
                    ),
                    clientDataJSON: Base64.fromUint8Array(new Uint8Array(assertion.response.clientDataJSON), true),
                    signature: Base64.fromUint8Array(new Uint8Array(assertion.response.signature), true),
                    userHandle: Base64.fromUint8Array(new Uint8Array(assertion.response.userHandle), true),
                },
            });

            // This means the browser produced an assertion and Kubidm is about to
            // submit it. It must never be interpreted as authentication success.
            dispatchWebauthnEvent("submit");
            document.getElementById("cred-form").submit();
        })
        .catch((error) => {
            const name = error?.name || "Error";
            console.error(`Failed to complete passkey authentication: ${error}`);

            if (name === "NotAllowedError" || name === "AbortError") {
                // WebAuthn intentionally groups user cancellation, timeout, and some
                // unavailable-credential cases under NotAllowedError. Keep the guide
                // reaction neutral and do not claim to know which one occurred.
                dispatchWebauthnEvent("cancelled", { reason: name });
            } else {
                dispatchWebauthnEvent("error", { reason: name });
            }
        });
}

const passkeyButton = document.getElementById("start-passkey-button");
if (passkeyButton) {
    passkeyButton.addEventListener("click", () => {
        passkey_login();
    });
}

const seckeyButton = document.getElementById("start-seckey-button");
if (seckeyButton) {
    seckeyButton.addEventListener("click", () => {
        passkey_login();
    });
}

try {
    addEventListener("load", () => {
        // Legacy authentication keeps its direct native prompt. Guided mode first
        // explains the method and lets the user start WebAuthn from the visible CTA.
        if (!document.querySelector("[data-guide-scene]")) {
            passkey_login();
        }
    });
} catch (error) {
    console.error(`Failed to add load-time event listener for passkey authentication: ${error}`);
}
