import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import JSON5 from "json5";
import { DEFAULT_AGENT_WORKSPACE_DIR, ensureAgentWorkspace } from "../agents/workspace.js";
import { type OpenClawConfig, createConfigIO, writeConfigFile } from "../config/config.js";
import { formatConfigPath, logConfigUpdated } from "../config/logging.js";
import { resolveSessionTranscriptsDir } from "../config/sessions.js";
import type { CliBackendConfig } from "../config/types.agent-defaults.js";
import { ensureControlUiAssetsBuilt } from "../infra/control-ui-assets.js";
import { isWSL2Sync } from "../infra/wsl.js";
import type { RuntimeEnv } from "../runtime.js";
import { defaultRuntime } from "../runtime.js";
import { shortenHomePath } from "../utils.js";
import { dashboardCommand } from "./dashboard.js";
import {
  DEFAULT_CODEX_MINIMUM_VERSION,
  ensureManagedCodex,
  hasCodexAuth,
  probeCodexCompatibility,
} from "./codex-managed.js";
import { runNonInteractiveOnboardingLocal } from "./onboard-non-interactive/local.js";
import { loginOpenAICodexOAuth } from "./openai-codex-oauth.js";
import { openUrl } from "./onboard-helpers.js";
import { applyAuthProfileConfig, setOpenaiApiKey, writeOAuthCredentials } from "./onboard-auth.js";
import {
  clearSetupState,
  readSetupState,
  resolveSetupStatePath,
  writeSetupState,
} from "./setup-state.js";
import type { WizardProgress, WizardPrompter } from "../wizard/prompts.js";

type SetupOptions = {
  workspace?: string;
  oneClick?: boolean;
  authChoice?: "openai-codex" | "openai-api-key" | "skip";
  openaiApiKey?: string;
  noOpenDashboard?: boolean;
};

async function readConfigFileRaw(configPath: string): Promise<{
  exists: boolean;
  parsed: OpenClawConfig;
}> {
  try {
    const raw = await fs.readFile(configPath, "utf-8");
    const parsed = JSON5.parse(raw);
    if (parsed && typeof parsed === "object") {
      return { exists: true, parsed: parsed as OpenClawConfig };
    }
    return { exists: true, parsed: {} };
  } catch {
    return { exists: false, parsed: {} };
  }
}

function createSetupPrompter(runtime: RuntimeEnv): WizardPrompter {
  const makeProgress = (label: string): WizardProgress => ({
    update: (message) => runtime.log(`${label} ${message}`.trim()),
    stop: (message) => {
      if (message) {
        runtime.log(message);
      }
    },
  });
  const unsupported = async () => {
    throw new Error("Interactive prompt requested during one-click setup.");
  };
  return {
    intro: async (title) => runtime.log(title),
    outro: async (message) => runtime.log(message),
    note: async (message, title) => runtime.log(title ? `${title}: ${message}` : message),
    select: unsupported,
    multiselect: unsupported,
    text: unsupported,
    confirm: unsupported,
    progress: makeProgress,
  };
}

function applyBaseSetupConfig(params: {
  cfg: OpenClawConfig;
  workspace: string;
  codexPath?: string;
  openaiApiKey?: string;
}): OpenClawConfig {
  const defaults = params.cfg.agents?.defaults ?? {};
  const codexDefaults = defaults.codex ?? {};
  const existingCodexBackend: Partial<CliBackendConfig> = defaults.cliBackends?.codex ?? {};
  const codexEnv =
    params.openaiApiKey && params.openaiApiKey.trim()
      ? {
          ...existingCodexBackend.env,
          OPENAI_API_KEY: params.openaiApiKey.trim(),
        }
      : existingCodexBackend.env;

  return {
    ...params.cfg,
    agents: {
      ...params.cfg.agents,
      defaults: {
        ...defaults,
        workspace: params.workspace,
        codex: {
          ...codexDefaults,
          ...(params.codexPath ? { command: params.codexPath } : {}),
          defaultModel: codexDefaults.defaultModel ?? "gpt-5.4",
          provider: codexDefaults.provider ?? "openai",
          approvalPolicy: codexDefaults.approvalPolicy ?? "on-request",
          sandbox: codexDefaults.sandbox ?? "workspace-write",
          minimumVersion: codexDefaults.minimumVersion ?? DEFAULT_CODEX_MINIMUM_VERSION,
          experimentalApi: codexDefaults.experimentalApi ?? true,
        },
        cliBackends: {
          ...defaults.cliBackends,
          codex: {
            ...existingCodexBackend,
            command: params.codexPath ?? existingCodexBackend.command ?? "codex",
            args:
              existingCodexBackend.args && existingCodexBackend.args.length > 0
                ? existingCodexBackend.args
                : ["app-server", "--listen", "stdio://"],
            ...(codexEnv ? { env: codexEnv } : {}),
          },
        },
      },
    },
    gateway: {
      ...params.cfg.gateway,
      mode: "local",
    },
  };
}

