use error::Error;
use hex;
use sha1::{Digest, Sha1};
use std::fmt;

pub mod error;

#[derive(Debug, PartialEq)]
pub struct File {
    meta: FileMeta,
    content: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct FileMeta {
    size: u32,
    hash: [u8; 20],
    width: u16,
    height: u16,
    extension: FileExt,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileExt {
    Gif = 0,
    Jpg = 1,
    Webp = 2,
    Png = 3,
}

impl FileMeta {
    pub fn build(
        size: u32,
        hash: [u8; 20],
        width: u16,
        height: u16,
        extension: FileExt,
    ) -> Result<Self, Error> {
        let file = Self {
            size,
            hash,
            width,
            height,
            extension,
        };
        file.check()?;
        Ok(file)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(29);
        bytes.extend(&self.size.to_be_bytes());
        bytes.extend(&self.hash);
        bytes.extend(&self.width.to_be_bytes());
        bytes.extend(&self.height.to_be_bytes());
        bytes.push(self.extension as u8);
        bytes
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn hash(&self) -> [u8; 20] {
        self.hash
    }

    fn check(&self) -> Result<(), Error> {
        // 宽度和高度应该介于 100 - 20000（包含）
        // 检查宽度
        if self.width < 100 || self.width > 20000 {
            return Err(Error::InvalidWidth(self.width));
        };
        // 检查高度
        if self.height < 100 || self.height > 20000 {
            return Err(Error::InvalidHeight(self.height));
        };
        // 宽度高度之和应该大于 250（不包含）
        let sum = self.width + self.height;
        if sum <= 250 {
            return Err(Error::InsufficientResolution(sum));
        };

        // 检查文件大小
        if self.size == 0 {
            return Err(Error::InvalidFileSizeZero);
        };
        // 检查大小是否符合扩展名
        // GIF 不超过 10 MiB
        // JPG 不超过 20 MiB
        // WEBP 不超过 20 MiB
        // PNG 不超过 50 MiB
        let max_size = match self.extension {
            FileExt::Gif => 10 * 1024 * 1024,
            FileExt::Jpg => 20 * 1024 * 1024,
            FileExt::Webp => 20 * 1024 * 1024,
            FileExt::Png => 50 * 1024 * 1024,
        };
        if self.size > max_size {
            return Err(Error::SizeExceedsLimitForExtension(self.size, max_size));
        }

        Ok(())
    }
}

impl TryFrom<&str> for FileMeta {
    type Error = Error;

    // Hash-体积-宽度-高度-扩展名
    // 示例
    // 0020241169685837135e56e30ea29293951ad977-3112798-432-240-gif
    // 0020002eed10ac9e0416070c4ce0cfebb192a6b5-60129-800-700-jpg
    // 0020000a1c10db9f5321d41d9632dea899d09325-59968-1280-720-wbp
    // 00201f9208c45eefbdea7db4da2a3ba9ee37107d-73033-1130-1600-png
    fn try_from(s: &str) -> Result<Self, Error> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return Err(Error::InvalidStringFormat);
        }
        let mut hash = [0u8; 20];
        hex::decode_to_slice(parts[0], &mut hash)?;
        let size: u32 = parts[1].parse()?;
        let width: u16 = parts[2].parse()?;
        let height: u16 = parts[3].parse()?;
        let extension = match parts[4].to_lowercase().as_str() {
            "gif" => FileExt::Gif,
            "jpg" => FileExt::Jpg,
            "wbp" => FileExt::Webp,
            "png" => FileExt::Png,
            s => return Err(Error::InvalidFileExtName(s.to_string())),
        };
        let file = Self {
            size,
            hash,
            width,
            height,
            extension,
        };
        file.check()?;
        Ok(file)
    }
}

impl TryFrom<&[u8; 29]> for FileMeta {
    type Error = Error;

    fn try_from(bytes: &[u8; 29]) -> Result<Self, Error> {
        if bytes.len() != 29 {
            return Err(Error::InvalidByteArrayLength(bytes.len()));
        }
        let size = u32::from_be_bytes(bytes[0..4].try_into()?);
        let hash = bytes[4..24].try_into()?;
        let width = u16::from_be_bytes(bytes[24..26].try_into()?);
        let height = u16::from_be_bytes(bytes[26..28].try_into()?);
        let extension = match bytes[28] {
            0 => FileExt::Gif,
            1 => FileExt::Jpg,
            2 => FileExt::Webp,
            3 => FileExt::Png,
            ext => return Err(Error::InvalidFileExtNum(ext)),
        };
        let file = Self {
            size,
            hash,
            width,
            height,
            extension,
        };
        file.check()?;
        Ok(file)
    }
}

impl fmt::Display for FileMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}-{}-{}",
            hex::encode(&self.hash),
            self.size,
            self.width,
            self.height,
            match self.extension {
                FileExt::Gif => "gif",
                FileExt::Jpg => "jpg",
                FileExt::Webp => "wbp",
                FileExt::Png => "png",
            }
        )
    }
}

impl File {
    pub fn build(meta: FileMeta, content: Vec<u8>) -> Result<Self, Error> {
        if meta.size as usize != content.len() {
            return Err(Error::FileSizeMismatch);
        }
        #[cfg(feature = "checksum")]
        {
            let computed_hash = Sha1::digest(&content);
            if computed_hash[..] != meta.hash {
                return Err(Error::FileHashMismatch);
            }
        }
        let file = Self { meta, content };
        Ok(file)
    }

    pub fn into_parts(self) -> (FileMeta, Vec<u8>) {
        (self.meta, self.content)
    }
}
