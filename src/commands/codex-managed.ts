import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { pipeline } from "node:stream/promises";
import { createWriteStream } from "node:fs";
import { request } from "node:https";
import { CodexRpcClient } from "../agents/codex-rpc-client.js";
import type { OpenClawConfig } from "../config/config.js";
import { extractArchive } from "../infra/archive.js";
import { readCodexCliCredentials } from "../agents/cli-credentials.js";
import { runCommandWithTimeout } from "../process/exec.js";
import type { RuntimeEnv } from "../runtime.js";
import { CONFIG_DIR, pathExists } from "../utils.js";

export const DEFAULT_CODEX_MINIMUM_VERSION = "0.112.0";

type ReleaseAsset = {
  name?: string;
  browser_download_url?: string;
  digest?: string;
};

type ReleaseResponse = {
  tag_name?: string;
  assets?: ReleaseAsset[];
};

type NamedAsset = {
  name: string;
  browser_download_url: string;
  digest?: string;
};

function normalizeCodexArgs(cfg?: OpenClawConfig): { command: string; args: string[] } {
  const configured = cfg?.agents?.defaults?.codex;
  const rawArgs = Array.isArray(configured?.args) ? configured.args.filter(Boolean) : [];
  return {
    command: configured?.command?.trim() || "codex",
    args:
      rawArgs.length > 0 ? rawArgs : ["app-server", "--listen", configured?.listen ?? "stdio://"],
  };
}

function parseVersion(raw: string): number[] {
  return raw
    .replace(/^rust-v/i, "")
    .replace(/^v/i, "")
    .split(/[^0-9]+/)
    .filter(Boolean)
    .map((part) => Number.parseInt(part, 10))
    .filter((part) => Number.isFinite(part));
}

function compareVersions(left: string, right: string): number {
  const l = parseVersion(left);
  const r = parseVersion(right);
  const max = Math.max(l.length, r.length);
  for (let index = 0; index < max; index += 1) {
    const lv = l[index] ?? 0;
    const rv = r[index] ?? 0;
    if (lv > rv) {
      return 1;
    }
    if (lv < rv) {
      return -1;
    }
  }
  return 0;
}

function pickCodexAsset(assets: ReleaseAsset[]): NamedAsset | undefined {
  const withName = assets.filter(
    (asset): asset is NamedAsset => Boolean(asset.name && asset.browser_download_url),
  );
  const archives = withName.filter((asset) => asset.name.endsWith(".tar.gz"));
  const arch =
    process.arch === "arm64" ? "aarch64" : process.arch === "x64" ? "x86_64" : process.arch;
  const platform =
    process.platform === "darwin"
      ? "apple-darwin"
      : process.platform === "linux"
        ? "unknown-linux-gnu"
        : null;
  if (!platform) {
    return undefined;
  }
  return archives.find((asset) => asset.name.includes(arch) && asset.name.includes(platform));
}

function resolveManagedCodexDir(version: string) {
  return path.join(CONFIG_DIR, "tools", "codex", version);
}

async function downloadToFile(url: string, dest: string, maxRedirects = 5): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const req = request(
      url,
      {
        headers: {
          "User-Agent": "openclaw",
          Accept: "application/octet-stream",
        },
      },
      (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400) {
          const location = res.headers.location;
          if (!location || maxRedirects <= 0) {
            reject(new Error("Redirect loop or missing Location header"));
            return;
          }
          const redirectUrl = new URL(location, url).href;
          resolve(downloadToFile(redirectUrl, dest, maxRedirects - 1));
          return;
        }
        if (!res.statusCode || res.statusCode >= 400) {
          reject(new Error(`HTTP ${res.statusCode ?? "?"} downloading file`));
          return;
        }
        const out = createWriteStream(dest);
        pipeline(res, out).then(resolve).catch(reject);
      },
    );
    req.on("error", reject);
    req.end();
  });
}

async function sha256File(filePath: string): Promise<string> {
  const hash = createHash("sha256");
  const data = await fs.readFile(filePath);
  hash.update(data);
  return hash.digest("hex");
}

async function findCodexBinary(root: string): Promise<string | null> {
  const queue = [root];
  while (queue.length > 0) {
    const current = queue.shift()!;
    const entries = await fs.readdir(current, { withFileTypes: true }).catch(() => []);
    for (const entry of entries) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(full);
        continue;
      }
      if (entry.isFile() && entry.name === "codex") {
        return full;
      }
    }
  }
  return null;
}

export async function readCodexVersion(command: string): Promise<string | null> {
  try {
    const result = await runCommandWithTimeout([command, "--version"], {
      timeoutMs: 10_000,
    });
    if (result.code !== 0) {
      return null;
    }
    const raw = result.stdout.trim() || result.stderr.trim();
    const match = raw.match(/(?:rust-v|v)?(\d+\.\d+\.\d+)/i);
    return match?.[1] ?? null;
  } catch {
    return null;
  }
}

