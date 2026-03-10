import { html, nothing } from "lit";
import type { AppViewState } from "../app-view-state.ts";
import {
  buildDefaultOperatorResolutionDraft,
  formatOperatorRequestTitle,
  getOperatorMcpAction,
  getOperatorMcpContent,
  getOperatorPermissionNetworkEnabled,
  getOperatorPermissionPaths,
  getOperatorPermissionScope,
  getOperatorToolAnswerValue,
  getOperatorToolQuestions,
  isOperatorApprovalRequest,
  type OperatorRequest,
} from "../controllers/operator-request.ts";

function formatRemaining(ms: number): string {
  const remaining = Math.max(0, ms);
  const totalSeconds = Math.floor(remaining / 1000);
  if (totalSeconds < 60) {
    return `${totalSeconds}s`;
  }
  const minutes = Math.floor(totalSeconds / 60);
  if (minutes < 60) {
    return `${minutes}m`;
  }
  const hours = Math.floor(minutes / 60);
  return `${hours}h`;
}

function renderMetaRow(label: string, value?: string | null) {
  if (!value) {
    return nothing;
  }
  return html`<div class="exec-approval-meta-row"><span>${label}</span><span>${value}</span></div>`;
}

function renderPayloadSummary(entry: OperatorRequest) {
  const payload = entry.request.payload;
  if (entry.request.kind === "codex_command_approval") {
    const command = typeof payload.command === "string" ? payload.command : "";
    return command ? html`<div class="exec-approval-command mono">${command}</div>` : nothing;
  }
  if (entry.request.kind === "codex_tool_input") {
    const questions = Array.isArray(payload.questions) ? payload.questions : [];
    return html`
      <div class="exec-approval-meta">
        ${
          questions.length > 0
            ? questions.map((question, index) => {
                const header =
                  typeof question === "object" &&
                  question &&
                  typeof (question as { header?: unknown }).header === "string"
                    ? (question as { header: string }).header
                    : `Question ${index + 1}`;
                const text =
                  typeof question === "object" &&
                  question &&
                  typeof (question as { question?: unknown }).question === "string"
                    ? (question as { question: string }).question
                    : "";
                return html`<div class="exec-approval-meta-row"><span>${header}</span><span>${text}</span></div>`;
              })
            : html`<div class="muted">No question metadata was provided.</div>`
        }
      </div>
    `;
  }
  if (entry.request.kind === "codex_mcp_elicitation") {
    const message = typeof payload.message === "string" ? payload.message : "";
    const mode = typeof payload.mode === "string" ? payload.mode : "";
    const url = typeof payload.url === "string" ? payload.url : "";
    return html`
      ${message ? html`<div class="exec-approval-command">${message}</div>` : nothing}
      <div class="exec-approval-meta">
        ${renderMetaRow("Mode", mode)}
        ${renderMetaRow("URL", url)}
        ${
          typeof payload.serverName === "string"
            ? renderMetaRow("Server", payload.serverName)
            : nothing
        }
      </div>
    `;
  }
  if (entry.request.kind === "codex_permissions_approval") {
    const reason = typeof payload.reason === "string" ? payload.reason : "";
    return html`
      ${reason ? html`<div class="exec-approval-command">${reason}</div>` : nothing}
      <div class="exec-approval-meta">
        ${renderMetaRow("Thread", entry.request.threadId)}
        ${renderMetaRow("Turn", entry.request.turnId)}
      </div>
    `;
  }
  if (entry.request.kind === "codex_file_change_approval") {
    const reason = typeof payload.reason === "string" ? payload.reason : "";
    const grantRoot = typeof payload.grantRoot === "string" ? payload.grantRoot : "";
    return html`
      ${reason ? html`<div class="exec-approval-command">${reason}</div>` : nothing}
      <div class="exec-approval-meta">
        ${renderMetaRow("Grant root", grantRoot)}
      </div>
    `;
  }
  return nothing;
}

