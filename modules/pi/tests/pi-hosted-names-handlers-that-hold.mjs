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
// The clock the patch reads, moved by the handlers rather than by waiting,
// so the bar is pinned to the millisecond and no case leans on a builder's
// load. A handler taking ms awaits once, as a slow one would.
let now = 0;
const taking = (ms) => async () => {
  await Promise.resolve();
  now += ms;
};

// A runner of these extensions, each a path and its handlers by event,
// standing in for the parts emit reaches, and what it wrote to stderr; a
// write given in its place stands for a stderr that can't be written.
const run = async (extensions, emitting, write) => {
  const errors = [];
  const runner = Object.create(ExtensionRunner.prototype);
  Object.assign(runner, {
    extensions: extensions.map(([path, handlers]) => ({ path, handlers: new Map(handlers) })),
    createContext: () => ({}),
    emitError: (error) => errors.push(error.error),
  });
  const written = [];
  const stderrWrite = process.stderr.write;
  const clock = performance.now;
  process.stderr.write =
    write ??
    ((chunk) => {
      written.push(String(chunk));
      return true;
    });
  performance.now = () => now;
  let result;
  try {
    result = await emitting(runner);
  } finally {
    process.stderr.write = stderrWrite;
    performance.now = clock;
  }
  const lines = written.join("").split("\n").filter(Boolean);
  return { errors, lines, result };
};

// Only the slow extension is said, its two handlers summed, though each
// alone is under the bar. Each extension is timed from its own start, so
// the fast one after it isn't said, nor one a millisecond under the bar.
{
  const { errors, lines } = await run(
    [
      ["slow", [["session_start", [taking(125), taking(125)]]]],
      ["fast", [["session_start", [() => undefined]]]],
      ["under", [["session_start", [taking(249)]]]],
    ],
    (runner) => runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.deepEqual(errors, []);
  assert.deepEqual(lines, ['pi-host: "slow" session_start took 250 ms']);
}

// Two slow extensions are each said on a line of their own, in the order
// they ran, with a fast one between them said by neither.
{
  const { lines } = await run(
    [
      ["first", [["session_start", [taking(300)]]]],
      ["between", [["session_start", [() => undefined]]]],
      ["second", [["session_start", [taking(400)]]]],
    ],
    (runner) => runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.deepEqual(lines, [
    'pi-host: "first" session_start took 300 ms',
    'pi-host: "second" session_start took 400 ms',
  ]);
}

// A turn's streamed events, which emit sends too, are never said, however
// long their handlers take: a slow one would otherwise say a line a token.
// Their handlers are shown to run, so the silence is the patch's.
{
  const ran = [];
  const slowly = (name) => async () => {
    await taking(300)();
    ran.push(name);
  };
  const message = { role: "assistant", content: [] };
  const { errors, lines } = await run(
    [
      [
        "streaming",
        [
          ["message_update", [slowly("message_update")]],
          ["tool_execution_update", [slowly("tool_execution_update")]],
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
  assert.deepEqual(
    ran,
    ["message_update", "tool_execution_update"],
    "a streamed handler never ran",
  );
  assert.deepEqual(errors, []);
  assert.deepEqual(lines, [], "said a turn's streamed event");
}

// A slow handler that throws is said too, and its error still reaches pi.
{
  const failing = async () => {
    await taking(300)();
    throw new Error("broke");
  };
  const { errors, lines } = await run([["failing", [["session_start", [failing]]]]], (runner) =>
    runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.deepEqual(errors, ["broke"]);
  assert.deepEqual(lines, ['pi-host: "failing" session_start took 300 ms']);
}

// A slow handler that cancels is said once, before emit returns its result
// at once: neither its extension's next handler nor the next extension runs.
// Where stderr's write throws, the cancel still comes back, unsaid.
const cancellingRun = (write) => {
  const ran = [];
  const cancelling = async () => {
    await taking(300)();
    return { cancel: true };
  };
  const after = (name) => () => {
    ran.push(name);
  };
  return run(
    [
      ["cancelling", [["session_before_switch", [cancelling, after("its next")]]]],
      ["later", [["session_before_switch", [after("the next extension")]]]],
    ],
    (runner) => runner.emit({ type: "session_before_switch", reason: "new" }),
    write,
  ).then((seen) => ({ ...seen, ran }));
};
{
  const { lines, ran, result } = await cancellingRun();
  assert.deepEqual(result, { cancel: true });
  assert.deepEqual(ran, [], "a handler ran after the cancel");
  assert.deepEqual(lines, ['pi-host: "cancelling" session_before_switch took 300 ms']);
}
{
  let tried = 0;
  const { ran, result } = await cancellingRun(() => {
    tried += 1;
    throw new Error("stderr is gone");
  });
  assert.equal(tried, 1, "the cancel's line was never written");
  assert.deepEqual(result, { cancel: true }, "a write that threw took the cancel's place");
  assert.deepEqual(ran, [], "a handler ran after the cancel");
}

// resources_discover, which pi emits on its own path, is said the same way,
// and what the slow extension found still reaches pi.
{
  const finding = async () => {
    await taking(300)();
    return { skillPaths: ["/skills/one"] };
  };
  const { lines, result } = await run(
    [["finding", [["resources_discover", [finding]]]]],
    (runner) => runner.emitResourcesDiscover("/work", "startup"),
  );
  assert.deepEqual(result.skillPaths, [{ path: "/skills/one", extensionPath: "finding" }]);
  assert.deepEqual(lines, ['pi-host: "finding" resources_discover took 300 ms']);
}

// A path holding a newline is said on one line, quoted as JSON.
{
  const { lines } = await run([["odd\nname", [["session_start", [taking(300)]]]]], (runner) =>
    runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.equal(lines.length, 1);
  assert.match(lines[0], SLOW);
  assert.equal(lines[0], 'pi-host: "odd\\nname" session_start took 300 ms');
}
