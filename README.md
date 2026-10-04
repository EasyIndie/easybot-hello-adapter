# easybot-hello-adapter

EasyBot 官方插件**入门样例**（echo 适配器）——一个最小可跑的插件雏形，也是插件开发的教学对照物。

> 插件命名规则：官方插件统一 `easybot-xxx` 前缀，且同一个名字贯穿仓库名 / package name / 可执行产物 / `plugin.yaml` 的 `name` 与 `command` / `platform_name()`。

> **SDK 版本要求**：本样例依赖 `easybot-plugin-sdk` 的 **`v0.0.42`**（进程外插件 `run_plugin!`
> 自该版本引入）。首次 `cargo build` 需能访问该 tag；`Cargo.lock` 未提交，首次构建后会自动生成。

## 这是什么

- **最小可跑**：完整实现 `PlatformAdapter` 的 echo 适配器，加载即 Connected，`send()` 回显事件。
- **教学对照物**：开发全过程（逐文件讲解 + 踩坑记录 + 调试过程）沉淀在 [**插件开发指南**](docs/plugin-development-guide.md)。
- **自包含**：SDK 走 git tag 依赖，作者无需 clone 主仓；`cargo test` 全部离线（内存宿主 + 进程外宿主）。
- **进程外架构**：编译为独立可执行文件，宿主 EasyBot 以子进程方式启动，经 stdin/stdout JSON 协议通信——**不使用 dlopen/cdylib**（官方 Linux 发行版是 musl 全静态二进制，无法加载动态库）。

## 快速开始

```bash
cargo test                     # 单元 + PluginTestHost 离线测试
cargo build --release          # 产出自包含可执行文件（进程外插件）
```

手动装入宿主（dev 联调）：

```bash
mkdir -p ~/.easybot/plugins/easybot-hello-adapter
cp target/release/easybot-hello-adapter ~/.easybot/plugins/easybot-hello-adapter/   # Windows: .exe
cp plugin.yaml ~/.easybot/plugins/easybot-hello-adapter/
easybot --debug                # 日志出现 "Loaded plugin 'easybot-hello-adapter'"
```

## 教程

1. [插件开发指南](docs/plugin-development-guide.md) —— 从真实开发过程提炼的完整指南（**推荐从这里开始**）
2. 主仓文档：[快速上手](https://github.com/EasyIndie/EasyBot/blob/main/docs/plugin-quickstart.md) · [完整参考](https://github.com/EasyIndie/EasyBot/blob/main/docs/plugin-guide.md) · [方法论](https://github.com/EasyIndie/EasyBot/blob/main/docs/plugin-methodology.md) · [安全模型](https://github.com/EasyIndie/EasyBot/blob/main/docs/SECURITY.md)

## 用这个样例创建你自己的插件

```bash
# 方式一：脚手架（推荐）
easybot plugin new my-adapter

# 方式二：照抄本仓库改名
#   1. 全局替换 easybot-hello-adapter → my-adapter、HelloAdapter → MyAdapter
#   2. Cargo.toml 改 package.name / SDK tag
#   3. src/lib.rs 的 platform_name() 与 plugin.yaml 的 name 改成 my-adapter
#   4. 按指南第 9 节清单补真实平台逻辑
```

## 发布

见 [插件开发指南 → 发布](docs/plugin-development-guide.md#10-发布)。签名/信任/安装语义见主仓 [`docs/SECURITY.md`](https://github.com/EasyIndie/EasyBot/blob/main/docs/SECURITY.md)。

---

**License**: GPL-3.0 · **作者**: EasyBot Contributors
