// The host-only InteractiveMode patch keeps a hosted screen inside its own
// session: closing it never ends the process, a copy (a selection's too)
// reaches its viewers' terminals rather than the host's, every screen
// hears a theme change, and a viewer who joins late is sent the images
// already on the screen.
// The real methods run against fake runtimes; no TTY, model request or
// extension is started. Ordinary signal and suspend paths retain their
// process semantics. Arguments name the package and, optionally, source.
import assert from "node:assert/strict";
import { dirname, join } from "node:path";
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
function loadModule(path) {
  return jiti ? jiti.import(path) : import(path);
}
function loadMode() {
  return loadModule(source);
}
const { InteractiveMode, createInteractiveTui, createInteractiveTuiReference } = await loadMode();
const themeSource = join(dirname(source), `theme/theme${source.endsWith(".ts") ? ".ts" : ".js"}`);
const { onThemeChange, setThemeInstance } = await loadModule(themeSource);
const { Image, setCapabilities } = await import(
  join(agentPackage, "node_modules/@earendil-works/pi-tui/dist/index.js")
);
const osc52 = (text) => `\u001b]52;c;${Buffer.from(text).toString("base64")}\u0007`;

afterEach(() => mock.restoreAll());

function screen() {
  const exits = [];
  const notices = [];
  const mode = Object.create(InteractiveMode.prototype);
  Object.assign(mode, {
    options: { hosted: { onExit: async (code, error) => exits.push({ code, error }) } },
    isShuttingDown: false,
    signalCleanupHandlers: [],
    stop: mock.fn(),
    showError: (message) => notices.push(message),
    showStatus: (message) => notices.push(message),
    getCrashExtensionHint: () => undefined,
    recordCrash: () => false,
    pendingUserInputs: [],
    runtimeHost: {
      dispose: mock.fn(),
      session: {
        sessionManager: { isPersisted: () => false },
        modelRuntime: { getError: () => undefined },
        settingsManager: {
          getExternalEditorCommand: () => "false",
          getFullscreenExitOutput: () => "transcript",
          getShowTerminalProgress: () => false,
        },
        prompt: mock.fn(),
      },
    },
    editor: { getText: () => "draft", setText: mock.fn() },
    themeController: { disableAutoSync: mock.fn() },
    ui: {
      stop: mock.fn(),
      start: mock.fn(),
      requestRender: mock.fn(),
      terminal: { drainInput: mock.fn(), write: mock.fn() },
    },
  });
  return { mode, exits, notices };
}

it("every screen hears a theme change until it stops", () => {
  const heard = [];
  const stopFirst = onThemeChange(() => heard.push("first"));
  const stopSecond = onThemeChange(() => heard.push("second"));
  // An instance rather than a named theme, so no theme file is read.
  setThemeInstance({});
  assert.deepEqual(heard, ["first", "second"]);
  stopFirst();
  setThemeInstance({});
  assert.deepEqual(heard, ["first", "second", "second"]);
  stopSecond();
});

it("stopping a screen stops its theme listener", () => {
  const { mode } = screen();
  const unsubscribe = mock.fn();
  Object.assign(mode, {
    themeChangeUnsubscribe: unsubscribe,
    disposeActiveSelector: mock.fn(),
    clearStatusIndicator: mock.fn(),
    clearExtensionTerminalInputListeners: mock.fn(),
    footer: { dispose: mock.fn() },
    footerDataProvider: { dispose: mock.fn() },
    isInitialized: false,
  });
  // The real stop, over a screen that never initialised its renderer.
  InteractiveMode.prototype.stop.call(mode);
  assert.equal(unsubscribe.mock.callCount(), 1);
  assert.equal(mode.themeChangeUnsubscribe, undefined);
});

it("a hosted copy reaches its viewers' terminals and never the host's output", async () => {
  const { mode, notices } = screen();
  mode.runtimeHost.session.getLastAssistantText = () => "hello";
  const hostOutput = mock.method(process.stdout, "write", () => true);
  await mode.handleCopyCommand();
  const hostWrites = hostOutput.mock.callCount();
  mock.restoreAll();
  assert.deepEqual(
    mode.ui.terminal.write.mock.calls.map((call) => call.arguments[0]),
    [osc52("hello")],
  );
  assert.equal(hostWrites, 0);
  assert.deepEqual(notices, ["Copied last agent message to clipboard"]);
});

