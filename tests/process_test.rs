//! 进程外宿主测试：把插件作为**真实子进程**启动，走完整的 stdio JSON 协议
//! （`handshake → init → connect → send → 事件回传 → shutdown`）。
//!
//! 与 `host_test.rs`（内存宿主，第 2 层）互补：本测试验证插件作为**可执行文件**
//! 的实际行为——生产路径就是它（测试金字塔第 2.5 层）。
//!
//! 需要先构建本插件的可执行文件：
//!     cargo build && cargo test
//! （`CARGO_BIN_EXE_<name>` 由 cargo 在测试时自动提供，无需手动找路径。）

use std::time::Duration;

use easybot_plugin_sdk::prelude::*;
use easybot_plugin_sdk::testing::{ProcessPluginTestHost, recv_event};

fn plugin_bin() -> &'static str {
    env!("CARGO_BIN_EXE_easybot-hello-adapter")
}

#[tokio::test]
async fn process_host_lifecycle_and_events() {
    let host = ProcessPluginTestHost::spawn(plugin_bin(), "easybot-hello-adapter", "Hello Adapter")
        .await
        .expect("spawn plugin process");

    let mut rx = host.subscribe(event_types::MESSAGE_INBOUND);

    let result = host.send_text("chat-1", "hello").await.expect("send");
    assert!(result.success, "send should succeed");

    let event = recv_event(&mut rx, Duration::from_secs(5))
        .await
        .expect("plugin should push a message.inbound event");
    assert_eq!(event.event_type, event_types::MESSAGE_INBOUND);
    assert_eq!(event.source, "easybot-hello-adapter");
    assert_eq!(event.data["text"], "hello");

    assert!(!host.child_exited(), "plugin process should still be alive");
    host.shutdown().await.expect("shutdown");
}
