// The host-only InteractiveMode patch does in its viewers' terminals what
// pi does in its own: a copy (a selection's too) reaches them rather than
// the host's clipboard, a viewer who joins late is sent the images already
// on the screen, every frame is drawn within its terminal's render scope,
// the external editor opens where the viewer who asked
// is, through its host, or is refused when its host can't, a quit asks its
// host first, while the key that asked is still in hand, and an
// extension's own component, overlays too, says it waits on the person.
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

// Whether each frame a renderer in `tuiMode` drew was inside its terminal's
// render scope: one drawn now, one forced and one throttled, as a theme
// change asks for them from another screen's call.
async function framesInScope(tuiMode) {
  let scoped = false;
  const terminal = viewerTerminal();
  // A hosted screen's terminal draws as the conversation it shows.
  terminal.renderScope = (draw) => {
    scoped = true;
    try {
      draw();
    } finally {
      scoped = false;
    }
  };
  const renderer = createInteractiveTui({
    tuiMode,
    showHardwareCursor: false,
    logDirectory: process.env.HOME,
    terminal,
  });
  const drawn = [];
  renderer.addChild({
    render: () => {
      drawn.push(scoped);
      return [`frame ${drawn.length}`];
    },
    invalidate: () => undefined,
  });
  // A throttled frame waits out pi's frame interval; a second is ample.
  const drawnPast = (count, until = Date.now() + 1000) =>
    drawn.length > count || Date.now() > until
      ? Promise.resolve()
      : new Promise((settle) => setImmediate(settle)).then(() => drawnPast(count, until));
  renderer.start();
  renderer.renderNow(true);
  const now = drawn.length;
  renderer.requestRender(true);
  await drawnPast(now);
  const forced = drawn.length;
  renderer.requestRender();
  await drawnPast(forced);
  // Stopping prints the transcript, which is no frame.
  const frames = [...drawn];
  renderer.stop();
  return frames;
}

it("every frame is drawn within its terminal's render scope, whoever asked for it", async () => {
  const [regular, fullscreen] = await Promise.all([
    framesInScope("regular"),
    framesInScope("fullscreen"),
  ]);
  for (const [tuiMode, frames] of Object.entries({ regular, fullscreen })) {
    assert.ok(frames.length >= 3, `${tuiMode}: three frames were drawn, not ${frames.length}`);
    assert.deepEqual(
      frames.filter((inScope) => !inScope),
      [],
      `${tuiMode}: every frame is drawn in scope`,
    );
  }
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

// A screen an extension's own component takes, with what it reports kept.
function panelScreen(hosted) {
  const { mode } = screen();
  if (!hosted) {
    mode.options = {};
  }
  Object.assign(mode, {
    extensionDialogs: [],
    editorContainer: { clear: mock.fn(), addChild: mock.fn() },
    programStatus: { setBlocked: mock.fn() },
    keybindings: {},
  });
  Object.assign(mode.ui, {
    setFocus: mock.fn(),
    showOverlay: mock.fn(() => ({ hide: mock.fn() })),
  });
  const calls = () => mode.programStatus.setBlocked.mock.calls.map((call) => call.arguments);
  return { mode, calls };
}

// Show an extension's component, once pi has it on the screen.
async function shown(mode, options) {
  let answer;
  const answered = mode.showExtensionCustom((_tui, _theme, _keys, done) => {
    answer = done;
    return { render: () => [], invalidate: () => undefined };
  }, options);
  await new Promise((settle) => setImmediate(settle));
  return { answer: (value) => answer(value), answered };
}

const ASKING = { kind: "question", message: "An extension is asking" };

it("a hosted extension's overlay says it waits on the person until answered", async () => {
  const { mode, calls } = panelScreen(true);
  const panel = await shown(mode, { overlay: true });
  assert.deepEqual(
    calls().map(([, status]) => status),
    [ASKING],
  );
  panel.answer("yes");
  assert.equal(await panel.answered, "yes");
  assert.deepEqual(
    calls().map(([, status]) => status),
    [ASKING, undefined],
  );
});

it("a hosted overlay clears only its own report, while another stays up", async () => {
  const { mode, calls } = panelScreen(true);
  const first = await shown(mode, { overlay: true });
  await shown(mode, { overlay: true });
  first.answer("done");
  await first.answered;
  assert.equal(
    calls().length,
    3,
    "each overlay reports as it shows, and the first clears as it goes",
  );
  const [[opened], [other], [cleared, status]] = calls();
  assert.equal(cleared, opened, "the first overlay clears the report it made");
  assert.equal(status, undefined);
  assert.notEqual(other, opened, "the second reports under a name of its own");
});

it("a hosted component in the editor's place says it waits, in its own words if given", async () => {
  const { mode, calls } = panelScreen(true);
  await shown(mode);
  await shown(mode, { blocked: { kind: "permission", message: "Run it?" } });
  assert.deepEqual(calls(), [
    ["extension-dialog", ASKING],
    ["extension-dialog", { kind: "permission", message: "Run it?" }],
  ]);
});

it("ordinary pi reports an extension's component only as the extension says", async () => {
  const { mode, calls } = panelScreen(false);
  await shown(mode, { overlay: true, blocked: { kind: "question", message: "Pick" } });
  await shown(mode);
  assert.deepEqual(calls(), [["extension-dialog", undefined]]);
});

// A hosted screen whose host answers each quit it is asked about with `asked`.
function quittingScreen(asked) {
  const { mode } = screen();
  const exits = [];
  mode.stop = mock.fn();
  mode.pendingUserInputs = [];
  mode.options.hosted.onExit = async (code) => exits.push(code);
  mode.options.hosted.quit = mock.fn(() => asked);
  return { mode, exits, quit: mode.options.hosted.quit };
}

it("a host can see this pi asks it before a hosted quit", () => {
  assert.equal(InteractiveMode.hostedQuit, true);
});

for (const holds of [true, false]) {
  const told = holds ? "stops" : "runs on";
  it(`a hosted screen asks its host before it quits, and ${told} as told`, async () => {
    const answer = Promise.withResolvers();
    const { mode, exits, quit } = quittingScreen(answer.promise);
    const quitting = mode.shutdown();
    // Asked before shutdown's first await, so the host still knows whose key it was.
    assert.equal(quit.mock.callCount(), 1, "the host was asked at once");
    assert.equal(mode.stop.mock.callCount(), 0, "nothing stopped before the host answered");
    answer.resolve(holds);
    await quitting;
    assert.equal(mode.stop.mock.callCount(), holds ? 1 : 0);
    assert.deepEqual(exits, holds ? [0] : []);
  });
}

it("a hosted screen already quitting asks its host nothing more", async () => {
  const { mode, exits, quit } = quittingScreen(Promise.resolve(true));
  await mode.shutdown();
  await mode.shutdown();
  assert.equal(quit.mock.callCount(), 1);
  assert.deepEqual(exits, [0]);
});