// A terminal that keeps what it is sent, for a real renderer to draw on.
function viewerTerminal() {
  return {
    columns: 80,
    rows: 24,
    kittyProtocolActive: true,
    start: mock.fn(),
    stop: mock.fn(),
    drainInput: async () => undefined,
    write: mock.fn(),
    moveBy: mock.fn(),
    hideCursor: mock.fn(),
    showCursor: mock.fn(),
    clearLine: mock.fn(),
    clearFromCursor: mock.fn(),
    clearScreen: mock.fn(),
    setTitle: mock.fn(),
    setProgress: mock.fn(),
    setProgramStatus: mock.fn(),
  };
}

it("a hosted fullscreen selection is copied to its viewers' terminals", async () => {
  const { mode } = screen();
  const terminal = viewerTerminal();
  // The screen's own renderer switch builds the fullscreen renderer.
  mode.renderer = createInteractiveTui({
    tuiMode: "regular",
    showHardwareCursor: false,
    logDirectory: process.env.HOME,
    terminal,
  });
  mode.ui = createInteractiveTuiReference(() => mode.renderer);
  mode.options.tuiMode = "regular";
  mode.fullscreenLayoutRoot = { render: () => [], invalidate: () => undefined };
  Object.assign(mode.runtimeHost.session.settingsManager, {
    getFullscreenCopyOnSelect: () => true,
    getFullscreenWheelScrollLines: () => "auto",
  });
  assert.equal(mode.switchTuiMode("fullscreen", false, false), true);
  assert.equal(mode.renderer.mode, "fullscreen");
  terminal.write.mock.resetCalls();
  const hostOutput = mock.method(process.stdout, "write", () => true);
  // What a drag-select and the copy command both end in.
  const copied = await mode.renderer.copyTextToClipboard("selected");
  const hostWrites = hostOutput.mock.callCount();
  mock.restoreAll();
  assert.equal(copied, true);
  assert.deepEqual(
    terminal.write.mock.calls.map((call) => call.arguments[0]),
    [osc52("selected")],
  );
  assert.equal(hostWrites, 0);
});

it("a repaint for a viewer who joined late sends the images already on the screen", async () => {
  const terminal = viewerTerminal();
  const renderer = createInteractiveTui({
    tuiMode: "fullscreen",
    showHardwareCursor: false,
    logDirectory: process.env.HOME,
    terminal,
  });
  setCapabilities({ images: "kitty", trueColor: true, hyperlinks: false });
  // A one-pixel PNG.
  const pixel =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
  renderer.addChild(new Image(pixel, "image/png", { fallbackColor: (text) => text }));
  const uploads = () =>
    terminal.write.mock.calls.filter((call) => call.arguments[0].includes("\u001b_Ga=T")).length;
  const frame = () => new Promise((settle) => setImmediate(settle));
  renderer.start();
  renderer.renderNow(true);
  assert.equal(uploads(), 1, "the first frame uploads the image");

  // Ordinary pi's forced redraw only places what its terminal already holds.
  renderer.requestRender(true);
  await frame();
  assert.equal(uploads(), 1, "a forced redraw places the image it uploaded");

  renderer.repaintForNewTerminal();
  await frame();
  renderer.stop();
  assert.equal(uploads(), 2, "a viewer who joined late is sent the image's data");
});

it("a hosted copy too large for the terminal says so and writes nothing", async () => {
  const { mode, notices } = screen();
  mode.runtimeHost.session.getLastAssistantText = () => "x".repeat(80_000);
  await mode.handleCopyCommand();
  assert.equal(mode.ui.terminal.write.mock.callCount(), 0);
  assert.deepEqual(notices, ["Clipboard unavailable: text exceeds the OSC 52 size limit"]);
});

it("a hosted screen refuses the external editor without leaving its viewers' screen", async () => {
  const { mode, notices } = screen();
  await mode.handleOpenExternalEditor();
  assert.equal(mode.ui.stop.mock.callCount(), 0);
  assert.equal(mode.editor.setText.mock.callCount(), 0);
  assert.deepEqual(notices, ["The external editor is unavailable in a hosted session"]);
});

it("the host can check patch version before constructing a screen", () => {
  assert.equal(InteractiveMode.hostedLifecycleVersion, 1);
  assert.throws(
    () => new InteractiveMode(undefined, { hosted: { onExit: async () => undefined } }),
    (error) => error.message === "Hosted interactive mode requires an independent terminal",
  );
});

function forbidProcessExit() {
  mock.method(process, "exit", () => {
    throw new Error("screen attempted process.exit");
  });
  mock.method(process, "kill", () => {
    throw new Error("screen attempted process.kill");
  });
}