function renderGuidedControls(state: AppViewState, entry: OperatorRequest, draft: string) {
  if (entry.request.kind === "codex_permissions_approval") {
    return html`
      <div class="exec-approval-meta">
        <label class="config-field">
          <span>Grant scope</span>
          <select
            .value=${getOperatorPermissionScope(entry, draft)}
            @change=${(event: Event) =>
              state.handleOperatorPermissionDraftChange({
                scope: ((event.target as HTMLSelectElement | null)?.value ?? "turn") as
                  | "turn"
                  | "session",
              })}
          >
            <option value="turn">Turn only</option>
            <option value="session">Whole session</option>
          </select>
        </label>
        <label class="config-field">
          <span>Network</span>
          <input
            type="checkbox"
            .checked=${getOperatorPermissionNetworkEnabled(entry, draft)}
            @change=${(event: Event) =>
              state.handleOperatorPermissionDraftChange({
                networkEnabled: (event.target as HTMLInputElement | null)?.checked === true,
              })}
          />
        </label>
        <label class="config-field">
          <span>Readable paths</span>
          <textarea
            rows="3"
            .value=${getOperatorPermissionPaths(entry, draft, "read")}
            @input=${(event: Event) =>
              state.handleOperatorPermissionDraftChange({
                readPaths: (event.target as HTMLTextAreaElement | null)?.value ?? "",
              })}
          ></textarea>
        </label>
        <label class="config-field">
          <span>Writable paths</span>
          <textarea
            rows="3"
            .value=${getOperatorPermissionPaths(entry, draft, "write")}
            @input=${(event: Event) =>
              state.handleOperatorPermissionDraftChange({
                writePaths: (event.target as HTMLTextAreaElement | null)?.value ?? "",
              })}
          ></textarea>
        </label>
      </div>
    `;
  }

  if (entry.request.kind === "codex_tool_input") {
    const questions = getOperatorToolQuestions(entry);
    return html`
      <div class="exec-approval-meta">
        ${
          questions.length > 0
            ? questions.map(
                (question) => html`
                  <label class="config-field">
                    <span>${question.header || question.question}</span>
                    ${
                      question.options.length > 0 && !question.isOther
                        ? html`
                            <select
                              .value=${getOperatorToolAnswerValue(entry, draft, question.id)}
                              @change=${(event: Event) =>
                                state.handleOperatorToolAnswerChange(
                                  question.id,
                                  (event.target as HTMLSelectElement | null)?.value ?? "",
                                )}
                            >
                              <option value="">Select…</option>
                              ${question.options.map(
                                (option) =>
                                  html`<option value=${option.label}>${option.label}</option>`,
                              )}
                            </select>
                          `
                        : html`
                            <input
                              type=${question.isSecret ? "password" : "text"}
                              .value=${getOperatorToolAnswerValue(entry, draft, question.id)}
                              @input=${(event: Event) =>
                                state.handleOperatorToolAnswerChange(
                                  question.id,
                                  (event.target as HTMLInputElement | null)?.value ?? "",
                                )}
                            />
                          `
                    }
                    ${
                      question.question && question.header !== question.question
                        ? html`<div class="muted">${question.question}</div>`
                        : nothing
                    }
                  </label>
                `,
              )
            : nothing
        }
      </div>
    `;
  }

  if (entry.request.kind === "codex_mcp_elicitation") {
    return html`
      <div class="exec-approval-meta">
        <label class="config-field">
          <span>Action</span>
          <select
            .value=${getOperatorMcpAction(entry, draft)}
            @change=${(event: Event) =>
              state.handleOperatorMcpDraftChange({
                action: ((event.target as HTMLSelectElement | null)?.value ?? "accept") as
                  | "accept"
                  | "decline"
                  | "cancel",
              })}
          >
            <option value="accept">Accept</option>
            <option value="decline">Decline</option>
            <option value="cancel">Cancel</option>
          </select>
        </label>
        <label class="config-field">
          <span>Content</span>
          <textarea
            rows="6"
            .value=${getOperatorMcpContent(entry, draft)}
            @input=${(event: Event) =>
              state.handleOperatorMcpDraftChange({
                content: (event.target as HTMLTextAreaElement | null)?.value ?? "",
              })}
          ></textarea>
        </label>
      </div>
    `;
  }

  return nothing;
}

export function renderOperatorRequestPrompt(state: AppViewState) {
  const active = state.operatorRequestQueue[0];
  if (!active) {
    return nothing;
  }
  const remainingMs = active.expiresAtMs - Date.now();
  const remaining = remainingMs > 0 ? `expires in ${formatRemaining(remainingMs)}` : "expired";
  const queueCount = state.operatorRequestQueue.length;
  const approvalButtons = isOperatorApprovalRequest(active);
  const draft =
    state.operatorRequestDraft.trim() || buildDefaultOperatorResolutionDraft(active);
  return html`
    <div class="exec-approval-overlay" role="dialog" aria-live="polite">
      <div class="exec-approval-card">
        <div class="exec-approval-header">
          <div>
            <div class="exec-approval-title">${formatOperatorRequestTitle(active)}</div>
            <div class="exec-approval-sub">${remaining}</div>
          </div>
          ${
            queueCount > 1
              ? html`<div class="exec-approval-queue">${queueCount} pending</div>`
              : nothing
          }
        </div>
        ${renderPayloadSummary(active)}
        ${renderGuidedControls(state, active, draft)}
        <div class="exec-approval-meta">
          ${renderMetaRow("Kind", active.request.kind)}
          ${renderMetaRow("Session", active.request.sessionKey)}
          ${renderMetaRow("Thread", active.request.threadId)}
          ${renderMetaRow("Turn", active.request.turnId)}
          ${renderMetaRow("Item", active.request.itemId)}
          ${renderMetaRow("Method", active.request.method)}
        </div>
        <label class="config-field">
          <span>Resolution JSON</span>
          <textarea
            rows="10"
            .value=${draft}
            @input=${(event: Event) =>
              state.handleOperatorRequestDraftChange(
                (event.target as HTMLTextAreaElement | null)?.value ?? "",
              )}
          ></textarea>
        </label>
        ${
          state.operatorRequestError
            ? html`<div class="exec-approval-error">${state.operatorRequestError}</div>`
            : nothing
        }
        <div class="exec-approval-actions">
          ${
            approvalButtons
              ? html`
                  <button
                    class="btn primary"
                    ?disabled=${state.operatorRequestBusy}
                    @click=${() => state.handleOperatorRequestDecision("allow-once")}
                  >
                    Allow once
                  </button>
                  <button
                    class="btn"
                    ?disabled=${state.operatorRequestBusy}
                    @click=${() => state.handleOperatorRequestDecision("allow-always")}
                  >
                    Allow session
                  </button>
                  <button
                    class="btn danger"
                    ?disabled=${state.operatorRequestBusy}
                    @click=${() => state.handleOperatorRequestDecision("deny")}
                  >
                    Deny
                  </button>
                `
              : nothing
          }
          <button
            class="btn ${approvalButtons ? "" : "primary"}"
            ?disabled=${state.operatorRequestBusy}
            @click=${() => state.handleOperatorRequestSubmitDraft()}
          >
            Submit response
          </button>
        </div>
      </div>
    </div>
  `;
}
