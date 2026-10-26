// Drives the launcher's Ask row (modules/quickshell/launcher/Ask.qml)
// through Quickshell itself, over two stand-ins for pi-ask. One ends at once
// with an error, as pi-ask did without its proxy's key: the row says it
// can't start and why before anything is asked, and a question asked says
// why within 3 s, Return asking again rather than closing. The other speaks
// pi's RPC as the work machine's pi does (tests/desktop/stand-in-ask.py):
// a question is thought about at once and answered in place; a second gets
// a pi of its own, never one that carries the first; and closing the
// launcher while it answers stops it, the next answered whole. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string fjord: "A fjord is a long, narrow inlet of the sea."
    property var pending: null

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${JSON.stringify(said)}`);
    }

    function fail(error) {
        console.warn(`FAIL ${error.message}`);
        pending = null;
        Qt.callLater(() => Qt.exit(1));
    }

    // Goes on once what's waited for holds, looked at each time an Ask says
    // it has more, and at once.
    function until(what, holds, then) {
        pending = { what: what, holds: holds, then: then };
        heard();
    }

    function heard() {
        const waiting = pending;
        if (waiting === null)
            return;
        try {
            if (!waiting.holds())
                return;
            pending = null;
            waiting.then();
        } catch (error) {
            fail(error);
        }
    }

    // The launcher as Ask reaches it: whether it's open.
    QtObject {
        id: launcher

        property bool open: true
        property var held: []
        property string typed: ""
    }

    Launcher.Ask {
        id: broken

        launcher: launcher
        settings: ({ name: "ask", command: Quickshell.env("ASK_BROKEN") })
        onAnswered: root.heard()
    }

    Launcher.Ask {
        id: ask

        launcher: launcher
        settings: ({ name: "ask", command: Quickshell.env("ASK_STAND_IN") })
        onAnswered: root.heard()
    }

    function cantStart() {
        const offered = broken.answer("what is a fjord?");
        expect("before it's asked, it says it can't start and why",
            offered.note === "it can't start: Failed to get a token." && offered.hue === "alert"
            && offered.stays === true, offered);
        const asked = Date.now();
        offered.run();
        until("the question says why", () => broken.stage === "failed", () => {
            const took = Date.now() - asked;
            expect("it says why within 3 s", took < 3000, took);
            const said = broken.answer("what is a fjord?");
            expect("the reason is the row's name, and Return asks again",
                said.headline === "Failed to get a token." && said.verb === "ask"
                && said.stays === true && said.hue === "alert", said);
            answers();
        });
    }

    function answers() {
        ask.answer("what is a fjord?").run();
        const thinking = ask.answer("what is a fjord?");
        expect("a question is thought about at once", thinking.note === "thinking"
            && thinking.stays === true, thinking);
        until("it's answered in place", () => ask.stage === "answered", () => {
            const said = ask.answer("what is a fjord?");
            expect("its first sentence named, the whole below", said.headline === root.fjord
                && said.reads.includes("Glaciers carved") && said.note.startsWith("answered in")
                && said.stays === undefined, said);
            ask.answer("what is a glacier?").run();
            until("a second is answered", () => ask.stage === "answered", () => {
                const second = ask.answer("what is a glacier?");
                expect("a second gets a pi of its own", second.headline === root.fjord, second);
                stopped();
            });
        });
    }

    function stopped() {
        ask.answer("what is a moraine?").run();
        until("it's answering", () => ask.stage === "answering", () => {
            launcher.open = false;
            expect("closing stops it", ask.stage === "stopped", ask.stage);
            launcher.open = true;
            ask.answer("what is a tarn?").run();
            until("the next is answered", () => ask.stage === "answered", () => {
                const said = ask.answer("what is a tarn?");
                expect("the next is answered whole", said.headline === root.fjord, said);
                Qt.callLater(() => Qt.exit(0));
            });
        });
    }

    Component.onCompleted: until("the broken pi ends", () => broken.failures > 0,
        () => root.cantStart())

    Timer {
        interval: 30000
        running: true
        onTriggered: root.fail(new Error(`timed out: ${root.pending?.what ?? "nothing"}`))
    }
}
