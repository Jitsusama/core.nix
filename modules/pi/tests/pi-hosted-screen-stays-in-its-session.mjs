// The host-only InteractiveMode patch keeps a hosted screen inside its own
// session: closing it never ends the process, it asks its host first and
// carries on when the host says the quit doesn't hold, a stopped screen sends no
// more of its first words and builds nothing an extension asks of it, a
// suspend stops only the viewer that asked, every screen hears a theme
// change, and a closed screen disposes what an extension put on it.
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
const { InteractiveMode } = await loadMode();
const themeSource = join(dirname(source), `theme/theme${source.endsWith(".ts") ? ".ts" : ".js"}`);
const { onThemeChange, setThemeInstance } = await loadModule(themeSource);

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

it("the host can check patch version before constructing a screen", () => {
  assert.equal(InteractiveMode.hostedLifecycleVersion, 1);
  assert.equal(InteractiveMode.hostedQuit, true, "the host can see this pi asks before a quit");
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

for (const quitOn of ["first", "second"]) {
  it(`a hosted screen that quits on its ${quitOn} words sends none after`, async () => {
    forbidProcessExit();
    process.env.PI_OFFLINE = "1";
    // A screen that starts whole, then is handed its first words.
    const { mode, exits } = startingScreen("never");
    mode.options.initialMessage = "first";
    mode.options.initialMessages = ["second", "third"];
    Object.assign(mode, {
      checkForPackageUpdates: () => Promise.resolve([]),
      checkTmuxKeyboardSetup: () => Promise.resolve(undefined),
      maybeWarnAboutAnthropicSubscriptionAuth: () => Promise.resolve(),
      showWarning: mock.fn(),
      getUserInput: () => Promise.reject(new Error("a stopped screen waited for input")),
    });
    // Startup ends by loading syntax grammars, which redraws once they load.
    mode.ui.invalidate = mock.fn();
    const sent = [];
    mode.runtimeHost.session.prompt = async (words) => {
      sent.push(words);
      if (words === quitOn) {
        await mode.shutdown();
      }
    };
    const path = process.env.PATH;
    process.env.PATH = "";
    try {
      await mode.run();
    } finally {
      process.env.PATH = path;
      // The fake stop keeps what the real one releases.
      mode.themeChangeUnsubscribe?.();
    }
    assert.deepEqual(sent, quitOn === "first" ? ["first"] : ["first", "second"]);
    assert.deepEqual(exits, [{ code: 0, error: undefined }]);
  });
}

it("a hosted screen that has stopped builds nothing an extension asks of it", () => {
  const { mode } = screen();
  mode.hostedExit = Promise.resolve();
  const built = mock.fn(() => ({ render: () => [], invalidate: () => undefined }));
  const listen = mock.fn(() => () => undefined);
  Object.assign(mode, {
    builtInHeader: {},
    extensionWidgetsAbove: new Map(),
    extensionWidgetsBelow: new Map(),
    footerContainer: { clear: mock.fn(), addChild: mock.fn() },
    headerContainer: { children: [] },
    renderWidgets: mock.fn(),
  });
  mode.ui.addInputListener = listen;
  const asks = {
    widget: () => mode.setExtensionWidget("w", built),
    footer: () => mode.setExtensionFooter(built),
    header: () => mode.setExtensionHeader(built),
    editor: () => mode.setCustomEditorComponent(built),
  };
  for (const [what, ask] of Object.entries(asks)) {
    ask();
    assert.equal(built.mock.callCount(), 0, `a ${what} was built for a stopped screen`);
  }
  const unsubscribe = mode.addExtensionTerminalInputListener(() => undefined);
  assert.equal(listen.mock.callCount(), 0, "a stopped screen took an input listener");
  unsubscribe();
});

// A screen showing an extension's own footer, header and editor beside
// pi's own; `disposed` counts each of the extension's disposals in turn.
function extendedScreen() {
  const started = screen();
  const parts = [0, 1, 2, 3, 4].map(() => ({ dispose: mock.fn() }));
  const [customFooter, customHeader, editor, defaultEditor, builtInHeader] = parts;
  Object.assign(started.mode, { customFooter, customHeader, editor, defaultEditor, builtInHeader });
  const disposed = () => parts.slice(0, 3).map((part) => part.dispose.mock.callCount());
  return { ...started, parts, disposed };
}

it("a closed hosted screen disposes its extension's footer, header and editor", async () => {
  forbidProcessExit();
  const { mode, exits, disposed } = extendedScreen();
  await mode.shutdown();
  assert.deepEqual(disposed(), [1, 1, 1], "each disposed once");
  // The host then invalidates the session, which resets the extension UI.
  mode.setExtensionFooter(undefined);
  mode.setExtensionHeader(undefined);
  mode.setCustomEditorComponent(undefined);
  assert.deepEqual(disposed(), [1, 1, 1], "none disposed again");
  assert.deepEqual(exits, [{ code: 0, error: undefined }]);
});

it("a closed hosted screen leaves pi's own editor to pi", async () => {
  const { mode, parts } = extendedScreen();
  mode.editor = mode.defaultEditor;
  await mode.shutdown();
  assert.equal(parts[3].dispose.mock.callCount(), 0);
});

it("a part that fails to dispose leaves the rest disposed and the host told", async () => {
  forbidProcessExit();
  const { mode, exits, parts, disposed } = extendedScreen();
  const failure = new Error("footer dispose failed");
  parts[0].dispose.mock.mockImplementation(() => {
    throw failure;
  });
  await mode.shutdown();
  assert.deepEqual(disposed(), [1, 1, 1], "each disposed once");
  assert.equal(mode.stop.mock.callCount(), 1, "the screen still stopped");
  assert.deepEqual(exits, [{ code: 1, error: failure }]);
});

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

for (const holds of [true, false]) {
  const told = holds ? "stops" : "runs on";
  it(`a hosted screen asks its host before it quits, and ${told} as told`, async () => {
    forbidProcessExit();
    const { mode, exits } = screen();
    const asked = Promise.withResolvers();
    mode.options.hosted.quit = mock.fn(() => asked.promise);
    const quitting = mode.shutdown();
    // Asked before shutdown's first await, so the host still knows whose key it was.
    assert.equal(mode.options.hosted.quit.mock.callCount(), 1, "the host was asked at once");
    assert.equal(mode.stop.mock.callCount(), 0, "nothing stopped before the host answered");
    asked.resolve(holds);
    await quitting;
    assert.equal(mode.stop.mock.callCount(), holds ? 1 : 0);
    assert.deepEqual(exits, holds ? [{ code: 0, error: undefined }] : []);
    assert.equal(mode.isShuttingDown, holds);
  });
}

it("a hosted screen already quitting asks its host nothing more", async () => {
  forbidProcessExit();
  const { mode, exits } = screen();
  mode.options.hosted.quit = mock.fn(async () => true);
  await mode.shutdown();
  await mode.shutdown();
  assert.equal(mode.options.hosted.quit.mock.callCount(), 1);
  assert.deepEqual(exits, [{ code: 0, error: undefined }]);
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

it("hosted suspend stops only the viewer that asked, when its host can", () => {
  forbidProcessExit();
  const able = screen();
  const unable = screen();
  able.mode.options.hosted.suspend = mock.fn(() => true);
  unable.mode.options.hosted.suspend = mock.fn(() => false);
  able.mode.handleCtrlZ();
  unable.mode.handleCtrlZ();
  assert.equal(able.mode.options.hosted.suspend.mock.callCount(), 1);
  assert.deepEqual(able.notices, []);
  assert.equal(able.mode.ui.stop.mock.callCount(), 0, "the screen runs on");
  assert.deepEqual(unable.notices, [
    "Suspend is unavailable in a hosted session; close the viewer to detach",
  ]);
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
