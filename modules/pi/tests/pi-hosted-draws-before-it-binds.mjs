// A hosted screen draws its conversation before its extensions bind, so an
// extension whose session_start never settles, as one reading a stalled
// disk, leaves the window showing what was said rather than blank. Its
// startup runs for real against a fake runtime; no TTY, model request or
// extension is started. Arguments name the package and, optionally, source.
import assert from "node:assert/strict";
import { join } from "node:path";
import { afterEach, mock, test as it } from "node:test";

const [
  ,
  ,
  agentPackage,
  source = join(agentPackage, "dist/modes/interactive/interactive-mode.js"),
] = process.argv;
// Production checks read rebuilt JavaScript. A source override supports the
// red/green prototype against the installed package's own TypeScript loader.
const jiti = source.endsWith(".ts")
  ? (await import(join(agentPackage, "../../node_modules/jiti/lib/jiti.mjs"))).createJiti(
      import.meta.url,
    )
  : undefined;
const { InteractiveMode } = await (jiti ? jiti.import(source) : import(source));

const path = process.env.PATH;
const offline = process.env.PI_OFFLINE;
afterEach(() => {
  process.env.PATH = path;
  if (offline === undefined) {
    delete process.env.PI_OFFLINE;
  } else {
    process.env.PI_OFFLINE = offline;
  }
  mock.restoreAll();
});

/** A conversation's entry, as the session's context lists it. */
const entry = (id) => ({ id, type: "message" });

// A screen whose startup runs for real, with its extensions' bind held
// until `bound` is called, as a session_start on a stalled disk holds it;
// `binding` settles once startup has reached the bind. `entries` are the
// conversation's, which a session_start may add to while it binds.
function startingScreen(hosted, entries = [entry("said"), entry("answered")]) {
  let bound;
  const held = new Promise((resolve) => {
    bound = resolve;
  });
  let reached;
  const binding = new Promise((resolve) => {
    reached = resolve;
  });
  const container = () => ({ clear: mock.fn(), addChild: mock.fn(), children: [] });
  const mode = Object.create(InteractiveMode.prototype);
  Object.assign(mode, {
    options: hosted ? { hosted: { onExit: async () => undefined } } : {},
    isShuttingDown: false,
    isInitialized: false,
    signalCleanupHandlers: [],
    pendingUserInputs: [],
    pendingTools: new Map(),
    stop: mock.fn(),
    showError: mock.fn(),
    showStatus: mock.fn(),
    registerSignalHandlers: mock.fn(),
    getChangelogForDisplay: () => undefined,
    getExtensionChangelogUpdates: () => undefined,
    renderer: { mode: "regular" },
    getModelScopeNotice: () => undefined,
    renderWidgets: mock.fn(),
    mountInteractiveTui: mock.fn(),
    defaultEditor: { onAction: mock.fn() },
    programStatus: { report: mock.fn(), reset: mock.fn() },
    ensurePngTranscoder: mock.fn(),
    shouldShowStartupHeader: () => false,
    headerContainer: container(),
    showManagedToolStatus: () => Promise.resolve(),
    setupKeyHandlers: mock.fn(),
    setupEditorSubmitHandler: mock.fn(),
    applyRuntimeSettings: mock.fn(),
    bindCurrentSessionExtensions: mock.fn(() => {
      reached();
      return held;
    }),
    subscribeToAgent: mock.fn(),
    updateAvailableProviderCount: () => Promise.resolve(),
    updateEditorBorderColor: mock.fn(),
    updateTerminalTitle: mock.fn(),
    renderInitialMessages: mock.fn(),
    renderSessionEntries: mock.fn(),
    rebuildChatFromMessages: mock.fn(),
    footerDataProvider: { onBranchChange: mock.fn() },
    themeController: {
      applyFromSettings: mock.fn(),
      disableAutoSync: mock.fn(),
      waitForTerminalColors: () => Promise.resolve(),
    },
    editor: { getText: () => "", setText: mock.fn() },
    runtimeHost: {
      session: {
        sessionManager: { isPersisted: () => false, buildContextEntries: () => [...entries] },
        settingsManager: { getFullscreenScrollbar: () => "auto" },
      },
    },
    ui: {
      start: mock.fn(),
      stop: mock.fn(),
      setFocus: mock.fn(),
      renderNow: mock.fn(),
      requestRender: mock.fn(),
      invalidate: mock.fn(),
      terminal: { drainInput: mock.fn(), write: mock.fn() },
    },
  });
  for (const part of [
    "documentContainer",
    "loadedResourcesContainer",
    "chatContainer",
    "pendingMessagesContainer",
    "statusContainer",
    "widgetContainerAbove",
    "editorContainer",
    "widgetContainerBelow",
    "footerContainer",
  ]) {
    mode[part] = container();
  }
  return { mode, bound, binding, entries };
}

