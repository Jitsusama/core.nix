// What the patched claude-code provider, whose folder is the first argument,
// does when the model calls one of pi's tools that isn't loaded (one loaded
// on demand, such as jobs): the call goes to pi, which answers it as not
// found, rather than ending the turn, and a relayed stream that breaks says
// what broke it rather than only "Error". Claude Code is a stand-in, and so
// is pi's own library, which these paths import but never reach.
import assert from "node:assert/strict";
import { chmodSync, cpSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import * as http from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";

const provider = process.argv[2];
const work = mkdtempSync(join(tmpdir(), "claude-unloaded-"));
cpSync(join(provider, "src"), join(work, "src"), { recursive: true });

const library = join(work, "node_modules", "@earendil-works", "pi-ai");
mkdirSync(join(library, "providers"), { recursive: true });
writeFileSync(
  join(library, "package.json"),
  JSON.stringify({
    name: "@earendil-works/pi-ai",
    type: "module",
    exports: { ".": "./index.js", "./providers/all": "./providers/all.js" },
  }),
);
writeFileSync(
  join(library, "index.js"),
  [
    "registerSessionResourceCleanup",
    "getDeclaredTools",
    "getInitialSystemMessage",
    "hasToolRedefinitions",
    "renderSystemMessageUpdate",
  ]
    .map((name) => `export function ${name}() {}`)
    .join("\n"),
);
writeFileSync(join(library, "providers", "all.js"), "export const getBuiltinModels = () => [];");

// A relayed stream broken by what pi throws while reading it says what was thrown.
const { TurnRelay } = await import(join(work, "src", "turn-relay.ts"));
const upstream = http.createServer(async (request, response) => {
  for await (const _chunk of request);
  response.writeHead(200, { "content-type": "text/event-stream" });
  const start = { type: "message_start", message: { id: "m1", role: "assistant", content: [], usage: {} } };
  response.end(`data: ${JSON.stringify(start)}\n\n`);
});
await new Promise((resolve) => upstream.listen(0, "127.0.0.1", resolve));
const relay = new TurnRelay(`http://127.0.0.1:${upstream.address().port}`, 5_000, () => {
  throw new Error("Claude Code returned unavailable tool jobs");
});
const base = await relay.start();
await fetch(`${base}/v1/messages`, { method: "POST", body: "{}" })
  .then((response) => response.text())
  .catch(() => "");
await relay.settled();
assert.equal(relay.failure, "Error: Claude Code returned unavailable tool jobs");
await relay.close();
await new Promise((resolve) => upstream.close(resolve));

// A call to a pi tool not active now comes back to pi rather than ending the turn.
const claude = join(work, "claude");
writeFileSync(
  claude,
  // The node running this, since a build has no /usr/bin/env.
  `#!${process.execPath}
import readline from "node:readline";
if (process.argv.includes("auth")) {
  console.log(JSON.stringify({loggedIn: true, authMethod: "claude.ai", subscriptionType: "max"}));
  process.exit(0);
}
for await (const line of readline.createInterface({input: process.stdin})) {
  if (JSON.parse(line).type !== "user") continue;
  const usage = {input_tokens: 3, output_tokens: 24};
  const content = [{type: "tool_use", id: "toolu_jobs", name: "mcp__pi__jobs", input: {action: "list"}}];
  console.log(JSON.stringify({type: "stream_event", event: {type: "message_stop"}}));
  console.log(JSON.stringify({type: "assistant", message: {id: "m_jobs", role: "assistant", content, stop_reason: "tool_use", usage}}));
  console.log(JSON.stringify({type: "result", subtype: "error_max_turns", is_error: true, usage}));
}
`,
);
chmodSync(claude, 0o755);
const { closeClaudeProcesses, runClaudeRequest } = await import(join(work, "src", "claude-cli.ts"));
try {
  const answered = await runClaudeRequest({
    model: "sonnet",
    cwd: work,
    frames: [{ type: "user", message: { role: "user", content: [{ type: "text", text: "status?" }] } }],
    systemPrompt: "system",
    manifest: [],
    extraBody: { tools: [] },
    toolNames: new Set(["bash"]),
    env: { PATH: process.env.PATH, HOME: work, PI_CLAUDE_CODE_COMMAND: claude },
    timeoutMs: 5_000,
  });
  assert.equal(answered.message.stop_reason, "tool_use");
  assert.equal(answered.message.content[0].name, "mcp__pi__jobs");
} finally {
  await closeClaudeProcesses();
}
