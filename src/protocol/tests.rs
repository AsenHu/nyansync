use super::*;
use crate::file::FileExt;
use futures::io::Cursor;
use sha1::{Digest, Sha1};

#[tokio::test]
async fn read_check_version() {
    // 模拟一个网络字节流
    let reader = Cursor::new(vec![REQUEST_CHECK_VERSION, PROTOCOL_VERSION]); // 命令 0，版本 0
    let mut writer = Cursor::new(Vec::new());
    // 用模拟数据初始化 Protocol
    let mut protocol = StreamProtocol::new(reader, &mut writer);
    let command = protocol.next_command().await.unwrap();
    // 断言解析结果是否符合预期
    let writer = writer.into_inner();
    assert_eq!(writer, Vec::new());
    assert_eq!(
        command,
        Command::CheckVersion {
            version: PROTOCOL_VERSION
        }
    );
}

#[tokio::test]
async fn write_check_version() {
    // 准备模拟字节流
    let reader = Cursor::new(vec![]);
    let mut writer = Cursor::new(Vec::new());
    // 运行被测试函数
    let mut protocol = StreamProtocol::new(reader, &mut writer);
    protocol.respond_check_version().await.unwrap();
    // 整理数据并断言
    let writer = writer.into_inner();
    assert_eq!(writer, vec![RESPONSE_CHECK_VERSION, PROTOCOL_VERSION]);
}

#[tokio::test]
async fn read_list_dir() {
    // 模拟字节流
    let reader = Cursor::new(vec![REQUEST_LIST_DIR, 38, 104]);
    let mut writer = Cursor::new(Vec::new());
    // 运行函数
    let mut protocol = StreamProtocol::new(reader, &mut writer);
    let command = protocol.next_command().await.unwrap();
    // 整理断言
    let writer = writer.into_inner();
    assert_eq!(writer, Vec::new());
    assert_eq!(command, Command::ListDir { path: [38, 104] });
}

#[tokio::test]
async fn read_save_file() {
    // 原始文件数据
    let content: Vec<u8> = b"Hello, World!".to_vec();
    let size: u32 = content.len() as u32;
    let hash: [u8; 20] = Sha1::digest(&content).into();
    let width: u16 = 12300;
    let height: u16 = 15200;
    let extension = FileExt::Png;

    // 构造数据流
    let mut data = Vec::new();
    data.push(REQUEST_SAVE_FILE);
    data.extend(size.to_be_bytes());
    data.extend(hash);
    data.extend(width.to_be_bytes());
    data.extend(height.to_be_bytes());
    data.push(extension as u8);
    data.extend(&content);

    // 准备模拟字节流
    let reader = Cursor::new(data);
    let mut writer = Cursor::new(Vec::new());
    // 运行被测函数
    let mut protocol = StreamProtocol::new(reader, &mut writer);
    let command = protocol.next_command().await.unwrap();
    // 整理数据和断言
    let file_meta = FileMeta::build(size, hash, width, height, extension).unwrap();
    let file = File::build(file_meta, content).unwrap();
    assert_eq!(command, Command::SaveFile { file });
}

async fn check_respond_list_dir_run_case(count: usize, fill: u8) -> Vec<u8> {
    // 构造输入数据
    let items: Vec<[u8; 20]> = (0..count).map(|_| [fill; 20]).collect();
    let files = stream::iter(items);

    // 构造假的 reader/writer
    let reader = Cursor::new(vec![]);
    let mut writer = Cursor::new(Vec::new());
    let mut protocol = StreamProtocol::new(reader, &mut writer);

    // 执行
    protocol.respond_list_dir(files).await.unwrap();

    writer.into_inner()
}

#[tokio::test]
async fn test_respond_list_dir_zero() {
    let left = check_respond_list_dir_run_case(0, 0xAA).await;
    assert_eq!(left, vec![RESPONSE_LIST_DIR, 0, 0]);
}

