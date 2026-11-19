// What a QML test, in Qt's own engine or offscreen Quickshell, says of its
// cases: a case that doesn't hold throws, naming it and what it held, and a
// failure is warned with FAIL before qml or Quickshell exits with 1, so its
// check fails. A test imports it as `import "Expect.js" as Expect`, its check
// copying it beside the test.
.pragma library

function expect(what, holds, said) {
    if (!holds) {
        throw new Error(`${what}: ${said}`);
    }
}

// A case whose answer is a value: it holds where the value said is the
// one meant, as JSON says each.
function same(what, said, meant) {
    const was = JSON.stringify(said);
    expect(what, was === JSON.stringify(meant), `said ${was}, meant ${JSON.stringify(meant)}`);
}

// Runs a test's cases once it's made, as `Component.onCompleted:
// Expect.run(cases)`: qml or Quickshell exits with 0 when they all hold,
// and the first that doesn't fails the test, whatever it threw. run exits
// as soon as `cases` returns, so every case must be done by then: a test
// that waits on something keeps its own stages and calls `fail` itself.
function run(cases) {
    try {
        cases();
        Qt.exit(0);
    } catch (error) {
        fail(thrown(error));
    }
}

// What a case threw, in words: its message, else the value itself, else
// what kind of thing it was when even that can't be said, so a failing
// case always says something.
function thrown(error) {
    try {
        return error?.message || String(error);
    } catch (unsaid) {
        const why = `since saying it threw a ${typeof unsaid}`;
        return `a thrown ${typeof error} that can't be said, ${why}`;
    }
}

function fail(message) {
    // biome-ignore lint/suspicious/noConsole: a QML test's failure is said on its log.
    console.warn(`FAIL ${message}`);
    // The exit waits for the event loop, so the warning is written first.
    Qt.callLater(Qt.exit, 1);
}
