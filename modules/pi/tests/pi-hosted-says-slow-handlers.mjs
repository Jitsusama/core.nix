// pi as the host runs it says, on stderr, each extension whose handlers for
// one event took 250 ms or more. A runner awaits every extension's handlers
// one after another, so a conversation's session_start costs their sum, and
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

const SLOW = /^pi-host: (".*") (\S+) took (\d+) ms$/;
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
// alone is under the bar; one with no handler for the event is never timed.
{
  const { lines } = await run(
    [
      ["fast", [["session_start", [() => undefined]]]],
      ["slow", [["session_start", [waiting(150), waiting(150)]]]],
      ["other", [["turn_end", [waiting(300)]]]],
    ],
    (runner) => runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.equal(lines.length, 1, `said other than the slow one: ${JSON.stringify(lines)}`);
  const [, path, event, took] = lines[0].match(SLOW) ?? [];
  assert.equal(path, '"slow"');
  assert.equal(event, "session_start");
  assert.ok(Number(took) >= 250, `said ${took} ms for 300 ms of handlers`);
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
  assert.match(lines[0] ?? "", /^pi-host: "failing" session_start took \d+ ms$/);
}

// A slow handler that cancels is said before emit returns its result.
{
  const cancelling = async () => {
    await waiting(300)();
    return { cancel: true };
  };
  const { lines, result } = await run(
    [["cancelling", [["session_before_switch", [cancelling]]]]],
    (runner) => runner.emit({ type: "session_before_switch", reason: "new" }),
  );
  assert.deepEqual(result, { cancel: true });
  assert.match(lines[0] ?? "", /^pi-host: "cancelling" session_before_switch took \d+ ms$/);
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
  assert.match(lines[0] ?? "", /^pi-host: "finding" resources_discover took \d+ ms$/);
}

// A path holding a newline is said on one line, quoted as JSON.
{
  const { lines } = await run([["odd\nname", [["session_start", [waiting(300)]]]]], (runner) =>
    runner.emit({ type: "session_start", reason: "startup" }),
  );
  assert.equal(lines.length, 1);
  assert.match(lines[0], /^pi-host: "odd\\nname" session_start took \d+ ms$/);
}
