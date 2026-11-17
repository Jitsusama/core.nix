// pi as the host runs it: a run the host's agent starts itself, with no
// starting message, is prepared by extensions and settled by pi as a
// prompt's run is, against the built package with a faux provider, so no
// credential or request leaves it.
// Run as: node THIS PI_ROOT [dist|src], where src reads the patched TypeScript.
import assert from "node:assert/strict";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test as it } from "node:test";
import { pathToFileURL } from "node:url";

const SETTLE_TIMEOUT_MS = 5000;
const RESIDENT_SECTION = /<resident>/u;
const CARRIED_NOTE = "noted on the carry";
const BEFORE_RESTART = /before the restart/u;
const AFTER_RESTART = /after the restart/u;
const FAILED_START = /the start failed/u;
const HELD_TEXT = /sent as the run started/u;
const root = resolve(process.argv[2]);
const codeDirectory = process.argv[3] ?? "dist";
assert.ok(
  ["src", "dist"].includes(codeDirectory),
  "Choose src for a source proof or dist for the host build",
);
const extension = codeDirectory === "src" ? "ts" : "js";
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
assert.ok(sessionModule, "the built coding agent's session module is in the tree");
const core = dirname(sessionModule);
const load = (path) => import(pathToFileURL(path).href);
const { AgentSession } = await load(sessionModule);
const { createAgentSession } = await load(`${core}/sdk.${extension}`);
const { SessionManager } = await load(`${core}/session-manager.${extension}`);
const { SettingsManager } = await load(`${core}/settings-manager.${extension}`);
const { DefaultResourceLoader } = await load(`${core}/resource-loader.${extension}`);
const { Agent } = await load(`${root}/node_modules/@earendil-works/pi-agent-core/dist/index.js`);
const compat = await load(`${root}/node_modules/@earendil-works/pi-ai/dist/compat.js`);
const { fauxAssistantMessage, fauxToolCall, getCurrentTools } = await load(
  `${root}/node_modules/@earendil-works/pi-ai/dist/index.js`,
);

function deadline(promise) {
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      const timer = setTimeout(
        () => reject(new Error("The run did not settle in five seconds")),
        SETTLE_TIMEOUT_MS,
      );
      timer.unref();
    }),
  ]);
}

// An extension that adds a section to every run it prepares, returns a
// message for a carried run, and notes each run it hears start and settle.
// Told to, it takes every tool from a carried run by editing its options.
function preparesEveryRun(heard, sessionOf, { dropsCarriedTools = false } = {}) {
  return {
    name: "prepares-every-run",
    factory: (pi) => {
      pi.on("before_agent_start", (event) => {
        heard.starts.push(event.messageType);
        heard.busy.push(sessionOf().isStreaming);
        event.systemPromptOptions.sections = {
          ...event.systemPromptOptions.sections,
          resident: "kept for every run",
        };
        if (event.messageType === "continue") {
          if (dropsCarriedTools) {
            event.systemPromptOptions.selectedTools = [];
          }
          return { message: { customType: "carried-note", content: CARRIED_NOTE } };
        }
      });
      pi.on("agent_settled", (event) => {
        heard.settles.push(event.aborted);
      });
    },
  };
}

// A tool active only when named or restored, as an MCP server's tools are,
// whose extension registers it at load or, when told, only later, as one
// whose server is still connecting.
const LATE_TOOL = {
  name: "late",
  label: "late",
  description: "A tool that registers late",
  parameters: { type: "object", properties: {} },
  defaultActive: false,
  execute: async () => ({ content: [{ type: "text", text: "late" }], details: {} }),
};

function registersLate({ atLoad }) {
  let register;
  return {
    registerNow: () => register(),
    extension: {
      name: "registers-late",
      factory: (pi) => {
        register = () => pi.registerTool(LATE_TOOL);
        if (atLoad) {
          register();
        }
      },
    },
  };
}

