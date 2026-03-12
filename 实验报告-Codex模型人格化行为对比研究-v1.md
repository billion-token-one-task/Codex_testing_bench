# Codex 模型人格化行为对比研究实验报告 v1

## 0. 报告定位

这份报告不是最终论文，而是当前代码库内已有实验产物的系统性研究总结。它服务于当前研究主线：

- 研究对象始终是：
  - 模型本身
  - Codex harness 机制
  - 两者的交互效应
- 研究问题不是“某个 benchmark 多解出几题”，而是：
  - `gpt-5.4` 是否比 `gpt-5.3-codex` 更倾向于“说更多”
  - 这些额外输出到底是什么
  - 这些输出如何与工具调用、验证、patch 行为以及 Codex 自身机制耦合
  - `friendly` 和 `pragmatic` 是不是只改 tone，还是已经开始改变 agent policy

本报告基于当前仓库中两批最相关实验：

- 单题 `2x2` 先导实验：
  - [artifacts/swebench-study-2026-03-11T15-35-40Z-4c8c79a3](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-11T15-35-40Z-4c8c79a3)
- `5` 题 `2x2` 主批次实验：
  - [artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3)

本轮结论以 `5` 题 `2x2` 主批次为主，以单题先导作方向性补充。

---

## 1. 当前研究假设

当前代码库中的正式假设定义见：

- [studies/hypotheses/model-behavior-v1.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/hypotheses/model-behavior-v1.json)

核心假设为：

- `H1`：`gpt-5.4` 的用户可见输出总体上多于 `gpt-5.3-codex`
- `H2`：`gpt-5.4` 多出来的输出，更偏任务桥接语、验证 framing、决策解释，而不是纯礼貌包装
- `H3`：`gpt-5.4` 的“说更多”与工具调用存在更强耦合，而不是单纯 verbosity
- `H4`：`friendly` 会放大可观察状态外显化，`pragmatic` 更接近压缩执行风格
- `H5`：这些差异并不纯粹来自 base model，而是和 Codex 的 instruction layering、tool mediation、config freeze、compaction/reconstruction 等 harness 机制共同作用

---

## 2. 数据来源与方法

### 2.1 实验矩阵

本轮主批次使用固定 `2x2` 条件：

- `gpt-5.3-codex × pragmatic`
- `gpt-5.3-codex × friendly`
- `gpt-5.4 × pragmatic`
- `gpt-5.4 × friendly`

任务集为 `5` 道 SWE-bench Verified 样本题，每题在四个 cohort 下配对运行，因此主批次总计 `20` 个 runs。

### 2.2 主批次产物

主批次核心报告：

- [report.txt](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/report.txt)
- [model-comparison.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/model-comparison.md)
- [verbosity-analysis.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/verbosity-analysis.md)
- [tool-language-coupling.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/tool-language-coupling.md)
- [phrase-and-tone-analysis.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/phrase-and-tone-analysis.md)
- [tool-inventory.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/tool-inventory.md)
- [personality-mechanism-analysis.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/personality-mechanism-analysis.md)
- [cohort-pair-analysis.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/cohort-pair-analysis.md)

主批次核心数据集：

- [campaign_runs.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/campaign_runs.csv)
- [message_metrics.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/message_metrics.csv)
- [message_style.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/message_style.csv)
- [message_discourse_summary.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/message_discourse_summary.csv)
- [tool_inventory.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/tool_inventory.csv)
- [verbosity_tool_coupling.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/verbosity_tool_coupling.csv)
- [model_pair_deltas.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/model_pair_deltas.csv)
- [model_phrase_deltas.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/model_phrase_deltas.csv)
- [personality_phrase_deltas.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/personality_phrase_deltas.csv)
- [personality_mechanism.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/personality_mechanism.csv)

### 2.3 方法学约束

本轮所有结论都只针对：

- **用户可见输出**
- **Codex 原生事件和其可稳定推导出的行为指标**

不对 hidden CoT 作强结论。

同时必须明确：

- 本轮主批次**没有 official grading**
- 因此这里讨论的是**行为差异**
- 不是 solve rate 或 official resolved rate 差异

---

## 3. 主批次总体概况

