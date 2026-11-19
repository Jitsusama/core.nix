// What a place's row says, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-place-reads-alike-everywhere` check copies
// modules/quickshell/launcher/PlaceRow.js beside this file and runs it
// offscreen. Home's rows are made by the same function a place's row is
// everywhere else (Launcher.qml's homeRow returns resolved), so each case
// holds a row at home and inside its parent alike: nothing after its
// name; at its right how it stands, else that it holds none, else how
// many it holds; and its members by name in its drawer (VKCF3Z, dae7fb67).
// Home kept for the next opening and still unfilled a second after it was
// asked for holds its rows in order, each saying how it stands as being
// read, its note as it was (VKCF3Z, 2026-11-10).
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
        // as a run of dots, which the drawer lays as a run, breaking between
        // members with the dot going with the break.
        const machine = ["Power", "Desk Light", "Sound"];
        const drawn = PlaceRow.drawer("Machine", machine, "place");
        check(failures, "its drawer is titled with its name", drawn.title, "Machine");
        check(failures, "and its eyebrow says it's a place", drawn.eyebrow, "place");
        check(failures, "and names its members in their order", drawn.body,
            "Power \u00b7 Desk Light \u00b7 Sound");
        check(failures, "which the drawer lays as a run", drawn.runs, true);
        check(failures, "a sentence is no run",
            PlaceRow.drawer("Keyboard", ["A"], "place", { holding: "Every key." }).runs,
            undefined);
        check(failures, "holding one kind, it says what they are instead",
            PlaceRow.drawer("Keyboard", ["A", "B"], "place", { holding: "Every key." }).body,
            "Every key.");
        check(failures, "holding nothing to name, no drawer", PlaceRow.drawer("Media", []),
            null);

        // Holding none, its drawer says so in the place's own sentence, as
        // inside it, so every row has a drawer (Y2YBTS, 641b64f8; Joel,
        // 2026-11-11 22:00).
        const empty = PlaceRow.drawer("Windows", [], "place", { empty: "No other windows." });
        check(failures, "holding none, its drawer says its empty sentence",
            empty?.body, "No other windows.");
        check(failures, "under its name and eyebrow", `${empty?.eyebrow} ${empty?.title}`,
            "place Windows");
        check(failures, "a sentence, not a run", empty?.runs, undefined);
        check(failures, "holding some, it names them rather than its empty sentence",
            PlaceRow.drawer("Windows", ["Kite Notes"], "place", { empty: "No other windows." })
                .body, "Kite Notes");

        // Now's parts give way by whole names (Launcher.qml's listed).
        check(failures, "names listed in their room", PlaceRow.listed(["a", "bb", "ccc"], 6),
            "a, bb");
        check(failures, "none in no room", PlaceRow.listed(["a"], 0), "");

        // Unfilled a second after an opening asked for them, home's rows
        // stand where they stood, and what each said of how it stands says
        // it's being read, never the old reading as now.
        const home = [
            { key: "collection:now", name: "Now", side: "11:52",
                sideItems: [{ said: "11:52" }], note: "" },
            { key: "place:computer", name: "Computer", side: "81%", note: "12 GiB spare" },
            { key: "place:keyboard", name: "Keyboard", side: "", note: "" }
        ];
        const held = PlaceRow.unfilled(home);
        check(failures, "unfilled, the rows stand in their order",
            held.map(row => row.key).join(" "), home.map(row => row.key).join(" "));
        check(failures, "by the same names", held.map(row => row.name).join(" "),
            "Now Computer Keyboard");
        check(failures, "how one stood says it's being read", held[1].side, "\u2026");
        check(failures, "its note stays as it was", held[1].note, "12 GiB spare");
        check(failures, "one said in items says it in one", held[0].side, "\u2026");
        check(failures, "its items let go", held[0].sideItems.length, 0);
        check(failures, "with no note, it gains none", held[0].note, "");
        check(failures, "one that said nothing says nothing", held[2].side, "");
        check(failures, "and is the row it was", held[2], home[2]);
        check(failures, "the rows read are left as they were", home[1].side, "81%");
        check(failures, "held again, it says the same", PlaceRow.unfilled(held)[1].side,
            "\u2026");

        for (const failure of failures)
            console.log(`FAIL ${failure}`);
        console.log(`place-row: ${failures.length} of the cases failed`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
