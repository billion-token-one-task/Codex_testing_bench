import { NavLink } from "react-router-dom";
import type { PropsWithChildren } from "react";

import {
  useActiveRuns,
  useEventStreamStatus,
  useLiveRuns,
  useLatestRunningProcesses,
  useProcesses,
  useRecentEventLines,
  useRecentEventTypes,
  useWorkspaceIndex,
} from "../lib/store";
import { formatCompact, formatDate, formatRelativeTime, truncateMiddle } from "../lib/format";
import { ActionLauncher } from "./ActionLauncher";
import { EventRail } from "./EventRail";
import { RunCard } from "./RunCard";
import { StatusBadge } from "./StatusBadge";

const navItems = [
  ["Campaigns", "/campaigns", "CP"],
  ["Live", "/live", "LV"],
  ["Runs", "/runs", "RN"],
  ["Compare", "/compare", "CM"],
  ["Artifacts", "/artifacts", "AR"],
  ["Research", "/research", "RS"],
];

export function Shell({ children }: PropsWithChildren) {
  const workspace = useWorkspaceIndex();
  const stream = useEventStreamStatus();
  const processes = useProcesses();
  const liveRuns = useLiveRuns();
  const runningProcesses = useLatestRunningProcesses(processes.data ?? []);
  const activeRuns = useActiveRuns(workspace.data ?? null);
  const recentLines = useRecentEventLines(32);
  const recentRunEvents = useRecentEventTypes(
    [
      "run.updated",
      "run.timeline.appended",
      "run.message.appended",
      "run.tool.appended",
      "run.patch.appended",
      "run.command.appended",
      "run.personality.appended",
      "run.skill.appended",
      "run.token.appended",
    ],
    10,
  );
  const recentMessageCount = recentRunEvents.filter((event) => event.type === "run.message.appended").length;
  const recentToolCount = recentRunEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended").length;
  const recentPatchCount = recentRunEvents.filter((event) => event.type === "run.patch.appended").length;
  const recentMechanismCount = recentRunEvents.filter((event) => event.type === "run.personality.appended" || event.type === "run.skill.appended" || event.type === "run.token.appended").length;
  const latestCampaign = workspace.data?.campaigns[0];
  const campaignPulse = (workspace.data?.campaigns ?? []).slice(0, 3);
  const activeRunCards = liveRuns.data?.length ? liveRuns.data : activeRuns;
  const latestPulseAt = (() => {
    const first = activeRunCards[0];
    if (first && "progress" in first) {
      return first.last_event_at ?? latestCampaign?.created_at;
    }
    return activeRuns[0]?.latest_updated_at ?? latestCampaign?.created_at;
  })();
  const liveWarnings = activeRunCards.flatMap((run) => ("warnings" in run ? run.warnings : []));
  const streamTone =
    stream.status === "connected"
      ? "running"
      : stream.status === "degraded"
        ? "warning"
        : stream.status === "disconnected"
          ? "failed"
          : "neutral";

  return (
    <div className="app-shell">
      <div className="shell-accent-bar" aria-hidden="true" />
      <div className="shell-scanlines" aria-hidden="true" />

      <a href="#main-content" className="skip-link">
        跳到主内容
      </a>

      <aside className="left-rail">
        <div className="brand-block">
          <div className="brand-kicker">Codex Testing Bench</div>
          <h1>Research Console</h1>
          <p>把 Codex 求解、评分、artifact、机制链路与研究对比聚合到一个指挥台里。</p>
        </div>

        <nav className="nav-stack" aria-label="Primary">
          {navItems.map(([label, href, index]) => (
            <NavLink
              key={href}
              to={href}
              className={({ isActive }) => `nav-item${isActive ? " nav-item-active" : ""}`}
            >
              <span className="nav-index">{index}</span>
              <span className="nav-label">{label}</span>
            </NavLink>
          ))}
        </nav>

        <section className="rail-section">
          <div className="section-label">Workspace Summary</div>
          <div className="metric-stack">
            <div className="metric-card metric-neutral">
              <div className="metric-label">Campaigns</div>
              <strong className="metric-value">{formatCompact(workspace.data?.summary.campaign_count)}</strong>
            </div>
            <div className="metric-card metric-pressure">
              <div className="metric-label">Runs</div>
              <strong className="metric-value">{formatCompact(workspace.data?.summary.run_count)}</strong>
            </div>
            <div className="metric-card metric-signal">
              <div className="metric-label">Visible Tokens</div>
              <strong className="metric-value">{formatCompact(workspace.data?.summary.total_visible_output_tokens_est)}</strong>
            </div>
            <div className="metric-card metric-verify">
              <div className="metric-label">Tool Calls</div>
              <strong className="metric-value">{formatCompact(workspace.data?.summary.total_tool_calls)}</strong>
            </div>
          </div>
        </section>

        <section className="rail-section">
          <div className="section-label">Campaign Pulse</div>
          <div className="campaign-pulse-stack">
            {campaignPulse.length === 0 ? (
              <div className="empty-box">暂无 indexed campaign。</div>
            ) : (
              campaignPulse.map((campaign) => (
                <div className="brief-card" key={campaign.campaign_id}>
                  <div className="brief-head">
                    <strong>{campaign.experiment_name}</strong>
                    <StatusBadge
                      tone={
                        campaign.status.includes("running")
                          ? "running"
                          : campaign.status.includes("graded") || campaign.status.includes("completed")
                            ? "completed"
                            : "neutral"
                      }
                    >
                      {campaign.status}
                    </StatusBadge>
                  </div>
                  <div className="brief-meta">
                    <span>{campaign.benchmark_name}</span>
                    <span>{campaign.stage_name ?? "—"}</span>
                  </div>
                  <div className="brief-meta">
                    <span>{campaign.sample_size} tasks</span>
                    <span>{campaign.cohort_count} cohorts</span>
                  </div>
                  <div className="brief-meta">
                    <span>{campaign.active_run_count} active</span>
                    <span>{campaign.completed_run_count} completed</span>
                  </div>
                </div>
              ))
            )}
          </div>
        </section>

        <section className="rail-section">
          <div className="section-label">Quick Launch</div>
          <ActionLauncher />
        </section>
      </aside>

      <div className="app-main">
        <header className="status-strip">
          <div className="status-cell">
            <span className="strip-label">Stream Bus</span>
            <strong><StatusBadge tone={streamTone}>{stream.status}</StatusBadge></strong>
            <span className="strip-detail">
              {stream.lastEventAt ? `${formatRelativeTime(stream.lastEventAt)} · ${stream.eventCount} evt` : "waiting for first live event"}
            </span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Latest Campaign</span>
            <strong>{latestCampaign?.campaign_id ?? "No campaign indexed"}</strong>
            <span className="strip-detail">{formatDate(latestCampaign?.created_at)}</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Workspace Refresh</span>
            <strong>{formatDate(workspace.data?.generated_at)}</strong>
            <span className="strip-detail">{workspace.loading ? "refreshing index…" : workspace.data?.repo_root ?? "Pending"}</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Live Runs</span>
            <strong>{activeRunCards.length}</strong>
            <span className="strip-detail">{runningProcesses.length} managed processes</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Signal</span>
            <strong>{formatCompact(workspace.data?.summary.total_tokens)}</strong>
            <span className="strip-detail">total tokens observed</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Recent Pulse</span>
            <strong>{formatRelativeTime(latestPulseAt)}</strong>
            <span className="strip-detail">latest run activity</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Event Mix</span>
            <strong>{recentMessageCount}/{recentToolCount}/{recentPatchCount}/{recentMechanismCount}</strong>
            <span className="strip-detail">msg / tool / patch / mech</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Warnings</span>
            <strong>{liveWarnings.length}</strong>
            <span className="strip-detail">{liveWarnings[0] ?? "no active warning"}</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Bus Errors</span>
            <strong>{stream.errorCount}</strong>
            <span className="strip-detail">{stream.lastErrorAt ? formatRelativeTime(stream.lastErrorAt) : "no transport error"}</span>
          </div>
        </header>
        <main id="main-content" className="canvas">
          {children}
        </main>
      </div>

      <aside className="right-rail">
        <section className="rail-section">
          <div className="section-label">
            <span className="live-indicator">
              <span className="live-dot" aria-hidden="true" />
              Active Run Rail
            </span>
          </div>
          <div className="rail-stack">
            {activeRunCards.length === 0 ? (
              <div className="empty-box">当前没有活跃 run。</div>
            ) : (
              activeRunCards.slice(0, 4).map((run) =>
                "progress" in run ? (
                  <div className="process-card process-card-live" key={run.run_id}>
                    <div className="process-card-head">
                      <strong>{run.instance_id}</strong>
                      <div className="run-status-stack">
                        <StatusBadge tone="running">{run.progress.current_phase}</StatusBadge>
                        <StatusBadge tone={run.activity_heat === "hot" ? "warning" : run.activity_heat === "stalled" ? "failed" : "running"}>
                          {run.activity_heat}
                        </StatusBadge>
                      </div>
                    </div>
                    <div className="brief-meta">
                      <span>{run.model}</span>
                      <span>{run.personality_mode ?? "none"}</span>
                      <span>{run.task_class}</span>
                    </div>
                    <div className="brief-meta">
                      <span>{formatCompact(run.telemetry.total_tokens)} tok</span>
                      <span>{run.progress.tool_count} tools</span>
                      <span>{run.progress.command_count} cmds</span>
                    </div>
                    <div className="mono-note">{truncateMiddle(run.current_focus ?? run.latest_message_preview ?? run.latest_command ?? "live run", 84)}</div>
                    {run.warnings.length ? (
                      <div className="warning-tape">
                        {run.warnings.slice(0, 2).map((warning) => (
                          <span key={warning}>{warning}</span>
                        ))}
                      </div>
                    ) : null}
                  </div>
                ) : (
                  <RunCard key={run.run_id} run={run} compact />
                ),
              )
            )}
          </div>
        </section>

        <section className="rail-section">
          <div className="section-label">
            <span className="live-indicator">
              <span className="live-dot" aria-hidden="true" />
              Structured Run Events
            </span>
          </div>
          <EventRail rows={recentRunEvents} emptyLabel="等待 run / message / tool / patch 事件。" />
        </section>

        <section className="rail-section">
          <div className="section-label">
            <span className="live-indicator">
              <span className="live-dot" aria-hidden="true" />
              Active Process Rail
            </span>
          </div>
          <div className="rail-stack">
            {runningProcesses.length === 0 ? (
              <div className="empty-box">当前没有受管进程在运行。</div>
            ) : (
              runningProcesses.slice(0, 6).map((process) => (
                <div className="process-card" key={process.id}>
                  <div className="process-card-head">
                    <strong>{process.kind}</strong>
                    <StatusBadge tone="running">running</StatusBadge>
                  </div>
                  <div className="mono-note">{truncateMiddle(process.command.join(" "), 84)}</div>
                  <div className="brief-meta">
                    <span>{formatDate(process.started_at)}</span>
                    <span>{truncateMiddle(process.cwd, 42)}</span>
                  </div>
                </div>
              ))
            )}
          </div>
        </section>

        <section className="rail-section">
          <div className="section-label">
            <span className="live-indicator">
              <span className="live-dot" aria-hidden="true" />
              Live Telemetry
            </span>
          </div>
          <div className="telemetry-log">
            {recentLines.length === 0 ? (
              <div className="empty-box">等待新的 stdout / stderr / artifact 事件。</div>
            ) : (
              recentLines.map((line) => (
                <div key={line.id} className={`telemetry-line telemetry-${line.stream}`}>
                  <span className="telemetry-stamp">{line.timestamp.slice(11, 19)}</span>
                  <span className="telemetry-stream">{line.stream}</span>
                  <span className="telemetry-text">{line.line}</span>
                </div>
              ))
            )}
          </div>
        </section>
      </aside>
    </div>
  );
}
