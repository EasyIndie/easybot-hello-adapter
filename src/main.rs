//! EasyBot Hello Adapter — 进程外插件入口。
//!
//! 宿主 EasyBot 把本可执行文件作为**子进程**启动，双方通过 stdin/stdout
//! 逐行 JSON（`easybot-plugin-protocol`）通信：
//! `handshake → init → connect → 收发消息/事件回传 → shutdown`。
//!
//! `run_plugin!` 负责建 tokio 运行时、响应协议请求、驱动 `PlatformAdapter`
//! 生命周期，并把适配器发布到事件总线的事件回传给宿主。
//!
//! 适配器实现见 `src/lib.rs`；开发指南见 `docs/plugin-development-guide.md`。

use easybot_hello_adapter::HelloAdapter;

easybot_plugin_sdk::run_plugin!(HelloAdapter, HelloAdapter::new);
