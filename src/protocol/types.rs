use crate::file;

#[derive(Debug, PartialEq)]
pub enum Command {
    CheckVersion { version: u8 },
    ListDir { path: [u8; 2] },
    SaveFile { file: file::File },
}