// A session with that one extension and any others given, answering from the
// faux provider through the agent the host builds, so each request's
// messages, prompt and offered tools are kept. It has no tools unless given
// some by name, or told to restore its transcript's as pi does by default.
async function openPreparedSession({
  tools,
  dropsCarriedTools,
  restoresTools = false,
  sessionManager,
  extensions = [],
} = {}) {
  const cwd = mkdtempSync(join(tmpdir(), "pi-host-runs-"));
  const agentDir = join(cwd, "agent");
  const settingsManager = SettingsManager.inMemory();
  const faux = compat.registerFauxProvider();
  const heard = { starts: [], settles: [], busy: [] };
  const requests = [];
  let session;
  const resourceLoader = new DefaultResourceLoader({
    cwd,
    agentDir,
    settingsManager,
    noSkills: true,
    noPromptTemplates: true,
    noThemes: true,
    noContextFiles: true,
    extensionFactories: [
      preparesEveryRun(heard, () => session, { dropsCarriedTools }),
      ...extensions,
    ],
  });
  await resourceLoader.reload();
  ({ session } = await createAgentSession({
    cwd,
    agentDir,
    model: faux.getModel(),
    sessionManager: sessionManager ?? SessionManager.inMemory(cwd),
    settingsManager,
    resourceLoader,
    ...(tools ? { tools } : restoresTools ? {} : { noTools: "all" }),
    createAgent: (options) =>
      new Agent({
        ...options,
        streamFn: (model, request, streamOptions) => {
          requests.push({
            messages: request.messages,
            prompt: session.systemPrompt,
            // The model is offered the tools the transcript's system messages add up to.
            tools: getCurrentTools(request.messages).map((tool) => tool.name),
          });
          return compat.streamSimple(model, request, { ...streamOptions, apiKey: "faux" });
        },
      }),
  }));
  const close = () => {
    session.dispose();
    faux.unregister();
  };
  return { session, faux, heard, requests, close };
}

const userMessage = (text) => ({
  role: "user",
  content: [{ type: "text", text }],
  timestamp: Date.now(),
});

const systemMessages = (request) => request.messages.filter((message) => message.role === "system");

const requestTexts = (request) =>
  request.messages.flatMap((message) =>
    typeof message.content === "string"
      ? [message.content]
      : message.content.filter((part) => part.type === "text").map((part) => part.text),
  );

// pi's own prepared run, as a host's typed words or an extension's message start one.
const promptRun = (session, text) =>
  deadline(
    session.sendCustomMessage(
      { customType: "typed", content: text, display: true },
      { triggerTurn: true },
    ),
  );

it("the host can tell its runtime runs its agent's own turns through pi", () => {
  assert.equal(AgentSession.hostRuns, true);
  assert.equal(typeof AgentSession.prototype.runForHost, "function");
});

it("a run the host's agent starts itself is prepared and settled as a prompt's run is", async () => {
  const { session, faux, heard, requests, close } = await openPreparedSession();
  try {
    faux.setResponses([
      fauxAssistantMessage("answered as typed"),
      fauxAssistantMessage("answered as carried"),
    ]);
    await promptRun(session, "typed words");
    let busyInWork;
    let handed;
    await deadline(
      session.runForHost(async (prepared) => {
        busyInWork = session.isStreaming;
        handed = prepared;
        await session.agent.prompt(userMessage("asked again"));
      }),
    );
    assert.equal(busyInWork, true, "the session is busy through the host's work");
    assert.deepEqual(handed, [], "an unchanged prompt hands the work nothing to place");
    assert.equal(session.isStreaming, false, "and idle once it ends");
    assert.deepEqual(heard.starts, ["custom", "continue"], "each run is prepared once");
    assert.deepEqual(heard.busy, [true, true], "each is busy from before it is prepared");
    assert.deepEqual(heard.settles, [false, false], "each run settles once");
    assert.equal(requests.length, 2);
    assert.match(requests[0].prompt, RESIDENT_SECTION, "a prompt's run holds the section");
    assert.equal(
      requests[1].prompt,
      requests[0].prompt,
      "the host's run sends the same prompt, its sections included",
    );
    assert.deepEqual(
      systemMessages(requests[1]),
      systemMessages(requests[0]),
      "and records no change to it, so the provider's cache holds",
    );
  } finally {
    close();
  }
});

