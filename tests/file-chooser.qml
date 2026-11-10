// Drives the launcher's chooser of files (modules/open-with/
// FileChooser.qml) through Quickshell itself, as open-with asks it, over
// folders the check makes, with the launcher stood in and a socket standing
// in for open-with's. Asked to open, it opens in the folder asked for, every
// folder on the way held above it and each saying the one held deepest, so
// the path is one folder; it lists the files opened lately
// elsewhere first, then its folders, then the files the filter passes, by
// name, then the program's filters, each group apart; a file is one row,
// though it was opened lately too, and the hidden ones show once a "." is
// typed; a file kept in the Nix store goes by its own name past its hash
// when opened lately elsewhere, and keeps the hash where the store lists
// it; a file in the folder shown says its size and when it changed
// rather than its folder, and its plate where it is before it's previewed,
// it and those selected after it asked of the one file-preview kept for
// the ask (the check's script reads which); a kind the filter names passes
// its narrower kinds too; another filter picked shows what it passes; and a
// V selection's Return, taking one file, answers its first with the filter
// in use, one holding no file being left to the launcher. Asked to save, the line holds the name the
// program gave, a prompt's words, its caret before the extension, kept
// from folder to folder; the line is each folder's row Return saves by,
// the name not listed again below, nothing narrowed by it, and a
// name already there asks once, "Replace a.md? (y/n)", no saving nothing;
// a name typed is saved by :w in the folder it's typed in, asking nothing,
// and :w with a name already there asks once. Its commands are :sort,
// name, changed or size, :sort! turning it round, the folder made afresh,
// an order the chooser made again after a restart keeps; :set hidden and
// nohidden; :cd and :e, their paths completed from folders read, going
// to a folder as if the ask opened there; and a save's :w and :x. Asked
// for a folder, it lists no files and each folder leads with Open This
// Folder. In every folder, the field asks for one file, several, a folder
// or where to save, in the program's word. Closing the launcher answers nothing, a request that isn't JSON
// is refused at once, and a withdrawn one closes the launcher. Where several
// may be opened, :arga gathers the cursor row's file or a range's, each
// once, a folder passed over; a file gathered wears the mark in an act's
// hue, and Return says how many it opens; :args steps into Gathered, the
// files each with its folder, then Forget Them and Copy the Paths, its
// plate counting them; and Return answers every one gathered, in the order
// they were, wherever each was. A
// folder holds its Copy the Path and filters only once it's read. A place kept elsewhere, given by a
// stand-in for Google's Drive, is listed after home's files, apart, and
// found by typing from another folder, never for a local folder alone;
// held, it says
// its folder's name; a file there is drawn as a file, or as the row it is
// everywhere, and picked once fetched, the launcher staying and saying so
// until then; gathered with others, it's answered in the order gathered, and
// one that can't be fetched keeps the launcher, saying why, until Return
// again fetches it. Saving there, it holds only the folders a file can
// be saved in, each line the row the name saves by, a name already
// there asking once; the name typed is saved instead, and Return makes the
// folder the place stages it in and answers the path there, closing the
// launcher once it's made, the program that asked written beside it (the
// check reads it). A save here keeps the folder it went to, one
// asked from inside the shell too, never an open nor a save staged
// elsewhere, and the chooser made again reads it. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string notes: `${home}/notes`

    // What open-with's socket was told, an answer a connection, in order.
    property var answers: []
    property int asked: 0
    property int stage: 0

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: theLauncher

        property bool open: false
        property var held: []
        property var selected: null
        property string into_: ""
        property int homeDepth: 0
        readonly property var sources: [theChooser, drive]

        function into(name) {
            open = true;
            into_ = name;
            held = [{ row: { key: "", from: theChooser }, rows: [] }];
        }

        function enter(row) {
            held = held.concat([{ row: row, rows: row.verbs() }]);
            return true;
        }

        function back() {
            held = held.slice(0, -1);
        }

        function openedHere() {
            homeDepth = held.length;
        }

        // Each walk to another place by several steps, counted as the one
        // jump the launcher's jump list keeps of it.
        property int jumps: 0

        function oneJump(walk) {
            const place = () => held.map(step => step.row.key).join(" ");
            const was = place();
            walk();
            if (place() !== was)
                jumps += 1;
        }

        // Each question asked once, as the launcher's line asks it.
        property var asked: []

        function askOnce(question, yes, no) {
            asked = asked.concat([{ question: question, yes: yes, no: no }]);
        }
    }

    Launcher.FileChooser {
        id: theChooser

        launcher: theLauncher
        settings: ({
            name: "fileChooser",
            recent: [Quickshell.env("RECENT_LIST")],
            kinds: Quickshell.env("KINDS"),
            preview: [Quickshell.env("PREVIEWER")],
            savedIn: `${root.home}/state/open-with/saved-in`,
            orderIn: `${root.home}/state/open-with/order`
        })
    }

    // The chooser again, as the shell makes it once it's restarted, made
    // at the end to read where a save last went.
    property var restarted: null

    Component {
        id: restarting

        Launcher.FileChooser {
            launcher: theLauncher
            settings: theChooser.settings
            onSavedLastChanged: Qt.callLater(root.step)
            onOrderChanged: Qt.callLater(root.step)
        }
    }

    // A stand-in for Google's Drive (modules/google): one place, a folder in
    // it, a file and a Google document there, each fetch kept for the case
    // to answer.
    QtObject {
        id: drive

        property var fetches: []
        // What the picker gave it to make rows with, as it was last asked.
        property var asked: null

        function fetched(key) {
            return (done, failed) => {
                drive.fetches = drive.fetches.concat([{ key: key, done: done, failed: failed }]);
                Qt.callLater(root.step);
            };
        }

        function chooserPlaces(chooser) {
            if (chooser.folders)
                return [];
            drive.asked = chooser;
            if (chooser.saving)
                return [chooser.folder({ key: "drive", name: "Google Drive", note: "",
                    inside: () => [chooser.folder(root.savedIn("root", { key: "drive:folder:mine",
                        name: "My Drive", note: "", inside: () => [chooser.folder(root.savedIn("plans", {
                            key: "drive:folder:plans", name: "Plans", note: "", inside: () => [] }))] }))] })];
            const files = () => [
                chooser.file({ key: "drive:report", name: "report.pdf", note: "In My Drive.",
                    drawer: null, adopt: null, fetch: fetched("drive:report") }),
                chooser.file({ key: "gdoc:plan", name: "Plan", note: "As a .docx copy.", drawer: null,
                    adopt: { key: "gdoc:plan", what: "document", component: "document", name: "Plan",
                        glyph: "d", hue: "program", side: "Docs", verb: "open", run: () => {},
                        verbs: () => [] },
                    fetch: fetched("gdoc:plan") })
            ];
            return [chooser.folder({ key: "drive", name: "Google Drive", note: "",
                inside: () => [chooser.folder({ key: "drive:folder:mine", name: "My Drive", note: "",
                    inside: files }), chooser.folder({ key: "drive:folder:shared", name: "Shared with Me",
                    note: "", inside: () => [], empty: () => "Listing it." })] })];
        }
    }

    // Where the stand-in stages a file saved in its folder, and the names
    // its files have.
    function savedIn(folder, fields) {
        return Object.assign(fields, {
            saveTo: name => `${root.home}/staged/${folder}/${name}`,
            names: () => folder === "root" ? ["report.pdf"] : []
        });
    }

    // A socket of open-with's, by its number: each answer it's given.
    Variants {
        model: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

        SocketServer {
            id: socket

            required property int modelData

            active: true
            path: `${root.runtime}/open-with/${socket.modelData}.sock`
            handler: Socket {
                id: told

                property string said: ""

                parser: SplitParser {
                    onRead: line => told.said += line
                }
                onConnectionStateChanged: {
                    if (!told.connected) {
                        root.answers = root.answers.concat([told.said]);
                        Qt.callLater(root.step);
                    }
                }
            }
        }
    }

    function ask(number, request) {
        asked = number;
        theChooser.requested(`${runtime}/open-with/${number}.sock`, typeof request === "string"
            ? request : JSON.stringify(request));
    }

    function names(rows) {
        return rows.map(row => `${row.gap ? "| " : ""}${row.name}${row.side === "chosen" ? " (chosen)" : ""}`)
            .join(", ");
    }

    function rowsIn(path) {
        return theChooser.rowsOf(path);
    }

    // The row a save's line is in the folder given: the name Return saves.
    function lineIn(path) {
        return theChooser.folderRow(path).line();
    }

    function named(rows, name) {
        return rows.find(row => row.name === name);
    }

    function heldPaths() {
        return theLauncher.held.slice(1).map(step => step.row.key).join(" ");
    }

    // What the field asks in the folder held deepest, as the launcher
    // asks it of the step it's on.
    function fieldAsks() {
        return theLauncher.held[theLauncher.held.length - 1].row.ask;
    }

    // Whether the folders it needs have been read: notes, and those on the
    // way, and, once it's read, the files opened lately; and the kinds
    // known, both the endings and the kinds each is one of, since the
    // Markdown filter passes a file by its kind. Under a loaded builder the
    // folders came first, and the case read notes with no files in it
    // (Y9XISP, 2026-11-09: "sub, | Markdown (chosen), All Files").
    function ready(paths) {
        return paths.every(path => theChooser.listed[path] !== undefined)
            && Object.keys(theChooser.kinds.endings).length > 0
            && Object.keys(theChooser.kinds.broader).length > 0
            && (theChooser.settings.recent.length === 0 || theChooser.lately.length > 0);
    }

    // Every folder a case reads from: notes, and each on the way to it, the
    // home :cd completes "~/no" from among them. An answer forgets what was
    // read, and each folder is read again on its own, so a case waiting on
    // notes alone could find home not yet read under load, and :cd
    // completing nothing (2026-11-07).
    function readFrom() {
        return ["/", notes].concat(theChooser.onTheWay(notes));
    }

    Connections {
        target: theChooser

        function onChanged() {
            Qt.callLater(root.step);
        }

        function onPreviewsChanged() {
            Qt.callLater(root.step);
        }

        function onKindsChanged() {
            Qt.callLater(root.step);
        }
    }

    // The file Return picks once the files selected have been previewed,
    // and the one whose preview is waited on.
    property var picking: null
    property string previewing: ""

    // Each case, once what it waits on is there.
    function step() {
        try {
            if (stage === 0 && ready(readFrom())) {
                stage = 1;
                opening();
            } else if (stage === 1 && previewing === "a.md" && `${notes}/a.md` in theChooser.previews) {
                // Two more selected in turn, the later of which the check's
                // script finds was asked of the one file-preview a.md was;
                // the one passed over may never be asked.
                previewing = "old.md";
                const rows = rowsIn(notes);
                named(rows, "notes.txt").drawer();
                named(rows, "old.md").drawer();
            } else if (stage === 1 && previewing === "old.md"
                && `${home}/elsewhere/old.md` in theChooser.previews) {
                stage = 2;
                // A V selection's Return, taking one file, answers its first.
                taken = theChooser.folderRow(notes).takes([{ row: picking, at: 1 },
                    { row: named(rowsIn(notes), "notes.txt"), at: 2 }]);
            } else if (stage === 2 && answers.length === 1) {
                stage = 3;
                expect("a V selection's Return answers its first file, taking one, and the filter",
                    taken && !theLauncher.open
                        && answers[0] === JSON.stringify({ paths: [`${notes}/a.md`], filter: 1,
                            choices: {} }), answers[0]);
                ask(2, {
                    mode: "save",
                    title: "Save the Notes",
                    accept: "_Save",
                    folder: notes,
                    name: "a.md",
                    filters: [],
                    choices: [{ id: "encoding", label: "Encoding", default: "utf8",
                        options: [{ id: "utf8", label: "UTF-8" }, { id: "latin1", label: "Latin-1" }] }]
                });
            } else if (stage === 3 && theChooser.asking?.reply.endsWith("/2.sock")
                && ready(readFrom())) {
                stage = 4;
                save();
            } else if (stage === 4 && answers.length === 2) {
                stage = 5;
                expect("what's typed saves in the folder it's typed in, with each choice's option",
                    answers[1] === JSON.stringify({ paths: [`${notes}/sub/new.md`], filter: null,
                        choices: { encoding: "latin1" } }), answers[1]);
                expect("a save keeps the folder it went to", theChooser.savedLastIn() === `${notes}/sub`,
                    theChooser.savedLastIn());
                ask(3, { mode: "open", title: "Pick a Folder", folders: true, folder: notes });
            } else if (stage === 5 && theChooser.asking?.reply.endsWith("/3.sock")
                && ready(readFrom())) {
                stage = 6;
                folder();
            } else if (stage === 6 && answers.length === 3) {
                stage = 7;
                expect("Open This Folder answers the folder", answers[2]
                    === JSON.stringify({ paths: [`${notes}/sub`], filter: null, choices: {} }), answers[2]);
                ask(4, { mode: "open", title: "Open", folder: notes });
            } else if (stage === 7 && theChooser.shown.endsWith("/4.sock")) {
                stage = 8;
                command(notes, "quit").run(called(""));
            } else if (stage === 8 && answers.length === 4) {
                stage = 9;
                expect(":q closes the launcher, answering nothing", answers[3] === ""
                    && !theLauncher.open, answers[3]);
                ask(5, "not a request");
            } else if (stage === 9 && answers.length === 5) {
                stage = 10;
                expect("a request that isn't JSON is refused at once", answers[4] === ""
                    && theChooser.asks.length === 0, `${answers[4]}, ${theChooser.asks.length} waiting`);
                ask(6, { mode: "open", title: "Open", folder: notes });
            } else if (stage === 10 && theChooser.shown.endsWith("/6.sock")) {
                stage = 11;
                theChooser.withdrawn(`${runtime}/open-with/6.sock`);
                expect("a withdrawn request closes the launcher", !theLauncher.open
                    && theChooser.asks.length === 0, `${theLauncher.open}, ${theChooser.asks.length} waiting`);
                ask(7, { mode: "open", title: "Open Several", multiple: true, folder: notes });
            } else if (stage === 11 && theChooser.asking?.reply.endsWith("/7.sock") && ready([notes])) {
                stage = 12;
                gathering();
            } else if (stage === 12 && answers.length === 6) {
                stage = 13;
                expect("Return answers every one gathered, in the order they were",
                    answers[5] === JSON.stringify({ paths: [`${notes}/a.md`, `${home}/elsewhere/old.md`],
                        filter: null, choices: {} }), answers[5]);
                theChooser.chooseToSave({ name: "copy.pdf", folder: notes, title: "Save a Copy", local: true },
                    root.heardFromTheShell);
            } else if (stage === 13 && theChooser.shown.startsWith("shell:") && ready([notes])) {
                stage = 14;
                savingForTheShell();
            } else if (stage === 14 && shellPicked !== null) {
                stage = 15;
                expect("an ask from inside the shell is answered with the path, never on a socket",
                    JSON.stringify(shellPicked) === JSON.stringify([`${notes}/copy.pdf`])
                        && answers.length === 6, `${JSON.stringify(shellPicked)}, ${answers.length} answers`);
                expect("a save asked from inside the shell keeps its folder too",
                    theChooser.savedLastIn() === notes, theChooser.savedLastIn());
                shellPicked = null;
                theChooser.chooseToSave({ name: "copy.pdf", folder: notes }, root.heardFromTheShell);
            } else if (stage === 15 && theChooser.shown.startsWith("shell:")) {
                stage = 16;
                theLauncher.open = false;
            } else if (stage === 16 && shellPicked !== null) {
                stage = 17;
                expect("closing the launcher on an ask from inside the shell answers none",
                    shellPicked.length === 0 && theChooser.asks.length === 0,
                    `${JSON.stringify(shellPicked)}, ${theChooser.asks.length} waiting`);
                ask(8, { mode: "open", title: "Open", folder: home });
            // Shown, not only asked: an ask's places are made as it's
            // shown, a turn after it comes, and until then they're the
            // save's before it, which lists no Shared with Me.
            } else if (stage === 17 && theChooser.shown.endsWith("/8.sock") && ready([home])) {
                stage = 18;
                elsewhere();
            } else if (stage === 18 && drive.fetches.length === 1) {
                stage = 19;
                expect("a file kept elsewhere keeps the launcher while it's fetched, saying so",
                    theLauncher.open && answers.length === 6 && reportRow.note === "Fetching it.",
                    `${theLauncher.open}, ${answers.length} answers, ${reportRow.note}`);
                drive.fetches[0].done(`${runtime}/picked/report.pdf`);
            } else if (stage === 19 && answers.length === 7) {
                stage = 20;
                expect("once it's fetched, its path is answered and the launcher closes",
                    answers[6] === JSON.stringify({ paths: [`${runtime}/picked/report.pdf`], filter: null,
                        choices: {} }) && !theLauncher.open, `${answers[6]}, ${theLauncher.open}`);
                expect("what's opened never moves where a save went", theChooser.savedLastIn() === notes,
                    theChooser.savedLastIn());
                ask(9, { mode: "open", title: "Open Several", multiple: true, folder: home });
            } else if (stage === 20 && theChooser.asking?.reply.endsWith("/9.sock") && ready([home])) {
                stage = 21;
                severalElsewhere();
            } else if (stage === 21 && drive.fetches.length === 3) {
                stage = 22;
                drive.fetches[1].done(`${runtime}/picked/plan.docx`);
                drive.fetches[2].failed("Google Drive couldn't be reached.");
                expect("one that can't be fetched keeps the launcher, saying why",
                    theLauncher.open && answers.length === 7
                        && severalRow.note === "Google Drive couldn't be reached.",
                    `${theLauncher.open}, ${answers.length} answers, ${severalRow.note}`);
                severalRow.run();
            } else if (stage === 22 && drive.fetches.length === 5) {
                stage = 23;
                drive.fetches[3].done(`${runtime}/picked/plan.docx`);
                drive.fetches[4].done(`${runtime}/picked/report.pdf`);
            } else if (stage === 23 && answers.length === 8) {
                stage = 24;
                expect("gathered with others, each is answered once fetched, in the order gathered",
                    answers[7] === JSON.stringify({ paths: [`${runtime}/picked/plan.docx`,
                        `${runtime}/picked/report.pdf`], filter: null, choices: {} }) && !theLauncher.open,
                    `${answers[7]}, ${theLauncher.open}`);
                ask(10, { mode: "save", title: "Save", name: "report.pdf", folder: home });
            } else if (stage === 24 && theChooser.asking?.reply.endsWith("/10.sock") && ready([home])) {
                stage = 25;
                savingElsewhere();
            } else if (stage === 25 && answers.length === 9) {
                stage = 26;
                expect("Return makes the folder the place stages it in, then answers the path there",
                    answers[8] === JSON.stringify({ paths: [`${home}/staged/plans/new.md`], filter: null,
                        choices: {} }) && !theLauncher.open, `${answers[8]}, ${theLauncher.open}`);
                expect("a save staged for a place kept elsewhere never moves where a save here went",
                    theChooser.savedLastIn() === notes, theChooser.savedLastIn());
                theChooser.orderBy("changed", true);
                restarted = restarting.createObject(root);
            } else if (stage === 26 && restarted?.savedLastIn() === notes
                && restarted.order === "changed" && restarted.reversed) {
                stage = 27;
                console.log("every case holds");
                Qt.callLater(() => Qt.exit(0));
            }
        } catch (error) {
            fail(error.message);
        }
    }

    function opening() {
        expect("it opens in its collection", theLauncher.into_ === "fileChooser", theLauncher.into_);
        const way = theChooser.onTheWay(notes).map(path => `folder:${path}`).join(" ");
        expect("every folder on the way is held above the one asked for", heldPaths() === way,
            `${heldPaths()}, not ${way}`);
        expect("the folder asked for is where it opened, so Ctrl+O has no folder on the way",
            theLauncher.homeDepth === theLauncher.held.length, `${theLauncher.homeDepth}`);
        expect("and the dialog has nothing above it, for - to leave the ask for",
            theChooser.collection.outermost === true, JSON.stringify(theChooser.collection));
        expect("and at its top the path says it is /, as a folder below says its own",
            theChooser.collection.place === "/", JSON.stringify(theChooser.collection));
        const titles = theLauncher.held.slice(1).map(step => step.row.title);
        expect("the path says one folder, the one held deepest",
            titles.every(title => title === "~/notes"), titles.join(" "));
        let rows = rowsIn(notes);
        expect("lately elsewhere, then folders, then files the filter passes, then the filters",
            names(rows) === "old.md, | sub, a.md, notes.txt, | Markdown (chosen), All Files",
            names(rows));
        const unread = names(theChooser.folderRow(`${home}/unread`).verbs());
        const read = names(theChooser.folderRow(notes).verbs());
        expect("a folder holds its Copy the Path and filters only once it's read, so they come last",
            unread === "" && read.endsWith("All Files, Copy the Path"), `${unread}; ${read}`);
        expect("a file opened lately elsewhere says where", rows[0].side === "~/elsewhere"
            && rows[0].key === `file:${home}/elsewhere/old.md`, `${rows[0].side} ${rows[0].key}`);
        const own = named(rows, "a.md");
        expect("a file in the folder shown says its size and when it changed, not its folder",
            own.side === "0\u00a0B \u00b7 just now", own.side);
        expect("a folder in it says when it changed, its count once it's known",
            named(rows, "sub").side === "just now", named(rows, "sub").side);
        const plate = own.drawer();
        const facts = plate.facts.map(fact => `${fact.label} ${fact.value}`).join(", ");
        expect("a file's plate says where it is and its name at once, before it's previewed,"
            + " its size and day its row's",
            plate.title === "a.md" && plate.preview === undefined
            && plate.eyebrow === "in ~/notes" && facts === "",
            `${plate.eyebrow}: ${plate.title}: ${facts}`);
        storeNames();
        expect("each file is drawn as every file is", rows.every(row => row.component
            || row.key.startsWith("file-chooser:")), rows.map(row => row.key).join(" "));
        const typed = theLauncher.held[theLauncher.held.length - 1].row.typed(".");
        expect("the hidden ones show once a dot is typed", names(typed) === ".secret, .hidden.txt",
            names(typed));
        named(rows, "All Files").run();
        rows = rowsIn(notes);
        expect("another filter picked shows what it passes",
            names(rows)
                === "old.md, | sub, a.md, b.png, notes.txt, | Markdown, All Files (chosen)",
            names(rows));
        const a = named(rows, "a.md");
        expect("Return on a file opens it by the program's word", a.verb === "upload"
            && a.verbs()[0].name === "Upload", `${a.verb} ${a.verbs()[0].name}`);
        expect("taking one file, the field asks for one, by the program's word, in every folder",
            fieldAsks() === "upload which file?"
                && theChooser.folderRow(`${notes}/sub`).ask === fieldAsks(), fieldAsks());
        besides();
        // Picked once a.md's plate, drawn above, is previewed.
        picking = a;
        previewing = "a.md";
    }

    // A file kept in the Nix store goes by its own name past its hash when
    // it's opened lately elsewhere, but where the store's own listing holds
    // it, it keeps the hash, or every row there would read alike (VKCF3Z
    // and X0G8U9, 2026-11-08). Its plate isn't asked for here, since asking
    // one asks file-preview, whose asks the check's script counts.
    function storeNames() {
        const whole = "bdmylmvw562fyhfd39fkn89zcw7qn66w-track.mp3";
        const path = `/nix/store/${whole}`;
        const away = theChooser.fileRow(path, true);
        expect("a store file opened lately elsewhere goes by its own name",
            away.name === "track.mp3" && away.side === "/nix/store", `${away.name} ${away.side}`);
        expect("one the store's listing holds keeps its hash",
            theChooser.fileRow(path).name === whole, theChooser.fileRow(path).name);
        expect("a store folder listed there keeps its hash too",
            theChooser.folderRow("/nix/store/bdmylmvw562fyhfd39fkn89zcw7qn66w-pkg").name
                === "bdmylmvw562fyhfd39fkn89zcw7qn66w-pkg",
            theChooser.folderRow("/nix/store/bdmylmvw562fyhfd39fkn89zcw7qn66w-pkg").name);
    }

    // ]b and [b go to the folder beside this one, as the folder holding it
    // lists its folders, round from the last to the first, held where this
    // one was; in a folder whose holder isn't read yet, once it's heard.
    function besides() {
        const depth = theLauncher.held.length;
        const deepest = () => theLauncher.held[theLauncher.held.length - 1].row.key;
        const around = rowsIn(home).map(row => row.key)
            .filter(key => key.startsWith(`folder:${home}/`));
        const next = around[(around.indexOf(`folder:${notes}`) + 1) % around.length];
        const jumps = theLauncher.jumps;
        theLauncher.held[depth - 1].row.beside(1);
        expect("]b goes to the next folder its holder lists, held where this one was",
            around.length > 1 && deepest() === next && theLauncher.held.length === depth,
            `${deepest()} at ${theLauncher.held.length}, not ${next} at ${depth} of ${around}`);
        expect("in one jump, as :bnext is", theLauncher.jumps === jumps + 1,
            `${theLauncher.jumps - jumps} jumps`);
        theLauncher.held[depth - 1].row.beside(-1);
        expect("[b goes to the one before", deepest() === `folder:${notes}`, deepest());
        for (let pressed = 0; pressed < around.length; pressed += 1)
            theLauncher.held[depth - 1].row.beside(1);
        expect("]b goes round from the last to the first", deepest() === `folder:${notes}`,
            deepest());
        const deep = `${home}/elsewhere/deep`;
        theLauncher.enter(theChooser.folderRow(`${deep}/one`));
        theLauncher.held[depth].row.beside(1);
        expect("]b in a folder whose holder isn't read waits for it",
            deepest() === `folder:${deep}/one`, deepest());
        theChooser.heard(deep, [{ name: "one", path: `${deep}/one`, folder: true },
            { name: "two", path: `${deep}/two`, folder: true }]);
        expect("and goes once it's heard", deepest() === `folder:${deep}/two`
            && theLauncher.held.length === depth + 1, deepest());
        theLauncher.back();
    }

    function save() {
        expect("saving, the field asks where, by the name given", fieldAsks() === "save a.md where?",
            fieldAsks());
        const rows = rowsIn(notes);
        expect("saving, the name given is the line's alone, then the folders lately, then the rest",
            names(rows) === "elsewhere, | sub, b.png, notes.txt, | Encoding", names(rows));
        const there = lineIn(notes);
        there.run();
        const asked = theLauncher.asked[0];
        expect("a name already there asks once before Return saves over it",
            there.verb === "save" && theLauncher.asked.length === 1
                && asked.question === "Replace a.md? (y/n)" && answers.length === 1,
            `${there.verb} ${JSON.stringify(theLauncher.asked)} ${answers.length} answers`);
        asked.no();
        expect("and no saves nothing", answers.length === 1 && theChooser.asking !== null,
            `${answers.length} answers`);
        named(rows, "Encoding").verbs()[1].run();
        sorting();
        writing();
        naming(theLauncher.held[theLauncher.held.length - 1].row);
        const sub = named(rows, "sub");
        theLauncher.enter(sub);
        expect("the name is kept from folder to folder", sub.words() === "new.md"
            && lineIn(`${notes}/sub`).name === "new.md", `${sub.words()}`);
        const typed = lineIn(`${notes}/sub`);
        expect("the name typed saves in the folder it's typed in", typed.verb === "save"
            && typed.side === "~/notes/sub", `${typed.verb} ${typed.side}`);
        theLauncher.open = false;
        command(`${notes}/sub`, "write").run({ name: "write", bang: false, arg: "", things: [],
            count: 0 });
        expect(":w on a name not there saves at once, asking nothing", theLauncher.asked.length === 1,
            JSON.stringify(theLauncher.asked));
    }

    // The picker's command named, in the folder given.
    function command(path, name) {
        return theChooser.commandsIn(path).find(row => row.command === name);
    }

    // What a command is run with, as V3's line calls it.
    function called(arg, bang) {
        return { name: "", bang: bang === true, arg: arg, things: [], count: 0 };
    }

    // A save's :w with a name already there asks once, as :w says E13.
    function writing() {
        const listed = theChooser.commandsIn(notes).map(row => row.command).join(" ");
        expect("saving, the folder's commands are sort, set, mkdir, edit, cd, quit, write and xit",
            listed === "sort set mkdir edit cd quit write xit", listed);
        command(notes, "write").run(called("b.png"));
        const asked = theLauncher.asked[theLauncher.asked.length - 1];
        expect(":w with a name already there asks once, naming it", theLauncher.asked.length === 2
            && asked.question === "Replace b.png? (y/n)" && answers.length === 1,
            `${JSON.stringify(theLauncher.asked)} ${answers.length} answers`);
        asked.no();
        theLauncher.asked = theLauncher.asked.slice(0, 1);
        expect("and :w's name is the line's", theChooser.saveName === "b.png", theChooser.saveName);
        theChooser.saveName = "a.md";
    }

    // :sort lists every folder in an order, :sort! turned round, made
    // afresh where it's shown; :set hidden lists the hidden files, and
    // :set nohidden leaves them out; :cd and :e go to a folder, its path
    // completed from the folders already read.
    function sorting() {
        const sort = command(notes, "sort");
        expect(":sort takes name, changed and size, each saying how", JSON.stringify(sort.args)
            === JSON.stringify([{ value: "name", note: "A to Z" },
                { value: "changed", note: "newest first" }, { value: "size", note: "largest first" }])
            && sort.short === "sor" && sort.bang === true, JSON.stringify(sort));
        const depth = theLauncher.held.length;
        sort.run(called("size"));
        const step = () => theLauncher.held[theLauncher.held.length - 1];
        expect(":sort size lists the folder largest first, made afresh",
            theChooser.order === "size" && !theChooser.reversed && theLauncher.held.length === depth
                && step().row.key === `folder:${notes}`
                && names(step().rows).includes("| sub, notes.txt, b.png, | Encoding"),
            `${theChooser.order} ${theLauncher.held.length}/${depth} ${names(step().rows)}`);
        sort.run(called("", true));
        expect(":sort! turns the order in use round, folders still first",
            theChooser.order === "size" && theChooser.reversed
                && names(step().rows).includes("| sub, b.png, notes.txt, | Encoding"),
            `${theChooser.order} ${theChooser.reversed} ${names(step().rows)}`);
        theChooser.orderBy("name", false);
        command(notes, "set").run(called("hidden"));
        expect(":set hidden lists the hidden files", theChooser.showsHidden
            && names(step().rows).includes(".hidden.txt"), names(step().rows));
        command(notes, "set").run(called("nohidden"));
        expect(":set nohidden leaves them out", !theChooser.showsHidden
            && !names(step().rows).includes(".hidden.txt"), names(step().rows));
        const cd = command(notes, "cd");
        expect(":cd completes folders alone, from the folders read",
            JSON.stringify(cd.args("")) === JSON.stringify([{ value: "sub/", note: "folder" }])
                && JSON.stringify(cd.args("~/no")) === JSON.stringify([{ value: "~/notes/",
                    note: "folder" }]), `${JSON.stringify(cd.args(""))} ${JSON.stringify(cd.args("~/no"))}`);
        const edit = command(notes, "edit");
        expect(":e completes files too", edit.args("n").map(each => each.value).join(" ")
            === "notes.txt", JSON.stringify(edit.args("n")));
        const jumps = theLauncher.jumps;
        cd.run(called("sub"));
        expect(":cd goes to the folder in one jump, each folder on the way held",
            step().row.key === `folder:${notes}/sub` && theLauncher.jumps === jumps + 1
                && theLauncher.held.length === 1 + theChooser.onTheWay(`${notes}/sub`).length,
            `${heldPaths()}, ${theLauncher.jumps - jumps} jumps`);
        edit.run(called("~/notes/"));
        expect(":e on a folder goes there", step().row.key === `folder:${notes}`, heldPaths());
    }

    // The folder a save is asked in gives the line the name it saves as,
    // its caret before the extension, told each change, the line's own row
    // the name, and lists every other row it holds, never narrowed by it.
    function naming(folder) {
        expect("saving, the line holds the name given, a prompt's words", folder.words() === "a.md"
            && typeof folder.worded === "function", folder.words());
        expect("its caret starts before the extension", folder.caretIn("a.md") === 1,
            `${folder.caretIn("a.md")}`);
        folder.worded("new.md");
        const rows = folder.finds("new.md");
        const line = folder.line();
        expect("a name typed is the line's row, saved by Return, and narrows nothing",
            names(rows) === "elsewhere, | sub, a.md, b.png, notes.txt, | Encoding"
                && line.name === "new.md" && line.verb === "save" && line.side === "~/notes",
            `${line.name} ${line.verb} ${line.side}; ${names(rows)}`);
        expect("only the line's row says it names the line's words, so h and l step on the rest",
            line.names === true && rows.every(row => row.names !== true),
            rows.map(row => `${row.name}:${row.names}`).join(" "));
        folder.worded("sub");
        expect("a folder's name gives nothing to save over", folder.line() === null
            && names(folder.finds("sub")) === "elsewhere, | sub, a.md, b.png, notes.txt, | Encoding",
            names(folder.finds("sub")));
        folder.worded("new.md");
    }

    function folder() {
        const rows = rowsIn(notes);
        expect("asked for a folder, it lists no files", names(rows) === "elsewhere, | sub",
            names(rows));
        expect("asked for a folder, the field asks for one", fieldAsks() === "open which folder?",
            fieldAsks());
        const inside = named(rows, "sub").verbs();
        expect("each folder leads with Open This Folder", inside[0].name === "Open This Folder",
            names(inside));
        theLauncher.open = false;
        inside[0].run();
    }

    // Where several may be opened, :arga gathers the cursor row's file or
    // those a range bound, each once, a folder passed over; a file gathered
    // wears the mark, and Return says and answers how many are; :args steps
    // into Gathered, its plate counting them, and Forget Them lets them go.
    // It leaves a.md and old.md gathered, in that order, for Return to
    // answer.
    function gathering() {
        expect("taking several files, the field asks for files", fieldAsks() === "open which files?",
            fieldAsks());
        const rows = rowsIn(notes);
        expect("no lead: lately, then folders and files",
            names(rows) === "old.md, | sub, a.md, b.png, notes.txt", names(rows));
        const listed = theChooser.commandsIn(notes).map(row => row.command).join(" ");
        expect("its commands gather and list what's gathered",
            listed === "sort set mkdir edit cd quit argadd args", listed);
        const a = named(rows, "a.md");
        const plain = a.glyph;
        expect("with none gathered, Return opens the file", a.verb === "open" && a.hue === "program",
            `${a.verb} ${a.hue}`);
        const arga = command(notes, "argadd");
        expect(":arga takes a range", arga.ranged === true && arga.short === "arga",
            JSON.stringify(arga));
        arga.run(Object.assign(called(""), { things: [a, named(rows, "sub"), named(rows, "old.md")] }));
        expect(":arga gathers the files a range bound, in order, a folder passed over",
            JSON.stringify(theChooser.marked) === JSON.stringify([`${notes}/a.md`,
                `${home}/elsewhere/old.md`]), JSON.stringify(theChooser.marked));
        theLauncher.selected = a;
        arga.run(Object.assign(called(""), { things: [theLauncher.held[theLauncher.held.length - 1].row] }));
        expect("the folder says how many are gathered, for the mode word's room",
            theChooser.folderRow(notes).standing() === "2 gathered", theChooser.folderRow(notes).standing());
        expect("with no range, the line standing for the folder, the cursor row's, each once",
            theChooser.marked.length === 2,
            JSON.stringify(theChooser.marked));
        expect("a file gathered wears the mark in an act's hue, and Return says how many it opens",
            a.glyph !== plain && a.hue === "action" && a.verb === "open 2"
                && named(rows, "b.png").verb === "open 2" && named(rows, "b.png").hue === "program",
            `${a.glyph} ${a.hue} ${a.verb}`);
        command(notes, "args").run(called(""));
        const place = theLauncher.held[theLauncher.held.length - 1];
        expect(":args steps into Gathered: the files, each with its folder, then Forget and Copy",
            place.row.title === "Gathered"
                && names(place.rows) === "a.md, old.md, Forget Them, Copy the Paths"
                && place.rows[0].note === "~/notes" && place.rows[1].note === "~/elsewhere"
                && place.rows[3].copy === `${notes}/a.md\n${home}/elsewhere/old.md`,
            `${place.row.title}: ${names(place.rows)}`);
        const plate = place.row.drawer();
        expect("its plate counts them and lists each with its folder, their size waiting on old.md's",
            plate.eyebrow === "2\u00a0gathered" && plate.title === "Gathered"
                && JSON.stringify(plate.preview) === JSON.stringify({
                    lines: ["a.md \u00b7 ~/notes", "old.md \u00b7 ~/elsewhere"], quiet: true })
                && plate.facts.length === 1 && plate.facts[0].value === null,
            JSON.stringify(plate));
        named(place.rows, "Forget Them").run();
        expect("Forget Them lets them go", theChooser.marked.length === 0 && a.verb === "open",
            JSON.stringify(theChooser.marked));
        theLauncher.back();
        const here = theLauncher.held[theLauncher.held.length - 1].row;
        expect("a V selection holding no file is left to the launcher",
            here.takes([{ row: named(rows, "sub"), at: 2 }]) === false && answers.length === 5,
            `${answers.length} answers`);
        arga.run(Object.assign(called(""), { things: [a, named(rows, "old.md")] }));
        theLauncher.open = false;
        named(rows, "b.png").run();
    }

    // The file kept elsewhere, as the case picking it last drew it.
    property var reportRow: null

    // Google Drive's rows, from home.
    function driveFiles() {
        const place = named(rowsIn(home), "Google Drive");
        theLauncher.enter(place);
        const mine = named(place.verbs(), "My Drive");
        theLauncher.enter(mine);
        return mine.verbs();
    }

    function elsewhere() {
        const rows = rowsIn(home);
        expect("a place kept elsewhere is listed after home's files, apart",
            names(rows).endsWith(", | Google Drive"), names(rows));
        const away = theChooser.folderRow(notes).typed("drive");
        expect("typed in another folder, it's found", names(away) === "Google Drive", names(away));
        const there = theChooser.folderRow(home).typed("drive");
        expect("typed at home, where it's listed, it isn't found twice", there.length === 0, names(there));
        const listing = named(named(rows, "Google Drive").verbs(), "Shared with Me");
        expect("a folder still being listed says so", listing.empty === "Listing it.", listing.empty);
        let told = false;
        const heard = () => {
            told = true;
        };
        theChooser.changed.connect(heard);
        drive.asked.changed();
        theChooser.changed.disconnect(heard);
        expect("a place saying its folder has come tells the launcher", told, "nothing said");
        const files = driveFiles();
        expect("a file there is a file, and one of Google's kinds the row it is everywhere",
            names(files) === "report.pdf, Plan" && files[0].component === "file"
                && files[0].key === "drive:report" && files[1].component === "document"
                && files[1].note === "As a .docx copy.",
            `${names(files)} ${files[0].component} ${files[1].component} ${files[1].note}`);
        const titles = theLauncher.held.slice(1).map(step => step.row.title);
        expect("held, every folder says the one held deepest", titles.every(title => title === "My Drive"),
            titles.join(", "));
        reportRow = files[0];
        expect("Return picks it by the program's word, keeping the launcher",
            reportRow.verb === "open" && reportRow.stays === true, `${reportRow.verb} ${reportRow.stays}`);
        reportRow.run();
    }

    function severalElsewhere() {
        const files = driveFiles();
        theChooser.gather([named(files, "Plan"), named(files, "report.pdf")]);
        severalRow = named(files, "report.pdf");
        expect("files kept elsewhere are gathered as any is", theChooser.marked.length === 2
            && severalRow.verb === "open 2" && severalRow.stays === true,
            `${JSON.stringify(theChooser.marked)} ${severalRow.verb} ${severalRow.stays}`);
        severalRow.run();
    }

    // A file kept elsewhere, gathered with another, as the case before drew it.
    property var severalRow: null
    // Whether the folder took a V selection's Return.
    property bool taken: false

    // What the shell's own ask was answered with, none until it is.
    property var shellPicked: null

    // The shell's own ask answered: kept, and the next case looked for, as
    // an answer on a socket is.
    function heardFromTheShell(paths) {
        shellPicked = paths;
        Qt.callLater(root.step);
    }

    function savingElsewhere() {
        const place = named(rowsIn(home), "Google Drive");
        expect("saving, a place kept elsewhere is offered", place !== undefined, names(rowsIn(home)));
        theLauncher.enter(place);
        expect("saving, it holds only the folders a file can be saved in",
            names(place.verbs()) === "My Drive", names(place.verbs()));
        const mine = place.verbs()[0];
        theLauncher.enter(mine);
        const rows = mine.verbs();
        const given = mine.line();
        expect("its line is the name's row, its folders alone listed", names(rows) === "Plans"
            && given.name === "report.pdf", `${given?.name}; ${names(rows)}`);
        const asked = theLauncher.asked.length;
        given.run();
        expect("a name already there asks once", given.verb === "save"
            && theLauncher.asked.length === asked + 1
            && theLauncher.asked[asked].question === "Replace report.pdf? (y/n)",
            `${given.verb} ${JSON.stringify(theLauncher.asked)}`);
        theLauncher.asked[asked].no();
        const plans = named(rows, "Plans");
        theLauncher.enter(plans);
        expect("there too, the line holds the name", plans.words() === "report.pdf"
            && plans.caretIn("report.pdf") === 6, plans.words());
        plans.worded("new.md");
        const typed = plans.line();
        expect("the name typed is the file Return saves there, saying the folder",
            typed.name === "new.md" && typed.verb === "save" && typed.side === "Plans"
                && typed.stays === true && plans.finds("new.md").length === 0,
            `${typed.name} ${typed.side} ${typed.stays}`);
        typed.run();
        expect("the launcher stays while the folder is made", theLauncher.open && answers.length === 8,
            `${theLauncher.open}, ${answers.length} answers`);
    }

    function savingForTheShell() {
        expect("asked for a local folder alone, no place kept elsewhere is offered",
            theChooser.places.length === 0, names(theChooser.places));
        const given = lineIn(notes);
        expect("asked from inside the shell, its line is the name given, saved by Return",
            given.name === "copy.pdf" && given.verb === "save", `${given.name} ${given.verb}`);
        theLauncher.open = false;
        given.run();
    }

    Component.onCompleted: {
        ask(1, {
            mode: "open",
            title: "Upload Notes",
            accept: "_Upload",
            folder: notes,
            filters: [
                { name: "Markdown", globs: [], kinds: ["text/plain"] },
                { name: "All Files", globs: ["*"], kinds: [] }
            ],
            filter: 0,
            choices: []
        });
    }

    Timer {
        interval: 50000
        running: true
        onTriggered: root.fail(root.stage === 26
            ? "the chooser made again never read where a save last went"
            : `stopped at stage ${root.stage}`)
    }
}
