# EasyBot 插件开发指南

> 本指南从官方入门样例 [`easybot-hello-adapter`](../../) 的**真实开发过程**中提炼：每一个文件、每一条命令、每一处踩过的坑，都是开发时实际遇到并解决的。跟随本指南，以最小的心智负担创建出你自己的插件雏形。
>
> 对照源码阅读效果最佳：本仓库就是教程的"活体对照物"，代码注释即要点。

---

## 目录

- [0. 这份指南怎么用](#0-这份指南怎么用)
- [1. 插件是什么（5 分钟理解模型）](#1-插件是什么5-分钟理解模型)
- [2. 命名规则：`easybot-xxx`](#2-命名规则easybot-xxx)
- [3. 工程骨架（逐文件讲解）](#3-工程骨架逐文件讲解)
- [4. 核心：实现 `PlatformAdapter`](#4-核心实现-platformadapter)
- [5. 测试：先离线，再上真宿主](#5-测试先离线再上真宿主)
- [6. 构建与本地联调](#6-构建与本地联调)
- [7. 端到端验证（send 回环）](#7-端到端验证send-回环)
- [8. 开发中踩过的坑（注意事项）](#8-开发中踩过的坑注意事项)
- [9. 从雏形到生产插件（进阶清单）](#9-从雏形到生产插件进阶清单)
- [10. 发布](#10-发布)
- [附录：调试速查](#附录调试速查)

---

## 0. 这份指南怎么用

- **读者**：想给 EasyBot 写一个适配器插件（对接新的 IM 平台）的 Rust 开发者。不需要提前 clone 主仓库。
- **前置知识**：Rust 基础、`async/await`、`tokio`。不了解插件内部机制也能跟着跑通。
- **最小路径**：先按 [3](#3-工程骨架逐文件讲解)–[7](#7-端到端验证send-回环) 跑通一个能加载、能 echo 的雏形；再按 [8](#8-开发中踩过的坑注意事项) 避开已知的坑；最后按 [9](#9-从雏形到生产插件进阶清单) 深化成真实平台适配器。
- **快捷入口**：不想手写骨架？用 `easybot plugin new <name>` 脚手架生成，再把本仓库当作"填好了的实现"对照着看。

---

## 1. 插件是什么（5 分钟理解模型）

插件是一个 **Rust cdylib 动态库**（Linux `.so` / macOS `.dylib` / Windows `.dll`），由宿主 EasyBot 主程序 **进程内 dlopen** 加载，通过 FFI 边界交互：

```
宿主 EasyBot（主程序）
  └─ PluginLoader
       └─ dlopen → libeasybot_xxx.{so|dylib|dll}
            └─ easybot_plugin_create() → Box<dyn PlatformAdapter>  （FFI）
                 └─ 注册进 AdapterRegistry → 与内置适配器同等对待
```

**FFI 边界上只有三个 C 导出函数**（由 `declare_plugin!` 宏生成，你不用手写）：

| 导出函数 | 作用 |
|---|---|
| `easybot_abi_version()` | 返回 SDK ABI 版本，宿主加载时比对 |
| `easybot_plugin_create()` | 创建适配器实例，返回 `*mut c_void` |
| `easybot_plugin_destroy(ptr)` | 销毁实例（幂等） |

接口本身不是 C，而是 Rust trait `PlatformAdapter`——宿主拿到 `Box<dyn PlatformAdapter>` 后像调用普通 Rust 对象一样调用 `init/connect/send/...`。**好处**：你写的是普通 Rust；**代价**：两侧必须遵守几条 ABI 硬约束（见下）。

**理解模型一句话**：插件 = 一个实现了 `PlatformAdapter` trait 的 cdylib，加载后被当作内置适配器运行。就这么简单。

### 生命周期状态机

```
init(config) → connect() → send()/... → disconnect()
  Created → Starting → Connecting → Connected → Reconnecting → Failed → Stopped
```

`init` 只解析配置、存凭据（**不建网络连接**）；`connect` 才建立连接、启动后台任务。

### ABI 硬约束速览（详细见第 8 节）

| 约束 | 原因 | 处置 |
|---|---|---|
| `panic = "abort"` | panic 越过 extern "C" 边界解栈是 UB | release profile 已设 |
| **与宿主共用默认分配器** | String/Vec 的堆所有权跨 FFI 转移 | 宿主不得用自定义 allocator（第 8.1 节，最重要的坑） |
| `sdk_version` 必须等于 SDK 常量 | ABI 布局兼容 | 编译期即确定 |
| 命名 `easybot-xxx` | 会话 key / 路由 / 市场统一 | 见第 2 节 |

---

## 2. 命名规则：`easybot-xxx`

官方插件统一使用 **`easybot-xxx`** 前缀，且**同一个名字贯穿所有层面**，避免在不同地方叫不同名字的错乱：

| 层面 | 约定 | 本仓库示例 |
|---|---|---|
| 仓库名 | `easybot-xxx` | `EasyIndie/easybot-hello-adapter` |
| `Cargo.toml [package].name` | `easybot-xxx` | `easybot-hello-adapter` |
| cdylib 产物名 | Rust 用下划线连接 crate 名 → `libeasybot_xxx.{so,dylib,dll}` | `libeasybot_hello_adapter.dylib` |
| `plugin.yaml` `name` | `easybot-xxx` | `"easybot-hello-adapter"` |
| `platform_name()` | `easybot-xxx` | `"easybot-hello-adapter"` |
| 市场安装名 | `easybot-xxx` | `easybot plugin install EasyIndie/easybot-hello-adapter`（发布者为 GitHub 组织/用户） |

> ⚠ **命名一经发布即对外稳定**：`platform_name()` 参与会话 key（`{platform}:{chatId}`）与路由。改名会让旧会话无法归并、市场安装路径变化。创建仓库/生成工程时就定好，发布后不轻易变更。
>
> 社区/第三方插件不强制 `easybot-` 前缀，但建议遵循同一 kebab-case 规范，并通过 `publisher/name` 限定来源。

---

## 3. 工程骨架（逐文件讲解）

脚手架生成产物即此形状。以下逐个文件说明**为什么**这么写。

### 3.1 `Cargo.toml`

```toml
[package]
name = "easybot-hello-adapter"        # 命名规则：easybot-xxx
version = "0.1.0"
edition = "2024"
publish = false                       # 不走 crates.io

[lib]
crate-type = ["cdylib", "rlib"]       # cdylib 供宿主 dlopen；rlib 让 cargo test 能链接

[dependencies]
easybot-plugin-sdk = { git = "https://github.com/EasyIndie/EasyBot", tag = "v0.0.34", package = "easybot-plugin-sdk" }
serde_json = "1"
tracing = "0.1"

[dev-dependencies]
easybot-plugin-sdk = { git = ".../EasyBot", tag = "v0.0.34", package = "easybot-plugin-sdk", features = ["testing"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }

[profile.release]
opt-level = 3
lto = "fat"            # 减小 cdylib 体积
codegen-units = 1
strip = "symbols"
panic = "abort"        # ABI 硬约束：panic 不越过 FFI 边界
```

要点：

- **SDK 走 git tag 依赖**（`git = ... + tag = "v0.0.34"`）：插件作者端**无需 clone 主仓库**，cargo 自动拉取；tag 固定版本保证 ABI 一致。**不要**用 crates.io（SDK 尚未发布）。
- **`testing` feature 只放 `[dev-dependencies]`**：`PluginTestHost` 是测试宿主，绝不能进插件产物本体。
- **`panic = "abort"`**：cdylib 内 panic 若跨 FFI unwind 是 UB，abort 保证定义行为（崩溃也比内存破坏好定位）。

### 3.2 `.cargo/config.toml`（本地联调 [patch]，可选）

```toml
# 提交前必须注释回！[patch] 路径是本机绝对路径，CI/他人 clone 后无法解析
[patch."https://github.com/EasyIndie/EasyBot"]
easybot-plugin-sdk = { path = "/你/本地/EasyBot/crates/easybot-plugin-sdk" }
```

作用：开发迭代时把 SDK 指向本地主仓 checkout，秒级重编，不用每次拉 git 依赖。**发布前必须注释回**，否则 CI 失败。第 8.4 节详述。

### 3.3 `plugin.yaml`

```yaml
name: "easybot-hello-adapter"     # 须与 platform_name() 一致
display_name: "Hello Adapter"
description: "An EasyBot adapter plugin named easybot-hello-adapter (官方入门样例)."
version: "0.1.0"
sdk_version: 1                    # = SDK 的 EASYBOT_PLUGIN_ABI_VERSION
author: "EasyBot Contributors"
```

宿主以此清单识别插件（市场安装时由宿主根据 `easybot-plugin.json` **自动合成**此清单；手动安装才自己写）。

### 3.4 `.gitignore` / `LICENSE`

- `.gitignore`：`/target`、`.env`（凭据严禁提交，发布 CI 有 gitleaks 扫描）、编辑器文件。
- `LICENSE`：GPL-3.0（与主仓一致），发布前补齐。

### 3.5 `.github/workflows/plugin-publish.yml`

发布者 CI 模板（从主仓复制，自包含、只引用公开 action）：6-target 交叉编译 → gitleaks 扫密钥 → ed25519 签名 → 组装 `easybot-plugin.json` → 发 Release。见第 10 节。

---

## 4. 核心：实现 `PlatformAdapter`

`src/lib.rs` 是插件本体，结构固定：

```rust
#![allow(unsafe_code)]              // declare_plugin! 展开 FFI；对齐 SDK 自身处理

use easybot_plugin_sdk::prelude::*;
use std::sync::Arc;

pub struct HelloAdapter {
    state: AdapterState,
    event_bus: Option<Arc<EventBus>>,
    // TODO: token: Option<String>,
    // TODO: client: Option<reqwest::Client>,
}

#[async_trait]
impl PlatformAdapter for HelloAdapter {
    fn platform_name(&self) -> &str { "easybot-hello-adapter" }   // 命名规则
    fn display_name(&self) -> &str { "Hello Adapter" }
    fn capabilities(&self) -> &[Capability] { /* 声明 Text 支持、Interactive 不支持 */ }
    fn set_event_bus(&mut self, bus: Arc<EventBus>) { self.event_bus = Some(bus); }

    async fn init(&mut self, _config: AdapterConfig) -> Result<InitResult, GatewayError> {
        // 只解析配置、存凭据，不建连接
        self.state = AdapterState::Starting;
        Ok(InitResult { ok: true, error: None })
    }

    async fn connect(&mut self) -> Result<ConnectResult, GatewayError> {
        // TODO: 建立真实连接。失败分类：网络→Transient，凭据拒→Permanent
        self.state = AdapterState::Connected;
        Ok(ConnectResult::ok(Some(BotInfo { name: "Hello Adapter".into(), username: Some("hello-adapter".into()), id: "hello-adapter".into() })))
    }

    async fn send(&self, params: SendTextParams) -> Result<SendResult, GatewayError> {
        // TODO: 调用平台发送 API。网络失败用 GatewayError::Transient 包裹
        tracing::info!(chat_id = %params.chat_id, text = %params.message.text, "send");
        // 演示事件总线：把发送文本回显为 message.inbound 事件
        if let Some(bus) = &self.event_bus {
            bus.publish(GatewayEvent::new(
                event_types::MESSAGE_INBOUND,
                self.platform_name(),
                serde_json::json!({ "chat_id": params.chat_id, "text": params.message.text }),
            ));
        }
        Ok(SendResult::ok(format!("{}-{}", self.platform_name(), params.chat_id)))
    }

    // ... disconnect / state / health / get_chat_info / runtime_config / status_summary
}

declare_plugin!(HelloAdapter, HelloAdapter::new);
```

要点：

- **`declare_plugin!(Struct, Constructor)` 每个插件且只能调用一次**，生成三个 FFI 导出。
- **`set_event_bus`**：入站事件（消息/回调）经它注入的 `EventBus` 发布给宿主；`PluginTestHost` 测试也用它断言。
- **`send()` 是演示 echo 的核心**：宿主调用 → 插件发布 `message.inbound` 事件 → 回环验证（第 7 节）。
- **错误分类**：网络层失败用 `GatewayError::Transient`（瞬态→重连退避），凭据被拒用 `Permanent`（立即停用），业务失败用 `SendError`。分类贯穿健康监测的重试/停用决策。这是最容易写错、影响最大的细节之一，进阶见第 9 节。

---

## 5. 测试：先离线，再上真宿主

测试金字塔（按成本从低到高）：**单元 → PluginTestHost → wiremock → e2e**。日常开发 90% 的工作量在头两层，离线秒级跑。

### 5.1 单元测试（`tests/unit.rs`）

只测不依赖宿主的纯逻辑：

```rust
#[test]
fn platform_identity() {
    assert_eq!(HelloAdapter::platform_name(), "easybot-hello-adapter");
}

#[test]
fn declares_text_capability() {
    let caps = HelloAdapter::capabilities();
    assert!(caps.iter().any(|c| c.name == CapabilityName::Text && c.supported));
}

#[test]
fn starts_in_created_state() {
    assert_eq!(HelloAdapter::new().state(), AdapterState::Created);
}
```

### 5.2 PluginTestHost 集成测试（`tests/host_test.rs`）

SDK 的 `testing` feature 提供 `PluginTestHost`——**内存宿主**，模拟 attach/init/connect/send/事件流，无需启动真实网关：

```rust
#[tokio::test]
async fn lifecycle_and_send_roundtrip() {
    let host = PluginTestHost::new();
    let mut adapter = HelloAdapter::new();
    host.attach(&mut adapter);                                  // 挂进内存宿主
    host.init(&mut adapter, PluginTestHost::config())           // 默认 AdapterConfig
        .await.unwrap();
    let mut rx = host.subscribe(event_types::MESSAGE_INBOUND);  // 订阅插件事件

    let conn = host.connect(&mut adapter).await.unwrap();
    assert!(conn.ok, "connect should succeed");

    let result = host.send(&mut adapter, SendTextParams {
        chat_id: "c1".into(),
        message: OutboundMessage { text: "hello".into(), parse_mode: ParseMode::None },
        reply_to: None,
        metadata: None,
    }).await.unwrap();
    assert!(result.success);

    // 插件发出的 send 请求被宿主记录，可断言参数形态
    assert_eq!(host.send_log().len(), 1);
    assert_eq!(host.send_log()[0].message.text, "hello");

    // 插件在 send 内发布了 message.inbound 事件（回显文本）
    let ev = recv_event(&mut rx, std::time::Duration::from_secs(1))
        .await.expect("plugin should publish an inbound event on send");
    assert_eq!(ev.event_type, event_types::MESSAGE_INBOUND);
    assert_eq!(ev.source, "easybot-hello-adapter");
    assert_eq!(ev.data["text"], "hello");

    host.disconnect(&mut adapter).await.unwrap();
    assert_eq!(adapter.state(), AdapterState::Stopped);
}
```

> API 形态：宿主方法统一 `host.xxx(&mut adapter, ...)`（内存宿主模拟宿主侧语义），
> `send_log()` 记录插件发出的 send 调用、`recv_event` 收取插件发布的事件、`inbound_text/
> media/callback` 构造器注入伪造入站事件。全部离线，无需启动真实网关。

**方法论：传输可注入**——真实适配器的 HTTP client 经构造器或 `init(config)` 注入，测试替换为 wiremock，离线跑通协议交互（对应测试金字塔第三层）。

### 5.3 跑法

```bash
cargo test    # 单元 + PluginTestHost，全部离线
```

---

## 6. 构建与本地联调

```bash
# 构建 release cdylib（自包含，作者无需 clone 主仓）
cargo build --release
# 产物：target/release/libeasybot_hello_adapter.{so|dylib|dll}

# 手动装入宿主插件目录（dev 联调）
mkdir -p ~/.easybot/plugins/easybot-hello-adapter
cp target/release/libeasybot_hello_adapter.dylib ~/.easybot/plugins/easybot-hello-adapter/
cp plugin.yaml ~/.easybot/plugins/easybot-hello-adapter/
```

启动宿主：

```bash
easybot --debug    # 或 RUST_LOG=easybot_hello_adapter=debug easybot
```

加载成功应看到日志 `Loaded plugin 'easybot-hello-adapter'`，且：

```bash
curl -H "Authorization: Bearer <token>" http://localhost:8080/api/v1/plugins
# → sdk_version: 1, load_error: null

curl -H "Authorization: Bearer <token>" http://localhost:8080/api/v1/adapters
# → easybot-hello-adapter: Connected（无凭据自动启用）
```

> 无凭据也能 Connected：插件未声明需要 token（`runtime_config().token_configured = false`），宿主自动启用。真实适配器在 `init` 里校验 token，缺失则 `InitResult { ok: false }`。

---

## 7. 端到端验证（send 回环）

插件在真实宿主里最关键的验证：**宿主 → 插件 `send()` → 插件发布 echo 事件 → 事件流回宿主**。

```bash
# 1. 登录拿 token（EASYBOT_ADMIN_PASSWORD 为测试密码）
curl -s -X POST http://localhost:8080/admin/login \
  -H 'Content-Type: application/json' \
  -d '{"password":"test-admin-pw-123"}' | jq -r '.session_key'

# 2. 发消息（target = {platform}:{chatId}，platform = easybot-hello-adapter）
curl -s -X POST http://localhost:8080/api/v1/messages/send \
  -H "Authorization: Bearer <token>" -H 'Content-Type: application/json' \
  -d '{"target":"easybot-hello-adapter:chat-1","text":"hello from curl"}'
# → message_id: "easybot-hello-adapter-chat-1"

# 3. 事件回环（插件把发送文本回显为 message.inbound）
#    通过 WebSocket /api/v1/ws 订阅，或观察日志
```

验证清单：

- [ ] `/api/v1/plugins` 中插件已加载（load_error = null）
- [ ] `/api/v1/adapters` 中 Connected
- [ ] `send` 返回 `message_id`（证明宿主→插件 FFI 双向传递了带堆所有权的值）
- [ ] echo 事件 `source == "easybot-hello-adapter"`（证明插件→宿主事件总线）

> **"send 返回正常"是插件 ABI 健康度的终极测试**：它意味着跨 FFI 的 String/Vec 所有权转移没有触发分配器冲突（第 8.1 节）。任何分配器不匹配都会在这一步崩溃或死锁。

---

## 8. 开发中踩过的坑（注意事项）

这是本指南**最有价值**的部分——每一条都是真实开发中遇到并解决的。

### 8.1 ⚠ FFI 分配器契约（最深的坑，务必先看）

**症状 1 —— SIGABRT**：宿主 `send` 调用插件时进程崩溃，报错：

```
___BUG_IN_CLIENT_OF_LIBMALLOC_POINTER_BEING_FREED_WAS_NOT_ALLOCATED
```

**症状 2 —— 死锁**：给插件加上 mimalloc 后不再崩溃，但宿主 teardown 时卡死，主线程卡在：

```
drop_in_place<Runtime> → __rust_dealloc → mi_free → mi_bfield_atomic_clear_once_set → _mi_prim_thread_yield  （自旋）
```

**根因**：插件通过 FFI 与宿主收发 `SendTextParams`/`SendResult` 等**带堆所有权的值**（String/Vec/Value）。这些值经常"宿主构造、插件 Drop"（或反向）。Rust 要求**谁的分配器分配、谁的 Drop 释放**。而宿主原来用 mimalloc 做全局分配器、插件用系统 malloc：

| 宿主分配器 | 插件分配器 | 结果 |
|---|---|---|
| mimalloc | 系统 malloc（默认） | 宿主分配的值被插件用系统 free 释放 → **SIGABRT** |
| mimalloc | 插件自己静态链接一份 mimalloc | 进程内两套 mimalloc 堆，析构 abandoned-page 自旋 → **死锁** |

**唯一正确解**：**两侧共用同一全局分配器**。插件的 `send()` 必须能安全 Drop 宿主传入的 `SendTextParams`，反之亦然。因此：

- **插件侧**：**不要**声明自定义 `#[global_allocator]`，保持默认（= 系统分配器）。
- **宿主侧**：宿主**必须**使用默认系统分配器，**不得**引入 mimalloc 等自定义 `#[global_allocator]`（`bin/Cargo.toml` 有注释警示防止回归）。

**在你的插件里如何避免**：什么都不用做——只要不声明 `#[global_allocator]` 即可。真正的雷在宿主侧；若宿主已含 mimalloc（旧版本），升级宿主到不含自定义分配器的版本。

> 这是为什么本样例的 `src/lib.rs` 顶部有一大段注释、`Cargo.toml` 有一大段注释——它们不是废话，是防止后人"优化"时重新踩进去。

### 8.2 `panic = "abort"` 是硬要求

panic 越过 extern "C" FFI 边界 unwinding 是 UB。release profile 已设 `panic = "abort"`：插件内 panic → 进程直接终止（可定位），而不是内存破坏。**不要**改成 unwind。

### 8.3 SDK git tag 与本地 [patch] 版本必须一致

本地联调把 SDK patch 到本地 checkout 后，`Cargo.lock` 里 SDK 版本被本地版本覆盖。**发布基于 git tag 的 SDK**，两者 ABI 必须一致。换 tag 前确认该 tag 已包含你要的 SDK API。

### 8.4 [patch] 提交前必须注释回

`.cargo/config.toml` 的 `[patch]` 是本机绝对路径，提交/CI/他人 clone 后无法解析。**发布前注释回**，让构建回落到 git tag 依赖。这是发布流程里的标准动作。

### 8.5 产物命名：Rust 用下划线连 crate 名

crate `easybot-hello-adapter`（kebab）→ cdylib `libeasybot_hello_adapter.{so|dylib|dll}`（下划线）。签名、`easybot-plugin.json` 的 `library` 字段、手动安装都按此文件名。

### 8.6 `platform_name()` 与 `plugin.yaml` 的 `name` 必须一致

宿主用 `platform_name()` 做会话 key 与路由。两者不一致 → 安装/启停/会话归并错乱。这也是命名规则 `easybot-xxx` 保证的。

### 8.7 调试技巧：怎么定位崩溃

| 场景 | 手段 |
|---|---|
| 插件逻辑错误 | ⚠ **插件 `tracing` 不转发到宿主日志**（dylib 有独立 tracing 注册表，宿主订阅不到；SDK 暂无日志桥）。dev 调试直接用 `eprintln!`（与宿主共享进程 stderr，出现在宿主控制台/日志重定向里）。真实适配器可把诊断信息经 `SendResult`/`GatewayError` 或事件总线回传给宿主 |
| 加载失败 | `/api/v1/plugins` 返回 load_error；`easybot plugin inspect <name>` 转储清单/签名/错误 |
| 崩溃（SIGABRT/SIGSEGV） | 优先怀疑分配器（8.1）；用 `lldb easybot` 复现，backtrace 定位 `drop_in_place` 栈帧 |
| 死锁 | 优先怀疑双 mimalloc（8.1）；`sample easybot <pid>` 看各线程栈 |

**崩溃诊断的通用流程**（本仓库开发时实际用过）：

1. 看崩溃栈里是否有 `drop_in_place<Vec/...>` / `mi_free` / `__rust_dealloc` → 指向**分配器不匹配**。
2. 复现时把宿主 + 插件的分配器状态列出来（宿主 `#[global_allocator]`？插件有没有？）。
3. 交叉验证：宿主 mimalloc + 插件系统 malloc → 崩溃；插件也加 mimalloc → 死锁；两侧都系统 malloc → 正常。**结论从症状反推，不要猜**。

---

## 9. 从雏形到生产插件（进阶清单）

把 echo 雏形深化成真实平台适配器时，按此清单逐项落地（与主仓 `docs/plugin-methodology.md` 同一标准）：

- [ ] **HTTP client 注入**：`reqwest::Client` 经构造器或 `init(config)` 注入，测试用 wiremock 替换（传输可注入）。
- [ ] **错误分类**：网络层失败 `GatewayError::Transient`；凭据被拒 `ConnectResult::failed(_, ConnectErrorKind::Permanent)`；业务失败 `SendError`。分类驱动健康监测的重试/停用。
- [ ] **心跳**：只在错误重试路径 `heartbeat.beat()`；**禁止**独立定时器无条件 beat（会让健康监测误判）。
- [ ] **缓存上限**：任何缓存必须带大小上限或 TTL 淘汰（参考主仓 `CHAT_TYPE_CACHE_LIMIT` 等模式），防内存无限增长。
- [ ] **并发上限**：`tokio::spawn` 的轮询/重试循环用 `Semaphore` 或 `JoinSet` 限并发。
- [ ] **媒体按 chat type 区分**：不同会话类型用不同 msg_type（QQ 群/频道/私聊各异的教训见主仓 CLAUDE.md）。
- [ ] **凭据纪律**：不得硬编码/日志输出 token；Token 结构体 Debug 须脱敏。
- [ ] **ABI 纪律**：`sdk_version` 永远等于编译所用 SDK 常量；升级 SDK = 重新发布。

---

## 10. 发布

发布全自动走 CI（本仓库 `.github/workflows/plugin-publish.yml`，自包含模板）：

```bash
# 1. 生成发布者密钥对（只跑一次，私钥存 GitHub Actions secret）
easybot-plugin-sign gen-keypair          # 产出 PUBLISHER_PRIVATE_KEY + PUBLIC_KEY

# 2. 登记公钥（进入宿主 trusted_publishers 后，插件才能被验签安装）
#    —— 向 EasyBot 主仓库提交 PR，把公钥加入 trusted_publishers 默认列表

# 3. 打 tag 推送
git tag v0.1.0 && git push origin v0.1.0
# plugin-publish.yml 自动：6-target 交叉编译 → gitleaks 扫密钥 → ed25519 签名
# → 组装 easybot-plugin.json → 发 GitHub Release

# 4. 用户侧安装
easybot plugin install EasyIndie/easybot-hello-adapter
```

关键点：

- **签名对象 = 产物字节本身**（`.so/.dylib/.dll`）；元数据被 sha256 间接锚定。安装后 + 启动加载时**双时点验签**。
- **签名 ≠ 安全**：ed25519 只证"作者 + 完整性"，不证代码无害。插件无沙箱，以宿主权限运行——生产隔离用容器化兜底。详见主仓 `docs/SECURITY.md`。
- **首次安装信任**：发布者不在宿主信任列表时，客户端会要求确认；`--yes` 不自动加入 `.trust`（显式 `easybot plugin trust <publisher> --public-key <k>` 才加入）。

---

## 附录：调试速查

```bash
# 构建
cargo build --release                      # 产出自包含 cdylib
cargo test                                 # 单元 + PluginTestHost 离线测试

# 手动装入宿主（dev）
mkdir -p ~/.easybot/plugins/easybot-hello-adapter
cp target/release/libeasybot_hello_adapter.dylib ~/.easybot/plugins/easybot-hello-adapter/
cp plugin.yaml ~/.easybot/plugins/easybot-hello-adapter/

# 观察
easybot --debug
curl -H "Authorization: Bearer <token>" localhost:8080/api/v1/plugins
curl -H "Authorization: Bearer <token>" localhost:8080/api/v1/adapters
easybot plugin inspect easybot-hello-adapter

# 发布
easybot-plugin-sign gen-keypair            # 一次性
git tag v0.1.0 && git push origin v0.1.0   # 触发发布 CI
```
