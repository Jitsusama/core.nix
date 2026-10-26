// How often the patched claude-code provider, whose folder is the first
// argument, starts Claude Code to ask for its login: once for any number of
// asks within a minute, and those asking at once share it; an answer that
// isn't logged in is asked again, so a login is seen at once. Claude Code is
// a stand-in that counts its asks, and pi's own library, which the provider
// imports but these asks never reach, is a stand-in too.
import assert from "node:assert/strict";
import { chmodSync, cpSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const provider = process.argv[2];
const work = mkdtempSync(join(tmpdir(), "claude-login-"));
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

const asks = join(work, "asks");
const answer = join(work, "answer");
const claude = join(work, "claude");
writeFileSync(asks, "");
writeFileSync(
  claude,
  `#!/bin/sh\necho ask >> "${asks}"\nsleep 0.2\ncat "${answer}"\n`,
);
chmodSync(claude, 0o755);
const loggedIn = (yes) =>
  writeFileSync(answer, JSON.stringify({ loggedIn: yes, authMethod: "claude.ai", subscriptionType: "pro" }));
const asked = () => readFileSync(asks, "utf8").split("\n").filter(Boolean).length;

const { getClaudeStatus } = await import(join(work, "src", "claude-cli.ts"));
const env = { PATH: process.env.PATH, HOME: work, PI_CLAUDE_CODE_COMMAND: claude };

loggedIn(false);
assert.equal((await getClaudeStatus(env)).loggedIn, false);
assert.equal((await getClaudeStatus(env)).loggedIn, false);
assert.equal(asked(), 2, "an answer that isn't logged in was kept");

loggedIn(true);
const together = await Promise.all([1, 2, 3, 4].map(() => getClaudeStatus(env)));
assert.ok(together.every((status) => status.loggedIn && status.subscription));
assert.equal(asked(), 3, "asks made at once each started Claude Code");

assert.equal((await getClaudeStatus(env)).loggedIn, true);
assert.equal(asked(), 3, "a logged-in answer wasn't kept");

const elsewhere = { ...env, HOME: join(work, "elsewhere") };
await getClaudeStatus(elsewhere);
assert.equal(asked(), 4, "another login was answered from this one");

const stopped = new AbortController();
stopped.abort();
await assert.rejects(getClaudeStatus(env, stopped.signal));
assert.equal(asked(), 4);
