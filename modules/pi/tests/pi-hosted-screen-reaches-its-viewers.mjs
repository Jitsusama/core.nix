// The host-only InteractiveMode patch does in its viewers' terminals what
// pi does in its own: a copy (a selection's too) reaches them rather than
// the host's clipboard, a viewer who joins late is sent the images already
// on the screen, and the external editor opens where the viewer who asked
// is, through its host, or is refused when its host can't.
// The real methods run against fake runtimes and a fake terminal; no TTY,
// model request, editor or extension is started. Arguments name the
// package and, optionally, source.
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
const extension = source.endsWith(".ts") ? ".ts" : ".js";
function loadModule(path) {
  return jiti ? jiti.import(path) : import(path);
}
const { InteractiveMode, createInteractiveTui, createInteractiveTuiReference } =
  await loadModule(source);
const { initTheme } = await loadModule(join(dirname(source), `theme/theme${extension}`));
const { ExtensionEditorComponent } = await loadModule(
  join(dirname(source), `components/extension-editor${extension}`),
);
const { Image, setCapabilities } = await import(
  join(agentPackage, "node_modules/@earendil-works/pi-tui/dist/index.js")
);
const osc52 = (text) => `\u001b]52;c;${Buffer.from(text).toString("base64")}\u0007`;

afterEach(() => mock.restoreAll());

// A hosted screen over a fake runtime, with what it says kept as notices.
function screen() {
  const notices = [];
  const mode = Object.create(InteractiveMode.prototype);
  Object.assign(mode, {
    options: { hosted: { onExit: () => Promise.resolve() } },
    showError: (message) => notices.push(message),
    showStatus: (message) => notices.push(message),
    runtimeHost: {
      session: {
        settingsManager: { getExternalEditorCommand: () => "false" },
      },
    },
    editor: { getText: () => "draft", setText: mock.fn() },
    ui: {
      stop: mock.fn(),
      start: mock.fn(),
      requestRender: mock.fn(),
      terminal: { write: mock.fn() },
    },
  });
  return { mode, notices };
}

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

it("a hosted copy too large for the terminal says so and writes nothing", async () => {
  const { mode, notices } = screen();
  mode.runtimeHost.session.getLastAssistantText = () => "x".repeat(80_000);
  await mode.handleCopyCommand();
  assert.equal(mode.ui.terminal.write.mock.callCount(), 0);
  assert.deepEqual(notices, ["Clipboard unavailable: text exceeds the OSC 52 size limit"]);
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
  const repainted = uploads();
  // Stopping prints the transcript, its image too, on the main screen.
  renderer.stop();
  assert.equal(repainted, 2, "a viewer who joined late is sent the image's data");
});

it("a hosted screen whose host can't edit refuses the external editor in place", async () => {
  const { mode, notices } = screen();
  await mode.handleOpenExternalEditor();
  assert.equal(mode.ui.stop.mock.callCount(), 0);
  assert.equal(mode.editor.setText.mock.callCount(), 0);
  assert.deepEqual(notices, ["The external editor is unavailable in a hosted session"]);
});

it("a hosted screen edits its draft in the external editor of the viewer that asked", async () => {
  const { mode, notices } = screen();
  const asked = [];
  mode.options.hosted.editText = (text) => {
    asked.push(text);
    return Promise.resolve({ status: "complete", content: "edited" });
  };
  await mode.handleOpenExternalEditor();
  assert.deepEqual(asked, ["draft"]);
  assert.deepEqual(
    mode.editor.setText.mock.calls.map((call) => call.arguments[0]),
    ["edited"],
  );
  assert.equal(mode.ui.stop.mock.callCount(), 0, "the screen runs on for its other viewers");
  assert.deepEqual(notices, []);
});

it("a hosted edit that fails keeps the draft and says why", async () => {
  const { mode, notices } = screen();
  mode.options.hosted.editText = () =>
    Promise.reject(new Error("The viewer left before its edit came back"));
  await mode.handleOpenExternalEditor();
  assert.equal(mode.editor.setText.mock.callCount(), 0);
  assert.deepEqual(notices, ["The viewer left before its edit came back"]);
});

it("an extension's editor panel edits where its screen says, never in the host's terminal", async () => {
  initTheme("dark", false);
  const tui = {
    requestRender: mock.fn(),
    stop: mock.fn(),
    start: mock.fn(),
    terminal: { rows: 24, columns: 80 },
  };
  const panel = new ExtensionEditorComponent(
    tui,
    {},
    "Title",
    "draft",
    () => undefined,
    () => undefined,
    undefined,
    "false",
    (text) => Promise.resolve({ status: "complete", content: `${text}, edited` }),
  );
  await panel.handleOpenExternalEditor();
  assert.equal(panel.editor.getText(), "draft, edited");
  assert.equal(tui.stop.mock.callCount(), 0);
});