it("a handler's message for a carried run waits for the next prompt", async () => {
  const { session, faux, requests, close } = await openPreparedSession();
  try {
    faux.setResponses([fauxAssistantMessage("answered as carried"), fauxAssistantMessage("next")]);
    await deadline(session.runForHost(() => session.agent.prompt(userMessage("carried on"))));
    assert.ok(
      !requestTexts(requests[0]).includes(CARRIED_NOTE),
      "it lands nowhere inside the carried run",
    );
    await promptRun(session, "next words");
    assert.ok(requestTexts(requests[1]).includes(CARRIED_NOTE), "it comes with the next prompt");
  } finally {
    close();
  }
});

it("a handler's edit to a carried run's tools decides what the provider is offered", async () => {
  const { session, faux, requests, close } = await openPreparedSession({
    tools: ["read"],
    dropsCarriedTools: true,
  });
  try {
    faux.setResponses([
      fauxAssistantMessage("answered as typed"),
      fauxAssistantMessage("answered as carried"),
    ]);
    await promptRun(session, "typed words");
    await deadline(session.runForHost(() => session.agent.prompt(userMessage("carried on"))));
    assert.deepEqual(requests[0].tools, ["read"], "a prompt's run offers the session's tool");
    assert.deepEqual(
      requests[1].tools,
      [],
      "the carried run offers none, as its handler chose, as a prompt's run would",
    );
  } finally {
    close();
  }
});

// A conversation whose transcript holds the late tool, recorded by a prompt's
// run while that tool was at hand.
async function transcriptWithLateTool() {
  const first = await openPreparedSession({
    tools: [LATE_TOOL.name],
    extensions: [registersLate({ atLoad: true }).extension],
  });
  try {
    first.faux.setResponses([fauxAssistantMessage("answered with late at hand")]);
    await promptRun(first.session, "typed words");
    assert.deepEqual(first.session.getActiveToolNames(), [LATE_TOOL.name]);
    return first.session.sessionManager;
  } finally {
    first.close();
  }
}

async function reopenWithLateTool(sessionManager) {
  const late = registersLate({ atLoad: false });
  const opened = await openPreparedSession({
    restoresTools: true,
    sessionManager,
    extensions: [late.extension],
  });
  return { ...opened, registerLate: late.registerNow };
}

it("a restored tool that registers before any run becomes active", async () => {
  const { session, registerLate, close } = await reopenWithLateTool(await transcriptWithLateTool());
  try {
    registerLate();
    assert.deepEqual(session.getActiveToolNames(), [LATE_TOOL.name]);
  } finally {
    close();
  }
});

it("a carried run, as a prompt's, leaves a restored tool that registers after it starts inactive", async () => {
  const { session, faux, registerLate, close } = await reopenWithLateTool(
    await transcriptWithLateTool(),
  );
  try {
    faux.setResponses([fauxAssistantMessage("answered as carried")]);
    await deadline(session.runForHost(() => session.agent.prompt(userMessage("carried on"))));
    registerLate();
    assert.deepEqual(
      session.getActiveToolNames(),
      [],
      "the run recorded its loadout, so a tool registering after it is not restored into it",
    );
  } finally {
    close();
  }
});

// An extension whose section says what it is given, as one an upgrade changes.
const addsSection = (changing) => ({
  name: "changes-section",
  factory: (pi) => {
    pi.on("before_agent_start", (event) => {
      event.systemPromptOptions.sections = { ...event.systemPromptOptions.sections, changing };
    });
  },
});

// Sends a context-only message as a run starts, while pi holds it, then starts or fails.
const sendsAsItStarts = (session, { fails = false } = {}) => {
  const start = session._extensionRunner.emitBeforeAgentStart.bind(session._extensionRunner);
  session._extensionRunner.emitBeforeAgentStart = async (...args) => {
    session._extensionRunner.emitBeforeAgentStart = start;
    await session.sendCustomMessage(
      { customType: "held", content: "sent as the run started", display: false },
      { triggerTurn: false },
    );
    // No handler's error escapes pi's runner, so a start is failed from inside it.
    if (fails) {
      throw new Error("the start failed");
    }
    return start(...args);
  };
};

// A host's carried run, placing what it is handed ahead of its first new message.
const carry = (session) =>
  deadline(
    session.runForHost((prepared) =>
      session.agent.prompt([...(prepared ?? []), userMessage("carried on")]),
    ),
  );

// A carried run that takes a tool round, as a turn a restart cut off does, then answers.
const carryThroughToolRound = (session, faux) => {
  faux.setResponses([
    fauxAssistantMessage(fauxToolCall("read", { path: "absent" }), { stopReason: "toolUse" }),
    fauxAssistantMessage("answered as carried"),
  ]);
  return carry(session);
};

