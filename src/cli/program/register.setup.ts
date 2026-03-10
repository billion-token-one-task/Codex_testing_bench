import type { Command } from "commander";
import { onboardCommand } from "../../commands/onboard.js";
import { setupCommand } from "../../commands/setup.js";
import { defaultRuntime } from "../../runtime.js";
import { formatDocsLink } from "../../terminal/links.js";
import { theme } from "../../terminal/theme.js";
import { runCommandWithRuntime } from "../cli-utils.js";
import { hasExplicitOptions } from "../command-options.js";

export function registerSetupCommand(program: Command) {
  program
    .command("setup")
    .description("Initialize OpenClaw or run the full one-click local Codex bootstrap")
    .addHelpText(
      "after",
      () =>
        `\n${theme.muted("Docs:")} ${formatDocsLink("/cli/setup", "docs.openclaw.ai/cli/setup")}\n`,
    )
    .option(
      "--workspace <dir>",
      "Agent workspace directory (default: ~/.openclaw/workspace; stored as agents.defaults.workspace)",
    )
    .option("--wizard", "Run the interactive onboarding wizard", false)
    .option("--one-click", "Install/configure Codex, auth, daemon, health checks, and dashboard")
    .option("--non-interactive", "Run the wizard without prompts", false)
    .option("--mode <mode>", "Wizard mode: local|remote")
    .option("--remote-url <url>", "Remote Gateway WebSocket URL")
    .option("--remote-token <token>", "Remote Gateway token (optional)")
    .option(
      "--auth-choice <choice>",
      "One-click auth mode: openai-codex|openai-api-key|skip",
    )
    .option("--openai-api-key <key>", "OpenAI API key for one-click Codex auth")
    .option("--no-open-dashboard", "Do not open the local dashboard after one-click setup", false)
    .action(async (opts, command) => {
      await runCommandWithRuntime(defaultRuntime, async () => {
        const hasWizardFlags = hasExplicitOptions(command, [
          "wizard",
          "nonInteractive",
          "mode",
          "remoteUrl",
          "remoteToken",
        ]);
        if (opts.wizard || hasWizardFlags) {
          await onboardCommand(
            {
              workspace: opts.workspace as string | undefined,
              nonInteractive: Boolean(opts.nonInteractive),
              mode: opts.mode as "local" | "remote" | undefined,
              remoteUrl: opts.remoteUrl as string | undefined,
              remoteToken: opts.remoteToken as string | undefined,
            },
            defaultRuntime,
          );
          return;
        }
        await setupCommand(
          {
            workspace: opts.workspace as string | undefined,
            oneClick: Boolean(opts.oneClick),
            authChoice:
              opts.authChoice === "openai-codex" ||
              opts.authChoice === "openai-api-key" ||
              opts.authChoice === "skip"
                ? opts.authChoice
                : undefined,
            openaiApiKey: opts.openaiApiKey as string | undefined,
            noOpenDashboard: Boolean(opts.noOpenDashboard),
          },
          defaultRuntime,
        );
      });
    });
}
