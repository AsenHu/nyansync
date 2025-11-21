use nyansync::config;
use nyansync::{receiver, sender};

fn main() {
    // 日志
    env_logger::init();

    // 解析命令行参数
    let config = config::Config::parse();

    match config.get_config() {
        Ok(config::AppConfig::Receiver(config)) => {
            // 以接收者身份运行
            if let Err(e) = receiver::run(config) {
                log::error!("Error running receiver: {}", e);
            }
        }
        Ok(config::AppConfig::Sender(config)) => {
            // 以发送者身份运行
            if let Err(e) = sender::run(config) {
                log::error!("Error running sender: {}", e);
            }
        }
        Err(e) => {
            log::error!("Error parsing configuration: {}", e);
        }
    }
}
