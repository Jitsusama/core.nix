// What a place's row says, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-place-reads-alike-everywhere` check copies
// modules/quickshell/launcher/PlaceRow.js beside this file and runs it
// offscreen. Home's rows are made by the same function a place's row is
// everywhere else (Launcher.qml's homeRow returns resolved), so each case
// holds a row at home and inside its parent alike: nothing after its
// name; at its right how it stands, else that it holds none, else how
// many it holds; and its members by name in its drawer (VKCF3Z, dae7fb67).
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

        // At its right: how it stands where it says, else its word for
        // none while it holds nothing, else how many it holds.
        check(failures, "how it stands where it says", PlaceRow.side("99% busy", "4"),
            "99% busy");
        check(failures, "how it stands over holding none", PlaceRow.side("locked", "", "none"),
            "locked");
        check(failures, "how many where it says nothing", PlaceRow.side(null, "12"), "12");
        check(failures, "and where what it says is empty", PlaceRow.side("", "12"), "12");
        check(failures, "holding nothing, its word for none, as Media's",
            PlaceRow.side(null, "0", "none"), "none");
        check(failures, "a place that only groups others says nothing",
            PlaceRow.side(null, ""), "");

        // Its drawer names what it holds, in its own order, under its name,
        // as a run of dots that breaks only after a dot, between members.
        const machine = ["Power", "Desk Light", "Sound"];
        const drawn = PlaceRow.drawer("Machine", machine, "place");
        check(failures, "its drawer is titled with its name", drawn.title, "Machine");
        check(failures, "and its eyebrow says it's a place", drawn.eyebrow, "place");
        check(failures, "and names its members in their order", drawn.body,
            "Power\u00a0\u00b7 Desk\u00a0Light\u00a0\u00b7 Sound");
        check(failures, "holding one kind, it says what they are instead",
            PlaceRow.drawer("Keyboard", ["A", "B"], "place", "Every key.").body, "Every key.");
        check(failures, "holding nothing to name, no drawer", PlaceRow.drawer("Media", []),
            null);

        // Now's parts give way by whole names (Launcher.qml's listed).
        check(failures, "names listed in their room", PlaceRow.listed(["a", "bb", "ccc"], 6),
            "a, bb");
        check(failures, "none in no room", PlaceRow.listed(["a"], 0), "");

        for (const failure of failures)
            console.log(`FAIL ${failure}`);
        console.log(`place-row: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
