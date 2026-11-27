// pi as a windowed session runs it reads a log's entries with the large
// values of every entry the kept view leaves out read again from the log
// only when asked for, and holds what the model reads whole. Unpatched, pi
// holds every value of a log for the session's life: the 46 windowed pi on a
// Mac held 8,831 MB, and a 78 MB log held 104 MB of heap, where read light it
// holds 43 MB. Every reader still sees each entry whole, a write that moves
// the log's lines holds them whole first, and a log pi would change as it
// reads it is read whole, as before.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import fs, { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { syncBuiltinESMExports } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
const { SessionManager, buildSessionContext } = await import(
  pathToFileURL(`${root}/dist/core/session-manager.js`).href
);

const dir = mkdtempSync(join(tmpdir(), "pi-windowed-"));
const big = (seed) => `${seed} `.repeat(1000);
const usage = {
  input: 10,
  output: 5,
  cacheRead: 0,
  cacheWrite: 0,
  totalTokens: 15,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
};
const at = "2026-11-26T00:00:00.000Z";
const message = (id, parentId, body) => ({
  type: "message",
  id,
  parentId,
  timestamp: at,
  message: body,
});
const user = (id, parentId, text) =>
  message(id, parentId, { role: "user", content: [{ type: "text", text }], timestamp: 0 });
const assistant = (id, parentId, content) =>
  message(id, parentId, {
    role: "assistant",
    content,
    api: "x",
    provider: "x",
    model: "x",
    usage,
    stopReason: "stop",
    timestamp: 0,
  });

// A log with a compaction that leaves m1 to m3 out and keeps m4 on, and b1 on
// another branch written after it, which the kept view leaves out as well.
const entries = [
  { type: "session", version: 3, id: "s1", timestamp: at, cwd: dir },
  user("m1", null, big("ask")),
  assistant("m2", "m1", [
    { type: "thinking", thinking: big("think"), thinkingSignature: "sig" },
    { type: "text", text: big("say") },
    { type: "toolCall", id: "c1", name: "bash", arguments: { command: big("cmd") } },
  ]),
  message("m3", "m2", {
    role: "toolResult",
    toolCallId: "c1",
    toolName: "bash",
    content: [{ type: "text", text: big("out") }],
    isError: false,
    timestamp: 0,
  }),
  user("m4", "m3", "hi"),
  assistant("m5", "m4", [{ type: "text", text: big("answer") }]),
  {
    type: "compaction",
    id: "k1",
    parentId: "m5",
    timestamp: at,
    summary: big("summary"),
    firstKeptEntryId: "m4",
    tokensBefore: 100,
  },
  user("b1", "m4", big("elsewhere")),
  user("m6", "k1", big("again")),
  assistant("m7", "m6", [{ type: "text", text: big("reply") }]),
];
// Each line starts with spaces pi never writes, so a rewrite moves every line.
const lines = entries.map((entry) => `  ${JSON.stringify(entry)}\n`).join("");
const plain = entries.slice(1);

let opened = 0;
const open = (text = lines) => {
  const file = join(dir, `log-${++opened}.jsonl`);
  writeFileSync(file, text);
  return { file, manager: SessionManager.open(file, dir) };
};
const light = (holder, key) =>
  typeof Object.getOwnPropertyDescriptor(holder, key)?.get === "function";
const lightAnywhere = (manager) => manager.getEntries().some(anyLight);
function anyLight(value) {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  return Object.keys(value).some((key) => light(value, key) || anyLight(value[key]));
}
const part = (manager, id, i = 0) => manager.getEntry(id).message.content[i];
const same = (actual, expected, what) =>
  assert.equal(JSON.stringify(actual), JSON.stringify(expected), what);

// Read light, every reader still sees what a whole read sees.
{
  const { manager } = open();
  same(manager.getEntries(), plain, "the entries differ from the log's");
  same(
    manager.buildSessionContext(),
    buildSessionContext(plain),
    "the context differs from a whole read's",
  );
  const ids = (branch) => branch.map((entry) => entry.id);
  same(
    ids(manager.getBranch()),
    ["m1", "m2", "m3", "m4", "m5", "k1", "m6", "m7"],
    "the branch differs",
  );
  same(ids(manager.getBranch("b1")), ["m1", "m2", "m3", "m4", "b1"], "b1's branch differs");
  same(manager.getBranch("b1").at(-1), plain[6], "b1 differs from the log's");
}