根据 [report.txt](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/report.txt)，主批次情况如下：

- `20/20` runs 全部完成求解阶段
- 总 input tokens：`13,924,471`
- 总 output tokens：`173,037`
- 总 cache-read tokens：`12,721,280`
- 总 commands：`1,166`
- 总 tools：`1,234`
- 总 message metrics：`202`
- 总 skill events：`37`
- anomaly：`4`

任务类覆盖是完整的：

- `bootstrap-heavy`
- `compaction-likely`
- `patch-heavy`
- `search-heavy`
- `verification-heavy`

这意味着本轮不是只在单一任务类型上比较“话多不多”，而是在多种任务压力面上观察行为差异。

---

## 4. 核心发现概览

如果先给一句总括性结论，我会这样写：

> 当前证据整体支持 `gpt-5.4` 比 `gpt-5.3-codex` 更倾向于外显中间状态，并且这种外显不是纯礼貌包装；它和更高的工具调用密度、更多的规划/验证/决策解释语言同时出现。`friendly` 进一步放大这种外显化，但放大的不是单一的“温柔语气”，而是带有明显任务桥接和过程说明成分的 agent 行为风格。

更细一点说：

- `H1`：**强支持**
- `H2`：**中强支持**
- `H3`：**支持，但具有任务依赖性**
- `H4`：**部分支持，且不是简单的“friendly=字更多、pragmatic=字更少”**
- `H5`：**部分支持**
  - 可以确认 harness 机制稳定存在并参与行为表达
  - 但当前批次不足以把机制贡献定量分解到足够强的程度

---

## 5. 对 H1 的结论：`gpt-5.4` 是否更会“说”

### 5.1 主批次 cohort 均值

基于 [campaign_runs.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/campaign_runs.csv) 的配对样本聚合：

| Cohort | 平均 visible output tokens | 平均 tool count | 平均 command count | 平均 total tokens |
|---|---:|---:|---:|---:|
| `gpt-5.3-codex-pragmatic` | 547.6 | 49.6 | 47.6 | 611,491.4 |
| `gpt-5.3-codex-friendly` | 656.2 | 58.8 | 56.0 | 760,830.0 |
| `gpt-5.4-pragmatic` | 867.2 | 60.8 | 57.2 | 642,708.6 |
| `gpt-5.4-friendly` | 950.2 | 77.6 | 72.4 | 804,471.6 |

直接解释：

- `5.4` 在两种 personality 下都比 `5.3-codex` 更“会说”
- `friendly` 在两代模型上都进一步抬高了可见输出
- 主批次里，**最会说的是 `gpt-5.4-friendly`**

### 5.2 配对差值

同一题目的平均 cohort 差值：

| 比较 | 平均 visible output delta | 平均 tool delta | 平均 command delta | 平均 total tokens delta |
|---|---:|---:|---:|---:|
| `5.4-friendly - 5.3-friendly` | +294.0 | +18.8 | +16.4 | +43,641.6 |
| `5.4-pragmatic - 5.3-pragmatic` | +319.6 | +11.2 | +9.6 | +31,217.2 |
| `5.4-friendly - 5.4-pragmatic` | +83.0 | +16.8 | +15.2 | +161,763.0 |
| `5.3-friendly - 5.3-pragmatic` | +108.6 | +9.2 | +8.4 | +149,338.6 |

这对 `H1` 是很干净的支持：

- 不管是 `friendly` 还是 `pragmatic`
- `5.4` 都比 `5.3-codex` 多说
- 而且不是只有一题在拉动这个结果，而是 paired 平均都同向

### 5.3 单题 2x2 先导的补充

单题 `django__django-15525` 先导实验中，visible output 是：

- `5.3-pragmatic`: `727`
- `5.3-friendly`: `1027`
- `5.4-pragmatic`: `2310`
- `5.4-friendly`: `1612`

见：

- [先导 model-comparison.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-11T15-35-40Z-4c8c79a3/reports/model-comparison.md)

这个先导实验有个很有价值的提示：

- 在单题上，`5.4-pragmatic` 可以比 `5.4-friendly` 还更 verbose

而在 `5` 题主批次里，平均上是 `5.4-friendly > 5.4-pragmatic`。  
这说明：