/** Starts the screen with fd and rg on no path, offline, so it downloads nothing. */
function starting(mode) {
  process.env.PATH = "";
  process.env.PI_OFFLINE = "1";
  return mode.init();
}

it("a hosted screen draws its conversation while its extensions still bind", async () => {
  const { mode, bound, binding, entries } = startingScreen(true);
  const started = starting(mode);
  // Once the bind has begun, and while it is held, the window shows the
  // conversation; it hears the agent only once its extensions are live.
  await binding;
  assert.equal(
    mode.renderInitialMessages.mock.callCount(),
    1,
    "the window was blank while its extensions bound",
  );
  assert.equal(mode.ui.renderNow.mock.callCount(), 1, "the frame waited on the bind");
  assert.equal(mode.subscribeToAgent.mock.callCount(), 0, "it heard the agent before it bound");
  // An extension's session_start adds to the conversation while it binds.
  const note = entry("a session_start's note");
  entries.push(note);
  bound();
  await started;
  // Drawn once whole: after the bind it draws only what the bind added.
  assert.equal(mode.renderInitialMessages.mock.callCount(), 1);
  assert.deepEqual(
    mode.renderSessionEntries.mock.calls.map((call) => call.arguments[0]),
    [[note]],
    "what session_start added was never drawn, or the rest was drawn twice",
  );
  assert.equal(mode.rebuildChatFromMessages.mock.callCount(), 0, "it drew it all again");
  for (const part of ["chatContainer", "loadedResourcesContainer"]) {
    assert.equal(mode[part].clear.mock.callCount(), 0, `startup cleared the ${part}`);
  }
  assert.equal(mode.subscribeToAgent.mock.callCount(), 1);
  mode.themeChangeUnsubscribe?.();
});

it("a hosted screen draws its conversation again when the bind did more than add to it", async () => {
  const { mode, bound, binding, entries } = startingScreen(true);
  const started = starting(mode);
  await binding;
  // A session_start that compacts: what was drawn is no longer the start,
  // though the conversation is now longer than what was drawn.
  entries.splice(0, entries.length, entry("a compaction's summary"), entry("kept"), entry("next"));
  bound();
  await started;
  assert.equal(mode.renderSessionEntries.mock.callCount(), 0, "it drew a slice of another start");
  assert.equal(
    mode.rebuildChatFromMessages.mock.callCount(),
    1,
    "the window kept a conversation the agent no longer holds",
  );
  mode.themeChangeUnsubscribe?.();
});

it("a plain pi still draws its conversation after its extensions bind", async () => {
  const { mode, bound, binding } = startingScreen(false);
  const started = starting(mode);
  // The bind has begun and is held: a plain pi has drawn nothing of the
  // conversation and hears nothing yet.
  await binding;
  assert.equal(mode.renderInitialMessages.mock.callCount(), 0, "a plain pi drew before it bound");
  assert.equal(
    mode.subscribeToAgent.mock.callCount(),
    0,
    "a plain pi heard the agent before it bound",
  );
  bound();
  await started;
  assert.equal(mode.renderInitialMessages.mock.callCount(), 1);
  assert.equal(mode.renderSessionEntries.mock.callCount(), 0, "a plain pi drew a part");
  assert.equal(mode.rebuildChatFromMessages.mock.callCount(), 0, "a plain pi drew it again");
  assert.equal(mode.subscribeToAgent.mock.callCount(), 1);
  mode.themeChangeUnsubscribe?.();
});
