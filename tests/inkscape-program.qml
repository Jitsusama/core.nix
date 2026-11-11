// Drives Inkscape's step in the launcher (modules/inkscape/InkscapeProgram.qml,
// through modules/quickshell/launcher/Applications.qml) through Quickshell
// itself, over Inkscape's desktop entry written beside this, with niri and
// what's playing stood in, the shell's own watch of the clipboard
// (Copied.qml) with wl-paste stood in, so a copy is set by hand, the
// clipboard's history stood in by its newest copy alone, and the desktop's
// list of files opened lately as GLib writes it, read by recent-files as
// the list programs share and by the account's own inkscape-drawings, each
// run by the programs daemon. First, in Qt's own engine, Inkscape.js's
// cases: the sizes an SVG's root says, said; what inkscape-drawings
// prints, read, when to the microsecond, each once; what Inkscape's
// drawer in Programs says;
// what a copy makes a drawing of, and the commands that make each.
//
// Then Inkscape's Recent: each drawing Inkscape opened, newest first,
// merged by when with the list programs share, each file once, Inkscape's
// own row the one kept, one that's gone left out and its exported picture
// as the shared list has it; each says when beside it, and its drawer
// where it is and its kind, its size as its SVG says it, its weight and
// its path. Return opens it in Inkscape. New Drawing follows the desktop
// entry's own acts, and Return on it writes a fresh copy of the template
// that Inkscape, stood in, is given, the template left as it was; New
// Drawing From the Copied Picture is there only while an SVG is copied, or
// a picture the history keeps the size of, and makes a drawing of each.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect
import "Frames.js" as Frames
import "HeardWindows.js" as HeardWindows
import "launcher/Inkscape.js" as Inkscape

