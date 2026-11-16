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
const { fauxAssistantMessage } = await load(
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
function preparesEveryRun(heard, sessionOf) {
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
          return { message: { customType: "carried-note", content: CARRIED_NOTE } };
        }
      });
      pi.on("agent_settled", (event) => {
        heard.settles.push(event.aborted);
      });
    },
  };
}

// A session with that one extension, answering from the faux provider through
// the agent the host builds, so each request's messages and prompt are kept.
async function openPreparedSession() {
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
    extensionFactories: [preparesEveryRun(heard, () => session)],
  });
  await resourceLoader.reload();
  ({ session } = await createAgentSession({
    cwd,
    agentDir,
    model: faux.getModel(),
    sessionManager: SessionManager.inMemory(cwd),
    settingsManager,
    resourceLoader,
    noTools: "all",
    createAgent: (options) =>
      new Agent({
        ...options,
        streamFn: (model, request, streamOptions) => {
          requests.push({ messages: request.messages, prompt: session.systemPrompt });
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
    await deadline(
      session.runForHost(async () => {
        busyInWork = session.isStreaming;
        await session.agent.prompt(userMessage("asked again"));
      }),
    );
    assert.equal(busyInWork, true, "the session is busy through the host's work");
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

it("a host's work inside a run of pi's own joins that run", async () => {
  const { session, heard, close } = await openPreparedSession();
  try {
    let joined = false;
    await deadline(
      session.runForHost(() =>
        session.runForHost(() => {
          joined = session.isStreaming;
          return Promise.resolve();
        }),
      ),
    );
    assert.equal(joined, true, "the inner work ran inside the run");
    assert.deepEqual(heard.starts, ["continue"], "the inner work prepares no run of its own");
    assert.deepEqual(heard.settles, [false], "and the run settles once");
  } finally {
    close();
  }
});