// A screen whose startup runs for real until it reaches `quitAt`, where its
// viewer quits before startup carries on.
function startingScreen(quitAt) {
  const started = screen();
  const { mode } = started;
  const quit = (step) => (step === quitAt ? mode.shutdown() : Promise.resolve());
  let providerCounts = 0;
  Object.assign(mode, {
    isInitialized: false,
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
    headerContainer: { addChild: mock.fn() },
    // Offline, finding neither tool says so here rather than downloading it.
    showManagedToolStatus: () => quit("tools"),
    setupKeyHandlers: mock.fn(),
    setupEditorSubmitHandler: mock.fn(),
    applyRuntimeSettings: mock.fn(),
    bindCurrentSessionExtensions: () => quit("extensions"),
    subscribeToAgent: mock.fn(),
    updateAvailableProviderCount: () => {
      providerCounts += 1;
      // The first count is the session's rebind, the second startup's own.
      return providerCounts === 2 ? quit("providers") : Promise.resolve();
    },
    updateEditorBorderColor: mock.fn(),
    updateTerminalTitle: mock.fn(),
    renderInitialMessages: mock.fn(),
    footerDataProvider: { onBranchChange: mock.fn() },
  });
  for (const part of [
    "documentContainer",
    "pendingMessagesContainer",
    "statusContainer",
    "widgetContainerAbove",
    "editorContainer",
    "widgetContainerBelow",
    "footerContainer",
  ]) {
    mode[part] = {};
  }
  mode.options.initialMessage = "hello";
  mode.runtimeHost.session.settingsManager.getFullscreenScrollbar = () => "auto";
  Object.assign(mode.themeController, {
    applyFromSettings: mock.fn(),
    waitForTerminalColors: () => quit("colors"),
  });
  Object.assign(mode.ui, { setFocus: mock.fn(), renderNow: mock.fn() });
  return started;
}

for (const quitAt of ["colors", "tools", "extensions", "providers"]) {
  it(`a hosted screen that quits while startup waits on ${quitAt} binds nothing after`, async () => {
    forbidProcessExit();
    process.env.PI_OFFLINE = "1";
    const { mode, exits } = startingScreen(quitAt);
    // With fd and rg on no path, startup stops to say so rather than finding them.
    const path = process.env.PATH;
    process.env.PATH = "";
    try {
      await mode.run();
    } finally {
      process.env.PATH = path;
    }
    assert.deepEqual(exits, [{ code: 0, error: undefined }]);
    assert.equal(mode.ui.start.mock.callCount(), 1);
    // No theme listener outlives it in the host's one theme: its stop has
    // run, so one still held here would never be released.
    assert.equal(mode.themeChangeUnsubscribe, undefined, "a theme listener was left behind");
    // A subscription made before the quit is the screen's stop to release;
    // none is made after, and nothing is prompted or drawn.
    assert.equal(mode.subscribeToAgent.mock.callCount(), quitAt === "providers" ? 1 : 0);
    assert.equal(mode.runtimeHost.session.prompt.mock.callCount(), 0);
    assert.equal(mode.ui.renderNow.mock.callCount(), 0);
    // A late event never starts it again.
    await mode.init();
    assert.equal(mode.ui.start.mock.callCount(), 1);
  });
}

it("quitting one hosted screen leaves its sibling alive and notifies once", async () => {
  forbidProcessExit();
  const first = screen();
  const second = screen();
  await Promise.all([first.mode.shutdown(), first.mode.shutdown()]);
  assert.deepEqual(first.exits, [{ code: 0, error: undefined }]);
  assert.equal(first.mode.stop.mock.callCount(), 1);
  assert.equal(first.mode.runtimeHost.dispose.mock.callCount(), 0);
  assert.deepEqual(second.exits, []);
  assert.equal(second.mode.stop.mock.callCount(), 0);
  assert.equal(second.mode.isShuttingDown, false);
});

it("closing a waiting screen releases its input loop without prompting", {
  timeout: 2000,
}, async () => {
  forbidProcessExit();
  const { mode } = screen();
  mode.init = async () => undefined;
  mode.checkForPackageUpdates = async () => [];
  mode.checkTmuxKeyboardSetup = async () => undefined;
  mode.maybeWarnAboutAnthropicSubscriptionAuth = async () => undefined;
  const inputReady = Promise.withResolvers();
  const readInput = mode.getUserInput.bind(mode);
  mode.getUserInput = () => {
    const waiting = readInput();
    inputReady.resolve();
    return waiting;
  };
  const previousOffline = process.env.PI_OFFLINE;
  process.env.PI_OFFLINE = "1";
  try {
    const running = mode.run();
    await inputReady.promise;
    await mode.shutdown();
    assert.equal(mode.onInputCallback, undefined);
    assert.equal(mode.runtimeHost.session.prompt.mock.callCount(), 0);
    await running;
  } finally {
    if (previousOffline === undefined) {
      delete process.env.PI_OFFLINE;
    } else {
      process.env.PI_OFFLINE = previousOffline;
    }
  }
});

