// What a QML test, in Qt's own engine or offscreen Quickshell, says of its
// cases: a case that doesn't hold throws, naming it and what it held, and a
// failure is warned with FAIL before Quickshell exits with 1, so its check
// fails. A test imports it as `import "Expect.js" as Expect`, its check
// copying it beside the test.
.pragma library

function expect(what, holds, said) {
    if (!holds) {
        throw new Error(`${what}: ${said}`);
    }
}

function fail(message) {
    // biome-ignore lint/suspicious/noConsole: a QML test's failure is said on its log.
    console.warn(`FAIL ${message}`);
    // The exit waits for the event loop, so the warning is written first.
    Qt.callLater(Qt.exit, 1);
}
