import { NavLink } from "react-router-dom";
import type { PropsWithChildren } from "react";

import { useLatestRunningProcesses, useProcesses, useRecentEventLines, useWorkspaceIndex } from "../lib/store";
import { formatCompact, formatDate, truncateMiddle } from "../lib/format";
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
  const processes = useProcesses();
  const runningProcesses = useLatestRunningProcesses(processes.data ?? []);
  const recentLines = useRecentEventLines(32);
  const latestCampaign = workspace.data?.campaigns[0];

  return (
    <div className="app-shell">
      <a href="#main-content" className="skip-link">
        跳到主内容
      </a>
      <aside className="left-rail">
        <div className="brand-block">
          <div className="brand-kicker">Codex Testing Bench</div>
          <h1>Research Console</h1>
          <p>
            面向 Codex agent 运行、研究证据、人格机制、工具路由与 patch 链路的本地观察控制台。
          </p>
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
          <div className="section-label">Latest Campaign</div>
          <div className="brief-card">
            <strong>{latestCampaign?.experiment_name ?? "暂无实验"}</strong>
            <div className="brief-meta">
              <span>{latestCampaign?.benchmark_name ?? "—"}</span>
              <span>{latestCampaign?.stage_name ?? "—"}</span>
            </div>
            <div className="brief-meta">
              <span>{latestCampaign?.sample_size ?? 0} tasks</span>
              <span>{latestCampaign?.cohort_count ?? 0} cohorts</span>
            </div>
          </div>
        </section>
      </aside>

      <div className="app-main">
        <header className="status-strip">
          <div className="status-cell">
            <span className="strip-label">Latest Campaign</span>
            <strong>{latestCampaign?.campaign_id ?? "No campaign indexed"}</strong>
            <span className="strip-detail">{formatDate(latestCampaign?.created_at)}</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Workspace Refresh</span>
            <strong>{formatDate(workspace.data?.generated_at)}</strong>
            <span className="strip-detail">{workspace.data?.repo_root ?? "Pending"}</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Live Processes</span>
            <strong>{runningProcesses.length}</strong>
            <span className="strip-detail">{processes.data?.length ?? 0} tracked</span>
          </div>
          <div className="status-cell">
            <span className="strip-label">Signal</span>
            <strong>{formatCompact(workspace.data?.summary.total_tokens)}</strong>
            <span className="strip-detail">total tokens observed</span>
          </div>
        </header>
        <main id="main-content" className="canvas">
          {children}
        </main>
      </div>

      <aside className="right-rail">
        <section className="rail-section">
          <div className="section-label">Active Process Rail</div>
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
          <div className="section-label">Live Telemetry</div>
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
