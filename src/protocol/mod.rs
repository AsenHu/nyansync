use crate::file::{File, FileMeta};
use anyhow::{Error, Result};
use errors::Errors;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use futures::{StreamExt, stream};
use std::pin::Pin;
use types::Command;

pub mod errors;
pub mod types;

pub const PROTOCOL_VERSION: u8 = 0;

const REQUEST_CHECK_VERSION: u8 = 0;
const RESPONSE_CHECK_VERSION: u8 = 128;
const REQUEST_LIST_DIR: u8 = 1;
const RESPONSE_LIST_DIR: u8 = 129;
const REQUEST_SAVE_FILE: u8 = 2;

pub trait Protocol {
    fn next_command(&mut self) -> impl Future<Output = Result<Command, Error>> + Send;
    fn respond_check_version(&mut self) -> impl Future<Output = Result<(), Error>> + Send;
    fn respond_list_dir(
        &mut self,
        files: impl stream::Stream<Item = [u8; 20]> + Unpin + Send,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

pub struct StreamProtocol<R, W>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    reader: BufReader<R>,
    writer: BufWriter<W>,
}

impl<R, W> StreamProtocol<R, W>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }
}

impl<R, W> Protocol for StreamProtocol<R, W>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    async fn next_command(&mut self) -> Result<Command, Error> {
        let mut cmd = [0u8; 1];
        self.reader.read_exact(&mut cmd).await?;
        match cmd[0] {
            REQUEST_CHECK_VERSION => {
                let mut version = [0u8; 1];
                self.reader.read_exact(&mut version).await?;
                Ok(Command::CheckVersion {
                    version: version[0],
                })
            }
            REQUEST_LIST_DIR => {
                let mut path = [0; 2];
                self.reader.read_exact(&mut path).await?;
                Ok(Command::ListDir { path })
            }
            REQUEST_SAVE_FILE => {
                let mut buf = [0u8; 29];
                self.reader.read_exact(&mut buf).await?;
                let info = FileMeta::try_from(&buf)?;
                let mut content = vec![0; info.size() as usize];
                self.reader.read_exact(&mut content).await?;
                Ok(Command::SaveFile {
                    file: File::build(info, content)?,
                })
            }
            _ => Err(Errors::InvalidCommand(cmd[0]).into()),
        }
    }

    async fn respond_check_version(&mut self) -> Result<(), Error> {
        let bytes = vec![RESPONSE_CHECK_VERSION, PROTOCOL_VERSION];
        self.writer.write_all(&bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// 响应客户端的目录列表请求。
    ///
    /// 该函数不应该做数据分片的逻辑，它在之后将被重新设计
    ///
    /// 协议规定：
    /// - 先写入一个单字节的响应类型标识（RESPONSE_LIST_DIR）
    /// - 随后发送若干个“负载帧”(frame)，直到文件列表耗尽。
    ///
    /// 每个负载帧的格式为：
    /// - 前 2 字节：u16，大端序，表示当前帧中包含的 hash 数量
    /// - 后续数据：若干个 20 字节的 hash，连续排列
    ///
    /// 约定：
    /// - 每帧最多包含 `u16::MAX` (65535) 个 hash
    /// - 如果最后一帧数量小于 `u16::MAX`，则表示列表结束
    ///
    /// 参数：
    /// - `files`: 一个异步 Stream，按顺序提供 `[u8; 20]` 的 hash 值
    ///
    /// 返回：
    /// - 如果写入过程中发生 IO 错误，返回 `Err(Error)`
    /// - 否则在写入完成并 flush 后返回 `Ok(())`
    async fn respond_list_dir(
        &mut self,
        mut files: impl stream::Stream<Item = [u8; 20]> + Unpin + Send,
    ) -> Result<(), Error> {
        // 发送响应头
        self.writer.write_all(&[RESPONSE_LIST_DIR]).await?;

        // 构建负载帧
        // 每个负载帧包含 u16::max 个 Hash
        // 因此，先读取 u16::max 个内容，构成一个负载帧后一次写入
        // 当剩余内容不足 u16::max 时，退出循环，写入剩余内容
        // 负载帧的开头包含一个 u16 的长度字段
        // 把单个 hash 按 65535 个为一组打包，然后检查长度，写入长度后就写入 hash 列表，然后开始下一个帧，直到小于 65535 为止

        // 持续从 stream 中取出数据并打包成帧
        loop {
            // 从 stream 读取最多 65535 个 hash
            let (n, frame) = to_frame(Pin::new(&mut files)).await;

            // // 写入帧头 (数量) 和帧内容
            self.writer.write_all(&n.to_be_bytes()).await?;
            self.writer.write_all(&frame).await?;

            // 如果当前帧不足 65535 项，说明已经是最后一帧
            if n != u16::MAX {
                break;
            }
        }

        self.writer.flush().await?;
        Ok(())
    }
}

/// 从一个 `Stream<Item = [u8; 20]>` 中提取最多 `u16::MAX` 个元素，
/// 打包成一个负载帧（frame）并返回。
///
/// 帧的内容格式：
/// - 仅包含 hash 数据，不带长度字段（长度由调用方写入）
/// - 每个 hash 固定 20 字节，按顺序拼接到 Vec 中
///
/// 参数：
/// - `s`: 一个 pinned 的 Stream 引用
///
/// 返回：
/// - `(n, frame)`
///   - `n`: 实际提取的元素数量，范围 0..=65535
///   - `frame`: Vec<u8>，长度为 `n * 20`，顺序存放这些 hash
///
/// 说明：
/// - 如果 stream 中的数据不足 65535 个，则会提前停止
/// - 如果 stream 已经结束，则返回 `(0, vec![])`
async fn to_frame<T>(mut s: Pin<&mut T>) -> (u16, Vec<u8>)
where
    T: stream::Stream<Item = [u8; 20]>,
{
    // 预分配一个合理大小，减少扩容次数
    let mut frame = Vec::with_capacity(2048 * 20);
    let mut n = 0;

    for _ in 0..u16::MAX {
        match s.next().await {
            Some(hash) => {
                n += 1;
                frame.extend_from_slice(&hash);
            }
            None => break,
        }
    }

    (n, frame)
}

#[cfg(test)]
mod tests;