async function ensureWorkspaceSkeleton(workspace: string, runtime: RuntimeEnv, cfg: OpenClawConfig) {
  const ws = await ensureAgentWorkspace({
    dir: workspace,
    ensureBootstrapFiles: !cfg.agents?.defaults?.skipBootstrap,
  });
  runtime.log(`Workspace OK: ${shortenHomePath(ws.dir)}`);

  const sessionsDir = resolveSessionTranscriptsDir();
  await fs.mkdir(sessionsDir, { recursive: true });
  runtime.log(`Sessions OK: ${shortenHomePath(sessionsDir)}`);

  await fs.mkdir(path.join(workspace, ".agents", "skills"), { recursive: true });
  await fs.mkdir(path.join(os.homedir(), ".agents", "skills"), { recursive: true });
}

async function ensureSystemdReady(runtime: RuntimeEnv) {
  if (process.platform !== "linux") {
    return;
  }
  const systemctl = await import("../process/exec.js").then((mod) => mod.runCommandWithTimeout);
  const result = await systemctl(["systemctl", "--user", "status"], {
    timeoutMs: 10_000,
  }).catch(() => null);
  if (result?.code === 0) {
    return;
  }
  const detail = `${result?.stderr ?? ""}\n${result?.stdout ?? ""}`.trim();
  if (isWSL2Sync()) {
    throw new Error(
      [
        "WSL2 user services are unavailable because systemd is not enabled.",
        "Add the following to /etc/wsl.conf:",
        "[boot]",
        "systemd=true",
        "Then run `wsl --shutdown` from PowerShell and re-run `openclaw setup --one-click`.",
        detail ? `Details: ${detail}` : null,
      ]
        .filter(Boolean)
        .join("\n"),
    );
  }
  throw new Error(
    [
      "systemd --user is required for one-click local setup on Linux.",
      "Enable user services and re-run `openclaw setup --one-click`.",
      detail ? `Details: ${detail}` : null,
    ]
      .filter(Boolean)
      .join("\n"),
  );
}

async function ensureCodexAuth(params: {
  cfg: OpenClawConfig;
  runtime: RuntimeEnv;
  authChoice: "openai-codex" | "openai-api-key" | "skip";
  openaiApiKey?: string;
}) {
  if (params.authChoice === "skip") {
    return params.cfg;
  }
  if (params.authChoice === "openai-api-key") {
    const key = params.openaiApiKey?.trim() || process.env.OPENAI_API_KEY?.trim();
    if (!key) {
      throw new Error(
        "No OpenAI API key was provided. Pass --openai-api-key or set OPENAI_API_KEY before running one-click setup.",
      );
    }
    await setOpenaiApiKey(key);
    return applyAuthProfileConfig(
      applyBaseSetupConfig({
        cfg: params.cfg,
        workspace:
          params.cfg.agents?.defaults?.workspace?.trim() || DEFAULT_AGENT_WORKSPACE_DIR,
        codexPath: params.cfg.agents?.defaults?.codex?.command,
        openaiApiKey: key,
      }),
      {
        profileId: "openai:default",
        provider: "openai",
        mode: "api_key",
      },
    );
  }
  if (await hasCodexAuth()) {
    return params.cfg;
  }
  const creds = await loginOpenAICodexOAuth({
    prompter: createSetupPrompter(params.runtime),
    runtime: params.runtime,
    isRemote: false,
    openUrl: async (url) => {
      await openUrl(url);
    },
    localBrowserMessage: "Complete Codex sign-in in your browser…",
  });
  if (!creds) {
    throw new Error("OpenAI Codex authentication did not complete.");
  }
  const profileId = await writeOAuthCredentials("openai-codex", creds, undefined, {
    syncSiblingAgents: true,
  });
  return applyAuthProfileConfig(params.cfg, {
    profileId,
    provider: "openai-codex",
    mode: "oauth",
  });
}

