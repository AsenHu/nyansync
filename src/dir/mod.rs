use crate::file::FileMeta;
use async_stream::try_stream;
use futures::stream;
use log::warn;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

pub trait Dir {
    async fn list_dir(
        &self,
        path: [u8; 2],
    ) -> io::Result<impl stream::Stream<Item = io::Result<Option<[u8; 20]>>>>;
}

pub struct FsDir {
    cache_path: PathBuf,
    tmp_path: PathBuf,
}

impl FsDir {
    pub fn new(cache: String, tmp: String) -> Arc<Self> {
        Arc::new(Self {
            cache_path: PathBuf::from(cache),
            tmp_path: PathBuf::from(tmp),
        })
    }
}

impl Dir for FsDir {
    /// `FsDir` 的目录实现，用于异步地列出缓存目录中的文件。
    ///
    /// 该函数根据传入的 2 字节路径前缀，定位到具体的二级目录，
    /// 异步遍历目录下的文件，并尝试将文件名解析为 `FileMeta`。
    /// 对于能成功解析的文件，会从中提取哈希值并作为流项（`Stream`）返回；
    /// 对于无法解析的文件，会记录警告日志但不中断整个遍历过程。
    ///
    /// # 参数
    /// * `path` - 目录路径的 2 字节前缀，例如 `[0x1a, 0x2f]` 表示 `1a/2f` 子目录。
    ///
    /// # 返回值
    /// 返回一个异步流（`Stream`），流的每个项是：
    /// - 成功时：`Ok(Some([u8; 20]))`，表示文件哈希；
    /// - 失败时：`Err(io::Error)`，表示 I/O 或异步读取错误；
    /// - 遍历结束后：流自然结束。
    ///
    /// # 注意
    /// - 本函数不会因为单个文件名解析失败而中断整个目录遍历；
    /// - 日志系统需预先初始化以捕获 `warn!` 输出。
    async fn list_dir(
        &self,
        path: [u8; 2],
    ) -> io::Result<impl stream::Stream<Item = io::Result<Option<[u8; 20]>>>> {
        // 构拼接缓存目录路径，例如：cache/1a/2f
        let dir_path = self
            .cache_path
            .join(format!("{:02x}", path[0]))
            .join(format!("{:02x}", path[1]));

        // 异步打开目录
        let mut dir = fs::read_dir(dir_path).await?;

        // 构建一个异步 stream，逐个返回文件哈希

        // 异步读取下一个目录项
        Ok(try_stream! {loop {
            let Some(entry) = dir.next_entry().await? else {
                break;
            };
            let file_name = entry.file_name();

            // 尝试将文件名解析为 FileMeta
            match FileMeta::from_str(&file_name.to_string_lossy()) {
                Ok(file_meta) => {
                    // 成功解析则产出文件哈希
                    yield Some(file_meta.hash());
                }
                // 解析失败仅记录警告，不中断循环
                Err(e) => warn!(
                    "Failed to parse FileMeta from file '{}': {}",
                    file_name.to_string_lossy(),
                    e
                ),
            }
        }})
    }
}

#[cfg(test)]
mod tests;
