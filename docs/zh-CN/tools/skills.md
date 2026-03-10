---
read_when:
  - 添加或修改 Skills
  - 更改 Skills 门控或加载规则
summary: Skills 的标准目录、优先级、门控规则，以及 CodexPlusClaw 的执行模型。
title: Skills
x-i18n:
  generated_at: "2026-03-11T00:00:00Z"
  model: gpt-5
  provider: openai
  source_path: tools/skills.md
---

# Skills（OpenClaw）

CodexPlusClaw 中，**Skills 是 Codex 原生能力**。OpenClaw 负责发现、同步、
配置和展示；Codex 负责真正执行这些 skills。

## 目录与优先级

Skills 从三个位置加载：

1. 内置 skills
2. 共享/托管 skills：`~/.agents/skills`
3. 工作区 skills：`<workspace>/.agents/skills`

名称冲突时优先级如下：

`<workspace>/.agents/skills` → `~/.agents/skills` → 内置 skills

额外目录可以通过 `skills.load.extraDirs` 添加，优先级最低。

## CodexPlusClaw 执行模型

- OpenClaw 负责技能发现、同步、门控、配置注入和 UI。
- Codex 从标准目录读取 skills。
- 当 OpenClaw 主动引导 Codex 使用某个 skill 时，它可以同时发送
  `$skill-name` 文本标记和匹配的 skill 输入项路径。

也就是说，skills 不是被伪装成动态工具；它们仍然是 Codex 原生 skills。

## 单智能体与共享 skills

- 单智能体 skills：`<workspace>/.agents/skills`
- 共享 skills：`~/.agents/skills`

OpenClaw 仍会兼容旧的 `<workspace>/skills` 和 `~/.openclaw/skills`
布局，但这些都只是迁移兼容层，不是标准布局。

## ClawHub

`clawhub` 默认安装到当前目录下的 `./skills`。CodexPlusClaw 会把它同步到
标准的工作区 skill 根目录 `<workspace>/.agents/skills`，因此 OpenClaw 和
Codex 能看到同一份 skills。

更多内容参见：[ClawHub](/tools/clawhub)

## 格式

`SKILL.md` 至少需要：

```markdown
---
name: hello-world
description: Greet the user and confirm the skill wiring works.
---
```

OpenClaw 遵循 AgentSkills 的目录约定，并在加载时根据 `metadata.openclaw`
中的 `requires.bins`、`requires.env`、`requires.config` 等规则做门控。

## 共享配置

`~/.openclaw/openclaw.json` 中的 `skills.entries` 仍然用于：

- 启用或禁用某个 skill
- 注入 `env`
- 注入 `apiKey`
- 存储每个 skill 的额外配置

## 最后的规则

标准结论很简单：

- `~/.agents/skills` 是共享 skills 的标准位置
- `<workspace>/.agents/skills` 是工作区 skills 的标准位置
- 旧的 `~/.openclaw/skills` 与 `<workspace>/skills` 只用于迁移兼容
