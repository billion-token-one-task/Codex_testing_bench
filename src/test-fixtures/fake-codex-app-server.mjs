import readline from "node:readline";

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

const threads = new Map();

function send(payload) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

function ensureThread(threadId = "thread-1") {
  if (!threads.has(threadId)) {
    threads.set(threadId, {
      id: threadId,
      status: "ready",
      turns: [
        {
          id: "turn-history-1",
          status: "completed",
          items: [
            {
              id: "item-user-1",
              type: "userMessage",
              content: [{ type: "text", text: "hello from fake codex" }],
            },
            {
              id: "item-agent-1",
              type: "agentMessage",
              text: "hello from assistant",
            },
            {
              id: "item-command-1",
              type: "commandExecution",
              aggregatedOutput: "stdout: ok",
            },
          ],
        },
      ],
    });
  }
  return threads.get(threadId);
}

rl.on("line", (line) => {
  if (!line.trim()) {
    return;
  }
  const message = JSON.parse(line);
  const { id, method, params = {} } = message;

  if (!method) {
    return;
  }

  switch (method) {
    case "initialize":
      send({
        jsonrpc: "2.0",
        id,
        result: {
          serverInfo: {
            name: "fake-codex",
            version: "0.112.0",
          },
          capabilities: {
            experimentalApi: true,
          },
        },
      });
      return;
    case "initialized":
      return;
    case "skills/list":
      send({
        jsonrpc: "2.0",
        id,
        result: {
          skills: [
            {
              name: "fake-skill",
              description: "Fixture skill",
            },
          ],
        },
      });
      return;
    case "thread/start": {
      const thread = ensureThread("thread-1");
      send({
        jsonrpc: "2.0",
        id,
        result: {
          thread: {
            id: thread.id,
            status: thread.status,
          },
        },
      });
      return;
    }
    case "thread/resume": {
      const thread = ensureThread(params.threadId || "thread-1");
      send({
        jsonrpc: "2.0",
        id,
        result: {
          thread: {
            id: thread.id,
            status: "resumed",
          },
        },
      });
      return;
    }
    case "thread/read": {
      const thread = ensureThread(params.threadId || "thread-1");
      send({
        jsonrpc: "2.0",
        id,
        result: {
          thread,
        },
      });
      return;
    }
    case "thread/compact/start": {
      const threadId = params.threadId || "thread-1";
      ensureThread(threadId);
      send({ jsonrpc: "2.0", id, result: {} });
      setTimeout(() => {
        send({
          jsonrpc: "2.0",
          method: "context/compacted",
          params: {
            threadId,
            previousTokens: 2000,
            newTokens: 300,
          },
        });
      }, 5);
      return;
    }
    case "thread/fork":
      send({
        jsonrpc: "2.0",
        id,
        result: {
          thread: {
            id: "thread-fork-1",
            status: "ready",
          },
        },
      });
      return;
    case "turn/start": {
      const threadId = params.threadId || "thread-1";
      ensureThread(threadId);
      send({
        jsonrpc: "2.0",
        id,
        result: {
          turn: {
            id: "turn-1",
            status: "running",
          },
        },
      });
      setTimeout(() => {
        send({
          jsonrpc: "2.0",
          method: "item/started",
          params: {
            threadId,
            turn: { id: "turn-1", status: "running" },
            item: {
              id: "compaction-1",
              type: "contextCompaction",
            },
          },
        });
        send({
          jsonrpc: "2.0",
          method: "item/completed",
          params: {
            threadId,
            turn: { id: "turn-1", status: "running" },
            item: {
              id: "compaction-1",
              type: "contextCompaction",
            },
          },
        });
        send({
          jsonrpc: "2.0",
          method: "item/agentMessage/delta",
          params: {
            threadId,
            turnId: "turn-1",
            delta: "READY",
          },
        });
        send({
          jsonrpc: "2.0",
          method: "turn/completed",
          params: {
            threadId,
            turn: {
              id: "turn-1",
              status: "completed",
            },
          },
        });
      }, 10);
      return;
    }
    case "turn/interrupt":
      send({ jsonrpc: "2.0", id, result: {} });
      return;
    case "review/start":
      send({ jsonrpc: "2.0", id, result: {} });
      return;
    default:
      send({
        jsonrpc: "2.0",
        id,
        result: {},
      });
  }
});