#[tokio::test]
async fn test_respond_list_dir_one() {
    let left = check_respond_list_dir_run_case(1, 0xBB).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，1 个文件
    right.extend_from_slice(&1u16.to_be_bytes());
    right.extend_from_slice(&[0xBB; 20]);
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_respond_list_dir_u16max_minus1() {
    let left = check_respond_list_dir_run_case(u16::MAX as usize - 1, 0xCC).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，65534 个文件
    right.extend_from_slice(&(u16::MAX - 1).to_be_bytes());
    right.extend_from_slice(&[0xCC; 20 * (u16::MAX as usize - 1)]);
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_respond_list_dir_u16max() {
    let left = check_respond_list_dir_run_case(u16::MAX as usize, 0xDD).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0xDD; 20 * u16::MAX as usize]);
    // 第 2 帧，0 个文件
    right.extend_from_slice(&0u16.to_be_bytes());
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_respond_list_dir_u16max_plus1() {
    let left = check_respond_list_dir_run_case(u16::MAX as usize + 1, 0xEE).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0xEE; 20 * u16::MAX as usize]);
    // 第 2 帧，1 个文件
    right.extend_from_slice(&1u16.to_be_bytes());
    right.extend_from_slice(&[0xEE; 20]);
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_respond_list_dir_2u16max_minus1() {
    let left = check_respond_list_dir_run_case(2 * u16::MAX as usize - 1, 0x11).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0x11; 20 * u16::MAX as usize]);
    // 第 2 帧，65534 个文件
    right.extend_from_slice(&(u16::MAX - 1).to_be_bytes());
    right.extend_from_slice(&[0x11; 20 * (u16::MAX as usize - 1)]);
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_respond_list_dir_2u16max() {
    let left = check_respond_list_dir_run_case(2 * u16::MAX as usize, 0x22).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0x22; 20 * u16::MAX as usize]);
    // 第 2 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0x22; 20 * u16::MAX as usize]);
    // 第 3 帧，0 个文件
    right.extend_from_slice(&0u16.to_be_bytes());
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_respond_list_dir_2u16max_plus1() {
    let left = check_respond_list_dir_run_case(2 * u16::MAX as usize + 1, 0x33).await;
    let mut right = Vec::new();
    right.push(RESPONSE_LIST_DIR);
    // 第 1 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0x33; 20 * u16::MAX as usize]);
    // 第 2 帧，65535 个文件
    right.extend_from_slice(&u16::MAX.to_be_bytes());
    right.extend_from_slice(&[0x33; 20 * u16::MAX as usize]);
    // 第 3 帧，1 个文件
    right.extend_from_slice(&1u16.to_be_bytes());
    right.extend_from_slice(&[0x33; 20]);
    assert_eq!(left, right);
}

#[tokio::test]
async fn test_to_frame_empty_stream() {
    let mut s = stream::iter(Vec::<[u8; 20]>::new());
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, 0);
    assert!(frame.is_empty());
}

#[tokio::test]
async fn test_to_frame_one_item() {
    let mut s = stream::iter(vec![[42u8; 20]]);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, 1);
    assert_eq!(frame.len(), 20);
    assert_eq!(&frame[0..20], &[42u8; 20]);
}

#[tokio::test]
async fn test_to_frame_small_stream() {
    let items: Vec<[u8; 20]> = (0..3)
        .map(|i| {
            let mut arr = [0u8; 20];
            arr[0] = i;
            arr
        })
        .collect();

    let mut s = stream::iter(items.clone());
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, 3);
    assert_eq!(frame.len(), 3 * 20);

    // 验证数据拼接是否一致
    for (i, chunk) in frame.chunks_exact(20).enumerate() {
        assert_eq!(chunk[0], i as u8);
    }
}

#[tokio::test]
async fn test_to_frame_near_max_minus_two() {
    let count = u16::MAX as usize - 2;
    let items: Vec<[u8; 20]> = (0..count).map(|_| [7u8; 20]).collect();
    let mut s = stream::iter(items);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, u16::MAX - 2);
    assert_eq!(frame.len(), (u16::MAX as usize - 2) * 20);
    assert!(frame.chunks_exact(20).all(|c| c == &[7u8; 20]));
}

#[tokio::test]
async fn test_to_frame_near_max_minus_one() {
    let count = u16::MAX as usize - 1;
    let items: Vec<[u8; 20]> = (0..count).map(|_| [8u8; 20]).collect();
    let mut s = stream::iter(items);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, u16::MAX - 1);
    assert_eq!(frame.len(), (u16::MAX as usize - 1) * 20);
    assert!(frame.chunks_exact(20).all(|c| c == &[8u8; 20]));
}

