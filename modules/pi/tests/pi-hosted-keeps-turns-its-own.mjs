// pi as the host runs it: the patched seams are there to detect, and a turn its
// durable generation commits is the host's to end or continue, run against the
// built package with a faux provider, so no credential or request leaves it.
// Run as: node THIS PI_ROOT [dist|src], where src reads the patched TypeScript.
import assert from "node:assert/strict";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
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
const durableEntry =
  codeDirectory === "src"
    ? `${root}/packages/durable/src/index.ts`
    : `${root}/node_modules/@earendil-works/pi-durable/dist/index.js`;
const durable = await import(pathToFileURL(durableEntry).href);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const sessionModule = (
  codeDirectory === "src"
    ? [`${root}/packages/coding-agent/src/core/agent-session.ts`]
    : [
        `${root}/node_modules/@earendil-works/pi-coding-agent/dist/core/agent-session.js`,
        `${root}/packages/coding-agent/dist/core/agent-session.js`,
        `${root}/dist/core/agent-session.js`,
      ]
).find((path) => existsSync(path));
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

it("the host can tell both seams are there before it opens a conversation", async () => {
  assert.ok(sessionModule, "the built coding agent's session module is in the tree");
  const { AgentSession } = await import(pathToFileURL(sessionModule).href);
  assert.equal(
    AgentSession.hostAgentInterfaceVersion,
    1,
    "the session takes any agent by its shape",
  );
  assert.equal(GenerationTask.definition.version, 3, "the generation keeps the committed turn");
  for (const phase of ["committedTurn", "turnBoundary"]) {
    assert.equal(typeof GenerationTask.definition.phases[phase], "function", `${phase} is a phase`);
  }
});

it("the host builds the session's agent from the options pi gives its own", async () => {
  const extension = codeDirectory === "src" ? "ts" : "js";
  const core = dirname(sessionModule);
  const { createAgentSession } = await import(pathToFileURL(`${core}/sdk.${extension}`).href);
  const { SessionManager } = await import(
    pathToFileURL(`${core}/session-manager.${extension}`).href
  );
  const { SettingsManager } = await import(
    pathToFileURL(`${core}/settings-manager.${extension}`).href
  );
  const { Agent } = await import(
    pathToFileURL(`${root}/node_modules/@earendil-works/pi-agent-core/dist/index.js`).href
  );
  const cwd = mkdtempSync(join(tmpdir(), "pi-host-agent-"));
  let given;
  let built;
  const { session } = await createAgentSession({
    cwd,
    agentDir: join(cwd, "agent"),
    sessionManager: SessionManager.inMemory(cwd),
    settingsManager: SettingsManager.inMemory(),
    noTools: "all",
    createAgent: (options) => {
      given = options;
      built = new Agent(options);
      return built;
    },
  });
  assert.equal(session.agent, built, "the session drives the agent the host built");
  for (const name of ["streamFn", "convertToLlm", "transformContext", "onPayload"]) {
    assert.equal(typeof given[name], "function", `pi's own ${name} reaches the host`);
  }
  assert.equal(given.sessionId, session.sessionManager.getSessionId(), "the session id is pi's");
  session.dispose();
});

it("words a host's agent queued show as pending, and a dequeue clears them", async () => {
  const extension = codeDirectory === "src" ? "ts" : "js";
  const core = dirname(sessionModule);
  const { AgentSession } = await import(pathToFileURL(sessionModule).href);
  const { createAgentSession } = await import(pathToFileURL(`${core}/sdk.${extension}`).href);
  const { SessionManager } = await import(
    pathToFileURL(`${core}/session-manager.${extension}`).href
  );
  const { SettingsManager } = await import(
    pathToFileURL(`${core}/settings-manager.${extension}`).href
  );
  assert.equal(AgentSession.hostQueueDisplay, true, "the host can tell its words will show");
  const cwd = mkdtempSync(join(tmpdir(), "pi-host-queue-"));
  const { session } = await createAgentSession({
    cwd,
    agentDir: join(cwd, "agent"),
    sessionManager: SessionManager.inMemory(cwd),
    settingsManager: SettingsManager.inMemory(),
    noTools: "all",
  });
  const updates = [];
  session.subscribe((event) => {
    if (event.type === "queue_update") {
      updates.push(event);
    }
  });
  session.showQueued("sent as it reopened", "followUp");
  session.showQueued("and a steer", "steer");
  assert.deepEqual(session.getFollowUpMessages(), ["sent as it reopened"]);
  assert.deepEqual(session.getSteeringMessages(), ["and a steer"]);
  assert.equal(session.pendingMessageCount, 2, "both count as pending");
  assert.deepEqual(
    updates.at(-1),
    { type: "queue_update", steering: ["and a steer"], followUp: ["sent as it reopened"] },
    "the screen hears of them as it hears of pi's own",
  );
  assert.equal(session.agent.hasQueuedMessages(), false, "the agent isn't given them again");
  assert.deepEqual(session.clearQueue(), {
    steering: ["and a steer"],
    followUp: ["sent as it reopened"],
  });
  assert.equal(session.pendingMessageCount, 0, "a dequeue clears them");
  assert.deepEqual(updates.at(-1), { type: "queue_update", steering: [], followUp: [] });
  session.dispose();
});

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