const indexOf = (session, matches) => session.messages.findIndex(matches);
const isHeld = (message) => message.customType === "held";
const isToolResult = (message) => message.role === "toolResult";

// How many of a request's system messages hold the text.
const systemMessagesHolding = (request, text) =>
  systemMessages(request).filter((message) => text.test(JSON.stringify(message))).length;

it("a carried run's first request holds the sections as they stand after a restart", async () => {
  const before = await openPreparedSession({
    tools: ["read"],
    extensions: [addsSection("before the restart")],
  });
  try {
    before.faux.setResponses([fauxAssistantMessage("answered as typed")]);
    await promptRun(before.session, "typed words");
    assert.match(JSON.stringify(systemMessages(before.requests[0])), BEFORE_RESTART);
  } finally {
    before.close();
  }
  // The restart reopens the transcript under an upgrade that changed what the extension adds.
  const { session, faux, requests, close } = await openPreparedSession({
    tools: ["read"],
    sessionManager: before.session.sessionManager,
    extensions: [addsSection("after the restart")],
  });
  try {
    await carryThroughToolRound(session, faux);
    assert.equal(requests.length, 2, "the carried run takes a tool round and a second turn");
    assert.equal(
      systemMessagesHolding(requests[0], AFTER_RESTART),
      1,
      "the work is handed the update and places it, so the first request is current",
    );
    assert.equal(
      systemMessagesHolding(requests[1], AFTER_RESTART),
      1,
      "and the next turn's refresh finds it recorded, so it is sent once",
    );
  } finally {
    close();
  }
});

it("a message sent as a carried run starts lands after its first turn's tool result", async () => {
  const { session, faux, requests, close } = await openPreparedSession({ tools: ["read"] });
  try {
    sendsAsItStarts(session);
    await carryThroughToolRound(session, faux);
    assert.doesNotMatch(JSON.stringify(requests[0].messages), HELD_TEXT);
    // It waits for the turn to end, so it never splits a tool call from its result.
    assert.ok(indexOf(session, isHeld) > indexOf(session, isToolResult));
  } finally {
    close();
  }
});

it("a carried run that fails to start keeps what it held for the next turn", async () => {
  const { session, faux, close } = await openPreparedSession({ tools: ["read"] });
  try {
    sendsAsItStarts(session, { fails: true });
    await assert.rejects(deadline(session.runForHost(() => Promise.resolve())), FAILED_START);
    assert.equal(session.isStreaming, false);
    // The transcript it would carry on may owe a tool result, so nothing goes after it yet.
    assert.equal(indexOf(session, isHeld), -1);
    await carryThroughToolRound(session, faux);
    // The next run's turn places it once its tool result is in.
    assert.ok(indexOf(session, isHeld) > indexOf(session, isToolResult));
  } finally {
    close();
  }
});

it("a handler's setActiveTools() holds for a carried run", async () => {
  const narrows = (pi) =>
    pi.on("before_agent_start", () => {
      pi.setActiveTools([]);
    });
  const opened = { tools: ["read"], extensions: [{ name: "narrows", factory: narrows }] };
  const { session, faux, requests, close } = await openPreparedSession(opened);
  try {
    faux.setResponses([fauxAssistantMessage("answered as carried")]);
    await carry(session);
    assert.deepEqual(requests[0].tools, [], "the live loadout the handler set is what is offered");
  } finally {
    close();
  }
});

it("a host's work inside a run of pi's own joins that run", async () => {
  const { session, heard, close } = await openPreparedSession();
  try {
    let joined = false;
    let handedInside;
    await deadline(
      session.runForHost(() =>
        session.runForHost((prepared) => {
          joined = session.isStreaming;
          handedInside = prepared;
          return Promise.resolve();
        }),
      ),
    );
    assert.equal(joined, true, "the inner work ran inside the run");
    assert.deepEqual(handedInside, [], "and is handed nothing, since pi's run placed it");
    assert.deepEqual(heard.starts, ["continue"], "the inner work prepares no run of its own");
    assert.deepEqual(heard.settles, [false], "and the run settles once");
  } finally {
    close();
  }
});
