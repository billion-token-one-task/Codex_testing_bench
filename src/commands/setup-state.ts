import fs from "node:fs/promises";
import path from "node:path";
import { CONFIG_DIR } from "../utils.js";

export type SetupCheckpoint =
  | "config"
  | "workspace"
  | "codex-install"
  | "auth"
  | "gateway"
  | "health"
  | "complete";

export type SetupState = {
  version: 1;
  checkpoint: SetupCheckpoint;
  updatedAt: number;
  workspace?: string;
  codexPath?: string;
  codexVersion?: string;
  authChoice?: string;
  notes?: string[];
};

const SETUP_STATE_PATH = path.join(CONFIG_DIR, "setup-state.json");

export async function readSetupState(): Promise<SetupState | null> {
  try {
    const raw = await fs.readFile(SETUP_STATE_PATH, "utf8");
    const parsed = JSON.parse(raw) as SetupState;
    if (parsed?.version === 1 && typeof parsed.checkpoint === "string") {
      return parsed;
    }
    return null;
  } catch {
    return null;
  }
}

export async function writeSetupState(
  state: Omit<SetupState, "version" | "updatedAt">,
): Promise<void> {
  await fs.mkdir(path.dirname(SETUP_STATE_PATH), { recursive: true });
  await fs.writeFile(
    SETUP_STATE_PATH,
    JSON.stringify(
      {
        version: 1,
        updatedAt: Date.now(),
        ...state,
      } satisfies SetupState,
      null,
      2,
    ),
    "utf8",
  );
}

export async function clearSetupState(): Promise<void> {
  await fs.rm(SETUP_STATE_PATH, { force: true });
}

export function resolveSetupStatePath(): string {
  return SETUP_STATE_PATH;
}
