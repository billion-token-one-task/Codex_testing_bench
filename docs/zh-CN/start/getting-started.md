---
read_when:
  - 从零开始首次设置
  - 想要最快获得第一个可用聊天
summary: 使用一键引导完成 OpenClaw + Codex 的本地启动。
title: 入门指南
---

# 入门指南

目标：尽快从零到第一个可用聊天。

<Info>
中文文档的这一页已更新到 CodexPlusClaw 路径。更深层的高级细节若有缺失，请以英文文档为准。
</Info>

## 推荐路径

```bash
openclaw setup --one-click
```

它会完成：

- 安装或升级兼容的 Codex CLI
- 准备 OpenClaw 配置、工作区与 Skills 目录
- 完成 Codex 认证
- 安装本地 Gateway 服务
- 运行健康检查
- 打开 Control UI

## 前置条件

- Node 22 或更高
- macOS、Linux，或 Windows + WSL2

## 安装 CLI

```bash
curl -fsSL https://openclaw.ai/install.sh | bash
```

或：

```bash
npm install -g openclaw@latest
```

## 启动 Gateway 与控制界面

如果你使用了一键引导，Gateway 通常已经可用：

```bash
openclaw gateway status
openclaw dashboard
```

## 可选：连接渠道

```bash
openclaw channels login
```

## 何时使用手动向导

如果你需要远程 Gateway、精细化认证选择或逐步手动设置，请改用：

```bash
openclaw onboard
```
