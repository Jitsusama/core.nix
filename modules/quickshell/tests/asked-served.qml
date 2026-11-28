// Drives the launcher's Asked For (modules/cadence/Asked.qml) over what the
// cadence-spec daemon itself serves, through Quickshell itself, with a
// stand-in for cadence (modules/cadence/tests/asked-stand-in.sh). Before the
// daemon has answered, "asked" says it's still asking rather than nothing.
// With ASKED_READ=rows the daemon serves two asks and a head saying one is
// late, which Asked For gives as the daemon says it, warning, its rows in
// the order served. With ASKED_READ=unread cadence can't read the list, and
// the daemon heads Asked For unread in the launcher's words, which it gives
// as a warning with no rows, rather than asking forever. Either way, once
// the daemon stops answering, "asked" is no answer of its own, so the
// launcher says the daemon is down rather than showing what it last held
// (round 6 on #6875, F112). It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property bool unread: Quickshell.env("ASKED_READ") === "unread"

    readonly property var expect: Expect.expect
    readonly property var same: Expect.same

    // The launcher as Asked For asks it: open, with "asked" typed, so the
    // daemon is watched.
    QtObject {
        id: asking

        property bool open: true
        property string typed: "asked"
        property var matches: []
        property var held: []
        property var placed: []
        property var answer: null

        function wants(source) {
            return true;
        }

        function becomesAll(source, holds, how) {
        }

        function becameNot(key, why) {
        }

        function afterFrame(then) {
            then();
        }

        function enter(row) {
        }
    }

    Launcher.Asked {
        id: asked
        settings: ({ name: "asked" })
        launcher: asking
    }

    // The daemon, as its unit runs it, asking the stand-in for the rows;
    // what it says of itself goes on the check's log. It starts once the
    // first case has read Asked For with no daemon yet.
    Process {
        id: daemon

        running: false
        command: [Quickshell.env("CADENCE_SPEC"), `${root.runtime}/daemons/cadence-spec.sock`,
            Quickshell.env("CADENCE"), Quickshell.env("WORDS")]
        stderr: SplitParser {
            onRead: line => console.warn(`cadence-spec: ${line}`)
        }
    }

    Until {
        id: waits
    }

    // Once the daemon is gone, Asked For gives no answer of its own.
    function gone() {
        daemon.running = false;
        waits.until("the daemon not answering", () => asked.served.state === "not answering",
            () => {
                expect("a daemon not answering gives no answer of Asked For's own",
                    asked.answer("asked") === null, asked.answer("asked")?.headline);
                Qt.exit(0);
            });
    }

    Component.onCompleted: {
        // Until says what a stage threw; these run before any stage. An
        // answer holds its provider, so each says only what it's read by.
        try {
            const before = asked.answer("asked");
            expect("before the daemon answers, Asked For says it's still asking",
                before?.headline === "\u2026" && before?.asking === true,
                `${before?.headline} asking ${before?.asking}`);
            expect("words that don't ask it are no answer", asked.answer("weather") === null,
                asked.answer("weather")?.headline);
        } catch (error) {
            Expect.fail(Expect.thrown(error));
            return;
        }
        daemon.running = true;
        waits.until("Asked For's head served", () => asked.headed, () => {
            const said = asked.answer("asked");
            if (root.unread) {
                expect("a list cadence can't read is headed unread, in the launcher's words",
                    said?.headline === "can't read the list now", said?.headline);
                expect("and warns", said?.warns === true, said?.warns);
                same("with no rows", (said?.rows ?? []).map(row => row.name), []);
            } else {
                expect("the head as the daemon says it", said?.headline === "1 late of 2",
                    said?.headline);
                expect("warning while any is late", said?.warns === true, said?.warns);
                expect("wearing the alert glyph", said?.glyph === Theme.glyph.failed,
                    said?.glyph);
                same("its asks, in the order served", (said?.rows ?? []).map(row => row.name),
                    ["split the launcher's words", "say what moved"]);
            }
            root.gone();
        });
    }
}