// The entries the kept view leaves out read their large values again; what
// tells their parts apart, and every entry the view keeps, stays held.
{
  const { manager } = open();
  const m2 = manager.getEntry("m2").message.content;
  assert.ok(light(part(manager, "m1"), "text"), "m1's text, left out, is still held whole");
  assert.ok(
    light(m2[0], "thinking") && light(m2[1], "text"),
    "m2's thinking or text is still held whole",
  );
  assert.ok(light(m2[2], "arguments"), "m2's tool call arguments are still held whole");
  assert.ok(
    !(light(m2[2], "id") || light(m2[2], "name")),
    "a tool call's id or name was read light",
  );
  assert.ok(!light(m2[0], "thinkingSignature"), "a short value was read light");
  assert.ok(!light(manager.getEntry("m2").message, "usage"), "usage was read light");
  assert.ok(
    light(manager.getEntry("m3").message, "content"),
    "m3's tool result is still held whole",
  );
  assert.ok(light(part(manager, "b1"), "text"), "b1, on another branch, is still held whole");
  for (const id of ["m5", "m6", "m7"]) {
    assert.ok(!light(part(manager, id), "text"), `${id}, kept, was read light`);
  }
  assert.ok(
    !light(manager.getEntry("k1"), "summary"),
    "the kept compaction's summary was read light",
  );
  assert.notEqual(
    m2[2].arguments,
    m2[2].arguments,
    "a value read again came back as one object twice",
  );
  same(m2[2].arguments, { command: big("cmd") }, "m2's arguments read again differ");
}

// Moving to a left-out entry holds whole what the model reads from there.
{
  const { manager } = open();
  manager.branch("m2");
  same(manager.buildSessionContext(), buildSessionContext(plain, "m2"), "m2's context differs");
  assert.ok(
    !(light(part(manager, "m1"), "text") || light(part(manager, "m2", 1), "text")),
    "m2's context is still read light",
  );
}

// A compaction made later lets go of what it leaves out: entries the kept
// view held at the read, entries appended since, and the older compaction.
{
  const { file, manager } = open();
  const u1 = manager.appendMessage({
    role: "user",
    content: [{ type: "text", text: big("u1") }],
    timestamp: 0,
  });
  const a1 = manager.appendMessage({
    role: "user",
    content: [{ type: "text", text: big("a1") }],
    timestamp: 0,
  });
  manager.appendCompaction(big("later"), a1, 200);
  for (const id of ["m5", "m6", "m7", u1]) {
    assert.ok(light(part(manager, id), "text"), `${id}, now left out, is still held whole`);
  }
  assert.ok(
    light(manager.getEntry("k1"), "summary"),
    "the older compaction's summary is still held whole",
  );
  assert.ok(!light(part(manager, a1), "text"), "a1, kept, was read light");
  const written = readFileSync(file, "utf8")
    .trim()
    .split("\n")
    .map((line) => JSON.parse(line));
  same(manager.getEntries(), written.slice(1), "the entries differ from the log written");
}

// A rewrite of the log holds whole first what was read light from it, since
// every line moves.
{
  const { file, manager } = open();
  manager._rewriteFile();
  assert.ok(!lightAnywhere(manager), "an entry still reads from a line the rewrite moved");
  same(manager.getEntries(), plain, "the entries differ after the rewrite");
  same(
    readFileSync(file, "utf8"),
    entries.map((entry) => `${JSON.stringify(entry)}\n`).join(""),
    "the log",
  );
}

// An entry appended while something else writes the log has no known place,
// so a compaction leaving it out keeps it held.
{
  const { manager } = open();
  const append = fs.appendFileSync;
  fs.appendFileSync = (path, data) => {
    append(path, `${JSON.stringify(user("x1", null, "elsewhere"))}\n`);
    append(path, data);
  };
  syncBuiltinESMExports();
  let u1;
  try {
    u1 = manager.appendMessage({
      role: "user",
      content: [{ type: "text", text: big("u1") }],
      timestamp: 0,
    });
  } finally {
    fs.appendFileSync = append;
    syncBuiltinESMExports();
  }
  const a1 = manager.appendMessage({
    role: "user",
    content: [{ type: "text", text: big("a1") }],
    timestamp: 0,
  });
  manager.appendCompaction(big("later"), a1, 200);
  assert.ok(!light(part(manager, u1), "text"), "u1, with no known place, was let go of");
  same(part(manager, u1).text, big("u1"), "u1's text differs");
}

// A log pi would migrate, or one whose last line is unfinished, is read
// whole, as unpatched pi reads it, and the unfinished line is ended.
{
  const older = lines.replace('"version":3', '"version":2');
  assert.ok(!lightAnywhere(open(older).manager), "a log pi migrates was read light");
  const unfinished = lines.trimEnd();
  const { file, manager } = open(unfinished);
  assert.ok(!lightAnywhere(manager), "a log with an unfinished line was read light");
  same(manager.getEntries(), plain, "the unfinished log's entries differ");
  assert.ok(readFileSync(file, "utf8").endsWith("\n"), "the unfinished line wasn't ended");
}

rmSync(dir, { recursive: true, force: true });