#[tokio::test]
async fn test_to_frame_exactly_max() {
    let count = u16::MAX as usize;
    let items: Vec<[u8; 20]> = (0..count).map(|_| [9u8; 20]).collect();
    let mut s = stream::iter(items);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, u16::MAX);
    assert_eq!(frame.len(), u16::MAX as usize * 20);
    assert!(frame.chunks_exact(20).all(|c| c == &[9u8; 20]));
}

#[tokio::test]
async fn test_to_frame_max_plus_one() {
    let count = u16::MAX as usize + 1;
    let items: Vec<[u8; 20]> = (0..count).map(|_| [6u8; 20]).collect();
    let mut s = stream::iter(items);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    // to_frame 只会取 u16::MAX 个，多余的留在 stream 里
    assert_eq!(n, u16::MAX);
    assert_eq!(frame.len(), u16::MAX as usize * 20);
    assert!(frame.chunks_exact(20).all(|c| c == &[6u8; 20]));
}

#[tokio::test]
async fn test_to_frame_max_plus_two() {
    let count = u16::MAX as usize + 2;
    let items: Vec<[u8; 20]> = (0..count).map(|_| [5u8; 20]).collect();
    let mut s = stream::iter(items);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    // to_frame 应该最多只读 u16::MAX 个
    assert_eq!(n, u16::MAX);
    assert_eq!(frame.len(), u16::MAX as usize * 20);
    assert!(frame.chunks_exact(20).all(|c| c == &[5u8; 20]));
}

#[tokio::test]
async fn test_to_frame_max_limit() {
    // 构造超过 u16::MAX 的流
    let items: Vec<[u8; 20]> = (0..(u16::MAX as usize) + 10).map(|_| [1u8; 20]).collect();

    let mut s = stream::iter(items);
    let (n, frame) = to_frame(Pin::new(&mut s)).await;

    assert_eq!(n, u16::MAX);
    assert_eq!(frame.len(), u16::MAX as usize * 20);
    assert!(frame.chunks_exact(20).all(|c| c == &[1u8; 20]));
}

// #[tokio::test]
// async fn test_items_stream() {
//     let counts = [0, 65535, 65536, 65536 * 2];
//     for &count in &counts {
//         let hashes_iter = (0..count)
//             .map(|i| {
//                 let mut hash = [0u8; 20];
//                 // index
//                 hash[0] = i as u8;
//                 hash[1] = (i >> 8) as u8;
//                 hash
//             })
//             .collect::<Vec<[u8; 20]>>();
//         let file_stream = futures::stream::iter(hashes_iter);
//         let mut frame_stream = to_frame_stream(file_stream);
//         let mut remaining = count;
//         while remaining > 0 {
//             if let Some(frame) = frame_stream.next().await {
//                 let expected_count = if remaining > u16::MAX as usize {
//                     u16::MAX as usize
//                 } else {
//                     remaining
//                 };
//                 let expected_len = 2 + (expected_count * 20);
//                 assert_eq!(frame.len(), expected_len);
//                 let count_bytes: [u8; 2] = frame[..2].try_into().unwrap();
//                 let count = u16::from_be_bytes(count_bytes);
//                 assert_eq!(count as usize, expected_count);
//                 for i in 0..expected_count {
//                     let start = 2 + (i * 20);
//                     let end = start + 20;
//                     let hash_slice = &frame[start..end];
//                     let mut expected_hash = [0u8; 20];
//                     expected_hash[0] = (count as usize * (count as usize - remaining) + i) as u8;
//                     expected_hash[1] =
//                         ((count as usize * (count as usize - remaining) + i) >> 8) as u8;
//                     assert_eq!(
//                         hash_slice,
//                         expected_hash.as_slice(),
//                         "Hash at index {} does not match",
//                         i
//                     );
//                 }
//                 remaining -= expected_count;
//             } else {
//                 panic!("Expected a frame but got none.");
//             }
//         }
//         assert!(frame_stream.next().await.unwrap().as_slice() == &[0u8, 0]);
//     }
// }
