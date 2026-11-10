// Drives the reader's step in the launcher (modules/reader/ReaderProgram.qml,
// through modules/quickshell/launcher/Applications.qml) through Quickshell
// itself, over the reader's desktop entry written beside this, with niri,
// the desktop's shortcuts and what's playing stood in, the shell's own
// watch of the clipboard (Copied.qml) with wl-paste stood in, and reader
// --places by a script printing what it would. First, in Qt's own engine,
// Reader.js's cases: the lines reader --places prints, read, a share said
// in whole percent and never rounded up to the whole, where a place is,
// what a row says beside it, a drawer's plate, what's still to finish and
// the commands that read a file on or from a mark.
//
// Then the reader's Recent: the file a reader shows now listed above it
// and not in it, then each file the reader left, by when it left it,
// newest first, with the list programs share, which gives no times, after
// them, each file listed once. One left part way says the heading it was
// left under, how far down and when, "Installing · 40% · 12 min ago",
// and Return reads on there, with Read On, saying where, a Read From Mark
// for each mark by its letter, which reads from there, and Copy the Path
// inside; one read to its end says so instead. Each drawer says where it
// is and its kind, then the heading, how far down, each mark and its path,
// in the Name face, and the reader's own drawer how many files are still
// to finish. reader --places is run once the reader is stepped into,
// never before, once each opening. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect
import "Frames.js" as Frames
import "launcher/Reader.js" as Reader

