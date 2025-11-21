use hex;
use std::{array, num};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The file extension {0} is invalid")]
    InvalidFileExtNum(u8),

    #[error("The file extension {0} is invalid")]
    InvalidFileExtName(String),

    #[error("The byte array length {0} is invalid, expected 27")]
    InvalidByteArrayLength(usize),

    #[error("The width {0} is invalid, expected to be between 100 - 20000")]
    InvalidWidth(u16),

    #[error("The height {0} is invalid, expected to be between 100 - 20000")]
    InvalidHeight(u16),

    #[error("The sum of width and height is {0}, which is expected to be greater than 250")]
    InsufficientResolution(u16),

    #[error(
        "The file size {0} bytes is invalid; it exceeds the maximum size for this extension ({1} Bytes)."
    )]
    SizeExceedsLimitForExtension(u32, u32),

    #[error("The file size is 0")]
    InvalidFileSizeZero,

    #[error("The string is not in a correct format")]
    InvalidStringFormat,

    #[error("The file size does not match the expected size")]
    FileSizeMismatch,

    #[error("The file hash does not match the expected hash")]
    FileHashMismatch,

    #[error(transparent)]
    TryFromSliceError(#[from] array::TryFromSliceError),

    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),

    #[error(transparent)]
    ParseIntError(#[from] num::ParseIntError),
}
