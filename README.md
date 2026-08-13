# easybot-hello-adapter

EasyBot 官方插件**入门样例**（echo 适配器）——一个最小可跑的插件雏形，也是插件开发的教学对照物。

> 插件命名规则：官方插件统一 `easybot-xxx` 前缀，且同一个名字贯穿仓库名 / package name / cdylib 产物 / `plugin.yaml` name / `platform_name()`。

## 这是什么

- **最小可跑**：完整实现 `PlatformAdapter` 的 echo 适配器，加载即 Connected，`send()` 回显事件。
- **教学对照物**：开发全过程（逐文件讲解 + 踩坑记录 + 调试过程）沉淀在 [**插件开发指南**](docs/plugin-development-guide.md)。
- **自包含**：SDK 走 git tag 依赖，作者无需 clone 主仓；`cargo test` 全部离线（PluginTestHost）。

## 快速开始

```bash
cargo test                     # 单元 + PluginTestHost 离线测试
cargo build --release          # 产出自包含 cdylib
```

手动装入宿主（dev 联调）：

```bash
mkdir -p ~/.easybot/plugins/easybot-hello-adapter
cp target/release/libeasybot_hello_adapter.{so,dylib,dll} ~/.easybot/plugins/easybot-hello-adapter/
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