ShellRoot {
    id: root

    property string waiting: "the programs"
    // The drawing made of the SVG copied, which the next mustn't write over.
    property string vector: ""

    readonly property string home: Quickshell.env("HOME")
    readonly property string drawn: `${home}/Drawings`
    readonly property string made: `${home}/Made`
    readonly property real now: Number(Quickshell.env("NOW")) * 1000
    readonly property string program: "org.inkscape.Inkscape"

    function expect(what, holds, said) { Expect.expect(what, holds, said); }

    function fail(message) { Expect.fail(message); }

    function same(what, got, wanted) {
        expect(what, got === wanted, got);
    }

    function json(what, got, wanted) {
        same(what, JSON.stringify(got), JSON.stringify(wanted));
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function entry() {
        return applications.things.find(row => row.key === `program:${root.program}`);
    }

    // A drawer's facts as they read, a label and its value each.
    function facts(row) {
        return row.drawer.facts.map(fact => `${fact.label} ${fact.value}`).join(", ");
    }

    // The clipboard's history, by its newest copy alone.
    QtObject {
        id: copies

        property string what: "copy"
        property var all: []
    }

    QtObject {
        id: theLauncher

        property bool open: false
        property var selected: null
        property string typed: ""
        property var held: []
        property var belowChord: []
        property var sources: [inkscape, copies]
        property var partsOfRows: ({})
        property string underListed: ""

        // What waits for the next frame, as the launcher's afterFrame
        // keeps it (tests/lib/Frames.js), run as a frame is drawn.
        readonly property var frames: Frames.kept()
        function afterFrame(then) {
            frames.due.push(then);
        }
        function drawn() {
            Frames.drawn(frames);
        }
    }

    Launcher.InkscapeProgram {
        id: inkscape

        launcher: theLauncher
        settings: ({
            name: "inkscape",
            inkscape: Quickshell.env("INKSCAPE"),
            template: Quickshell.env("TEMPLATE"),
            drawings: root.made,
            lately: "listed:org.inkscape.Inkscape:drawings",
            paste: Quickshell.env("PASTE"),
            // Stand-ins for the theme's edge and sunken roles, which the
            // provider only passes on into the drawing.
            border: "the-edge",
            desk: "the-sunken"
        })
        onDrawingsChanged: root.ready()
    }

    Launcher.Applications {
        id: applications

        launcher: theLauncher
        settings: ({
            name: "applications",
            recents: [{
                // The list programs share, as recent-files prints it.
                watch: `${root.home}/data/recently-used.xbel`,
                list: [Quickshell.env("RECENT_FILES"), "list"],
                dated: true
            }]
        })
        onThingsChanged: root.ready()
    }

    Connections {
        target: applications.recents

        function onSharedChanged() {
            root.ready();
        }
    }

    // Runs what an act spawned, as niri would, then `then`.
    Process {
        id: spawning

        property var then: null

        running: false
        onExited: {
            const then = spawning.then;
            spawning.then = null;
            Qt.callLater(() => root.carryOn(then));
        }
    }

    function spawn(then) {
        spawning.command = Niri.spawned;
        spawning.then = then;
        spawning.running = true;
    }

    function carryOn(then) {
        try {
            then();
        } catch (error) {
            fail(error.message);
        }
    }

    FileView {
        id: reading

        printErrors: false
        blockLoading: true
    }

    // A file's text, or "" where there's none.
    function read(path) {
        reading.path = "";
        reading.path = path;
        return reading.text();
    }

    function ready() {
        if (root.waiting === "the programs" && root.entry() !== undefined
                && applications.recents.of(root.program).length === 5
                && inkscape.drawings.length === 4)
            Qt.callLater(root.cases);
    }

    function cases() {
        if (root.waiting !== "the programs")
            return;
        root.waiting = "its step";
        try {
            sums();
            HeardWindows.told(applications.windowsDaemon, Niri.windows,
                id => DesktopEntries.heuristicLookup(id), Niri.current);
            theLauncher.open = true;
            theLauncher.held = [{ row: entry(), rows: [] }];
            // A step's frame never waits on its drawings, which the
            // programs daemon reads.
            same("nothing waits for the step's frame", theLauncher.frames.due.length, 0);
            theLauncher.drawn();
            stepped();
        } catch (error) {
            fail(error.message);
        }
    }

    // Inkscape.js's own cases, as Qt's engine runs them.
    function sums() {
        // [its root's start tag, its size as said]
        const sizes = [
            ["<svg width=\"210mm\" height=\"297mm\" viewBox=\"0 0 210 297\">",
                "210 \u00d7 297\u00a0mm"],
            ["<svg width=\"8.5in\" height=\"280mm\">", "8.5\u00a0in \u00d7 280\u00a0mm"],
            ["<svg width=\"100%\" height=\"100%\" viewBox=\"0 0 1920 1080\">",
                "1920 \u00d7 1080\u00a0px"],
            ["<svg viewBox=\"0,0,48,32.25\">", "48 \u00d7 32.25\u00a0px"],
            ["<svg width=\"640\" height=\"480.123456\">", "640 \u00d7 480.123\u00a0px"],
            ["<svg width=\" 2 \" height=\"3Q\">", "2\u00a0px \u00d7 3\u00a0Q"],
            ["<svg width=\"1e2pt\" height=\"50PT\">", "100 \u00d7 50\u00a0pt"],
            ["<svg width=\"0\" height=\"10\" viewBox=\"0 0 0 10\">", ""],
            ["<svg width=\"10em\" height=\"10em\">", ""],
            ["<svg>", ""],
            ["", ""]
        ];
        for (const [tag, said] of sizes)
            same(`${tag}'s size`, Inkscape.dimensions(Inkscape.extent(tag)), said);

        const printed = ["/d/b.svg\t2000000\t40\t<svg viewBox=\"0 0 4 2\">",
            "/d/a.svg\t3000123\t5\t", "/d/x.svg\t\t1\t", "/d/c.svg\t1000000\t\t",
            "/d/c.svg\t1000000\tsome\t", "/d/a.svg\t1\t9\t"];
        json("what's still there, newest first, when to the microsecond, at most as many asked",
            Inkscape.drawings(printed, 1), [{ path: "/d/a.svg", at: 3000.123, size: 5,
                extent: null }]);
        json("each once, one without a time or a weight passed over",
            Inkscape.drawings(printed, 9).map(one => one.path), ["/d/a.svg", "/d/b.svg"]);
        same("each one's size read", Inkscape.dimensions(Inkscape.drawings(printed, 9)[1].extent),
            "4 \u00d7 2\u00a0px");
        json("nothing printed, no drawings", Inkscape.drawings([], 9), []);

        const words = inkscape.words;
        json("its drawer in Programs: the last, and how many", Inkscape.held([
            { path: "/d/a.svg", at: root.now - 720000 }, { path: "/d/b.svg", at: 1 }],
        root.now, words), [
            { id: "inkscape:last", value: "a.svg \u00b7 12\u00a0min ago" },
            { id: "inkscape:lately", value: "2" }
        ]);
        json("none, nothing said", Inkscape.held([], root.now, words), []);

        const newest = { picture: "/kept/1.png", across: 4, down: 2, about: "PNG \u00b7 70 B" };
        json("an SVG copied, Inkscape's own first",
            Inkscape.copied(["image/png", "image/svg+xml", "image/x-inkscape-svg"], "image/png",
                newest), { type: "image/x-inkscape-svg", vector: true, across: 0, down: 0,
                about: "" });
        json("a picture the history keeps",
            Inkscape.copied(["image/png"], "image/png", newest),
            { type: "image/png", vector: false, across: 4, down: 2, about: "PNG \u00b7 70 B" });
        same("one it doesn't, nothing", Inkscape.copied(["image/png"], "image/png", null), null);
        same("its size unknown, nothing", Inkscape.copied(["image/png"], "image/png",
            { picture: "/kept/1.png", across: 0, down: 0 }), null);
        same("text, nothing", Inkscape.copied(["text/plain"], "", newest), null);
        same("said beside it", Inkscape.copiedSaid(Inkscape.copied(["image/png"], "image/png",
            newest), words.said), "4 \u00d7 2 \u00b7 PNG \u00b7 70 B");
        same("an SVG said so", Inkscape.copiedSaid({ vector: true }, words.said), "SVG");
        same("a new drawing's file", Inkscape.fileFor("/d", false, "2026-11-09 08.30.12"),
            "/d/Drawing 2026-11-09 08.30.12");
        same("a copied one's", Inkscape.fileFor("/d", true, "2026-11-09 08.30.12"),
            "/d/Copied 2026-11-09 08.30.12");
    }

    // Stepped into Inkscape, its drawings read.
    function stepped() {
        const acts = entry().verbs();
        const at = acts.findIndex(row => row.name === "New Drawing");
        expect("New Drawing after the desktop entry's own acts", at > 0, names(acts));
        same("and nothing copied, nothing to make a drawing of",
            names(acts).includes("From the Copied Picture"), false);
        json("New Drawing found from the top", inkscape.foundActs,
            [{ program: root.program, key: "verb:inkscape:drawing" }]);

        const recent = acts.find(row => row.name === "Recent");
        const files = recent.targets();
        same("Inkscape's drawings by when, its export as the shared list has it, each once",
            files.map(row => row.key).join(" "),
            ["new.svg", "logo.svg", "export.png", "Tom & Jerry.svg", "icon.svgz"]
                .map(name => `file:${root.drawn}/${name}`).join(" "));
        same("Recent counts each once", recent.side, "5");
        const [fresh, logo, exported, tom, icon] = files;
        same("when it was opened beside it", logo.side, "12\u00a0min ago");
        same("just now", fresh.side, "just now");
        same("days ago", icon.side, Qt.formatDate(new Date(root.now - 172800000), "dddd"));
        same("an export is the shared list's row, saying when and then its folder",
            exported.side, "16\u00a0min ago \u00b7 ~/Drawings");
        same("its drawer says where it is and its kind", logo.drawer.eyebrow,
            "in ~/Drawings \u00b7 drawing");
        same("as written, since its folder is a name", logo.drawer.eyebrowNames, true);
        same("then its size, its weight and its path", facts(logo),
            `Dimensions 210 \u00d7 297\u00a0mm, Size ${logo.drawer.facts[1].value}, `
            + "Path ~/Drawings/logo.svg");
        expect("its weight in bytes", logo.drawer.facts[1].value.endsWith("\u00a0B"),
            facts(logo));
        json("its path marked an identifier", logo.drawer.facts.map(fact =>
            fact.identifies === true), [false, false, true]);
        same("each in its own units", facts(fresh).split(", ")[0],
            "Dimensions 8.5\u00a0in \u00d7 280\u00a0mm");
        same("a share, by its viewBox", facts(tom).split(", ")[0],
            "Dimensions 1920 \u00d7 1080\u00a0px");
        same("compressed, read through gzip", facts(icon).split(", ")[0],
            "Dimensions 48 \u00d7 32\u00a0px");
        logo.run();
        json("Return opens it in Inkscape", Niri.spawned,
            [Quickshell.env("INKSCAPE"), `${root.drawn}/logo.svg`]);
        same("inside, Open, then Copy the Path", names(logo.verbs()), "Open, Copy the Path");

        json("its drawer in Programs", inkscape.programFacts(root.program).map(fact =>
            `${fact.label} ${fact.value}`), ["Last Drawing new.svg \u00b7 just now",
            "Recent Drawings 4"]);
        same("another program's, nothing", inkscape.programFacts("mpv").length, 0);

        acts[at].run();
        spawn(root.madeFresh);
    }

    // New Drawing ran, as niri would.
    function madeFresh() {
        const given = read(`${root.home}/given`).trim();
        expect("Inkscape is given a new drawing in the folder",
            given.startsWith(`${root.made}/Drawing `) && given.endsWith(".svg"), given);
        const template = read(Quickshell.env("TEMPLATE"));
        expect("which holds the template", template !== "" && read(given) === template,
            read(given));
        same("the template left as it was", template, read(Quickshell.env("TEMPLATE_KEPT")));
        root.waiting = "a copy";
        Copied.types = ["image/svg+xml", "text/plain"];
        const offered = entry().verbs().find(row => row.name
            === "New Drawing From the Copied Picture");
        same("an SVG copied makes one, saying so", offered?.side, "SVG");
        json("found from the top too", inkscape.foundActs.map(one => one.key),
            ["verb:inkscape:drawing", "verb:inkscape:copied"]);
        offered.run();
        spawn(root.madeOfVector);
    }

    function madeOfVector() {
        const given = read(`${root.home}/given`).trim();
        expect("Inkscape is given a drawing of the copy", given.startsWith(`${root.made}/Copied `),
            given);
        root.vector = given;
        same("which holds the SVG copied, as it was copied", read(given),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"3\" height=\"1\"/>\n");
        Copied.types = ["image/png"];
        same("a picture the history doesn't keep makes none",
            names(entry().verbs()).includes("From the Copied Picture"), false);
        copies.all = [{ picture: `${root.home}/kept.png`, across: 4, down: 2,
            about: "PNG \u00b7 3 B" }];
        const offered = entry().verbs().find(row => row.name
            === "New Drawing From the Copied Picture");
        same("one it keeps does, saying its size", offered?.side,
            "4 \u00d7 2 \u00b7 PNG \u00b7 3 B");
        offered.run();
        spawn(root.madeOfPicture);
    }

    function madeOfPicture() {
        const given = read(`${root.home}/given`).trim();
        expect("in a file of its own, the one before left as it was",
            given.startsWith(`${root.made}/Copied `) && given !== root.vector
            && read(root.vector).startsWith("<svg xmlns"), given);
        const svg = read(given);
        expect("a drawing the picture's size, its desk in the theme's colours", svg.includes(
            "width=\"4\" height=\"2\" viewBox=\"0 0 4 2\"") && svg.includes(
            "bordercolor=\"the-edge\" inkscape:deskcolor=\"the-sunken\""), svg);
        expect("the picture in it whole", svg.includes(
            "<image width=\"4\" height=\"2\" href=\"data:image/png;base64,UE5H\" />"), svg);
        Copied.types = [];
        same("nothing copied, nothing offered", inkscape.foundActs.length, 1);
        Qt.exit(0);
    }
}