> `friendly` 与 `pragmatic` 的作用不是简单的固定“字数放大器/压缩器”，而是和具体任务类型、任务阶段以及 Codex 行为策略一起作用。

**H1 结论：强支持。**

---

## 6. 对 H2 的结论：`5.4` 多出来的到底是什么

### 6.1 不是纯社交包装

看 [phrase-and-tone-analysis.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/phrase-and-tone-analysis.md)，各 cohort 的主要 discourse 统计如下：

| Cohort | social_tone | verification | tool_bridge | planning | decision_explanation |
|---|---:|---:|---:|---:|---:|
| `5.3-pragmatic` | 8 | 40 | 16 | 31 | 12 |
| `5.3-friendly` | 23 | 47 | 27 | 35 | 19 |
| `5.4-pragmatic` | 10 | 54 | 27 | 48 | 27 |
| `5.4-friendly` | 26 | 55 | 31 | 55 | 52 |

这个表很重要。它说明 `5.4` 多出来的部分，不是主要落在 `social_tone`：

- `5.4-friendly` 比 `5.3-friendly` 的 `social_tone` 只多了 `3`
- 但：
  - `planning` 多了 `20`
  - `decision_explanation` 多了 `33`
  - `verification` 多了 `8`
  - `tool_bridge` 多了 `4`

对 `5.4-pragmatic` 也是一样：

- `social_tone` 只从 `8` 到 `10`
- 但：
  - `planning` 从 `31` 到 `48`
  - `decision_explanation` 从 `12` 到 `27`
  - `tool_bridge` 从 `16` 到 `27`
  - `verification` 从 `40` 到 `54`

这与 `H2` 是明显同向的。

### 6.2 5.4 更像“状态外显 + 决策解释”增强

从 [message_style.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/message_style.csv) 聚合后可以看到：

- `5.4` 两个 cohort 的 message 数都更多
- `orientation / planning / decision_explanation / task_restatement` 这些类别在 `5.4` 上明显更高

这更像是：

- 更频繁地把当前状态说出来
- 更频繁地说明接下来要做什么
- 更频繁地解释为什么这样做

而不是单纯增加礼貌开场和情绪性包装。

### 6.3 词汇层面的佐证

从 [model_phrase_deltas.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/model_phrase_deltas.csv) 的高差值项看，`5.4` 相比 `5.3` 更常出现：

- `i’m`
- `verif`
- `test`
- `edit`
- `check`
- `pytest`

这些词本身不是结论，但方向上是吻合的：

- `i’m` 更像 process verbalization 的主体化表达
- `verif / test / check / pytest` 更像验证过程显化
- `edit` 更像动作与改动过程显化

同时要诚实说明：

- 当前词项是本地启发式 NLP 的 stem/term 结果
- 存在噪声
- 例如个别 personality phrase delta 里会出现 `download friend`、`workspac`、`python` 这类明显带路径或 tokenization 噪声的项

所以：

- **phrase 结果可作为方向性证据**
- **不能拿单个 term 当强结论**

**H2 结论：中强支持。**

更准确地说：

> 5.4 多出来的输出，主成分更像规划、验证 framing、任务桥接与决策解释，而不是简单的礼貌包装。

---

## 7. 对 H3 的结论：`5.4` 的“说更多”是否和工具调用更强耦合

### 7.1 工具密度同步上升

先看 cohort 均值：

- `5.3-pragmatic`：平均 `49.6` 个 tools
- `5.3-friendly`：平均 `58.8`
- `5.4-pragmatic`：平均 `60.8`
- `5.4-friendly`：平均 `77.6`

这已经说明：

- `5.4` 不只是多说
- 同时也更多地调用工具

### 7.2 语言-工具耦合的直接证据

看 [tool-language-coupling.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/tool-language-coupling.md)：

例如 `pytest-dev__pytest-6202`：

- `5.3-pragmatic`：`tool_burst=17`, `micro_narrated=6`
- `5.3-friendly`：`tool_burst=24`, `micro_narrated=6`
- `5.4-pragmatic`：`tool_burst=36`, `micro_narrated=13`
- `5.4-friendly`：`tool_burst=41`, `micro_narrated=13`

