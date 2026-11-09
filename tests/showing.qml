// How the launcher tells a terminal what it shows
// (modules/quickshell/launcher/Showing.js), run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `the-launcher-shows-a-terminal-what-it-holds` check copies Showing.js
// beside this file and runs `qml -platform offscreen showing.qml`. Each
// case is a promise `launcher show` leans on: a step names its row by its
// name in any case or a unique start of it, a miss offers the names
// nearest it, and a row and its drawer reach the terminal as the words
// the launcher draws, with nothing it doesn't.
import QtQuick
import "Showing.js" as Showing

QtObject {
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    property int failed: 0

    function expect(what, got, wanted) {
        if (JSON.stringify(got) !== JSON.stringify(wanted)) {
            const said = `got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`;
            console.warn(`FAIL ${what}: ${said}`);
            failed += 1;
        }
    }

    function run() {
        const home = ["Now", "Agents", "Programs", "People", "Desktop", "Machine", "Network",
            "Keyboards"].map(name => ({ key: `category:${name.toLowerCase()}`, name }));
        const idOf = row => row.key;
        const named = found => found.row ? found.row.name : found;
        // Which row a step names.
        expect("by its name", named(Showing.step(home, "Machine", idOf)), "Machine");
        expect("in any case", named(Showing.step(home, "machine", idOf)), "Machine");
        expect("by its steady id", named(Showing.step(home, "category:people", idOf)), "People");
        expect("by the only name it starts", named(Showing.step(home, "mach", idOf)), "Machine");
        const inside = [{ key: "a", name: "Start a Session" }, { key: "b", name: "Sessions" },
            { key: "c", name: "Sessions to Reopen" }];
        expect("hyphens for spaces", named(Showing.step(inside, "start-a-session", idOf)),
            "Start a Session");
        expect("a whole name before a longer one it starts",
            named(Showing.step(inside, "sessions", idOf)), "Sessions");
        expect("a start of several offers each", Showing.step(home, "p", idOf),
            { near: ["Programs", "People"] });
        // A miss offers the names nearest it.
        expect("a swap is near", Showing.step(home, "mahcine", idOf), { near: ["Machine"] });
        expect("a letter dropped is near", Showing.step(home, "netwrk", idOf),
            { near: ["Network"] });
        expect("nothing near offers nothing", Showing.step(home, "zebra", idOf), { near: [] });
        expect("an empty step names nothing", Showing.step(home, "", idOf), { near: [] });
        expect("a row with no name is never named", Showing.step([{ key: "x", name: "" }],
            "x", idOf), { near: [] });
        // A row as the terminal reads it.
        expect("a figure", Showing.shaped({ key: "disk:space", name: "Space", side: "412 GB free",
            figure: true, glyph: "\u25c6", hue: "machine", run: () => {} },
            { id: "computer\u203adisk:space", kind: "action", verb: "copy", holds: false,
                drawer: null }),
            { id: "computer\u203adisk:space", name: "Space", kind: "figure", holds: false,
                key: "disk:space", glyph: "\u25c6", hue: "machine", side: "412 GB free",
                verb: "copy" });
        expect("a place that holds more, its getters left out",
            Showing.shaped({ key: "category:computer", name: "Machine", count: "Disk \u00b7 Power",
                side: () => "never", verbs: () => [] },
            { id: "category:computer", kind: "place", verb: "open", holds: true, drawer: null }),
            { id: "category:computer", name: "Machine", kind: "place", holds: true,
                key: "category:computer", count: "Disk \u00b7 Power", verb: "open" });
        // Its drawer as the drawer draws it.
        expect("a drawer's facts, one not read yet null, a missing one gone",
            Showing.drawer({ title: "nvme0n1", eyebrow: "Disk", facts: [{ label: "Model",
                value: "PM9A1" }, null, { label: "Temperature", value: null },
            { heading: "Health" }, { label: "Wear", value: 3, qualifier: "%" }], body: "" }),
            { eyebrow: "Disk", title: "nvme0n1", facts: [{ label: "Model", value: "PM9A1" },
                { label: "Temperature", value: null }, { heading: "Health" },
                { label: "Wear", value: "3", qualifier: "%" }] });
        expect("a moment's groups, an empty one gone, its say the body",
            Showing.drawer({ now: true, eyebrow: "Saturday", say: "1 needs you",
                groups: [[{ label: "Next", value: "Standup" }], [null]] }),
            { eyebrow: "Saturday", body: "1 needs you",
                groups: [[{ label: "Next", value: "Standup" }]] });
        expect("a styled body said as words", Showing.drawer({ title: "Mail", styled: true,
            body: "<b>Joel</b> &amp; Ada<br>said hi" }),
            { title: "Mail", body: "Joel & Ada\nsaid hi" });
        expect("a drawer saying nothing is none", Showing.drawer({ title: "", body: " " }), null);
        // Whether it's shown at all: only while it says more than its row.
        const folder = { name: "drafts", side: "2 items \u00b7 just now" };
        expect("a folder's plate with only a preview opens", Showing.adds({ title: "drafts",
            eyebrow: "in ~/picked \u00b7 Folder", facts: [],
            preview: { lines: ["older", "one.md"], quiet: true } }, folder), true);
        expect("as one with only a picture in its preview does", Showing.adds({ title: "a.png",
            preview: { picture: "/n/a.png" } }, { name: "a.png" }), true);
        expect("a plate with only its kind beside its row's name doesn't",
            Showing.adds({ title: "archive.zip", eyebrow: "in ~/picked \u00b7 ZIP", facts: [] },
                { name: "archive.zip", side: "48.0 kB \u00b7 just now" }), false);
        expect("nor does one whose body is its row's side", Showing.adds({ title: "drafts",
            body: folder.side, preview: { lines: [] } }, folder), false);
        expect("a fact, a picture or words the row doesn't say open it", [
            Showing.adds({ title: "drafts", facts: [null, { label: "Owner", value: "Ada" }] },
                folder),
            Showing.adds({ title: "drafts", picture: "/n/a.png" }, folder),
            Showing.adds({ title: "drafts", body: "Kept for later." }, folder),
            Showing.adds({ now: true }, folder)
        ], [true, true, true, true]);
        if (failed > 0) {
            console.warn(`${failed} failed`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}
