# Benchmark 参考资料

## 当前活跃的本地 benchmark 资源

- SWE-bench Verified
  - 本地数据快照：`/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/swebench-verified/verified-test.jsonl`
  - 上游数据集：[princeton-nlp/SWE-bench_Verified](https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified)
  - 在本 bench 中的角色：在真实代码仓库里做 patch 修复，并接受强回归验证

- NL2RepoBench
  - 本地 vendor 路径：`/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/NL2RepoBench`
  - 上游：[NL2RepoBench](https://github.com/THUDM/NL2RepoBench)
  - 在本 bench 中的角色：端到端仓库交付、从 0 到 1 的软件工程构建

- NewtonBench
  - 本地 vendor 路径：`/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/NewtonBench`
  - 上游：[NewtonBench](https://github.com/HKUST-KnowComp/NewtonBench)
  - 在本 bench 中的角色：交互式科学发现与实验推理

## 为什么选这三个

- SWE-bench 强调在已有代码库中定位、修复并验证 patch
- NL2RepoBench 强调从自然语言规格出发构建完整仓库
- NewtonBench 强调探索式实验、规律归纳和多步假设修正

三者放在一起，给 Codex research bench 提供了三种互补的观察场景：

- 在现有代码库里补丁修复
- 从零搭建完整软件系统
- 在实验环境里发现潜在规律

## 从 Adapter 视角看

当前 bench 中的 adapter 名称包括：

- `swebench`
- `nl2repo`
- `newtonbench`
- `repo-patch-jsonl`（作为通用复用层）

这意味着 benchmark 文档不只是 benchmark 参考表，同时也是 **adapter 体系的一部分说明**。
