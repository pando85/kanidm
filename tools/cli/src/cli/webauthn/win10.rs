use webauthn_authenticator_rs::win10::Win10;

pub type Backend = Win10;

pub fn get_authenticator_backend() -> Backend {
    Default::default()
}
