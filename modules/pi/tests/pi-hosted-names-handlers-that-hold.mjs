// pi as the host runs it says, on stderr, each extension whose handlers for
// one session event, or resources_discover, took 250 ms or more, and never a
// turn's streamed events. A runner awaits every extension's handlers one
// after another, so a conversation's session_start costs their sum, and
// nothing outside the runner can see which extension a slow open waited on:
// its extensions are private to it. Unpatched, it says nothing.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const runnerModule = [
  `${root}/node_modules/@earendil-works/pi-coding-agent/dist/core/extensions/runner.js`,
  `${root}/packages/coding-agent/dist/core/extensions/runner.js`,
  `${root}/dist/core/extensions/runner.js`,
].find((path) => existsSync(path));
assert.ok(runnerModule, `no extension runner under ${root}`);
const { ExtensionRunner } = await import(pathToFileURL(runnerModule).href);

const SLOW = /^pi-host: (".*") (\S+) took (\d+) ms$/u;
const waiting = (ms) => () => new Promise((done) => setTimeout(done, ms));

// A runner of these extensions, each a path and its handlers by event,
// standing in for the parts emit reaches, and what it wrote to stderr.
const run = async (extensions, emitting) => {
  const errors = [];
  const runner = Object.create(ExtensionRunner.prototype);
  Object.assign(runner, {
    extensions: extensions.map(([path, handlers]) => ({ path, handlers: new Map(handlers) })),
    createContext: () => ({}),
    emitError: (error) => errors.push(error.error),
  });
  const written = [];
  const write = process.stderr.write;
  process.stderr.write = (chunk) => {
    written.push(String(chunk));
    return true;
  };
  let result;
  try {
    result = await emitting(runner);
  } finally {
    process.stderr.write = write;
  }
  const lines = written.join("").split("\n").filter(Boolean);
  return { errors, lines, result };
};

// Only the slow extension is said, its two handlers summed, though each
// alone is under the bar. Each extension is timed from its own start, so
// the fast one after it isn't said, nor one under the bar.
{
  const { lines } = await run(
    [
      ["slow", [["session_start", [waiting(150), waiting(150)]]]],
      ["fast", [["session_start", [() => undefined]]]],
      ["under", [["session_start", [waiting(50)]]]],
    ],
    (runner) => runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.equal(lines.length, 1, `said other than the slow one: ${JSON.stringify(lines)}`);
  const [, path, event, took] = lines[0].match(SLOW) ?? [];
  assert.equal(path, '"slow"');
  assert.equal(event, "session_start");
  assert.ok(Number(took) >= 250, `said ${took} ms for 300 ms of handlers`);
}

// Two slow extensions are each said on a line of their own, in the order
// they ran, with a fast one between them said by neither.
{
  const { lines } = await run(
    [
      ["first", [["session_start", [waiting(300)]]]],
      ["between", [["session_start", [() => undefined]]]],
      ["second", [["session_start", [waiting(300)]]]],
    ],
    (runner) => runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.deepEqual(
    lines.map((line) => line.match(SLOW)?.[1]),
    ['"first"', '"second"'],
    `said other than the two slow ones: ${JSON.stringify(lines)}`,
  );
}

// A turn's streamed events, which emit sends too, are never said, however
// long their handlers take: a slow one would otherwise say a line a token.
{
  const message = { role: "assistant", content: [] };
  const { lines } = await run(
    [
      [
        "streaming",
        [
          ["message_update", [waiting(300)]],
          ["tool_execution_update", [waiting(300)]],
        ],
      ],
    ],
    async (runner) => {
      await runner.emit({
        type: "message_update",
        message,
        assistantMessageEvent: { type: "text_delta" },
      });
      await runner.emit({
        type: "tool_execution_update",
        toolCallId: "one",
        toolName: "bash",
        args: {},
      });
    },
  );
  assert.deepEqual(lines, [], "said a turn's streamed event");
}

// A slow handler that throws is said too, and its error still reaches pi.
{
  const failing = async () => {
    await waiting(300)();
    throw new Error("broke");
  };
  const { errors, lines } = await run([["failing", [["session_start", [failing]]]]], (runner) =>
    runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.deepEqual(errors, ["broke"]);
  assert.equal(lines.length, 1, `said other than once: ${JSON.stringify(lines)}`);
  assert.match(lines[0], /^pi-host: "failing" session_start took \d+ ms$/u);
}

// A slow handler that cancels is said once, before emit returns its result
// at once: neither its extension's next handler nor the next extension runs.
{
  const ran = [];
  const cancelling = async () => {
    await waiting(300)();
    return { cancel: true };
  };
  const after = (name) => () => {
    ran.push(name);
  };
  const { lines, result } = await run(
    [
      ["cancelling", [["session_before_switch", [cancelling, after("its next")]]]],
      ["later", [["session_before_switch", [after("the next extension")]]]],
    ],
    (runner) => runner.emit({ type: "session_before_switch", reason: "new" }),
  );
  assert.deepEqual(result, { cancel: true });
  assert.deepEqual(ran, [], "a handler ran after the cancel");
  assert.equal(lines.length, 1, `said other than once: ${JSON.stringify(lines)}`);
  assert.match(lines[0], /^pi-host: "cancelling" session_before_switch took \d+ ms$/u);
}

// resources_discover, which pi emits on its own path, is said the same way,
// and what the slow extension found still reaches pi.
{
  const finding = async () => {
    await waiting(300)();
    return { skillPaths: ["/skills/one"] };
  };
  const { lines, result } = await run(
    [["finding", [["resources_discover", [finding]]]]],
    (runner) => runner.emitResourcesDiscover("/work", "startup"),
  );
  assert.deepEqual(result.skillPaths, [{ path: "/skills/one", extensionPath: "finding" }]);
  assert.equal(lines.length, 1, `said other than once: ${JSON.stringify(lines)}`);
  assert.match(lines[0], /^pi-host: "finding" resources_discover took \d+ ms$/u);
}

// A path holding a newline is said on one line, quoted as JSON.
{
  const { lines } = await run([["odd\nname", [["session_start", [waiting(300)]]]]], (runner) =>
    runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.equal(lines.length, 1);
  assert.match(lines[0], /^pi-host: "odd\\nname" session_start took \d+ ms$/u);
}
