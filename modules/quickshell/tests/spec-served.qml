// Drives the launcher's Spec (modules/cadence/Spec.qml) over what the
// cadence-spec daemon itself serves, through Quickshell itself, with a
// stand-in for cadence printing two axes, one out of spec with a reading
// inside it (modules/cadence/tests/served-stand-in.sh). Spec gives the head as the
// daemon says it, "1 reading out of spec" wearing the alert glyph, which Agents'
// Spec row shows; its axes in the order served, each a step in; and the
// out axis stepped into, its reading. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")

    readonly property var expect: Expect.expect
    readonly property var same: Expect.same

    // The launcher as Spec asks it: open, so the daemon is watched.
    QtObject {
        id: asking

        property bool open: true

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

    Launcher.Spec {
        id: spec
        settings: ({ name: "spec" })
        launcher: asking
    }

    // The daemon, as its unit runs it, asking the stand-in for the rows;
    // what it says of itself goes on the check's log.
    Process {
        running: true
        command: [Quickshell.env("CADENCE_SPEC"), `${root.runtime}/daemons/cadence-spec.sock`,
            Quickshell.env("CADENCE"), Quickshell.env("WORDS")]
        stderr: SplitParser {
            onRead: line => console.warn(`cadence-spec: ${line}`)
        }
    }

    Until {
        id: waits
    }

    Component.onCompleted: waits.until("Spec's head served",
        () => spec.head() !== null && spec.axes().length > 0, () => {
            const head = spec.head();
            expect("the head as the daemon says it",
                head.said === "1 reading out of spec", head.said);
            // Drawn, as Agents' row draws it: the theme's mark, never its
            // name, which drew as the word "failed" (Y2YBTS, 2026-11-15).
            expect("the head wears the alert glyph while any is out",
                head.glyph === Theme.glyph.failed, head.glyph);
            const axes = spec.axes();
            same("its axes, in the order served", axes.map(row => row.name),
                ["Time to Done", "Quality"]);
            expect("an axis steps in", axes[0].verb === "more", axes[0].verb);
            const inside = typeof axes[0].targets === "function" ? axes[0].targets()
                : axes[0].targets ?? [];
            same("the out axis holds its reading", inside.map(row => row.name),
                ["merge to on 53 min, at most 18 min"]);
            Qt.exit(0);
        })
}