async function runOneClickSetup(
  opts: SetupOptions | undefined,
  runtime: RuntimeEnv,
  configPath: string,
  existingRaw: { exists: boolean; parsed: OpenClawConfig },
) {
  const desiredWorkspace =
    typeof opts?.workspace === "string" && opts.workspace.trim()
      ? opts.workspace.trim()
      : undefined;
  const workspace =
    desiredWorkspace ??
    existingRaw.parsed.agents?.defaults?.workspace ??
    DEFAULT_AGENT_WORKSPACE_DIR;
  const previousState = await readSetupState();
  if (previousState) {
    runtime.log(
      `Resuming setup from ${previousState.checkpoint} (state: ${shortenHomePath(resolveSetupStatePath())})`,
    );
  }

  let next = applyBaseSetupConfig({
    cfg: existingRaw.parsed,
    workspace,
  });
  await writeConfigFile(next);
  await writeSetupState({ checkpoint: "config", workspace });
  runtime.log(`Config ready: ${formatConfigPath(configPath)}`);

  await ensureWorkspaceSkeleton(workspace, runtime, next);
  await writeSetupState({ checkpoint: "workspace", workspace });

  const managedCodex = await ensureManagedCodex({
    runtime,
    cfg: next,
    minimumVersion: next.agents?.defaults?.codex?.minimumVersion,
  });
  next = applyBaseSetupConfig({
    cfg: next,
    workspace,
    codexPath: managedCodex.cliPath,
  });
  await writeConfigFile(next);
  await writeSetupState({
    checkpoint: "codex-install",
    workspace,
    codexPath: managedCodex.cliPath,
    codexVersion: managedCodex.version,
  });
  runtime.log(`Codex OK: ${shortenHomePath(managedCodex.cliPath)} (${managedCodex.version})`);

  next = await ensureCodexAuth({
    cfg: next,
    runtime,
    authChoice: opts?.authChoice ?? "openai-codex",
    openaiApiKey: opts?.openaiApiKey,
  });
  await writeConfigFile(next);
  await writeSetupState({
    checkpoint: "auth",
    workspace,
    codexPath: managedCodex.cliPath,
    codexVersion: managedCodex.version,
    authChoice: opts?.authChoice ?? "openai-codex",
  });

  await ensureSystemdReady(runtime);
  const controlUi = await ensureControlUiAssetsBuilt(runtime);
  if (!controlUi.ok) {
    throw new Error(controlUi.message);
  }

  await runNonInteractiveOnboardingLocal({
    opts: {
      workspace,
      nonInteractive: true,
      acceptRisk: true,
      authChoice: "skip",
      installDaemon: true,
      skipHealth: false,
      skipSkills: false,
      gatewayBind: "loopback",
      gatewayAuth: "token",
    },
    runtime,
    baseConfig: next,
  });
  await writeSetupState({
    checkpoint: "gateway",
    workspace,
    codexPath: managedCodex.cliPath,
    codexVersion: managedCodex.version,
    authChoice: opts?.authChoice ?? "openai-codex",
  });

  await probeCodexCompatibility({
    command: managedCodex.cliPath,
    cfg: next,
    workspaceDir: workspace,
    smokeTurn: true,
    model: next.agents?.defaults?.codex?.defaultModel ?? "gpt-5.4",
  });
  await writeSetupState({
    checkpoint: "health",
    workspace,
    codexPath: managedCodex.cliPath,
    codexVersion: managedCodex.version,
    authChoice: opts?.authChoice ?? "openai-codex",
  });

  await clearSetupState();
  runtime.log("One-click setup complete.");
  if (!opts?.noOpenDashboard) {
    await dashboardCommand(runtime);
  }
}

export async function setupCommand(
  opts?: SetupOptions,
  runtime: RuntimeEnv = defaultRuntime,
) {
  const io = createConfigIO();
  const configPath = io.configPath;
  const existingRaw = await readConfigFileRaw(configPath);

  if (opts?.oneClick) {
    try {
      await runOneClickSetup(opts, runtime, configPath, existingRaw);
    } catch (error) {
      runtime.error(String(error));
      runtime.error(
        `Setup progress was saved to ${shortenHomePath(resolveSetupStatePath())}. Re-run the same command after fixing the issue to resume.`,
      );
      throw error;
    }
    return;
  }

  const defaults = existingRaw.parsed.agents?.defaults ?? {};
  const workspace =
    typeof opts?.workspace === "string" && opts.workspace.trim()
      ? opts.workspace.trim()
      : defaults.workspace ?? DEFAULT_AGENT_WORKSPACE_DIR;

  const next = applyBaseSetupConfig({
    cfg: existingRaw.parsed,
    workspace,
  });

  if (
    !existingRaw.exists ||
    defaults.workspace !== workspace ||
    existingRaw.parsed.gateway?.mode !== next.gateway?.mode
  ) {
    await writeConfigFile(next);
    if (!existingRaw.exists) {
      runtime.log(`Wrote ${formatConfigPath(configPath)}`);
    } else {
      const updates: string[] = [];
      if (defaults.workspace !== workspace) {
        updates.push("set agents.defaults.workspace");
      }
      if (existingRaw.parsed.gateway?.mode !== next.gateway?.mode) {
        updates.push("set gateway.mode");
      }
      const suffix = updates.length > 0 ? `(${updates.join(", ")})` : undefined;
      logConfigUpdated(runtime, { path: configPath, suffix });
    }
  } else {
    runtime.log(`Config OK: ${formatConfigPath(configPath)}`);
  }

  await ensureWorkspaceSkeleton(workspace, runtime, next);
}
