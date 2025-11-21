use thiserror::Error;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("The file extension {0} is invalid")]
    InvalidFileExt(u8),

    #[error("The command {0} is invalid")]
    InvalidCommand(u8),
}
