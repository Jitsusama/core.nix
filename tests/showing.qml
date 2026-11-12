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
        expect("its nerdy bits, one with no value gone, as Drawer.qml leaves it out",
            Showing.drawer({ title: "Work Network", nerd: [{ label: "Reason",
                value: "NetworkHealthy" }, { label: "Tunnel Mode", value: "" }, null] }),
            { title: "Work Network", nerd: [{ label: "Reason", value: "NetworkHealthy" }] });
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
        // A drawer borrowed by another row leaves out what that row says.
        const keyboard = { title: "ErgoDox EZ", eyebrow: "on USB \u00b7 keyboard", facts: [
            { label: "Layer", value: "Numb \u00b7 locked" }, { label: "Locks", value: "none" },
            null, { heading: "Light" }] };
        expect("a borrowed drawer drops the fact its row's side says",
            Showing.besides(keyboard, { side: "Numb \u00b7 locked ", note: "" }).facts,
            [{ label: "Locks", value: "none" }, null, { heading: "Light" }]);
        expect("and keeps the rest of it, and every fact when the row says none", [
            Showing.besides(keyboard, { side: "Numb" }).facts.length,
            Showing.besides(keyboard, {}).eyebrow
        ], [4, "on USB \u00b7 keyboard"]);
        expect("none borrowed stays none", Showing.besides(null, { side: "on" }), null);
        // Which of a thing's rows are its acts, said apart for `launcher do`.
        const sure = () => [{ key: "sure", name: "Clear 3 GB?", sure: true }];
        const act = (one, verbs) => Showing.isAct(one, verbs, one.key);
        expect("Return's act and a plain verb are acts", [
            act({ key: "return:copy", name: "Copy" }, []),
            act({ key: "copy", name: "Copy" }, ["copy"])
        ], [true, true]);
        expect("a verb that asks first stays an act",
            act({ key: "clear", name: "Clear", verbs: sure, targets: sure }, ["clear"]), true);
        const pairing = () => [{ key: "pair", name: "Pair" }, { key: "trust", name: "Trust" }];
        expect("a verb that's a thing, a device heard, is a row",
            act({ key: "bluetooth:buds", name: "Buds", verbs: pairing }, ["bluetooth:buds"]),
            false);
        const insides = () => [{ key: "open", name: "Open" }, { key: "a.md", name: "a.md" }];
        expect("a folder inside a folder, holding its insides as its targets, is a row",
            act({ key: "drafts", name: "drafts", verbs: insides, targets: insides }, ["drafts"]),
            false);
        expect("a row that's none of the thing's verbs is a row",
            act({ key: "part", name: "Model" }, ["copy"]), false);
        // A row's note is said in its drawer, once, as a sentence.
        expect("a note becomes the drawer's sentence",
            Showing.noted(keyboard, "the layer lock key lets it go").body,
            "The layer lock key lets it go.");
        expect("its own words come first, the note after them, never unsaid",
            Showing.noted({ title: "Disks", body: "Filling fast." }, "full in 3 days").body,
            "Filling fast. Full in 3 days.");
        const sharing = { title: "Sharing", facts: [{ label: "Window", value: "Firefox" },
            { label: "Screen", value: "desktop \u00b7 2880x1800" }] };
        expect("a note its title or facts say, part by whole part, stays out", [
            Showing.noted(keyboard, "none \u00b7 ergodox ez").body,
            Showing.noted(sharing, "Firefox, desktop").body,
            Showing.noted(keyboard, "").body
        ], [undefined, undefined, undefined]);
        expect("a part found only inside a word is said",
            Showing.noted(keyboard, "on").body, "On.");
        expect("an address alone is said as written, and a path opening a sentence ends it", [
            Showing.noted(sharing, "portal.example/sign-in").body,
            Showing.noted(sharing, "/nix full in 6 h").body
        ], ["portal.example/sign-in", "/nix full in 6 h."]);
        expect("a note ending as a sentence keeps its end, and none stays none", [
            Showing.noted({ title: "Lag" }, "waiting on memory!").body,
            Showing.noted(null, "on")
        ], ["Waiting on memory!", null]);
        // The empty line says a place holds nothing, over its ways elsewhere too.
        const history = { name: "History", elsewhere: true };
        const none = "Nothing unread from the desktop.";
        expect("an empty place says so, and so does nothing found", [
            Showing.saysNothing("", [], ""),
            Showing.saysNothing("zz", [], none)
        ], [true, true]);
        expect("over only a way elsewhere it says so, the row still there",
            Showing.saysNothing("", [history], none), true);
        expect("but not with nothing to say it with, nor beside a row of its own", [
            Showing.saysNothing("", [history], ""),
            Showing.saysNothing("", [{ name: "Peter" }, history], none)
        ], [false, false]);
        expect("and with something typed, only when nothing answers",
            Showing.saysNothing("hist", [history], none), false);
        expect("the line saying nothing wears the mark its nothing means", [
            Showing.nothingMark("zz", null),
            Showing.nothingMark("zz", { key: "on:waiting:desktop" }),
            Showing.nothingMark("", { key: "on:waiting:desktop" }),
            Showing.nothingMark("", { key: "on:waitingOnYou" }),
            Showing.nothingMark("", { key: "capture:recent" }),
            Showing.nothingMark("", {})
        ], ["nothingFound", "nothingFound", "forYouEmpty", "forYouEmpty", "nothingHere",
            "nothingHere"]);
        expect("and with nothing typed and nothing stepped into, the greeting",
            Showing.nothingMark("", null), "greeting");
        // A drawer's acts are its row's verbs, Return's first by ↵ and the
        // rest by their own keys where they have one (the drawer spec's
        // part 6); a drawer giving its own keeps them, and a row of none
        // gives none.
        const verbs = [{ key: "verb:copy", name: "Copy It", actKey: "y" },
            { key: "verb:hour", name: "For an Hour", actKey: "a" },
            { key: "verb:peek", name: "Peek" }, { key: "verb:blank", name: "" }];
        expect("a drawer's acts are its row's verbs, Return's first",
            Showing.acted({ title: "Charge" }, { verbs: () => verbs }).acts, [
                { name: "Copy It", key: "\u21b5" }, { name: "For an Hour", key: "a" },
                { name: "Peek", key: "" }]);
        const own = { title: "Shot", acts: [{ name: "Take It", key: "\u21b5" }] };
        expect("a drawer's own acts stay, and a row of no verbs gives none", [
            Showing.acted(own, { verbs: () => verbs }),
            Showing.acted({ title: "Drafts" }, { name: "Drafts" }).acts,
            Showing.acted(null, { verbs: () => verbs })
        ], [own, undefined, null]);
        if (failed > 0) {
            console.warn(`${failed} failed`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}
