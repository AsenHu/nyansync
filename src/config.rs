use crate::{receiver, sender};
use clap::{Parser, Subcommand};
use std::error::Error;

#[derive(Parser)]
pub struct Config {
    /// Cache 目录路径
    #[arg(short, long, default_value = "./cache", value_name = "PATH")]
    cache: String,

    /// 子命令
    #[command(subcommand)]
    command: Command,
}

pub enum AppConfig {
    Receiver(receiver::Config),
    Sender(sender::Config),
}

#[derive(Subcommand)]
enum Command {
    /// 以接收者身份运行
    Recv {
        /// 监听的地址和端口
        #[arg(short, long, default_value = "[::]:37150", value_name = "LISTEN")]
        listen: String,

        /// 临时目录路径
        /// 接收到的文件会存储在此目录下，写入完成后会移动到 cache 目录
        /// 为了确保移动操作的原子性，该目录与 cache 目录必须在同一文件系统上
        /// 注意：如果指定的目录已存在，则会清空该目录
        #[arg(short, long, default_value = "./tmp", value_name = "PATH")]
        tmp: String,
    },
    /// 以发送者身份运行
    Send {
        /// 连接到的地址和端口
        #[arg(short, long, value_name = "DIAL")]
        dial: String,
    },
}

impl Config {
    /// 解析命令行参数
    pub fn parse() -> Self {
        Parser::parse()
    }

    /// 获取接受/发送者格式的配置
    pub fn get_config(self) -> Result<AppConfig, Box<dyn Error>> {
        match self.command {
            Command::Recv { listen, tmp } => {
                let config = receiver::Config {
                    cache: self.cache,
                    listen,
                    tmp,
                };
                Ok(AppConfig::Receiver(config))
            }
            Command::Send { dial } => {
                let config = sender::Config {
                    cache: self.cache,
                    dial,
                };
                Ok(AppConfig::Sender(config))
            }
        }
    }
}
