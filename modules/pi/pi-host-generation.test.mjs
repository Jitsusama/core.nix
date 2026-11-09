// Exercise the opt-in turn boundary against the patched runtime, without real credentials or provider requests.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { test as it } from "node:test";
import { pathToFileURL } from "node:url";

const ENTRY_PAGE_SIZE = 1000;
const SETTLE_TIMEOUT_MS = 5000;
const TEXT_TURN_COUNT = 3;
const TOOL_TASK_ID = 8;
const root = resolve(process.argv[2]);
const codeDirectory = process.argv[3] ?? "dist";
assert.ok(
  ["src", "dist"].includes(codeDirectory),
  "Choose src for a source proof or dist for the host build",
);
const extension = codeDirectory === "src" ? "ts" : "js";
const durable = await import(
  pathToFileURL(`${root}/packages/durable/${codeDirectory}/index.${extension}`).href
);
const ai = await import(
  pathToFileURL(`${root}/node_modules/@earendil-works/pi-ai/dist/index.js`).href
);
const schema = await import(
  pathToFileURL(`${root}/node_modules/@earendil-works/pi-schema/dist/index.js`).href
);
const {
  Harness,
  MemoryStorage,
  createRegistry,
  defineExtension,
  defineTool,
  hook,
  GenerationTask,
  LiveDoc,
} = durable;
const { createModels, fauxProvider, fauxAssistantMessage } = ai;
const { BACKGROUND_CONTEXT: context } = await import(
  pathToFileURL(`${root}/node_modules/@earendil-works/chord/dist/context/index.js`).href
);
const messageText = (message) =>
  typeof message.content === "string"
    ? message.content
    : message.content.find((part) => part.type === "text")?.text;

async function open(handlers, tool) {
  const registry = createRegistry();
  registry.install(
    defineExtension({
      name: "host-boundary-test",
      hooks: [hook(GenerationTask, handlers)],
      ...(tool ? { tools: [tool] } : {}),
    }),
  );
  const faux = fauxProvider({ tokensPerSecond: 100_000 });
  const models = createModels();
  models.setProvider(faux.provider);
  const reports = [];
  const harness = await Harness.open(
    new MemoryStorage(),
    { models, registry, onReport: (report) => reports.push(report.error) },
    context,
  );
  const conversation = await harness.root(context, {
    agent: { model: { provider: "faux", modelId: "faux-1" } },
  });
  harness.resume();
  return { harness, conversation, faux, reports };
}

async function messages(conversation) {
  const page = await conversation.entries({}, ENTRY_PAGE_SIZE, undefined, context);
  return [...page.items]
    .reverse()
    .flatMap((entry) => entry.model ?? [])
    .filter((message) => message.role !== "system");
}

function response(requests, text) {
  return (request) => {
    requests.push(request.messages);
    return fauxAssistantMessage(text);
  };
}

function deadline(promise) {
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      const timer = setTimeout(
        () => reject(new Error("Turn did not settle in five seconds")),
        SETTLE_TIMEOUT_MS,
      );
      timer.unref();
    }),
  ]);
}

it("legacy answer and idle continue remain unchanged without the hook", async () => {
  const { harness, conversation, faux } = await open({});
  try {
    faux.setResponses([fauxAssistantMessage("answer")]);
    const input = await conversation.submit({ type: "input", content: "hi" }, context);
    assert.equal((await deadline(input.wait(context))).status, "done");
    const next = await conversation.submit({ type: "continue" }, context);
    const ended = await deadline(next.wait(context));
    assert.equal(ended.status, "unanswered");
    assert.equal(ended.reason, "nothing_to_answer");
    assert.deepEqual((await messages(conversation)).map(messageText), ["hi", "answer"]);
  } finally {
    await harness.close(context);
  }
});

it("committed answers precede the hook; context-only continue leaves follow-ups for end", async () => {
  let calls = 0;
  let queued;
  let conversation;
  const requests = [];
  const setup = await open({
    onYield: () => {
      throw new Error("Legacy onYield must not run");
    },
    onCommittedTurn: async ({ assistant, results }, api, callContext) => {
      const page = await api.entries(
        { minEntryId: assistant, maxEntryId: assistant },
        1,
        undefined,
        callContext,
      );
      assert.equal(page.items[0].kind, "pi.assistant");
      assert.deepEqual(results, []);
      assert.equal(
        (await setup.harness.snapshot(LiveDoc, conversation.id, callContext)).generation,
        undefined,
        "A committed answer cannot be salvaged again from a live partial on recovery",
      );
      calls += 1;
      if (calls === 1) {
        queued = await conversation.submit(
          { type: "input", content: "follow", whenBusy: "followUp" },
          context,
        );
        return { action: "continue" };
      }
      return { action: "end" };
    },
  });
  ({ conversation } = setup);
  try {
    setup.faux.setResponses([
      response(requests, "one"),
      response(requests, "two"),
      response(requests, "three"),
    ]);
    const input = await conversation.submit({ type: "input", content: "hi" }, context);
    assert.equal((await deadline(input.wait(context))).status, "done");
    assert.equal((await deadline(queued.wait(context))).status, "done");
    assert.equal(calls, TEXT_TURN_COUNT);
    assert.deepEqual(requests[1].filter((message) => message.role !== "system").map(messageText), [
      "hi",
      "one",
    ]);
    assert.deepEqual((await messages(conversation)).map(messageText), [
      "hi",
      "one",
      "two",
      "follow",
      "three",
    ]);
    assert.deepEqual(setup.reports, []);
  } finally {
    await setup.harness.close(context);
  }
});

