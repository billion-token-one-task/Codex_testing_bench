import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { once } from "node:events";
import readline from "node:readline";

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

export class CodexRpcClient {
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly lineReader: readline.Interface;
  private readonly pending = new Map<number, PendingRequest>();
  private nextId = 1;
  private closed = false;

  constructor(
    command: string,
    args: string[],
    cwd: string,
    options?: {
      env?: NodeJS.ProcessEnv;
      version?: string;
      stderr?: (text: string) => void;
      exitLabel?: string;
    },
  ) {
    this.child = spawn(command, args, {
      cwd,
      stdio: ["pipe", "pipe", "pipe"],
      env: options?.env ?? process.env,
    });
    this.lineReader = readline.createInterface({ input: this.child.stdout });
    this.lineReader.on("line", (line) => {
      void this.handleLine(line);
    });
    this.child.stderr.on("data", (chunk: Buffer | string) => {
      const text = String(chunk ?? "").trim();
      if (text) {
        options?.stderr?.(text);
      }
    });
    this.child.once("exit", (code, signal) => {
      this.closed = true;
      const label = options?.exitLabel ?? "Codex app-server exited";
      const error = new Error(`${label} (code=${code ?? "null"} signal=${signal ?? "null"})`);
      for (const pending of this.pending.values()) {
        pending.reject(error);
      }
      this.pending.clear();
    });
  }

  async initialize(version = "2026.3.10") {
    await this.request("initialize", {
      clientInfo: {
        name: "openclaw",
        title: "OpenClaw",
        version,
      },
      capabilities: {
        experimentalApi: true,
        optOutNotificationMethods: [],
      },
    });
    await this.notify("initialized", {});
  }

  async request(method: string, params?: unknown): Promise<unknown> {
    const id = this.nextId++;
    const payload = { jsonrpc: "2.0", id, method, ...(params !== undefined ? { params } : {}) };
    const promise = new Promise<unknown>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
    this.writeMessage(payload);
    return await promise;
  }

  async notify(method: string, params?: unknown) {
    this.writeMessage({ jsonrpc: "2.0", method, ...(params !== undefined ? { params } : {}) });
  }

  async close() {
    if (this.closed) {
      return;
    }
    this.closed = true;
    this.lineReader.close();
    this.child.stdin.end();
    this.child.stdout.destroy();
    this.child.stderr.destroy();
    this.child.kill();
    await Promise.race([
      once(this.child, "exit"),
      new Promise((resolve) => setTimeout(resolve, 1_000)),
    ]).catch(() => undefined);
  }

  private writeMessage(payload: Record<string, unknown>) {
    if (this.closed || !this.child.stdin.writable) {
      throw new Error("Codex app-server is not writable.");
    }
    this.child.stdin.write(`${JSON.stringify(payload)}\n`);
  }

  private async handleLine(rawLine: string) {
    const line = rawLine.trim();
    if (!line) {
      return;
    }
    let message: unknown;
    try {
      message = JSON.parse(line);
    } catch {
      return;
    }
    if (!isRecord(message) || typeof message.id !== "number") {
      return;
    }
    const pending = this.pending.get(message.id);
    if (!pending) {
      return;
    }
    this.pending.delete(message.id);
    if ("result" in message) {
      pending.resolve(message.result);
      return;
    }
    const error = isRecord(message.error) ? message.error : undefined;
    pending.reject(new Error(asString(error?.message) ?? "Codex app-server request failed."));
  }
}