例如 `pydata__xarray-4629`：

- `5.3-pragmatic`：`tool_burst=9`, `micro_narrated=5`
- `5.3-friendly`：`17`, `9`
- `5.4-pragmatic`：`21`, `9`
- `5.4-friendly`：`25`, `10`

这说明：

- `5.4` 不只是 message 数高
- 它的 tool burst 也更高
- 而且 `micro-narrated tool burst` 往往也更高

换句话说：

> 5.4 多出来的语言，不只是“悬空地多说”，而是更多地嵌入在工具-动作-解释的链路中。

### 7.3 但这种耦合不是单一模式

同时也要避免过度简化：

例如 `django__django-11400`：

- `5.4-pragmatic` 的 `tool_burst_count=44`
- 但 `silent_tool_burst_count=30`

这意味着：

- `5.4` 不是“每次操作都多说一段”
- 更像是：
  - 总体工具更密
  - 其中一部分 burst 带更多桥接
  - 另一部分仍然是高密 silent execution

所以更准确的表述是：

> `5.4` 的“说更多”与工具调用存在更强耦合，但这种耦合是混合型的：既有更多微叙述化工具链，也保留了相当部分沉默 burst。

**H3 结论：支持，但明显受任务类型影响。**

---

## 8. 对 H4 的结论：`friendly` 是否放大状态外显化

### 8.1 `friendly` 确实通常会放大可见输出

从主批次 paired delta：

- `friendly_vs_prag_5.4`：visible output `+83.0`
- `friendly_vs_prag_5.3`：visible output `+108.6`

这说明总体上：

- `friendly` 确实更“会说”

### 8.2 但 `friendly` 的作用不只是字数

对 `5.4`：

- `5.4-friendly` 平均 visible output 比 `5.4-pragmatic` 高
- 同时工具数也更高：
  - `+16.8 tools`
  - `+15.2 commands`

对 `5.3`：

- `friendly` 同样会同时抬升：
  - visible output
  - tool count
  - command count

这说明 `friendly` 并不只是“说话更温柔”，而是在改变 agent 的操作节奏和外显程度。

### 8.3 但 `pragmatic` 不是简单的“少说”

单题先导实验就是一个重要反例：

- `django__django-15525`
- `5.4-pragmatic = 2310`
- `5.4-friendly = 1612`

也就是说：

- 在某些题上，`pragmatic` 不但不更短，反而可能更长

因此对 `H4`，最合理的表达是：

> `friendly` 整体上会放大状态外显化与社交/协作性表达，但 `pragmatic` 并不是一个简单的“少说模式”；它更像是可能在某些任务里把 verbosity 重新分配到更高密度的执行、检查和推进语言上。

### 8.4 社交 tone 并不是全部

从 cohort discourse counts 看：

- `5.4-friendly` 的 `social_tone` 确实高于 `5.4-pragmatic`
- 但 `planning / decision_explanation / tool_bridge / verification` 也一起上升

所以：

- `friendly` 放大的不是单一礼貌 tone
- 更像是**一种更愿意把 agent 内部状态显性化的风格**

**H4 结论：部分支持，而且要避免把 personality 误解成纯 cosmetic。**

---

## 9. 对 H5 的结论：差异是否来自 harness 交互，而不只是 base model

### 9.1 目前能确认的部分

从 [personality-mechanism-analysis.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/personality-mechanism-analysis.md) 和 [report.txt](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/report.txt) 可以确认：

- `personality_fallbacks = 0`
- `personality_mismatches = 0`
- `model_native_preserved_rows = 1`
- `instruction_shift_count = 3`
- `config_drift_count = 1`

这几件事说明：

- personality 机制在这批实验里是**稳定生效**的
- 不是“friendly 没真的上到模型，只是我们以为上了”
- Codex harness 确实存在：
  - config freeze
  - instruction layering / shift
  - tool mediation

### 9.2 tool mediation 也是实打实存在的

从 [tool-inventory.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/tool-inventory.md) 可见：

- shell
- apply_patch
- MCP (`tavily::tavily_search`)

都出现在真实 run 里，而且 route 数是有差异的。

这说明当前观察到的行为不是“裸模型文本输出”，而是在一个有明确 tool mediation 的 harness 中表达出来的。

