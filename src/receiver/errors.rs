use thiserror::Error;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("Sender protocol version is {0}, expected {1}")]
    VersionMismatch(u8, u8),
}
