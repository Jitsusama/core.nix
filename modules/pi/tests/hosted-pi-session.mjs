// The hosted pi the carried-run tests drive, not a test itself: pi loaded
// from the tree its test is run on (node TEST PI_ROOT [dist|src]), and a
// session prepared by an extension that notes every run, answering from a
// faux provider so no credential or request leaves it.
import assert from "node:assert/strict";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const SETTLE_TIMEOUT_MS = 5000;
/** The message the noting extension returns for a carried run. */
export const CARRIED_NOTE = "noted on the carry";
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
export const { AgentSession } = await load(sessionModule);
const { createAgentSession } = await load(`${core}/sdk.${extension}`);
const { SessionManager } = await load(`${core}/session-manager.${extension}`);
const { SettingsManager } = await load(`${core}/settings-manager.${extension}`);
const { DefaultResourceLoader } = await load(`${core}/resource-loader.${extension}`);
const { Agent } = await load(`${root}/node_modules/@earendil-works/pi-agent-core/dist/index.js`);
const compat = await load(`${root}/node_modules/@earendil-works/pi-ai/dist/compat.js`);
export const { fauxAssistantMessage, fauxToolCall, getCurrentTools } = await load(
  `${root}/node_modules/@earendil-works/pi-ai/dist/index.js`,
);

/** The promise, or a rejection once it has taken five seconds. */
export function deadline(promise) {
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

/**
 * A tool active only when named or restored, as an MCP server's tools are,
 * whose extension registers it at load or, when told, only later, as one
 * whose server is still connecting.
 */
export const LATE_TOOL = {
  name: "late",
  label: "late",
  description: "A tool that registers late",
  parameters: { type: "object", properties: {} },
  defaultActive: false,
  execute: async () => ({ content: [{ type: "text", text: "late" }], details: {} }),
};

/** The extension that registers the late tool, at load or once told to. */
export function registersLate({ atLoad }) {
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

/**
 * A session with the noting extension and any others given, answering from
 * the faux provider through the agent the host builds, so each request's
 * messages, prompt and offered tools are kept. It has no tools unless given
 * some by name, or told to restore its transcript's as pi does by default.
 */
export async function openPreparedSession({
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

/** A user's message of the text, as the host places it. */
export const userMessage = (text) => ({
  role: "user",
  content: [{ type: "text", text }],
  timestamp: Date.now(),
});

/** A request's system messages. */
export const systemMessages = (request) =>
  request.messages.filter((message) => message.role === "system");

/** Every text a request's messages hold. */
export const requestTexts = (request) =>
  request.messages.flatMap((message) =>
    typeof message.content === "string"
      ? [message.content]
      : message.content.filter((part) => part.type === "text").map((part) => part.text),
  );

/** pi's own prepared run, as a host's typed words or an extension's message start one. */
export const promptRun = (session, text) =>
  deadline(
    session.sendCustomMessage(
      { customType: "typed", content: text, display: true },
      { triggerTurn: true },
    ),
  );

/** A host's carried run, placing what it is handed ahead of its first new message. */
export const carry = (session) =>
  deadline(
    session.runForHost((prepared) =>
      session.agent.prompt([...(prepared ?? []), userMessage("carried on")]),
    ),
  );

/** A carried run that takes a tool round, as a turn a restart cut off does, then answers. */
export const carryThroughToolRound = (session, faux) => {
  faux.setResponses([
    fauxAssistantMessage(fauxToolCall("read", { path: "absent" }), { stopReason: "toolUse" }),
    fauxAssistantMessage("answered as carried"),
  ]);
  return carry(session);
};
