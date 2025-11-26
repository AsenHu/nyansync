use crate::dir::{Dir, FsDir};
use crate::protocol::{self, Protocol, types::Command};
use anyhow::{Error, Result};
use errors::Errors;
use futures::{StreamExt, stream};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub struct Config {
    pub cache: String,
    pub tmp: String,
    pub listen: String,
}

#[tokio::main]
pub async fn run(config: Config) -> Result<(), Error> {
    // 打开缓存目录
    let dir = Arc::new(FsDir::new(config.cache, config.tmp));

    // 准备 TCP 监听器
    let listener = TcpListener::bind(config.listen).await?;

    // 在新协程中处理连接
    tokio::spawn(acceptor(listener, dir)).await?;

    Ok(())
}

pub async fn acceptor(listener: TcpListener, dir: Arc<FsDir>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                // 克隆目录引用
                let dir = Arc::clone(&dir);

                // 将 Tokio 的 TcpStream 转换为 Futures 的异步读写流
                let (reader, writer) = tokio::io::split(stream);
                let protocol =
                    protocol::StreamProtocol::new(reader.compat(), writer.compat_write());

                tokio::spawn(async move {
                    if let Err(e) = handler(protocol, dir).await {
                        log::error!("Error handling connection: {}", e);
                    }
                });
            }
            Err(e) => {
                log::error!("Failed to accept connection: {}", e);
            }
        }
    }
}

pub async fn handler<P, D>(mut protocol: P, dir: Arc<D>) -> Result<(), Error>
where
    P: Protocol,
    D: Dir,
{
    loop {
        match protocol.next_command().await? {
            Command::CheckVersion { version } => {
                // 响应
                protocol.respond_check_version().await?;
                // 检查版本是否一致
                if version != protocol::PROTOCOL_VERSION {
                    return Err(Errors::VersionMismatch(version, protocol::PROTOCOL_VERSION).into());
                }
            }
            Command::ListDir { path } => {
                // protocol 的接口需要重新定义，将发送数据帧的接口暴露出来，由调用方负责分片逻辑
                // 这样当 list_dir 取出数据出错时，这里可以直接处理，而不是在 protocol 内部处理
                // 因此 protocol 需要重构，这里暂时先按原有逻辑实现
                // 获取目录列表
                let list = dir.list_dir(path).await?;
                // 全部收集到 Vec 中，有错误直接返回
                let mut files = Vec::new();
                // Tokio 的 Stream 需要 Pin 才能使用
                futures::pin_mut!(list);
                // 遍历流
                while let Some(entry) = list.next().await {
                    match entry {
                        Ok(Some(file)) => files.push(file),
                        Ok(None) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
                // 包装成流并响应
                let files = stream::iter(files.into_iter());
                protocol.respond_list_dir(files).await?;
            }
            Command::SaveFile { file } => {
                // file 里包含元数据和内容
                // 每个 file 在极端情况下也就 50 MiB，所以这里直接全部读入内存
                dir.save_file(file).await?;
                // 没想到这个命令居然能这么简单就完成
            }
        }
    }
}

mod errors;

#[cfg(test)]
mod tests;
