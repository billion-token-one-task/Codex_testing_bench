# Codex 参考资料

这个 bench 的 grounding 来自两部分：

- 本地 vendored 的 Codex 源码：[repos/codex](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex)
- 一小组外部架构参考资料，用来帮助设计 probe

## 主要外部参考

- [DeepWiki Codex](https://deepwiki.com/openai/codex)
- [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)
- [OpenAI: Introducing upgrades to Codex](https://openai.com/index/introducing-upgrades-to-codex/)
- [OpenAI: Introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)
- [OpenAI: Introducing Codex](https://openai.com/index/introducing-codex/)

## 如何正确使用这些参考

这些材料是 **架构参考**，不是本地源码的替代品。

在当前研究工作流里，它们最重要的作用之一，是帮助我们建立：

- [Codex 可观测面契约](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md)
- [结构化 observability map](/Users/kevinlin/Downloads/CodexPlusClaw/studies/observability/codex-observability-map.json)

它们适合用于：

- 快速建立对 App Server 概念的理解
- 定位 compaction、session lifecycle、tool orchestration 等可能的 seam
- 粗粒度交叉核对一个 probe 方向是否与公开架构描述相符

它们不适合用于：

- 覆盖本地 vendored 源码的真实实现
- 默认假设当前 pinned runtime 与公开描述完全一致
- 在没有本地 artifact 的情况下直接宣称行为结论

所以当前仓库的推荐顺序是：

1. 先读本地 artifact
2. 再看本地 vendored 源码
3. 最后用这些外部资料帮助解释 seam 和 probe 设计

## 为什么把 DeepWiki 放进仓库文档

DeepWiki 是目前最快的高层架构导览之一，尤其适合帮助理解：

- session lifecycle
- App Server request / response 结构
- tool orchestration
- context management 与 compaction
- persistence / resume 结构

所以它很适合在设计新 probe，或者在生成 `codex-architecture-map.json` 时做高层映射。

## Bench 的判定优先级

当以下三者发生张力时：

- 外部文档
- 本地 vendored 源码
- 实际 run artifact

优先级应当是：

1. 本地真实 run artifact
2. 本地 vendored 源码
3. 外部参考文档

## 这些参考如何映射到 Bench

bench 会用这些参考来塑造：

- architecture map
- raw probe family
- claim catalog
- `report.txt` 中 “Codex under observation” 一类章节

当前重点研究的 local seam 包括：

- session/config freeze
- model/personality 支持与 instruction 注入
- instruction 与 prompt assembly
- turn lifecycle dispatch
- context compaction 与 reconstruction
- tool mediation
- persistence / resume
- App Server translation / listener path
- reliability surface

并且这些 seam 现在都已经被明确映射进：

- [codex-observability-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md)
- [codex-observability-map.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/observability/codex-observability-map.json)

## 与内部研究方向的关系

这个 bench 不是为了“证明参考资料正确”。

它是为了借这些资料提出更好的问题，例如：

- Codex 是否比扁平 transcript 更像 layered state 系统
- compaction 是在保留 actionable state，还是在制造 rediscovery loop
- Codex 显现出来的行为里，有多少是模型驱动的，有多少是 harness 中介出来的
- Codex 架构在哪些地方支持、在哪些地方偏离长程 token 调度的研究直觉
