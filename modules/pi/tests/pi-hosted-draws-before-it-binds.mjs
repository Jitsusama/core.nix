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
afterEach(() => {
  process.env.PATH = path;
  mock.restoreAll();
});

// A screen whose startup runs for real, with its extensions' bind held
// until `bound` is called, as a session_start on a stalled disk holds it;
// `binding` settles once startup has reached the bind.
function startingScreen(hosted) {
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
    footerDataProvider: { onBranchChange: mock.fn() },
    themeController: {
      applyFromSettings: mock.fn(),
      disableAutoSync: mock.fn(),
      waitForTerminalColors: () => Promise.resolve(),
    },
    editor: { getText: () => "", setText: mock.fn() },
    runtimeHost: {
      session: {
        sessionManager: { isPersisted: () => false },
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
  return { mode, bound, binding };
}

/** Starts the screen with fd and rg on no path, offline, so it downloads nothing. */
function starting(mode) {
  process.env.PATH = "";
  process.env.PI_OFFLINE = "1";
  return mode.init();
}

it("a hosted screen draws its conversation while its extensions still bind", async () => {
  const { mode, bound, binding } = startingScreen(true);
  const started = starting(mode);
  // Once the bind has begun, and while it is held, the window shows the
  // conversation; it hears the agent only once its extensions are live.
  await binding;
  assert.equal(
    mode.renderInitialMessages.mock.callCount(),
    1,
    "the window was blank while its extensions bound",
  );
  assert.equal(mode.subscribeToAgent.mock.callCount(), 0, "it heard the agent before it bound");
  bound();
  await started;
  // Drawn once: startup's own draw after the bind is the one it replaced.
  assert.equal(mode.renderInitialMessages.mock.callCount(), 1);
  assert.equal(mode.subscribeToAgent.mock.callCount(), 1);
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
  assert.equal(mode.subscribeToAgent.mock.callCount(), 1);
  mode.themeChangeUnsubscribe?.();
});