it("a fatal runtime error closes only its screen and returns cancellation", async () => {
  forbidProcessExit();
  const first = screen();
  const second = screen();
  const failure = new Error("replacement failed");
  assert.deepEqual(await first.mode.handleFatalRuntimeError("Cannot switch", failure), {
    cancelled: true,
  });
  assert.deepEqual(first.exits, [{ code: 1, error: failure }]);
  assert.deepEqual(first.notices, ["Cannot switch: replacement failed"]);
  assert.deepEqual(second.exits, []);
});

it("a failed screen cleanup still notifies the host with failure", async () => {
  forbidProcessExit();
  const { mode, exits } = screen();
  const failure = new Error("screen cleanup failed");
  mode.stop = () => {
    throw failure;
  };
  await mode.shutdown();
  assert.deepEqual(exits, [{ code: 1, error: failure }]);
});

it("host-directed emergency and crash exits remain local", async () => {
  forbidProcessExit();
  const first = screen();
  const second = screen();
  const failure = new Error("owned callback failed");
  first.mode.uncaughtCrash(failure);
  await first.mode.hostedExit;
  assert.deepEqual(first.exits, [{ code: 1, error: failure }]);
  assert.deepEqual(second.exits, []);
  second.mode.emergencyTerminalExit();
  await second.mode.hostedExit;
  assert.deepEqual(second.exits, [{ code: 129, error: undefined }]);
});

it("ordinary quit still disposes the runtime, drains its terminal and exits", async () => {
  const { mode } = screen();
  mode.options = {};
  const exited = new Error("ordinary process exit");
  const exit = mock.method(process, "exit", () => {
    throw exited;
  });
  await assert.rejects(mode.shutdown(), (error) => error === exited);
  assert.equal(mode.runtimeHost.dispose.mock.callCount(), 1);
  assert.equal(mode.ui.terminal.drainInput.mock.callCount(), 1);
  assert.equal(mode.stop.mock.callCount(), 1);
  assert.deepEqual(exit.mock.calls[0].arguments, [0]);
});

it("ordinary fatal error retains its process exit", async () => {
  const { mode } = screen();
  mode.options = {};
  const exited = new Error("ordinary process exit");
  const exit = mock.method(process, "exit", () => {
    throw exited;
  });
  await assert.rejects(
    mode.handleFatalRuntimeError("Cannot switch", new Error("bad")),
    (error) => error === exited,
  );
  assert.equal(mode.stop.mock.callCount(), 1);
  assert.deepEqual(exit.mock.calls[0].arguments, [1]);
});

it("hosted screens register no process-wide signal, crash or stream listeners", () => {
  const { mode } = screen();
  const prepend = mock.method(process, "prependListener", () => process);
  const streams = [process.stdin, process.stdout, process.stderr].map((stream) =>
    mock.method(stream, "on", () => stream),
  );
  mode.registerSignalHandlers();
  assert.equal(prepend.mock.callCount(), 0);
  for (const stream of streams) {
    assert.equal(stream.mock.callCount(), 0);
  }
  assert.deepEqual(mode.signalCleanupHandlers, []);
});

it("hosted suspend refuses without stopping either screen or signalling the process", () => {
  forbidProcessExit();
  const first = screen();
  const second = screen();
  first.mode.handleCtrlZ();
  assert.deepEqual(first.notices, [
    "Suspend is unavailable in a hosted session; close the viewer to detach",
  ]);
  assert.equal(first.mode.ui.stop.mock.callCount(), 0);
  assert.equal(second.mode.ui.stop.mock.callCount(), 0);
});

it("ordinary pi still registers process handlers and unregisters them on cleanup", () => {
  const { mode } = screen();
  mode.options = {};
  const prepend = mock.method(process, "prependListener", () => process);
  const off = mock.method(process, "off", () => process);
  const streams = [process.stdin, process.stdout, process.stderr].map((stream) => ({
    on: mock.method(stream, "on", () => stream),
    off: mock.method(stream, "off", () => stream),
  }));
  mode.registerSignalHandlers();
  assert.equal(prepend.mock.callCount(), process.platform === "win32" ? 2 : 3);
  for (const stream of streams) {
    assert.equal(stream.on.mock.callCount(), 1);
  }
  mode.unregisterSignalHandlers();
  assert.equal(off.mock.callCount(), prepend.mock.callCount());
  for (const stream of streams) {
    assert.equal(stream.off.mock.callCount(), 1);
  }
  assert.deepEqual(mode.signalCleanupHandlers, []);
});
