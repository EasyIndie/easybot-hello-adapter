//! EasyBot Hello Adapter — 官方插件入门样例（echo 适配器）
//!
//! 这是插件开发指南的对照实现：
//!   - 开发指南：`docs/plugin-development-guide.md`（从零到发布的最小心智负担路径）
//!   - 在线教程：https://github.com/EasyIndie/EasyBot/blob/main/docs/plugin-quickstart.md
//!   - 完整参考：https://github.com/EasyIndie/EasyBot/blob/main/docs/plugin-guide.md
//!   - 方法论：  https://github.com/EasyIndie/EasyBot/blob/main/docs/plugin-methodology.md
//!
//! 构建 / 测试（SDK 走 git tag，作者无需 clone 主仓）：
//!   cargo test     # 单元 + PluginTestHost 离线测试（无需启动真实网关）
//!   cargo build --release  # 产出自包含可执行文件（进程外插件）

// 进程外插件：本 crate 只实现 `PlatformAdapter`；进程入口在 `src/main.rs`
// （`easybot_plugin_sdk::run_plugin!`）。无 FFI、无 unsafe、无分配器约束。

use easybot_plugin_sdk::prelude::*;
use std::sync::Arc;

/// 适配器主体：持有状态与可选的事件总线。
///
/// TODO: 按平台需求添加字段——HTTP client、token、缓存（须带大小上限或 TTL，见
/// docs/plugin-methodology.md「缓存」一节）。
pub struct HelloAdapter {
    state: AdapterState,
    event_bus: Option<Arc<EventBus>>,
    // TODO: token: Option<String>,
    // TODO: client: Option<reqwest::Client>,
}

impl HelloAdapter {
    /// 构造器（供 `run_plugin!` 使用）。
    ///
    /// 若平台需要非默认初始化，可在此注入 HTTP client 等依赖——测试用 wiremock
    /// 替换（传输可注入方法论，见 docs/plugin-methodology.md）。
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HelloAdapter {
    fn default() -> Self {
        Self {
            state: AdapterState::Created,
            event_bus: None,
        }
    }
}

#[async_trait]
impl PlatformAdapter for HelloAdapter {
    fn platform_name(&self) -> &str {
        // 平台名 = 插件名 = `easybot-xxx`（宿主用它做会话 key、路由与展示）。
        "easybot-hello-adapter"
    }

    fn display_name(&self) -> &str {
        "Hello Adapter"
    }

    fn capabilities(&self) -> &[Capability] {
        // 声明平台能力：宿主据此路由消息与展示。
        // TODO: 按平台调整 supported / limits（CapabilityName 见 SDK prelude）。
        &[
            Capability {
                name: CapabilityName::Text,
                supported: true,
                limits: None,
            },
            Capability {
                name: CapabilityName::Interactive,
                supported: false,
                limits: None,
            },
        ]
    }

    fn set_event_bus(&mut self, bus: Arc<EventBus>) {
        // 入站事件（消息/回调）经总线发布给宿主；测试宿主（PluginTestHost）也用它断言。
        self.event_bus = Some(bus);
    }

    async fn init(&mut self, _config: AdapterConfig) -> Result<InitResult, GatewayError> {
        // 解析配置。凭据优先来自适配器 env 或 gateway.yaml 的 `adapters.<platform>` 段。
        // TODO: self.token = _config.token.clone();
        // if self.token.is_none() {
        //     return Ok(InitResult {
        //         ok: false,
        //         error: Some("easybot-hello-adapter token required".into()),
        //     });
        // }
        self.state = AdapterState::Starting;
        Ok(InitResult {
            ok: true,
            error: None,
        })
    }

    async fn connect(&mut self) -> Result<ConnectResult, GatewayError> {
        // TODO: 建立真实连接（WebSocket / 长轮询）。失败时分类标记：
        //   网络/临时失败 → ConnectResult::failed(msg, Some(ConnectErrorKind::Transient))
        //   凭据被拒     → ConnectResult::failed(msg, Some(ConnectErrorKind::Permanent))
        // 分类会穿透健康监测的重试/停用决策，见 docs/plugin-methodology.md。
        // 注意：后台轮询任务用 tokio::spawn 时须带 Semaphore/JoinSet 并发上限。
        self.state = AdapterState::Connected;
        Ok(ConnectResult::ok(Some(BotInfo {
            name: "Hello Adapter".into(),
            username: Some("hello-adapter".into()),
            id: "hello-adapter".into(),
        })))
    }

    async fn disconnect(&mut self) -> Result<(), GatewayError> {
        // TODO: 停止后台任务、释放连接。
        self.state = AdapterState::Stopped;
        Ok(())
    }

    fn state(&self) -> AdapterState {
        self.state.clone()
    }

    async fn health(&self) -> HealthReport {
        HealthReport {
            status: if self.state == AdapterState::Connected {
                HealthStatus::Healthy
            } else {
                HealthStatus::Down
            },
            connected: self.state == AdapterState::Connected,
            last_connected_at: None,
            last_error_at: None,
            last_error: None,
            messages_in: 0,
            messages_out: 0,
            errors: 0,
            uptime: None,
        }
    }

    async fn send(&self, params: SendTextParams) -> Result<SendResult, GatewayError> {
        // TODO: 调用平台发送 API（reqwest）。网络层失败须用 GatewayError::Transient
        // 包裹，让重连路径按瞬态处理；业务失败用 GatewayError::SendError。

        // ⚠ 插件日志注意：`tracing::info!` 不会出现在宿主日志里（插件 dylib 有独立
        // 的 tracing 注册表，宿主订阅不到；SDK 暂无日志转发桥）。dev 调试直接
        // `eprintln!`——插件与宿主共享进程 stderr，会出现在宿主控制台/日志重定向里。
        // 详见 docs/plugin-development-guide.md §8.7。
        eprintln!(
            "[easybot-hello-adapter] send chat_id={} text={}",
            params.chat_id, params.message.text
        );
        tracing::info!(chat_id = %params.chat_id, text = %params.message.text, "send");

        // 演示事件总线：把发送文本回显为 message.inbound 事件（tests/host_test.rs 断言用）。
        if let Some(bus) = &self.event_bus {
            bus.publish(GatewayEvent::new(
                event_types::MESSAGE_INBOUND,
                self.platform_name(),
                serde_json::json!({
                    "chat_id": params.chat_id,
                    "text": params.message.text,
                }),
            ));
        }
        Ok(SendResult::ok(format!(
            "{}-{}",
            self.platform_name(),
            params.chat_id
        )))
    }

    async fn get_chat_info(&self, _chat_id: &str) -> Result<ChatInfo, GatewayError> {
        // TODO: 调用平台查询接口；暂未支持可返回 capability_not_supported。
        Err(GatewayError::capability_not_supported("get_chat_info"))
    }

    fn runtime_config(&self) -> AdapterRuntimeConfig {
        AdapterRuntimeConfig {
            enabled: true,
            token_configured: false,
            extra: serde_json::Value::Null,
        }
    }

    fn status_summary(&self) -> AdapterStatusSummary {
        AdapterStatusSummary {
            platform: self.platform_name().to_string(),
            display_name: self.display_name().to_string(),
            state: self.state.clone(),
            connected: self.state == AdapterState::Connected,
            health: None,
            last_error: None,
            uptime: None,
            messages_in: 0,
            messages_out: 0,
        }
    }
}

// 入口见 `src/main.rs`：
//     easybot_plugin_sdk::run_plugin!(HelloAdapter, HelloAdapter::new);