ShellRoot {
    id: root

    property string waiting: "the programs"

    readonly property string home: Quickshell.env("HOME")
    readonly property string notes: `${home}/notes`
    readonly property real now: Number(Quickshell.env("NOW")) * 1000

    function expect(what, holds, said) { Expect.expect(what, holds, said); }

    function fail(message) { Expect.fail(message); }

    function same(what, got, wanted) {
        expect(what, got === wanted, got);
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function program(id) {
        return applications.things.find(row => row.key === `program:${id}`);
    }

    function file(name) {
        return `file:${root.notes}/${name}.md`;
    }

    // How many times reader --places ran, as the script counts them.
    function listings() {
        return Number(counted.text().trim() || "0");
    }

    // A drawer's facts as they read, a label and its value each.
    function facts(row) {
        return row.drawer.facts.map(fact => `${fact.label} ${fact.value}`).join(", ");
    }

    QtObject {
        id: theLauncher

        property bool open: false
        property var selected: null
        property string typed: ""
        property var held: []
        property var belowChord: []
        property var sources: [reader]
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

    Launcher.ReaderProgram {
        id: reader

        launcher: theLauncher
        // Its places, which the programs daemon lists by this key.
        settings: ({ name: "reader", reader: "reader", places: "listed:reader:places" })
    }

    Launcher.Applications {
        id: applications

        launcher: theLauncher
        settings: ({
            name: "applications",
            recents: [{
                // The list programs share, as recent-files prints it.
                watch: `${root.home}/shared`,
                list: [Quickshell.env("SHARED_LIST")]
            }, {
                // What each reader shows now, as the reader's own source says.
                app: "reader",
                kind: "file",
                opened: [Quickshell.env("SHOWING_LIST")]
            }]
        })
        onThingsChanged: root.ready()
    }

    Connections {
        target: applications.recents

        function onSharedChanged() {
            root.ready();
        }

        function onOpenNowChanged() {
            root.ready();
        }
    }

    Connections {
        target: reader

        function onLeftChanged() {
            root.ready();
        }
    }

    FileView {
        id: counted

        path: `${root.home}/listed`
        printErrors: false
        blockLoading: true
    }

    function ready() {
        if (root.waiting === "the programs" && root.program("reader") !== undefined
                && applications.recents.of("reader").length === 2)
            Qt.callLater(root.cases);
        else if (root.waiting === "the reader's places" && reader.left.length === 4
                && applications.recents.openIn("reader").length === 1)
            Qt.callLater(root.stepped);
    }

    function cases() {
        if (root.waiting !== "the programs")
            return;
        root.waiting = "the reader's places";
        try {
            sums();
            Niri.revision += 1;
            theLauncher.open = true;
            program("reader").verbs();
            counted.reload();
            counted.waitForJob();
            same("nothing is listed before the reader is stepped into", listings(), 0);
            theLauncher.held = [{ row: program("reader"), rows: [] }];
            // The programs daemon runs reader --places, so a step's frame
            // never waits on starting a process, nor on anything after it.
            same("reader --places asked of the daemon, nothing left for the frame",
                theLauncher.frames.due.length, 0);
            theLauncher.drawn();
        } catch (error) {
            fail(error.message);
        }
    }

    // Reader.js's own cases, as Qt's engine runs them.
    function sums() {
        const read = Reader.places([
            JSON.stringify({ file: "/a.md", left: 1700000000, at_end: false, heading: "Installing",
                through: 0.4, marks: [{ letter: "b" }, { letter: "a", heading: "Using It",
                    through: 0.625 }, { letter: "ab" }] }),
            JSON.stringify({ file: "relative.md", left: 1700000000 }),
            JSON.stringify({ file: "/no-time.md" }),
            JSON.stringify({ file: "/old.md", left: 1700000000, heading: " ", through: "0.3" }),
            "not a place"]);
        same("only whole places are read, each mark of one letter, in the letters' order",
            JSON.stringify(read), JSON.stringify([{ path: "/a.md", at: 1700000000000, end: false,
                heading: "Installing", through: 0.4, marks: [
                    { letter: "a", heading: "Using It", through: 0.625 },
                    { letter: "b", heading: null, through: null }] },
            { path: "/old.md", at: 1700000000000, end: false, heading: null, through: null,
                marks: [] }]));
        const said = reader.words.said;
        // [how far down, as said]
        const shares = [[0, "0%"], [0.4, "40%"], [0.999, "99%"], [1, "100%"], [null, ""]];
        for (const [through, share] of shares)
            same(`${through} of the way down`, Reader.share(through, said), share);
        same("where, the heading and how far", Reader.where(read[0], said),
            "Installing \u00b7 40%");
        same("how far alone, under no heading", Reader.where({ heading: null, through: 0.25 },
            said), "25%");
        same("nothing, where neither is known", Reader.where(read[1], said), "");
        same("read to the end says so instead", Reader.whereLeft({ end: true,
            heading: "Last Words", through: 1 }, said), "read to the end");

        // [when it was left, before now, in seconds, as its side says it]
        const sides = [[30, "Installing \u00b7 40% \u00b7 just now"],
            [720, "Installing \u00b7 40% \u00b7 12\u00a0min ago"]];
        for (const [before, side] of sides)
            same(`its side, left ${before} s ago`, Reader.side(Object.assign({}, read[0],
                { at: root.now - before * 1000 }), root.now, reader.words), side);
        same("an old place says only when", Reader.side(Object.assign({}, read[1],
            { at: root.now }), root.now, reader.words), "just now");

        const counts = { file1: "1 file", files: "{count} files" };
        same("what's still to finish, a file read to its end not among them",
            JSON.stringify(Reader.held([read[0], read[1], { end: true }], counts)),
            JSON.stringify([{ id: "reader:to-finish", value: "2 files" }]));
        same("one file", Reader.held([read[0]], counts)[0].value, "1 file");
        same("nothing left part way, nothing said", Reader.held([{ end: true }], counts).length, 0);

        same("a mark named by its letter as it is", Reader.markName("Mark", "A"), "Mark A");
        same("reading on", JSON.stringify(Reader.reading("reader", "/a.md", "")),
            JSON.stringify(["reader", "/a.md"]));
        same("reading from a mark", JSON.stringify(Reader.reading("reader", "/a.md", "a")),
            JSON.stringify(["reader", "--at", "a", "/a.md"]));
    }

    // Stepped into the reader, with its places listed and what it shows now
    // read.
    function stepped() {
        if (root.waiting !== "the reader's places")
            return;
        root.waiting = "listed again";
        try {
            counted.reload();
            counted.waitForJob();
            same("reader --places runs as the reader is stepped into", listings(), 1);
            const acts = program("reader").verbs();
            const showing = acts.find(row => row.key === file("now"));
            same("the file shown now, above Recent", showing?.side, "open \u00b7 ~/notes");
            const recent = acts.find(row => row.name === "Recent");
            const files = recent.targets();
            same("each place by when left, then the shared list's rest, each once, none open",
                files.map(row => row.key).join(" "),
                ["guide", "old", "done", "shared"].map(name => file(name)).join(" "));
            same("so does the reader's row", program("reader").note, "1 open \u00b7 4 recent");

            const [guide, old, done, shared] = files;
            same("where it was left and when, beside it", guide.side,
                "Installing \u00b7 40% \u00b7 12\u00a0min ago");
            same("one under no heading", old.side, "25% \u00b7 50\u00a0min ago");
            same("one read to its end says so", done.side,
                `read to the end \u00b7 ${Qt.formatDate(new Date(root.now - 172800000), "dddd")}`);
            same("Return reads on", guide.verb, "read");
            expect("when it was left, to be ordered by", Math.abs(guide.at - (root.now - 720000))
                < 1, guide.at);
            same("its drawer says where it is and its kind", guide.drawer.eyebrow,
                "in ~/notes \u00b7 markdown");
            same("as written, since its folder is a name", guide.drawer.eyebrowNames, true);
            same("then the heading, how far, each mark by its letter and its path", facts(guide),
                "Heading Installing, Read 40%, Mark a Using It \u00b7 62%, Mark b kept, "
                + "Path ~/notes/guide.md");
            same("its path marked an identifier", JSON.stringify(guide.drawer.facts.map(fact =>
                fact.identifies === true)), JSON.stringify([false, false, false, false, true]));
            same("one read to its end, all of it", facts(done),
                "Heading Last Words, Read all of it, Path ~/notes/done.md");
            same("one the shared list alone has is as it has it", shared.note, "");

            guide.run();
            same("Return reads on, as the reader opens a file", JSON.stringify(Niri.spawned),
                JSON.stringify(["reader", `${root.notes}/guide.md`]));
            const inside = guide.verbs();
            same("inside, Read On first, then each mark by its letter", names(inside),
                "Read On, Read From Mark a, Read From Mark b, Copy the Path");
            same("Read On says where", inside[0].side, "Installing \u00b7 40%");
            same("a mark says where it is", inside[1].side, "Using It \u00b7 62%");
            same("named on the line by its letter", JSON.stringify(inside[1].fill),
                JSON.stringify({ letter: "a" }));
            inside[1].run();
            same("a mark reads from there", JSON.stringify(Niri.spawned),
                JSON.stringify(["reader", "--at", "a", `${root.notes}/guide.md`]));
            same("one read to its end reads on there too", names(done.verbs()),
                "Read On, Copy the Path");

            same("the reader's drawer says what's still to finish",
                JSON.stringify(reader.programFacts("reader")),
                JSON.stringify([{ label: "Still to Finish", value: "3 files" }]));
            same("and nothing of another program's", reader.recentOf("mpv").length
                + reader.programFacts("mpv").length, 0);

            // Stepped out and in again, nothing's listed again until the
            // launcher opens afresh.
            theLauncher.held = [];
            theLauncher.held = [{ row: program("reader"), rows: [] }];
            counted.reload();
            counted.waitForJob();
            same("once an opening", listings(), 1);
            theLauncher.held = [];
            theLauncher.open = false;
            theLauncher.open = true;
            theLauncher.held = [{ row: program("reader"), rows: [] }];
            theLauncher.drawn();
            listedAgain.start();
        } catch (error) {
            fail(error.message);
        }
    }

    // Long enough for a listing started on stepping in to have run.
    Timer {
        id: listedAgain

        interval: 1000
        onTriggered: {
            try {
                counted.reload();
                counted.waitForJob();
                root.same("and listed again once it's stepped into on the next opening",
                    root.listings(), 2);
                Qt.exit(0);
            } catch (error) {
                root.fail(error.message);
            }
        }
    }
}
