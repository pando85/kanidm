#[cfg(any(target_os = "linux", target_os = "macos"))]
mod u2fhid;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) use u2fhid::get_authenticator_backend;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use u2fhid::Backend;

#[cfg(target_os = "windows")]
mod win10;
#[cfg(target_os = "windows")]
pub(crate) use win10::get_authenticator_backend;
#[cfg(target_os = "windows")]
use win10::Backend;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) fn get_authenticator() -> webauthn_authenticator_rs::WebauthnAuthenticator<Backend> {
    webauthn_authenticator_rs::WebauthnAuthenticator::new(get_authenticator_backend())
}
