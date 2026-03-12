# 研究控制台升级路线图

## 目标

把当前本地研究控制台升级成真正和 Codex harness 紧耦合的战情台，而不是只会读 artifact 的轻量浏览器。

这条路线重点解决四类问题：

- 实时性不够真
- 和实际 harness 运行绑得不够深
- 页面信息架构还不够像指挥台
- 研究工作流还没有完全内建进 UI

## 当前重点

### 1. 真实时数据总线

- 把 `workspace` 轮询升级成 `artifact append + process output + run event` 的统一流式总线
- 让 `message / tool / patch / mechanism / token` 都成为一等 SSE 事件
- 为活跃 run 维护 `LiveRunSnapshot`

### 2. Live 页变成真正的 mission control

- 当前主战场与历史 spillover run 分离
- 并行槽位、焦点 run、全局告警、控制面健康统一呈现
- 让 operator 一眼看到“现在正在发生什么”

### 3. Run Detail 变成 war room

- 左：message / timeline
- 中：tool / patch / command
- 右：mechanism / token pressure / evidence dock
- 明确区分 raw truth、normalized evidence、inferred summaries

### 4. Compare 变成 2x2 论文工作台

- 同题四格矩阵
- model delta / personality delta / mechanism delta
- 直接跳转对应 run war room
- 不只是看 CSV，而是回答研究问题

### 5. Research 变成 hypothesis / evidence / methods 中枢

- hypothesis 状态
- claim evidence 覆盖度
- observed / inferred / estimated 观测层
- task-class / personality / instruction / patch / skill 机制视角

## 下一阶段

### Phase A

- 修稳 control plane 的 live snapshot 与 SSE 水合
- 清理空态、假态、错误态
- 明确当前 campaign 与历史残留 run 的边界

### Phase B

- 做强 Run Detail 多轨联动
- 增加 patch chain / tool route / mechanism rail 的 drilldown
- 把 message / tool / patch / token 之间的时序关系显出来

### Phase C

- 把 Compare 做成真正的 2x2 研究工作台
- 增加 phrase delta、bridge language、verification framing、state externalization 的对比视图

### Phase D

- 把 Research 页和 hypothesis / claim / methods 文档完全打通
- 支持从 UI 直接跳到 observability contract、artifact contract、研究方法说明

## 设计原则

- 不做通用 SaaS dashboard
- 要像“编辑部 + 指挥台 + 运维控制面”
- 结构化 panel、ledger、rail、signal strip 优先
- 颜色只服务于状态、压力、验证、异常、权威信号
- 一切以研究证据和现场判断为中心
