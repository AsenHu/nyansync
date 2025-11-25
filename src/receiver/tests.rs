use super::*;
use crate::dir::Dir;
use crate::file;
use crate::protocol::{Protocol, types};
use futures::stream;
use std::io;
use std::{collections::VecDeque, sync::Mutex};

#[derive(Debug, PartialEq)]
struct MockProtocolState {
    commands_to_return: VecDeque<types::Command>,
    calls_received: Vec<Call>,
}

struct MockProtocol {
    state: Arc<Mutex<MockProtocolState>>,
}

#[derive(Debug, PartialEq)]
enum Call {
    NextCommand,
    RespondCheckVersion,
}

impl MockProtocol {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProtocolState {
                commands_to_return: VecDeque::new(),
                calls_received: Vec::new(),
            })),
        }
    }
}

impl Protocol for MockProtocol {
    async fn next_command(&mut self) -> Result<Command, Error> {
        self.state
            .lock()
            .unwrap()
            .calls_received
            .push(Call::NextCommand);
        match self.state.lock().unwrap().commands_to_return.pop_front() {
            Some(command) => Ok(command),
            None => Err(Error::msg("No command to return")),
        }
    }

    async fn respond_check_version(&mut self) -> Result<(), Error> {
        self.state
            .lock()
            .unwrap()
            .calls_received
            .push(Call::RespondCheckVersion);
        Ok(())
    }

    async fn respond_list_dir(
        &mut self,
        files: impl stream::Stream<Item = [u8; 20]> + Unpin,
    ) -> Result<(), Error> {
        todo!();
    }
}

struct MockDir;

impl MockDir {
    fn new() -> Self {
        Self {}
    }
}

impl Dir for MockDir {
    async fn list_dir(
        &self,
        path: [u8; 2],
    ) -> io::Result<impl stream::Stream<Item = io::Result<Option<[u8; 20]>>>> {
        let files = vec![Ok(Some([0u8; 20])), Ok(Some([1u8; 20])), Ok(None)];
        Ok(stream::iter(files))
    }
    async fn save_file(&self, file: file::File) -> io::Result<()> {
        todo!();
    }
}

#[tokio::test]
async fn test_check_version() {
    // 准备模拟对象
    let protocol = MockProtocol::new();
    let dir = Arc::new(MockDir::new());

    // 复制模拟对象状态指针
    let state = Arc::clone(&protocol.state);

    // 模拟返回 CheckVersion 命令
    state
        .lock()
        .unwrap()
        .commands_to_return
        .push_back(Command::CheckVersion {
            version: protocol::PROTOCOL_VERSION,
        });

    // 运行 handler
    let _ = tokio::spawn(async move {
        let _ = handler(protocol, dir).await;
    })
    .await;

    // 构建理论的 State
    let exp_state = MockProtocolState {
        commands_to_return: VecDeque::new(),
        calls_received: vec![
            Call::NextCommand,
            Call::RespondCheckVersion,
            Call::NextCommand,
        ],
    };

    // 取出 State 并进行断言
    let state = state.lock().unwrap();
    assert_eq!(*state, exp_state);
}
