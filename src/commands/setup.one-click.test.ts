import fs from "node:fs/promises";
import path from "node:path";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { withTempHome } from "../../test/helpers/temp-home.js";
import { setupCommand } from "./setup.js";

const {
  ensureManagedCodexMock,
  hasCodexAuthMock,
  probeCodexCompatibilityMock,
  ensureControlUiAssetsBuiltMock,
  runNonInteractiveOnboardingLocalMock,
  dashboardCommandMock,
} = vi.hoisted(() => ({
  ensureManagedCodexMock: vi.fn(async () => ({
    cliPath: "/tmp/codex-managed",
    version: "0.112.0",
    managed: true,
  })),
  hasCodexAuthMock: vi.fn(async () => true),
  probeCodexCompatibilityMock: vi.fn(async () => undefined),
  ensureControlUiAssetsBuiltMock: vi.fn(async () => ({ ok: true, message: "" })),
  runNonInteractiveOnboardingLocalMock: vi.fn(async () => undefined),
  dashboardCommandMock: vi.fn(async () => undefined),
}));

vi.mock("./codex-managed.js", () => ({
  DEFAULT_CODEX_MINIMUM_VERSION: "0.112.0",
  ensureManagedCodex: ensureManagedCodexMock,
  hasCodexAuth: hasCodexAuthMock,
  probeCodexCompatibility: probeCodexCompatibilityMock,
}));

vi.mock("../infra/control-ui-assets.js", () => ({
  ensureControlUiAssetsBuilt: ensureControlUiAssetsBuiltMock,
}));

vi.mock("./onboard-non-interactive/local.js", () => ({
  runNonInteractiveOnboardingLocal: runNonInteractiveOnboardingLocalMock,
}));

vi.mock("./dashboard.js", () => ({
  dashboardCommand: dashboardCommandMock,
}));

describe("setupCommand one-click", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("writes Codex-first defaults with gpt-5.4 as the default model", async () => {
    await withTempHome(async (home) => {
      const runtime = {
        log: vi.fn(),
        error: vi.fn(),
        exit: vi.fn(),
      };

      await setupCommand(
        {
          oneClick: true,
          authChoice: "skip",
          noOpenDashboard: true,
        },
        runtime,
      );

      const configPath = path.join(home, ".openclaw", "openclaw.json");
      const raw = JSON.parse(await fs.readFile(configPath, "utf-8")) as {
        agents?: {
          defaults?: {
            workspace?: string;
            codex?: {
              command?: string;
              defaultModel?: string;
              provider?: string;
              approvalPolicy?: string;
              sandbox?: string;
              minimumVersion?: string;
              experimentalApi?: boolean;
            };
            cliBackends?: {
              codex?: {
                command?: string;
                args?: string[];
              };
            };
          };
        };
        gateway?: { mode?: string };
      };

      expect(raw.gateway?.mode).toBe("local");
      expect(raw.agents?.defaults?.codex?.command).toBe("/tmp/codex-managed");
      expect(raw.agents?.defaults?.codex?.defaultModel).toBe("gpt-5.4");
      expect(raw.agents?.defaults?.codex?.provider).toBe("openai");
      expect(raw.agents?.defaults?.codex?.approvalPolicy).toBe("on-request");
      expect(raw.agents?.defaults?.codex?.sandbox).toBe("workspace-write");
      expect(raw.agents?.defaults?.codex?.minimumVersion).toBe("0.112.0");
      expect(raw.agents?.defaults?.codex?.experimentalApi).toBe(true);
      expect(raw.agents?.defaults?.cliBackends?.codex?.command).toBe("/tmp/codex-managed");
      expect(raw.agents?.defaults?.cliBackends?.codex?.args).toEqual([
        "app-server",
        "--listen",
        "stdio://",
      ]);

      expect(ensureManagedCodexMock).toHaveBeenCalledTimes(1);
      expect(runNonInteractiveOnboardingLocalMock).toHaveBeenCalledTimes(1);
      expect(probeCodexCompatibilityMock).toHaveBeenCalledWith(
        expect.objectContaining({
          command: "/tmp/codex-managed",
          smokeTurn: true,
          model: "gpt-5.4",
        }),
      );
      expect(dashboardCommandMock).not.toHaveBeenCalled();
    });
  });
});