### 9.3 但当前还不能把 harness 贡献定量分解得很强

这里需要诚实：

- 当前批次里 `compaction_count = 0`
- 所以我们**没有**看到 compaction / reconstruction 真正被触发
- 因此关于：
  - `compaction`
  - `history rebuild`
  - `post-compaction rediscovery`

这些机制，本轮只能说：

- 它们是 Codex 架构中存在的研究对象
- 但本轮任务没有把它们真正激活到可判读程度

### 9.4 更稳妥的 H5 结论

所以对 `H5`，我认为最稳妥的表述是：

> 当前证据支持“差异不是纯 base model 的裸文本差异，而是发生在一个稳定存在的 Codex harness 机制底座之上”；但本轮还不足以把“模型贡献”和“具体 harness 子机制贡献”做强定量拆分，尤其是 compaction/reconstruction 这条链路尚未被有效激活。

**H5 结论：部分支持。**

---

## 10. 目前最稳的综合发现

如果把 H1-H5 合起来，当前最稳的发现是：

### 发现 1
`gpt-5.4` 在用户可见输出上**稳定比** `gpt-5.3-codex` 更 verbose。

### 发现 2
`5.4` 多出来的内容，不主要是“礼貌包装”，而更像：

- planning
- task restatement
- decision explanation
- verification framing
- tool bridge

### 发现 3
`5.4` 的 extra output 和 extra tooling 是一起长出来的。

也就是说：

- 它不是单纯更啰嗦
- 它更像把任务推进过程更频繁地外显出来

### 发现 4
`friendly` 不是一个简单的 surface tone toggle。

当前证据更像说明：

- `friendly` 会放大可见状态外显化
- 同时也常常伴随着更多 tool / command
- 这更像是一个**行为分布调制器**

### 发现 5
Codex harness 不是背景噪声，而是研究对象的一部分。

当前证据已经能确认：

- config freeze
- instruction stratification
- tool mediation
- personality 生效链条

都是真实存在并参与行为表达的。

因此你的原始研究立场——

> 不能把脚手架和模型割裂开来研究

——在当前实验里是被支持的。

---

## 11. 当前不应该过度声称的部分

为了让后续论文站得住，这些点我建议现在不要过度说满。

### 11.1 不应直接声称“看到了更多 CoT”

当前我们看到的是：

- 可见输出
- 可见桥接语
- 可见决策解释
- 可见验证 framing

这足以支持“state verbalization tendency”之类的说法，  
但**不足以**支持“5.4 释放了更多 hidden CoT”这样的强结论。

### 11.2 `actionable_commentary_ratio` 目前过于理想化

当前很多 run 都是：

- `actionable_commentary_ratio_bps = 10000`

这明显说明当前 heuristic 太宽松了。  
所以这个指标：

- 可以作为弱参考
- 不应成为核心支撑证据

### 11.3 `apply_patch` success/failure 目前不可靠

在 [tool-inventory.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/tool-inventory.md) 里：

- apply_patch 全部表现为 failures

但这与实际 run 完成并产出 patch 的事实并不一致。  
更合理的解释是：

- 当前 `apply_patch` success 语义映射还有问题

所以：

- apply_patch 的**次数**可用
- apply_patch 的**成功/失败**暂时不应用于强推断

### 11.4 phrase delta 有 tokenization 噪声

例如：

- `download friend`
- `workspac`
- `python`

这些说明 phrase mining 还存在路径/词干噪声。  
因此：

- phrase 分析适合看方向
- 不适合拿单个 term 做“解释学级别”的强结论

### 11.5 本轮没有 official grading

所以当前不能说：

- `5.4` 更会解题
- `friendly` 更高 resolved

本轮只能说：

- `5.4 / 5.3-codex` 的**行为模式**不同

---

## 12. 对论文方向最有价值的结论提炼

如果现在要把这轮实验转成论文里的一组“可写结论”，我认为最有价值的是下面这组。

### 12.1 第一层：模型行为层

`gpt-5.4` 相比 `gpt-5.3-codex`，表现出更强的：

- 可见输出倾向
- 规划/验证/决策解释输出
- 工具调用密度
- 工具-语言耦合强度

