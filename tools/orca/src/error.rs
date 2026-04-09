pub enum Error {
    Io,
    SerdeToml,
    SerdeJson,
    KubidmClient,
    ProfileBuilder,
    Tokio,
    Interrupt,
    Crossbeam,
    InvalidState,
    #[allow(dead_code)]
    RandomNumber(String),
}
