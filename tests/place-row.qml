// What a place's row says, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-place-reads-alike-everywhere` check copies
// modules/quickshell/launcher/PlaceRow.js beside this file and runs it
// offscreen. Home's rows are made by the same function a place's row is
// everywhere else (Launcher.qml's homeRow returns resolved), so each case
// holds a row at home and inside its parent alike: its members by name
// after its name, as many whole as fit, never an ellipsis, and how it
// stands at its right, or how many it holds.
import QtQuick
import "PlaceRow.js" as PlaceRow

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got "${got}", wanted "${want}"`);
    }

    function run() {
        const failures = [];
        const machine = ["Processor", "Memory", "Disk", "Power"];
        // [what, its members, its name, its side, the row's room, what it says
        // after its name]
        const holding = [
            ["every member fits, by name", machine, "Machine", "", 60,
                "Processor, Memory, Disk, Power"],
            ["as many whole as fit, nothing after the last", machine, "Machine", "", 28,
                "Processor, Memory"],
            ["what it says at its right takes its room first", machine, "Machine", "99% busy",
                38, "Processor, Memory"],
            ["a name just fits beside its side", ["Wi-Fi"], "Network", "1", 15, "Wi-Fi"],
            ["and not without the two spaces between", ["Wi-Fi"], "Network", "1", 14, ""],
            ["a first name too long for the room says nothing there", ["A Very Long Quest"],
                "Agents", "2 working", 25, ""],
            ["holding none, it says so as a run", [], "Agents", "0", 40, "none yet"]
        ];
        for (const [what, names, name, says, room, want] of holding)
            check(failures, what,
                PlaceRow.holds(names, { name: name, says: says, room: room, none: "none yet" }),
                want);
        check(failures, "never an ellipsis, however short the room",
            PlaceRow.holds(machine, { name: "Machine", says: "", room: 12 }).includes("\u2026"),
            false);
        check(failures, "holding none with no word for it says nothing",
            PlaceRow.holds([], { name: "Machine", says: "", room: 40 }), "");

        // At its right: how it stands where it says, else how many it holds.
        check(failures, "how it stands where it says", PlaceRow.side("99% busy", "4"),
            "99% busy");
        check(failures, "how many where it says nothing", PlaceRow.side(null, "12"), "12");
        check(failures, "and where what it says is empty", PlaceRow.side("", "12"), "12");

        // Now's parts give way the same way (Now.qml's listedAfter).
        check(failures, "names listed in their room", PlaceRow.listed(["a", "bb", "ccc"], 6),
            "a, bb");
        check(failures, "none in no room", PlaceRow.listed(["a"], 0), "");

        for (const failure of failures)
            console.log(`FAIL ${failure}`);
        console.log(`place-row: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