export async function ensureManagedCodex(params: {
  runtime: RuntimeEnv;
  cfg?: OpenClawConfig;
  minimumVersion?: string;
}): Promise<{ cliPath: string; version: string; managed: boolean }> {
  const minimumVersion = params.minimumVersion ?? DEFAULT_CODEX_MINIMUM_VERSION;
  const configuredCommand = normalizeCodexArgs(params.cfg).command;
  const configuredVersion = await readCodexVersion(configuredCommand);
  if (configuredVersion && compareVersions(configuredVersion, minimumVersion) >= 0) {
    return { cliPath: configuredCommand, version: configuredVersion, managed: false };
  }

  const response = await fetch("https://api.github.com/repos/openai/codex/releases/latest", {
    headers: {
      "User-Agent": "openclaw",
      Accept: "application/vnd.github+json",
    },
  });
  if (!response.ok) {
    throw new Error(`Failed to fetch Codex release metadata (${response.status}).`);
  }
  const release = (await response.json()) as ReleaseResponse;
  const asset = pickCodexAsset(release.assets ?? []);
  if (!asset) {
    throw new Error(`No compatible Codex release asset found for ${process.platform}/${process.arch}.`);
  }

  const version = (release.tag_name ?? "").replace(/^rust-v/i, "").replace(/^v/i, "").trim();
  const installDir = resolveManagedCodexDir(version || minimumVersion);
  const existingBinary = await findCodexBinary(installDir);
  if (existingBinary) {
    return { cliPath: existingBinary, version: version || minimumVersion, managed: true };
  }

  await fs.mkdir(installDir, { recursive: true });
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "openclaw-codex-"));
  const archivePath = path.join(tmpDir, asset.name);
  params.runtime.log(`Downloading Codex ${version || minimumVersion} (${asset.name})…`);
  await downloadToFile(asset.browser_download_url, archivePath);
  if (asset.digest?.startsWith("sha256:")) {
    const digest = await sha256File(archivePath);
    if (digest !== asset.digest.slice("sha256:".length)) {
      throw new Error("Codex download checksum mismatch.");
    }
  }
  await extractArchive({
    archivePath,
    destDir: installDir,
    timeoutMs: 120_000,
  });
  const binary = await findCodexBinary(installDir);
  if (!binary) {
    throw new Error("Codex binary was not found after extraction.");
  }
  await fs.chmod(binary, 0o755).catch(() => undefined);
  return {
    cliPath: binary,
    version: (await readCodexVersion(binary)) ?? version ?? minimumVersion,
    managed: true,
  };
}

export async function probeCodexCompatibility(params: {
  command: string;
  cfg?: OpenClawConfig;
  workspaceDir: string;
  smokeTurn?: boolean;
  model?: string;
}): Promise<void> {
  const runtimeArgs = normalizeCodexArgs(params.cfg).args;
  const client = new CodexRpcClient(params.command, runtimeArgs, params.workspaceDir, {
    exitLabel: "Codex probe exited",
  });
  try {
    await client.initialize();
    await client.request("skills/list", {
      cwd: params.workspaceDir,
      forceReload: true,
    });
    const model = params.model?.trim() || params.cfg?.agents?.defaults?.codex?.defaultModel || "gpt-5.4";
    const threadResponse = (await client.request("thread/start", {
      model,
      modelProvider: params.cfg?.agents?.defaults?.codex?.provider ?? "openai",
      cwd: params.workspaceDir,
      approvalPolicy: "never",
      sandbox: "workspace-write",
      baseInstructions: "OpenClaw compatibility probe.",
      dynamicTools: [
        {
          name: "openclaw_smoke_ok",
          description: "Returns READY for smoke tests.",
          inputSchema: {
            type: "object",
            additionalProperties: false,
          },
        },
      ],
      experimentalRawEvents: false,
      ephemeral: true,
      persistExtendedHistory: true,
    })) as { thread?: { id?: string } };
    const threadId = threadResponse?.thread?.id;
    if (!threadId) {
      throw new Error("Codex smoke test did not return a thread id.");
    }
    await client.request("thread/read", {
      threadId,
      includeTurns: true,
    });
    await client.request("thread/compact/start", {
      threadId,
    });
    await client.request("thread/fork", {
      threadId,
      cwd: params.workspaceDir,
      approvalPolicy: "never",
      sandbox: "workspace-write",
      persistExtendedHistory: true,
    });
    await client.request("turn/start", {
      threadId,
      input: [{ type: "text", text: "Reply with READY and nothing else.", text_elements: [] }],
      model,
    });
    if (!params.smokeTurn) {
      return;
    }
    await client.request("review/start", {
      threadId,
      target: {
        type: "custom",
        instructions: "Confirm this compatibility probe thread was created successfully.",
      },
      delivery: "inline",
    });
  } finally {
    await client.close().catch(() => undefined);
  }
}

export async function hasCodexAuth(): Promise<boolean> {
  return Boolean(readCodexCliCredentials()) || (await pathExists(path.join(os.homedir(), ".codex", "auth.json")));
}
