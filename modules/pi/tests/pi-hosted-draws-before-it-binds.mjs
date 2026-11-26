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
// conversation's, which a session_start may add to while it binds; `steps`
// lists, in order, each draw, each frame sent and the bind.
function startingScreen(hosted, entries = [entry("said"), entry("answered")]) {
  const steps = [];
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
      steps.push("bind");
      reached();
      return held;
    }),
    subscribeToAgent: mock.fn(),
    updateAvailableProviderCount: () => Promise.resolve(),
    updateEditorBorderColor: mock.fn(),
    updateTerminalTitle: mock.fn(),
    renderInitialMessages: mock.fn(() => steps.push("drawn")),
    renderSessionEntries: mock.fn(),
    rebuildChatFromMessages: mock.fn(),
    footerDataProvider: { onBranchChange: mock.fn() },
    themeController: {
      applyFromSettings: mock.fn(),
      disableAutoSync: mock.fn(),
      waitForTerminalColors: () => Promise.resolve(),
    },
    editor: { getText: () => "", setText: mock.fn(), addToHistory: mock.fn() },
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
      renderNow: mock.fn(() => steps.push("sent")),
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
  return { mode, bound, binding, entries, steps };
}

/** Starts the screen with fd and rg on no path, offline, so it downloads nothing. */
function starting(mode) {
  process.env.PATH = "";
  process.env.PI_OFFLINE = "1";
  return mode.init();
}

it("a hosted screen draws its conversation while its extensions still bind", async () => {
  const { mode, bound, binding, entries, steps } = startingScreen(true);
  const started = starting(mode);
  // Once the bind has begun, and while it is held, the window shows the
  // conversation, drawn and sent before the bind began; it hears the agent
  // only once its extensions are live.
  await binding;
  assert.deepEqual(
    steps,
    ["drawn", "sent", "bind"],
    "the window was blank, or its frame unsent, while its extensions bound",
  );
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
  assert.deepEqual(mode.renderSessionEntries.mock.calls[0].arguments[1], {
    updateFooter: true,
    populateHistory: true,
  });
  assert.equal(mode.rebuildChatFromMessages.mock.callCount(), 0, "it drew it all again");
  // The patch's own lines clear nothing that holds what was drawn; the
  // renderers that would are mocks here.
  for (const part of ["chatContainer", "loadedResourcesContainer"]) {
    assert.equal(mode[part].clear.mock.callCount(), 0, `startup cleared the ${part}`);
  }
  assert.equal(mode.subscribeToAgent.mock.callCount(), 1);
  mode.themeChangeUnsubscribe?.();
});

it("a hosted screen draws nothing more when the bind left its conversation as it was", async () => {
  const { mode, bound, binding } = startingScreen(true);
  const started = starting(mode);
  await binding;
  bound();
  await started;
  assert.equal(mode.renderInitialMessages.mock.callCount(), 1);
  assert.equal(mode.renderSessionEntries.mock.callCount(), 0, "it drew an empty tail");
  assert.equal(mode.rebuildChatFromMessages.mock.callCount(), 0, "it drew it all again");
  mode.themeChangeUnsubscribe?.();
});

// Each way a bind can do more than add: a compaction, which leaves the
// conversation longer than what was drawn though its start is another, and
// a shorter branch, which leaves nothing past what was drawn.
for (const [did, change] of [
  [
    "compacted",
    (entries) =>
      entries.splice(
        0,
        entries.length,
        entry("a compaction's summary"),
        entry("kept"),
        entry("next"),
      ),
  ],
  ["moved to a shorter branch", (entries) => entries.splice(1)],
]) {
  it(`a hosted screen draws its conversation again when the bind ${did}`, async () => {
    const { mode, bound, binding, entries } = startingScreen(true);
    const started = starting(mode);
    await binding;
    change(entries);
    bound();
    await started;
    assert.equal(mode.renderInitialMessages.mock.callCount(), 1);
    assert.equal(mode.renderSessionEntries.mock.callCount(), 0, "it drew a slice of another start");
    assert.equal(
      mode.rebuildChatFromMessages.mock.callCount(),
      1,
      "the window kept a conversation the agent no longer holds",
    );
    mode.themeChangeUnsubscribe?.();
  });
}

it("a hosted screen gives the editor the early draw's prompts in order", async () => {
  const { mode, bound, binding } = startingScreen(true);
  const history = mode.editor.addToHistory;
  mode.renderInitialMessages.mock.mockImplementation(() => {
    mode.editor.addToHistory("first");
    mode.editor.addToHistory("second");
  });
  const started = starting(mode);
  await binding;
  assert.equal(mode.editor.addToHistory, history, "the editor kept the draw's stand-in");
  assert.deepEqual(
    history.mock.calls.map((call) => call.arguments[0]),
    ["first", "second"],
    "the early draw's prompts never reached the editor's history",
  );
  bound();
  await started;
  mode.themeChangeUnsubscribe?.();
});

