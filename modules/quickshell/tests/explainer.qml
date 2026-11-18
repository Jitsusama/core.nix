// Drives Explain It's machinery (modules/quickshell/launcher/Explainer.qml)
// through Quickshell itself, over a stand-in for mycelium
// (tests/explain-it-node.sh) and a pi that answers as it's told. A session
// asked of reads while the node answers, then its pi is asked holding only
// its facts, and the act shows the answer while it's this act's, its first
// sentence as its name and the whole read; an answer to another question
// leaves it as it was. An address that isn't one asks nothing, one the
// node doesn't know or finds only by title says so, a node that can't say
// is no answer, a second row asked while the first reads makes the first
// the act again and is never given the first's answer, and a node that
// never answers is given up on, a row asked meanwhile read all the same.
// Its own pi is pi-ask with no tools. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string fjord: "A fjord is a long, narrow inlet of the sea."
    readonly property string whole: `${fjord} Glaciers carved its steep sides.`
    property int told: 0

    readonly property var expect: Expect.expect
    readonly property var same: Expect.same
    readonly property var until: waits.until

    Until {
        id: waits
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
        patience: 4000
        said: ({
            asks: "Say what this is.\n\n{facts}",
            reading: "finding what it is",
            asking: "asking",
            answered: "explained just now",
            unknown: "not running anywhere",
            failed: "couldn't be explained"
        })
        onChanged: root.told += 1
    }

    // What the stand-in was asked, a line each, by its log.
    function asked() {
        log.reload();
        log.waitForJob();
        return log.text().split("\n").filter(line => line !== "");
    }

    FileView {
        id: log

        path: Quickshell.env("MYCELIUM_ASKED")
        blockLoading: true
        printErrors: false
    }

    Component.onCompleted: {
        try {
            same("its own pi is pi-ask with no tools", explainer.pi, ["pi-ask", "--no-launcher"]);
            expect("nothing asked shows as the act's own", explainer.shown("a") === null,
                explainer.shown("a"));
            explainer.explain("a", "HARBOR");
            same("a session asked of reads first", explainer.shown("a")?.name,
                "finding what it is");
            until("its pi is asked of HARBOR", () => ask.asked === 1, answering);
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    function answering() {
        expect("its pi is told the session's title",
            ask.posed.includes("Its title: Tidy the Rows"), ask.posed);
        expect("and never its quest", !ask.posed.includes("QEST-"), ask.posed);
        expect("and never its folder", !ask.posed.includes("/home/"), ask.posed);
        same("while its pi thinks, the act says so", [explainer.shown("a")?.name,
            explainer.shown("a")?.note], ["\u2026", "asking"]);
        const before = told;
        ask.said = whole;
        ask.stage = "answered";
        ask.answered();
        expect("an answer told says the act again", told > before, told);
        const shown = explainer.shown("a");
        same("the answer's first sentence names the act", shown?.name, fjord);
        same("the whole of it is read", shown?.reads, whole);
        same("its note says it's explained", shown?.note, "explained just now");
        same("its hue is a voice's", shown?.hue, "voice");
        ask.pose("Something else entirely");
        same("an answer to another question isn't this act's", explainer.shown("a"), null);
        explainer.explain("b", "-all");
        same("an address that isn't one is none, said at once",
            [explainer.shown("b")?.name, explainer.shown("b")?.hue],
            ["not running anywhere", "alert"]);
        explainer.explain("c", "NOSUCH");
        until("a session the node doesn't know is said so",
            () => explainer.shown("c")?.name === "not running anywhere", titled);
    }

    function titled() {
        explainer.explain("t", "TITLED");
        until("a session found only by another's title words isn't the one asked",
            () => explainer.shown("t")?.name === "not running anywhere", () => {
                explainer.explain("g", "DOWN");
                until("a node that can't say is no answer, never an unknown session",
                    () => explainer.shown("g")?.name === "couldn't be explained", overtaken);
            });
    }

    function overtaken() {
        const posedBefore = ask.asked;
        explainer.explain("d", "SLOW");
        explainer.explain("e", "HARBOR");
        same("the first is the act again once a second is asked", explainer.shown("d"), null);
        until("the second is asked of its pi",
            () => ask.asked === posedBefore + 1, () => {
                expect("its pi is asked of the second", ask.posed.includes("Tidy the Rows"),
                    ask.posed);
                same("and the second is said to be asking", explainer.shown("e")?.note, "asking");
                // The first's late answer comes, and is let go.
                until("the first's late answer comes", () => asked().filter(line =>
                    line.includes(" SLOW ")).length === 1 && ask.asked === posedBefore + 1,
                    () => Qt.callLater(slowLetGo, posedBefore));
            });
    }

    // A second later the slow read has ended; its pi was asked only once.
    function slowLetGo(posedBefore) {
        lateCheck.posedBefore = posedBefore;
        lateCheck.start();
    }

    Timer {
        id: lateCheck

        property int posedBefore: 0

        interval: 1500
        onTriggered: {
            try {
                Expect.same("the first's late answer is never the pi's question",
                    ask.asked, lateCheck.posedBefore + 1);
                Expect.expect("and never named", !ask.posed.includes("Slow Work"), ask.posed);
                Expect.same("and the first stays the act", explainer.shown("d"), null);
                root.hung();
            } catch (error) {
                Expect.fail(error.message);
            }
        }
    }

    function hung() {
        explainer.explain("f", "HANG");
        until("a node that never answers is given up on",
            () => explainer.shown("f")?.name === "couldn't be explained", () => {
                explainer.explain("h", "HARBOR");
                until("a row asked after one given up on is read all the same",
                    () => explainer.shown("h")?.note === "asking", () => {
                        expect("the node was never asked of an address that isn't one",
                            !asked().some(line => line.includes("-all")), asked());
                        Qt.exit(0);
                    });
            });
    }
}