### 12.2 第二层：personality 层

`friendly` 与 `pragmatic` 的差异不是纯 cosmetic：

- `friendly` 倾向于放大可见状态外显化
- 但 `pragmatic` 并不是简单“少说”
- `pragmatic` 有时会保留甚至放大高密度执行导向叙述

### 12.3 第三层：harness 层

这些差异是在 Codex harness 的真实机制里表达出来的：

- session/config freeze
- instruction layering
- tool mediation
- personality 生效链

因此，模型行为与脚手架机制不能被切开看待。

### 12.4 第四层：对你们 thesis 的对齐

当前结果已经和你们 thesis 的方向形成了比较清晰的呼应：

- 单次上下文窗口不是唯一研究对象
- 更重要的是状态如何被外显、分层、桥接、交接
- 可见输出本身可能是 agentic coordination 的组成部分，而不是无意义冗余
- 模型代际与 personality 差异，会改变这种“外显—动作—验证”耦合模式

---

## 13. 下一步实验建议

### 13.1 第一优先级
补 official grading，并把：

- 行为差异
- official resolved/unresolved
- grader/harness failure

三者彻底拆开。

### 13.2 第二优先级
扩大 paired sample，从 `5` 题提升到 `10-15` 题，并保持：

- `2x2`
- task class 全覆盖

### 13.3 第三优先级
专门构造更容易触发：

- compaction
- reconstruction
- long-horizon continuity

的任务，以便真正验证 H5 里关于 compaction/rebuild 的部分。

### 13.4 第四优先级
继续改进 NLP：

- 收紧 `actionable_commentary_ratio`
- 清理 phrase delta 的 tokenization 噪声
- 把 state externalization / bridge / verification 这三类语言做得更稳

---

## 14. 本报告的最终判断

截至当前，我对这组假设的判断如下：

| 假设 | 当前判断 | 说明 |
|---|---|---|
| `H1` | 强支持 | `5.4` 在两种 personality 下都比 `5.3-codex` 更 verbose |
| `H2` | 中强支持 | 额外输出更偏 planning / verification / decision explanation / tool bridge，而不是纯礼貌包装 |
| `H3` | 支持 | `5.4` 的 verbosity 与更高工具密度、更多 tool burst 一起出现，但耦合形式受任务影响 |
| `H4` | 部分支持 | `friendly` 会放大外显化，但 `pragmatic` 不是简单的“少说模式” |
| `H5` | 部分支持 | harness 机制稳定存在并参与表达，但当前还不足以把具体机制贡献强定量分解 |

如果只保留一句最重要的话，我会写：

> 当前证据最支持的不是“5.4 更爱聊天”，而是“5.4 更倾向于把任务推进过程外显成可观察语言，并且这种外显与更多工具调用、更多验证 framing 和更多决策解释同时出现”；而 `friendly / pragmatic` 的区别也不只是表面语气，而是在 Codex harness 里改变了 agent 的可观察工作风格。

---

## 15. 关键引用路径

### 主批次

- [主批次 campaign 根目录](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3)
- [主批次总报告](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/report.txt)
- [主批次模型对比](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/model-comparison.md)
- [主批次语言-工具耦合](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/tool-language-coupling.md)
- [主批次短语与语气分析](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/phrase-and-tone-analysis.md)
- [主批次 personality 机制](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/reports/personality-mechanism-analysis.md)

### 主批次数据集

- [campaign_runs.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/campaign_runs.csv)
- [message_style.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/message_style.csv)
- [message_discourse_summary.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/message_discourse_summary.csv)
- [tool_inventory.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/tool_inventory.csv)
- [model_pair_deltas.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/model_pair_deltas.csv)
- [model_phrase_deltas.csv](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-12T11-22-46Z-6a2660a3/datasets/model_phrase_deltas.csv)

### 单题先导

- [单题先导 campaign](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-11T15-35-40Z-4c8c79a3)
- [单题先导模型对比](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-11T15-35-40Z-4c8c79a3/reports/model-comparison.md)
- [单题先导工具耦合](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/swebench-study-2026-03-11T15-35-40Z-4c8c79a3/reports/tool-language-coupling.md)