it("a hosted screen keeps its early draw when a call awaits a result not added", async () => {
  const { mode, bound, binding } = startingScreen(true);
  // After a kill mid-tool, the conversation ends on a call with no result,
  // and a bind that adds nothing has nothing to match it with.
  mode.renderInitialMessages.mock.mockImplementation(() =>
    mode.pendingTools.set("call", { waiting: true }),
  );
  const started = starting(mode);
  await binding;
  bound();
  await started;
  assert.equal(mode.renderSessionEntries.mock.callCount(), 0, "it drew an empty tail");
  assert.equal(
    mode.rebuildChatFromMessages.mock.callCount(),
    0,
    "it drew it all again, dropping startup's notices",
  );
  mode.themeChangeUnsubscribe?.();
});

it("a hosted screen draws it all again when a drawn tool call awaits its result", async () => {
  const { mode, bound, binding, entries } = startingScreen(true);
  // The early draw leaves a tool call waiting on its result, as pi's own
  // draw keeps it among its pending tools.
  mode.renderInitialMessages.mock.mockImplementation(() =>
    mode.pendingTools.set("call", { waiting: true }),
  );
  const started = starting(mode);
  await binding;
  entries.push(entry("the tool's result"));
  bound();
  await started;
  assert.equal(
    mode.renderSessionEntries.mock.callCount(),
    0,
    "a result was drawn apart from its call",
  );
  assert.equal(mode.rebuildChatFromMessages.mock.callCount(), 1);
  mode.themeChangeUnsubscribe?.();
});

it("a hosted screen whose early draw throws draws after the bind, as startup does", async () => {
  const { mode, bound, binding, steps } = startingScreen(true);
  // Startup showed a notice before the draw. A renderer wanting state its
  // session_start loads throws the first time, part way through the draw.
  const notice = { notice: true };
  mode.chatContainer.children.push(notice);
  const history = mode.editor.addToHistory;
  mode.renderInitialMessages.mock.mockImplementationOnce(() => {
    mode.chatContainer.children.push({ drawn: true });
    mode.editor.addToHistory("said");
    throw new Error("no state yet");
  });
  const said = mock.method(process.stderr, "write", () => true);
  const started = starting(mode);
  await binding;
  assert.deepEqual(
    mode.chatContainer.children,
    [notice],
    "half a draw was left on screen, or startup's notice went with it",
  );
  assert.equal(mode.editor.addToHistory, history, "the editor kept the draw's stand-in");
  assert.equal(history.mock.callCount(), 0, "half a draw's prompt reached the history");
  bound();
  await started;
  // Startup's own flush sends the frame once it is done, as stock pi does.
  assert.deepEqual(steps, ["bind", "drawn", "sent"], "it was not drawn after the bind");
  assert.equal(mode.renderSessionEntries.mock.callCount(), 0);
  assert.equal(mode.rebuildChatFromMessages.mock.callCount(), 0);
  assert.equal(mode.subscribeToAgent.mock.callCount(), 1, "startup failed for a renderer");
  assert.deepEqual(
    said.mock.calls.map((call) => call.arguments[0]),
    ["pi-host: the early draw threw, so it draws after the bind: Error: no state yet\n"],
    "it didn't say once why the window waited for the bind",
  );
  mode.themeChangeUnsubscribe?.();
});

it("a hosted screen whose early frame throws draws after the bind", async () => {
  const { mode, bound, binding, steps } = startingScreen(true);
  mode.ui.renderNow.mock.mockImplementationOnce(() => {
    throw new Error("a renderer wants its state");
  });
  const said = mock.method(process.stderr, "write", () => true);
  const started = starting(mode);
  await binding;
  bound();
  await started;
  assert.deepEqual(steps, ["drawn", "bind", "drawn", "sent"], "it was not drawn after the bind");
  assert.equal(mode.renderSessionEntries.mock.callCount(), 0);
  assert.equal(said.mock.callCount(), 1, "it didn't say once why the window waited for the bind");
  mode.themeChangeUnsubscribe?.();
});

it("stands in only for parts pi's screen has", () => {
  // The screen above mocks these; were pi to rename one, the stub would
  // test a part startup no longer calls.
  for (const part of [
    "renderInitialMessages",
    "renderSessionEntries",
    "rebuildChatFromMessages",
    "subscribeToAgent",
  ]) {
    assert.equal(typeof InteractiveMode.prototype[part], "function", `pi's screen has no ${part}`);
  }
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
