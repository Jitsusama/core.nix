// Drives Explain It's machinery (modules/quickshell/launcher/Explainer.qml)
// through Quickshell itself, over a stand-in for mycelium that says one
// session, answers another after a moment, never answers a third, can't
// say a fourth and knows no other, and a pi that answers as it's told. A
// session asked of reads while the node answers, then its pi is asked
// holding only its facts, and the act shows the answer while it's this
// act's, its first sentence as its name and the whole read; an answer to
// another question leaves it as it was. An address that isn't one asks
// nothing, one the node doesn't know says so, a node that can't say is no
// answer, a second asked while the first reads waits its turn and the
// first is the act again, and a node that never answers is given up on.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string fjord: "A fjord is a long, narrow inlet of the sea."
    readonly property string whole: `${fjord} Glaciers carved its steep sides.`
    property var pending: null
    property int told: 0

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${JSON.stringify(said)}`);
    }

    function fail(error) {
        console.warn(`FAIL ${error.message}`);
        pending = null;
        Qt.callLater(() => Qt.exit(1));
    }

    // Goes on once what's waited for holds, looked at whenever the
    // explainer says it changed, and each tenth of a second.
    function until(what, holds, then) {
        pending = { what: what, holds: holds, then: then, at: Date.now() };
        heard();
    }

    function heard() {
        const waiting = pending;
        if (waiting === null)
            return;
        try {
            if (!waiting.holds()) {
                if (Date.now() - waiting.at > 8000)
                    throw new Error(`never: ${waiting.what}`);
                return;
            }
            pending = null;
            waiting.then();
        } catch (error) {
            fail(error);
        }
    }

    Timer {
        interval: 100
        repeat: true
        running: true
        onTriggered: root.heard()
    }

    // The pi as the explainer reaches it, as Ask is: what it was asked
    // last, what it said and how that stands, and `answered` when more of
    // it is said.
    QtObject {
        id: ask

        property string posed: ""
        property string said: ""
        property string stage: ""
        property int asked: 0

        signal answered

        function pose(question) {
            posed = question;
            said = "";
            stage = "asking";
            asked += 1;
        }
    }

    Launcher.Explainer {
        id: explainer

        realm: "harbour"
        asker: ask
        patience: 2000
        said: ({
            asks: "Say what this is.\n\n{facts}",
            reading: "finding what it is",
            asking: "asking",
            answered: "explained just now",
            unknown: "not running anywhere",
            failed: "couldn't be explained"
        })
        onChanged: {
            root.told += 1;
            root.heard();
        }
    }

    // How many times the stand-in was asked, by its log.
    function asked() {
        log.reload();
        log.waitForJob();
        return log.text().split("\n").filter(line => line !== "").length;
    }

    FileView {
        id: log

        path: Quickshell.env("MYCELIUM_ASKED")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: {
        try {
            expect("nothing asked shows as the act's own", explainer.shown("a") === null,
                explainer.shown("a"));
            explainer.explain("a", "HARBOR");
            expect("a session asked of reads first", explainer.shown("a")?.name
                === "finding what it is", explainer.shown("a"));
            until("Ask is asked of HARBOR", () => ask.asked === 1, answering);
        } catch (error) {
            fail(error);
        }
    }

    function answering() {
        expect("Ask is told the session's title", ask.posed.includes("Its title: Tidy the Rows"),
            ask.posed);
        expect("and never its quest", !ask.posed.includes("QEST-"), ask.posed);
        expect("and never its folder", !ask.posed.includes("/home/"), ask.posed);
        expect("while Ask thinks, the act says so", explainer.shown("a")?.name === "\u2026"
            && explainer.shown("a")?.note === "asking", explainer.shown("a"));
        const before = told;
        ask.said = whole;
        ask.stage = "answered";
        ask.answered();
        expect("an answer told says the act again", told > before, told);
        const shown = explainer.shown("a");
        expect("the answer's first sentence names the act", shown?.name === fjord, shown);
        expect("the whole of it is read", shown?.reads === whole, shown);
        expect("its note says it's explained", shown?.note === "explained just now", shown);
        expect("its hue is a voice's", shown?.hue === "voice", shown);
        ask.pose("Something else entirely");
        expect("an answer to another question isn't this act's", explainer.shown("a") === null,
            explainer.shown("a"));
        const readsBefore = asked();
        explainer.explain("b", "-all");
        expect("an address that isn't one is none, said at once",
            explainer.shown("b")?.name === "not running anywhere"
            && explainer.shown("b")?.hue === "alert", explainer.shown("b"));
        expect("and the node isn't asked", asked() === readsBefore, asked());
        explainer.explain("c", "NOSUCH");
        until("a session the node doesn't know is said so",
            () => explainer.shown("c")?.name === "not running anywhere", () => {
                explainer.explain("g", "DOWN");
                until("a node that can't say is no answer, never an unknown session",
                    () => explainer.shown("g")?.name === "couldn't be explained", queued);
            });
    }

    function queued() {
        const posedBefore = ask.asked;
        explainer.explain("d", "SLOW");
        explainer.explain("e", "HARBOR");
        expect("the first is the act again once a second is asked",
            explainer.shown("d") === null, explainer.shown("d"));
        expect("the second reads while it waits", explainer.shown("e")?.name
            === "finding what it is", explainer.shown("e"));
        until("the second is asked of its pi once the first's reader ends",
            () => ask.asked === posedBefore + 1, () => {
                expect("the pi is asked of the second, not the first",
                    ask.posed.includes("Its title: Tidy the Rows"), ask.posed);
                expect("and the second is said to be asking",
                    explainer.shown("e")?.note === "asking", explainer.shown("e"));
                explainer.explain("f", "HANG");
                until("a node that never answers is given up on",
                    () => explainer.shown("f")?.name === "Couldn't be explained", () => {
                        Qt.exit(0);
                    });
            });
    }
}