it("end after tools sees committed results and sends no additional request", async () => {
  let executions = 0;
  let hooks = 0;
  const tool = defineTool({
    name: "once",
    description: "Test effect",
    parameters: schema.obj({}),
    execute: () => {
      executions += 1;
      return { output: [{ type: "text", text: "effect" }] };
    },
  });
  const setup = await open(
    {
      onCommittedTurn: async ({ assistant, results }, api, callContext) => {
        hooks += 1;
        assert.equal(results.length, 1);
        const page = await api.entries(
          { minEntryId: assistant },
          ENTRY_PAGE_SIZE,
          undefined,
          callContext,
        );
        const entries = [...page.items].reverse();
        assert.deepEqual(
          entries.map((entry) => entry.kind),
          ["pi.assistant", "pi.tool-result"],
        );
        return { action: "end" };
      },
    },
    tool,
  );
  try {
    setup.faux.setResponses([
      fauxAssistantMessage([{ type: "toolCall", id: "once-call", name: "once", arguments: {} }], {
        stopReason: "toolUse",
      }),
    ]);
    const input = await setup.conversation.submit({ type: "input", content: "act" }, context);
    assert.equal(
      (await deadline(input.wait(context))).status,
      "done",
      setup.reports.map(String).join("\n"),
    );
    assert.equal(executions, 1);
    assert.equal(hooks, 1);
    assert.deepEqual(setup.reports, []);
    assert.deepEqual(
      (await messages(setup.conversation)).map((message) => message.role),
      ["user", "assistant", "toolResult"],
    );
  } finally {
    await setup.harness.close(context);
  }
});

it("missing decision fails closed after committing the answer", async () => {
  let calls = 0;
  const setup = await open({
    onCommittedTurn: () => {
      calls += 1;
    },
  });
  try {
    setup.faux.setResponses([fauxAssistantMessage("kept")]);
    const input = await setup.conversation.submit({ type: "input", content: "hi" }, context);
    assert.equal((await deadline(input.wait(context))).status, "unanswered");
    assert.equal(calls, 1);
    assert.deepEqual((await messages(setup.conversation)).map(messageText), ["hi", "kept"]);
    assert.ok(
      setup.reports.some((error) =>
        String(error).includes("onCommittedTurn must return end or continue"),
      ),
    );
  } finally {
    await setup.harness.close(context);
  }
});

it("stored decision recovery skips callback and does not recreate tools", async () => {
  const live = {
    run: { taskId: 7, inputs: [], turns: 1 },
    tools: [{ callId: "once", taskId: 8, entry: 11 }],
  };
  let commits = 0;
  let hooks = 0;
  const tx = {
    latestHeadMarker: async () => undefined,
    doc: async (definition) => (definition.definition.kind === "pi.inbox" ? { items: [] } : live),
    settleSubmission: () => {
      throw new Error("The recovered turn has no unsettled submissions");
    },
    createTask: () => {
      throw new Error("Recovery cannot start another generation on end");
    },
  };
  const runtime = {
    conversationId: 1,
    taskId: 7,
    settings: { steeringMode: "one-at-a-time", followUpMode: "one-at-a-time" },
    now: () => 1,
    agent: async () => ({ extensions: [] }),
    outcomes: async () => [{ status: "completed", result: {} }],
    snapshot: async () => live,
    hooks: {
      each: () => {
        hooks += 1;
      },
    },
    commit: (fn) => {
      commits += 1;
      return fn(tx);
    },
  };
  const task = {
    state: {
      checkpoint: { phase: "turnBoundary", assistant: 10, tools: [TOOL_TASK_ID], action: "end" },
    },
  };
  await GenerationTask.definition.phases.turnBoundary(task, runtime, context);
  assert.equal(commits, 1);
  assert.equal(hooks, 0);
  assert.deepEqual(live, {});
});

it("unfinished callback recovery reuses committed entry IDs then stores its decision", async () => {
  const calls = [];
  let next;
  const runtime = {
    conversationId: 1,
    hooks: {
      each: async (name, invoke) => {
        if (name === "onCommittedTurn") {
          await invoke((turn) => {
            calls.push(turn);
            return { action: "end" };
          });
        }
      },
    },
    commit: async (fn) => {
      next = await fn({});
    },
  };
  const task = { state: { checkpoint: { phase: "committedTurn", assistant: 10 } } };
  await GenerationTask.definition.phases.committedTurn(task, runtime, context);
  await GenerationTask.definition.phases.committedTurn(task, runtime, context);
  assert.deepEqual(calls, [
    { assistant: 10, results: [] },
    { assistant: 10, results: [] },
  ]);
  assert.deepEqual(next, {
    status: "running",
    checkpoint: { phase: "turnBoundary", assistant: 10, action: "end" },
  });
});
