// The host-only InteractiveMode patch closes one screen without terminating
// its process. The real methods run against two fake runtimes; no TTY, model
// request or extension is started. Ordinary signal and suspend paths retain
// their process semantics. Arguments name the package and, optionally, source.
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
async function loadMode() {
  if (!source.endsWith(".ts")) {
    return import(source);
  }
  const loader = join(agentPackage, "../../node_modules/jiti/lib/jiti.mjs");
  const { createJiti } = await import(loader);
  return createJiti(import.meta.url, { moduleCache: false }).import(source);
}
const { InteractiveMode } = await loadMode();

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
        prompt: mock.fn(),
      },
    },
    themeController: { disableAutoSync: mock.fn() },
    ui: {
      stop: mock.fn(),
      start: mock.fn(),
      requestRender: mock.fn(),
      terminal: { drainInput: mock.fn() },
    },
  });
  return { mode, exits, notices };
}

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
