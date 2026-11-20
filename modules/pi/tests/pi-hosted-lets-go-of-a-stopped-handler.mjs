// pi as the host runs it lets go of an extension's handler pending when its
// run stops. A run awaited each handler for its turn's events with no bound,
// so one that never settled held its conversation for good, and stopping the
// run freed nothing. Now a tool call nobody finished judging is blocked, an
// event's handler let go is reported and the next one still runs, and one
// pending past 5 s is named in the journal. A handler in a run that goes on,
// or outside any run, counts as it always did.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { it } from "node:test";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const agentFile = (path) => {
  const found = [
    `${root}/node_modules/@earendil-works/pi-coding-agent/dist/${path}`,
    `${root}/packages/coding-agent/dist/${path}`,
    `${root}/dist/${path}`,
  ].find((candidate) => existsSync(candidate));
  assert.ok(found, `no coding agent ${path} under ${root}`);
  return found;
};
const { ExtensionRunner } = await import(
  pathToFileURL(agentFile("core/extensions/runner.js")).href
);

// Each case fails in 10 s rather than hanging, as one would on a pi that
// awaits a handler that never settles.
const BOUNDED = { timeout: 10_000 };

// A handler that never settles, as one waiting on a hung-up service.
const never = () =>
  new Promise(() => {
    // Never settled: what the run must let go of.
  });
const quick = async () => {
  // Settles at once.
};
const GATE_LET_GO = /extension \/ext\/gate\.ts's tool_call handler was let go/u;
const STOPPED_PENDING = /was let go, as its run stopped while it was pending/u;

// A runner over handlers given as [extension path, event, handler], in a run
// whose stop is `run.abort()`, or in no run at all when `run` is null.
const runnerOf = (handlers, run = new AbortController()) => {
  const byPath = new Map();
  for (const [path, event, handler] of handlers) {
    const extension = byPath.get(path) ?? { path, handlers: new Map() };
    extension.handlers.set(event, [...(extension.handlers.get(event) ?? []), handler]);
    byPath.set(path, extension);
  }
  const runner = new ExtensionRunner([...byPath.values()], {}, "/", {}, {});
  // The run's signal, as an agent session binds it through bindCore.
  runner.getSignalFn = () => run?.signal;
  const errors = [];
  runner.onError((error) => errors.push(error));
  return { runner, errors, run };
};

// What console.warn says while `body` runs.
const warnings = async (body) => {
  const said = [];
  const { warn } = console;
  console.warn = (line) => said.push(String(line));
  try {
    await body();
  } finally {
    console.warn = warn;
  }
  return said;
};

// A handler and the way to settle it, once other work has had its turns.
const settleable = () => {
  let settle;
  const handler = () =>
    new Promise((done, fail) => {
      settle = { done, fail };
    });
  return { handler, settle: () => settle };
};

const call = { type: "tool_call", toolName: "bash", toolCallId: "t1", input: {} };

it(
  "blocks a tool call whose handler was still judging it when the run stopped",
  BOUNDED,
  async () => {
    let laterAsked = false;
    const { runner, run } = runnerOf([
      ["/ext/gate.ts", "tool_call", never],
      [
        "/ext/after.ts",
        "tool_call",
        () => {
          laterAsked = true;
        },
      ],
    ]);
    const answer = runner.emitToolCall(call);
    setTimeout(() => run.abort(), 50);
    const result = await answer;
    assert.equal(result?.block, true, "a call nobody finished judging ran");
    assert.match(result.reason, GATE_LET_GO);
    assert.equal(laterAsked, false, "a handler after the blocked call was asked about it");
  },
);

it(
  "lets go of an event's handler when the run stops, reports it and runs the next",
  BOUNDED,
  async () => {
    let nextRan = false;
    const { runner, errors, run } = runnerOf([
      ["/ext/stuck.ts", "tool_execution_start", never],
      [
        "/ext/next.ts",
        "tool_execution_start",
        () => {
          nextRan = true;
        },
      ],
    ]);
    const emitted = runner.emit({
      type: "tool_execution_start",
      toolCallId: "t1",
      toolName: "bash",
      args: {},
    });
    setTimeout(() => run.abort(), 50);
    await emitted;
    assert.equal(nextRan, true, "the next extension's handler never ran");
    assert.deepEqual(
      errors.map(({ extensionPath, event }) => ({ extensionPath, event })),
      [{ extensionPath: "/ext/stuck.ts", event: "tool_execution_start" }],
    );
    assert.match(errors[0].error, STOPPED_PENDING);
  },
);

it(
  "keeps no message from a message_end handler let go, and reports what it does later",
  BOUNDED,
  async () => {
    const late = settleable();
    const { runner, errors, run } = runnerOf([["/ext/late.ts", "message_end", late.handler]]);
    const ended = runner.emitMessageEnd({
      type: "message_end",
      message: { role: "assistant", content: [] },
    });
    setTimeout(() => run.abort(), 50);
    assert.equal(await ended, undefined, "a let-go handler's message counted");
    late.settle().fail(new Error("the gateway hung up"));
    // The late rejection reaches its handler a turn after it fails.
    await new Promise((settled) => setImmediate(settled));
    assert.deepEqual(
      errors.map(({ error }) => error),
      [
        "extension /ext/late.ts's message_end handler was let go, as its run stopped while it was pending",
        "after it was let go: the gateway hung up",
      ],
    );
  },
);

it(
  "counts a handler's answer as before in a run that goes on, or outside any run",
  BOUNDED,
  async () => {
    const guard = async () => ({ block: true, reason: "refused by the guard" });
    const answers = await Promise.all(
      [new AbortController(), null].map((run) =>
        runnerOf([["/ext/guard.ts", "tool_call", guard]], run).runner.emitToolCall(call),
      ),
    );
    assert.deepEqual(answers, [
      { block: true, reason: "refused by the guard" },
      { block: true, reason: "refused by the guard" },
    ]);
  },
);

it("never lets go of a handler outside any run, however long it takes", BOUNDED, async () => {
  const slow = settleable();
  const { runner } = runnerOf([["/ext/slow.ts", "session_before_switch", slow.handler]], null);
  const emitted = runner.emit({ type: "session_before_switch", reason: "new" });
  setTimeout(() => slow.settle().done({ cancel: true }), 50);
  assert.deepEqual(await emitted, { cancel: true });
});

it(
  "names a handler pending past 5 s in the journal, and one that settles sooner not at all",
  BOUNDED,
  async () => {
    const said = await warnings(async () => {
      const slow = settleable();
      const { runner } = runnerOf([
        ["/ext/quick.ts", "tool_execution_end", quick],
        ["/ext/slow.ts", "tool_execution_end", slow.handler],
      ]);
      const emitted = runner.emit({
        type: "tool_execution_end",
        toolCallId: "t1",
        toolName: "bash",
      });
      // Settled just after its 5 s, so it is named once and only once.
      setTimeout(() => slow.settle().done(), 5400);
      await emitted;
    });
    assert.deepEqual(said, [
      "pi-host: extension /ext/slow.ts's tool_execution_end handler still pending after 5 s",
    ]);
  },
);
