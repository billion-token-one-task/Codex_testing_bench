# Codex and OpenClaw Harness Swap Research Plan

## Main Question

Can the inner agentic harness of OpenClaw be replaced by Codex so that the architecture becomes `Codex <-> OpenClaw integrations/runtime`, while remaining maintainable as both upstream projects continue to evolve?

## Subtopics

1. Upstream state and positioning
   - Confirm the canonical repositories, licensing, current structure, and how each project is presented/documented upstream.
2. Codex architecture
   - Identify Codex's harness boundaries, model/runtime assumptions, tool protocol, session lifecycle, and whether it can be embedded or adapted as a backend engine.
3. OpenClaw architecture
   - Identify OpenClaw's agent loop, provider/tool abstractions, local integrations, extension surfaces, and how tightly coupled the current harness is to the rest of the app.
4. Swap feasibility
   - Map possible seams where Codex could replace only the agentic core, preserving OpenClaw UI/integrations/local execution.
5. Forward-compatibility strategy
   - Determine what adapter layer would minimize churn as Codex and OpenClaw continue changing independently.

## Expected Output

- A grounded feasibility assessment
- A boundary diagram in prose
- Candidate adapter designs with trade-offs
- Main blockers, unknowns, and validation steps
- Source-backed conclusions with repository and web citations
