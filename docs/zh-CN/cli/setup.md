---
read_when:
  - 你在不使用完整新手引导向导的情况下进行首次设置
  - 你想设置默认工作区路径
summary: "`openclaw setup` 的 CLI 参考（一键设置、本地 Codex 引导、工作区与修复）"
title: setup
x-i18n:
  generated_at: "2026-02-01T20:21:26Z"
  model: claude-opus-4-5
  provider: pi
  source_hash: 7f3fc8b246924edf48501785be2c0d356bd31bfbb133e75a139a5ee41dbf57f4
  source_path: cli/setup.md
  workflow: 14
---

# `openclaw setup`

初始化 `~/.openclaw/openclaw.json`、本地 Gateway、Codex 兼容工作区和一键设置流程。

相关内容：

- 快速开始：[快速开始](/start/getting-started)
- 向导：[新手引导](/start/onboarding)

## 示例

```bash
openclaw setup
openclaw setup --one-click
openclaw setup --workspace ~/.openclaw/workspace
```

推荐的本地路径：

```bash
openclaw setup --one-click
```

这会安装或升级兼容的 Codex CLI，准备 `~/.agents/skills` 和工作区
`.agents/skills` 目录，配置本地 Gateway/服务，并在通过健康检查后打开
Control UI。

通过 setup 运行向导：

```bash
openclaw setup --wizard
```
