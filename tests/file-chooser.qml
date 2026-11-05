// Drives the launcher's chooser of files (modules/open-with/
// FileChooser.qml) through Quickshell itself, as open-with asks it, over
// folders the check makes, with the launcher stood in and a socket standing
// in for open-with's. Asked to open, it opens in the folder asked for, every
// folder on the way held above it and each saying the one held deepest, so
// the path is one folder; it lists the files opened lately
// elsewhere first, then its folders, then the files the filter passes, by
// name, then the program's filters, each group apart; a file is one row,
// though it was opened lately too, and the hidden ones show once a "." is
// typed; a file in the folder shown says its size and when it changed
// rather than its folder, and its plate its facts before it's previewed,
// it and those selected after it asked of the one file-preview kept for
// the ask (the check's script reads which); a kind the filter names passes
// its narrower kinds too; another filter picked shows what it passes; and Return on a file answers its
// path and the filter in use. Asked to save, the line holds the name the
// program gave, a prompt's words, its caret before the extension, kept
// from folder to folder; each folder leads with it, unnarrowed, and a
// name already there asks once, "Replace a.md? (y/n)", no saving nothing;
// a name typed is saved in the folder it's typed in, asking nothing. Its
// Order, last, steps into Name, Changed and Size, the one in use first,
// and Return on Size lists the folder largest first, made afresh, an
// order the chooser made again after a restart keeps. Asked
// for a folder, it lists no files and each folder leads with Open This
// Folder. In every folder, the field asks for one file, several, a folder
// or where to save, in the program's word. Closing the launcher answers nothing, a request that isn't JSON
// is refused at once, and a withdrawn one closes the launcher. Where several
// may be opened, Open the Marked leads from the start, saying none and
// nothing more, with no Return; Return marks a file, the launcher staying,
// and its row wears the mark in an act's hue, says what Return does now
// and holds Marked, yes; Return again unmarks it; Open the Marked counts
// them, names them in the order marked and lists them on its plate; inside
// it are the files, each with its folder, then Unmark All and Copy the
// Paths, one unmarked there staying until it's left; and Open the Marked
// answers every one marked, in the order they were, wherever each was. A
// folder holds its Copy the Path and filters only once it's read. A place kept elsewhere, given by a
// stand-in for Google's Drive, is listed after home's files, apart, and
// found by typing from another folder, never for a local folder alone;
// held, it says
// its folder's name; a file there is drawn as a file, or as the row it is
// everywhere, and picked once fetched, the launcher staying and saying so
// until then; marked with others, it's answered in the order marked, and
// one that can't be fetched keeps the launcher, saying why, until Return
// again fetches it. Saving there, it holds only the folders a file can
// be saved in, each leading with the name the line holds, a name already
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
        id: launcher

        property bool open: false
        property var held: []
        property string into_: ""
        property int homeDepth: 0
        readonly property var sources: [chooser, drive]

        function into(name) {
            open = true;
            into_ = name;
            held = [{ row: { key: "", from: chooser }, rows: [] }];
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

        // Each question asked once, as the launcher's line asks it.
        property var asked: []

        function askOnce(question, yes, no) {
            asked = asked.concat([{ question: question, yes: yes, no: no }]);
        }
    }

    Launcher.FileChooser {
        id: chooser

        launcher: launcher
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
            launcher: launcher
            settings: chooser.settings
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
        chooser.requested(`${runtime}/open-with/${number}.sock`, typeof request === "string"
            ? request : JSON.stringify(request));
    }

    function names(rows) {
        return rows.map(row => `${row.gap ? "| " : ""}${row.name}${row.side === "chosen" ? " (chosen)" : ""}`)
            .join(", ");
    }

    function rowsIn(path) {
        return chooser.rowsOf(path);
    }

    function named(rows, name) {
        return rows.find(row => row.name === name);
    }

    function heldPaths() {
        return launcher.held.slice(1).map(step => step.row.key).join(" ");
    }

    // What the field asks in the folder held deepest, as the launcher
    // asks it of the step it's on.
    function fieldAsks() {
        return launcher.held[launcher.held.length - 1].row.ask;
    }

    // Whether the folders it needs have been read: notes, and those on the
    // way, and, once it's read, the files opened lately.
    function ready(paths) {
        return paths.every(path => chooser.listed[path] !== undefined)
            && (chooser.settings.recent.length === 0 || chooser.lately.length > 0);
    }

    Connections {
        target: chooser

        function onChanged() {
            Qt.callLater(root.step);
        }

        function onPreviewsChanged() {
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
            if (stage === 0 && ready(["/", notes].concat(chooser.onTheWay(notes)))) {
                stage = 1;
                opening();
            } else if (stage === 1 && previewing === "a.md" && `${notes}/a.md` in chooser.previews) {
                // Two more selected in turn, the later of which the check's
                // script finds was asked of the one file-preview a.md was;
                // the one passed over may never be asked.
                previewing = "old.md";
                const rows = rowsIn(notes);
                named(rows, "notes.txt").drawer();
                named(rows, "old.md").drawer();
            } else if (stage === 1 && previewing === "old.md"
                && `${home}/elsewhere/old.md` in chooser.previews) {
                stage = 2;
                launcher.open = false;
                picking.run();
            } else if (stage === 2 && answers.length === 1) {
                stage = 3;
                expect("Return on a file answers its path and the filter in use",
                    answers[0] === JSON.stringify({ paths: [`${notes}/a.md`], filter: 1, choices: {} }),
                    answers[0]);
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
            } else if (stage === 3 && chooser.asking?.reply.endsWith("/2.sock") && ready([notes])) {
                stage = 4;
                save();
            } else if (stage === 4 && answers.length === 2) {
                stage = 5;
                expect("what's typed saves in the folder it's typed in, with each choice's option",
                    answers[1] === JSON.stringify({ paths: [`${notes}/sub/new.md`], filter: null,
                        choices: { encoding: "latin1" } }), answers[1]);
                expect("a save keeps the folder it went to", chooser.savedLastIn() === `${notes}/sub`,
                    chooser.savedLastIn());
                ask(3, { mode: "open", title: "Pick a Folder", folders: true, folder: notes });
            } else if (stage === 5 && chooser.asking?.reply.endsWith("/3.sock") && ready([notes])) {
                stage = 6;
                folder();
            } else if (stage === 6 && answers.length === 3) {
                stage = 7;
                expect("Open This Folder answers the folder", answers[2]
                    === JSON.stringify({ paths: [`${notes}/sub`], filter: null, choices: {} }), answers[2]);
                ask(4, { mode: "open", title: "Open", folder: notes });
            } else if (stage === 7 && chooser.shown.endsWith("/4.sock")) {
                stage = 8;
                launcher.open = false;
            } else if (stage === 8 && answers.length === 4) {
                stage = 9;
                expect("closing the launcher answers nothing", answers[3] === "", answers[3]);
                ask(5, "not a request");
            } else if (stage === 9 && answers.length === 5) {
                stage = 10;
                expect("a request that isn't JSON is refused at once", answers[4] === ""
                    && chooser.asks.length === 0, `${answers[4]}, ${chooser.asks.length} waiting`);
                ask(6, { mode: "open", title: "Open", folder: notes });
            } else if (stage === 10 && chooser.shown.endsWith("/6.sock")) {
                stage = 11;
                chooser.withdrawn(`${runtime}/open-with/6.sock`);
                expect("a withdrawn request closes the launcher", !launcher.open
                    && chooser.asks.length === 0, `${launcher.open}, ${chooser.asks.length} waiting`);
                ask(7, { mode: "open", title: "Open Several", multiple: true, folder: notes });
            } else if (stage === 11 && chooser.asking?.reply.endsWith("/7.sock") && ready([notes])) {
                stage = 12;
                marking();
            } else if (stage === 12 && answers.length === 6) {
                stage = 13;
                expect("Open the Marked answers every one marked, in the order they were",
                    answers[5] === JSON.stringify({ paths: [`${notes}/a.md`, `${home}/elsewhere/old.md`],
                        filter: null, choices: {} }), answers[5]);
                chooser.chooseToSave({ name: "copy.pdf", folder: notes, title: "Save a Copy", local: true },
                    root.heardFromTheShell);
            } else if (stage === 13 && chooser.shown.startsWith("shell:") && ready([notes])) {
                stage = 14;
                savingForTheShell();
            } else if (stage === 14 && shellPicked !== null) {
                stage = 15;
                expect("an ask from inside the shell is answered with the path, never on a socket",
                    JSON.stringify(shellPicked) === JSON.stringify([`${notes}/copy.pdf`])
                        && answers.length === 6, `${JSON.stringify(shellPicked)}, ${answers.length} answers`);
                expect("a save asked from inside the shell keeps its folder too",
                    chooser.savedLastIn() === notes, chooser.savedLastIn());
                shellPicked = null;
                chooser.chooseToSave({ name: "copy.pdf", folder: notes }, root.heardFromTheShell);
            } else if (stage === 15 && chooser.shown.startsWith("shell:")) {
                stage = 16;
                launcher.open = false;
            } else if (stage === 16 && shellPicked !== null) {
                stage = 17;
                expect("closing the launcher on an ask from inside the shell answers none",
                    shellPicked.length === 0 && chooser.asks.length === 0,
                    `${JSON.stringify(shellPicked)}, ${chooser.asks.length} waiting`);
                ask(8, { mode: "open", title: "Open", folder: home });
            } else if (stage === 17 && chooser.asking?.reply.endsWith("/8.sock") && ready([home])) {
                stage = 18;
                elsewhere();
            } else if (stage === 18 && drive.fetches.length === 1) {
                stage = 19;
                expect("a file kept elsewhere keeps the launcher while it's fetched, saying so",
                    launcher.open && answers.length === 6 && reportRow.note === "Fetching it.",
                    `${launcher.open}, ${answers.length} answers, ${reportRow.note}`);
                drive.fetches[0].done(`${runtime}/picked/report.pdf`);
            } else if (stage === 19 && answers.length === 7) {
                stage = 20;
                expect("once it's fetched, its path is answered and the launcher closes",
                    answers[6] === JSON.stringify({ paths: [`${runtime}/picked/report.pdf`], filter: null,
                        choices: {} }) && !launcher.open, `${answers[6]}, ${launcher.open}`);
                expect("what's opened never moves where a save went", chooser.savedLastIn() === notes,
                    chooser.savedLastIn());
                ask(9, { mode: "open", title: "Open Several", multiple: true, folder: home });
            } else if (stage === 20 && chooser.asking?.reply.endsWith("/9.sock") && ready([home])) {
                stage = 21;
                severalElsewhere();
            } else if (stage === 21 && drive.fetches.length === 3) {
                stage = 22;
                drive.fetches[1].done(`${runtime}/picked/plan.docx`);
                drive.fetches[2].failed("Google Drive couldn't be reached.");
                const lead = rowsIn(home)[0];
                expect("one that can't be fetched keeps the launcher, saying why",
                    launcher.open && answers.length === 7 && lead.note === "Google Drive couldn't be reached.",
                    `${launcher.open}, ${answers.length} answers, ${lead.note}`);
                lead.run();
            } else if (stage === 22 && drive.fetches.length === 5) {
                stage = 23;
                drive.fetches[3].done(`${runtime}/picked/plan.docx`);
                drive.fetches[4].done(`${runtime}/picked/report.pdf`);
            } else if (stage === 23 && answers.length === 8) {
                stage = 24;
                expect("marked with others, each is answered once fetched, in the order marked",
                    answers[7] === JSON.stringify({ paths: [`${runtime}/picked/plan.docx`,
                        `${runtime}/picked/report.pdf`], filter: null, choices: {} }) && !launcher.open,
                    `${answers[7]}, ${launcher.open}`);
                ask(10, { mode: "save", title: "Save", name: "report.pdf", folder: home });
            } else if (stage === 24 && chooser.asking?.reply.endsWith("/10.sock") && ready([home])) {
                stage = 25;
                savingElsewhere();
            } else if (stage === 25 && answers.length === 9) {
                stage = 26;
                expect("Return makes the folder the place stages it in, then answers the path there",
                    answers[8] === JSON.stringify({ paths: [`${home}/staged/plans/new.md`], filter: null,
                        choices: {} }) && !launcher.open, `${answers[8]}, ${launcher.open}`);
                expect("a save staged for a place kept elsewhere never moves where a save here went",
                    chooser.savedLastIn() === notes, chooser.savedLastIn());
                chooser.orderBy("changed");
                restarted = restarting.createObject(root);
            } else if (stage === 26 && restarted?.savedLastIn() === notes
                && restarted.order === "changed") {
                stage = 27;
                console.log("every case holds");
                Qt.callLater(() => Qt.exit(0));
            }
        } catch (error) {
            fail(error.message);
        }
    }

    function opening() {
        expect("it opens in its collection", launcher.into_ === "fileChooser", launcher.into_);
        const way = chooser.onTheWay(notes).map(path => `folder:${path}`).join(" ");
        expect("every folder on the way is held above the one asked for", heldPaths() === way,
            `${heldPaths()}, not ${way}`);
        expect("the folder asked for is where it opened, so Escape there refuses the ask",
            launcher.homeDepth === launcher.held.length, `${launcher.homeDepth}`);
        const titles = launcher.held.slice(1).map(step => step.row.title);
        expect("the path says one folder, the one held deepest",
            titles.every(title => title === "~/notes"), titles.join(" "));
        let rows = rowsIn(notes);
        expect("lately elsewhere, then folders, then files the filter passes, then the filters",
            names(rows) === "old.md, | sub, a.md, notes.txt, | Markdown (chosen), All Files, Order",
            names(rows));
        const unread = names(chooser.folderRow(`${home}/unread`).verbs());
        const read = names(chooser.folderRow(notes).verbs());
        expect("a folder holds its Copy the Path and filters only once it's read, so they come last",
            unread === "" && read.endsWith("All Files, Order, Copy the Path"), `${unread}; ${read}`);
        expect("a file opened lately elsewhere says where", rows[0].side === "~/elsewhere"
            && rows[0].key === `file:${home}/elsewhere/old.md`, `${rows[0].side} ${rows[0].key}`);
        const own = named(rows, "a.md");
        expect("a file in the folder shown says its size and when it changed, not its folder",
            own.side === "0\u00a0B \u00b7 just now", own.side);
        expect("a folder in it says when it changed, its count once it's known",
            named(rows, "sub").side === "just now", named(rows, "sub").side);
        const plate = own.drawer();
        const facts = plate.facts.map(fact => `${fact.label} ${fact.value}`).join(", ");
        expect("a file's plate says its name and its facts at once, before it's previewed",
            plate.title === "a.md" && plate.preview === undefined
            && facts === "Size 0\u00a0B, Changed just now, Where ~/notes",
            `${plate.title}: ${facts}`);
        expect("each file is drawn as every file is", rows.every(row => row.component
            || row.key.startsWith("file-chooser:")), rows.map(row => row.key).join(" "));
        const typed = launcher.held[launcher.held.length - 1].row.typed(".");
        expect("the hidden ones show once a dot is typed", names(typed) === ".secret, .hidden.txt",
            names(typed));
        named(rows, "All Files").run();
        rows = rowsIn(notes);
        expect("another filter picked shows what it passes",
            names(rows)
                === "old.md, | sub, a.md, b.png, notes.txt, | Markdown, All Files (chosen), Order",
            names(rows));
        const a = named(rows, "a.md");
        expect("Return on a file opens it by the program's word", a.verb === "upload"
            && a.verbs()[0].name === "Upload", `${a.verb} ${a.verbs()[0].name}`);
        expect("taking one file, the field asks for one, by the program's word, in every folder",
            fieldAsks() === "upload which file?"
                && chooser.folderRow(`${notes}/sub`).ask === fieldAsks(), fieldAsks());
        besides();
        // Picked once a.md's plate, drawn above, is previewed.
        picking = a;
        previewing = "a.md";
    }

    // ]b and [b go to the folder beside this one, as the folder holding it
    // lists its folders, round from the last to the first, held where this
    // one was; in a folder whose holder isn't read yet, once it's heard.
    function besides() {
        const depth = launcher.held.length;
        const deepest = () => launcher.held[launcher.held.length - 1].row.key;
        const around = rowsIn(home).map(row => row.key)
            .filter(key => key.startsWith(`folder:${home}/`));
        const next = around[(around.indexOf(`folder:${notes}`) + 1) % around.length];
        launcher.held[depth - 1].row.beside(1);
        expect("]b goes to the next folder its holder lists, held where this one was",
            around.length > 1 && deepest() === next && launcher.held.length === depth,
            `${deepest()} at ${launcher.held.length}, not ${next} at ${depth} of ${around}`);
        launcher.held[depth - 1].row.beside(-1);
        expect("[b goes to the one before", deepest() === `folder:${notes}`, deepest());
        for (let pressed = 0; pressed < around.length; pressed += 1)
            launcher.held[depth - 1].row.beside(1);
        expect("]b goes round from the last to the first", deepest() === `folder:${notes}`,
            deepest());
        const deep = `${home}/elsewhere/deep`;
        launcher.enter(chooser.folderRow(`${deep}/one`));
        launcher.held[depth].row.beside(1);
        expect("]b in a folder whose holder isn't read waits for it",
            deepest() === `folder:${deep}/one`, deepest());
        chooser.heard(deep, [{ name: "one", path: `${deep}/one`, folder: true },
            { name: "two", path: `${deep}/two`, folder: true }]);
        expect("and goes once it's heard", deepest() === `folder:${deep}/two`
            && launcher.held.length === depth + 1, deepest());
        launcher.back();
    }

    function save() {
        expect("saving, the field asks where, by the name given", fieldAsks() === "save a.md where?",
            fieldAsks());
        const rows = rowsIn(notes);
        expect("saving leads with the name given, then the folders lately, then the rest",
            names(rows) === "a.md, elsewhere, | sub, b.png, notes.txt, | Encoding, Order", names(rows));
        const there = rows[0];
        there.run();
        const asked = launcher.asked[0];
        expect("a name already there asks once before Return saves over it",
            there.verb === "save" && launcher.asked.length === 1
                && asked.question === "Replace a.md? (y/n)" && answers.length === 1,
            `${there.verb} ${JSON.stringify(launcher.asked)} ${answers.length} answers`);
        asked.no();
        expect("and no saves nothing", answers.length === 1 && chooser.asking !== null,
            `${answers.length} answers`);
        named(rows, "Encoding").verbs()[1].run();
        ordering();
        naming(launcher.held[launcher.held.length - 1].row);
        const sub = named(rows, "sub");
        launcher.enter(sub);
        expect("the name is kept from folder to folder", sub.words() === "new.md"
            && rowsIn(`${notes}/sub`)[0].name === "new.md", `${sub.words()}`);
        const typed = rowsIn(`${notes}/sub`)[0];
        expect("the name typed saves in the folder it's typed in", typed.verb === "save"
            && typed.side === "~/notes/sub", `${typed.verb} ${typed.side}`);
        launcher.back();
        launcher.open = false;
        typed.run();
        expect("a name not there saves at once, asking nothing", launcher.asked.length === 1,
            JSON.stringify(launcher.asked));
    }

    // The folder a save is asked in gives the line the name it saves as,
    // its caret before the extension, told each change, and lists every row
    // it holds, led by the name, never narrowed by it.
    function naming(folder) {
        expect("saving, the line holds the name given, a prompt's words", folder.words() === "a.md"
            && typeof folder.worded === "function", folder.words());
        expect("its caret starts before the extension", folder.caretIn("a.md") === 1,
            `${folder.caretIn("a.md")}`);
        folder.worded("new.md");
        const rows = folder.finds("new.md");
        expect("a name typed leads, saved by Return, and narrows nothing",
            names(rows) === "new.md, elsewhere, | sub, a.md, b.png, notes.txt, | Encoding, Order"
                && rows[0].verb === "save" && rows[0].side === "~/notes", names(rows));
        expect("only the name's row says it names the line's words, so h and l step on the rest",
            rows[0].names === true && rows.slice(1).every(row => row.names !== true),
            rows.map(row => `${row.name}:${row.names}`).join(" "));
        folder.worded("sub");
        expect("a folder's name gives nothing to save over", names(folder.finds("sub"))
            === "elsewhere, | sub, a.md, b.png, notes.txt, | Encoding, Order", names(folder.finds("sub")));
        folder.worded("new.md");
    }

    // Order comes last, saying the order in use, and steps into the three,
    // that one first; Return on one lists the folder so, back in it, made
    // afresh, as every folder after.
    function ordering() {
        const rows = rowsIn(notes);
        const order = rows[rows.length - 1];
        expect("Order comes last, saying the order in use, stepping in", order.name === "Order"
            && order.side === "name" && order.verb === "more", `${order.name} ${order.side}`);
        const depth = launcher.held.length;
        launcher.enter(order);
        const inside = launcher.held[depth].rows;
        expect("inside, the order in use first, then the others",
            names(inside) === "Name (chosen), Changed, Size", names(inside));
        inside[2].run();
        const step = launcher.held[launcher.held.length - 1];
        expect("Return on Size goes back to the folder, made afresh, its files largest first",
            chooser.order === "size" && launcher.held.length === depth
                && step.row.key === `folder:${notes}`
                && names(step.rows).includes("| sub, notes.txt, b.png, | Encoding, Order")
                && rowsIn(notes)[rowsIn(notes).length - 1].side === "size, largest first",
            `${chooser.order} ${launcher.held.length}/${depth} ${step.row.key} ${names(step.rows)}`);
        chooser.orderBy("name");
    }

    function folder() {
        const rows = rowsIn(notes);
        expect("asked for a folder, it lists no files", names(rows) === "elsewhere, | sub, | Order",
            names(rows));
        expect("asked for a folder, the field asks for one", fieldAsks() === "open which folder?",
            fieldAsks());
        const inside = named(rows, "sub").verbs();
        expect("each folder leads with Open This Folder", inside[0].name === "Open This Folder",
            names(inside));
        launcher.open = false;
        inside[0].run();
    }

    function marking() {
        expect("taking several files, the field asks for files", fieldAsks() === "open which files?",
            fieldAsks());
        let rows = rowsIn(notes);
        expect("Open the Marked leads from the start, then lately, folders and files",
            names(rows) === "Open the Marked, | old.md, | sub, a.md, b.png, notes.txt, | Order",
            names(rows));
        const lead = rows[0];
        expect("with none marked, it says none and nothing more, Return saying nothing",
            lead.side === "none" && lead.note === "" && lead.still === true && lead.stays === true
                && !lead.verbs && !lead.drawer && !lead.plated,
            `${lead.side} ${lead.note} ${lead.still} ${lead.stays} ${typeof lead.verbs}`);
        lead.run();
        expect("with none marked, Open the Marked opens nothing", answers.length === 5
            && chooser.asking?.reply.endsWith("/7.sock"), `${answers.length} answers`);
        const a = named(rows, "a.md");
        const unmarked = a.glyph;
        expect("Return marks a file, the launcher staying", a.verb === "mark" && a.stays === true
            && a.hue === "program", `${a.verb} ${a.stays} ${a.hue}`);
        a.run();
        expect("a file marked wears the mark in an act's hue and says Return unmarks it",
            a.glyph !== unmarked && a.hue === "action" && a.verb === "unmark",
            `${a.glyph} ${a.hue} ${a.verb}`);
        expect("inside, Marked says yes, then Open for it alone, then Copy the Path",
            names(a.verbs()) === "Marked, Open, Copy the Path" && a.verbs()[0].side === "yes",
            `${names(a.verbs())} ${a.verbs()[0].side}`);
        named(rows, "old.md").run();
        const b = named(rows, "b.png");
        b.run();
        b.run();
        expect("Return again unmarks it", b.verb === "mark" && b.glyph === unmarked
            && b.verbs()[0].side === "no", `${b.verb} ${b.verbs()[0].side}`);
        rows = rowsIn(notes);
        expect("Open the Marked says how many are and names them in the order marked",
            rows[0].name === "Open the Marked" && rows[0].side === "2"
                && rows[0].note === "a.md \u00b7 old.md" && rows[0].still === false,
            `${rows[0].name} ${rows[0].side} ${rows[0].note} ${rows[0].still}`);
        // The launcher draws a plate only for a thing or a row that says
        // it's plated, and the lead is neither by its kind.
        expect("Open the Marked says it's plated, so the launcher draws its plate",
            rows[0].plated === true, `${rows[0].plated}`);
        expect("stepped into, its crumb is the place, Marked", rows[0].title === "Marked",
            rows[0].title);
        const plate = rows[0].drawer();
        expect("its plate counts them and lists each with its folder, their size waiting on old.md's",
            plate.eyebrow === "2\u00a0marked" && plate.title === "Open the Marked"
                && JSON.stringify(plate.preview) === JSON.stringify({
                    lines: ["a.md \u00b7 ~/notes", "old.md \u00b7 ~/elsewhere"], quiet: true })
                && plate.facts.length === 1 && plate.facts[0].value === null,
            JSON.stringify(plate));
        insideMarked(rows[0]);
        launcher.open = false;
        rowsIn(notes)[0].run();
    }

    // The right arrow on Open the Marked steps into Marked: the files in
    // the order marked, each with its folder, then Unmark All, then Copy
    // the Paths; one unmarked there stays, unmarked, until it's left, and
    // so do all of them once Unmark All has been. It leaves a.md and old.md
    // marked again, in that order, for Open the Marked to answer.
    function insideMarked(lead) {
        launcher.enter(lead);
        const shown = () => rowsIn(notes)[0].verbs();
        let inside = shown();
        expect("inside Marked: the files marked, each with its folder, then Unmark All and Copy",
            names(inside) === "a.md, old.md, Unmark All, Copy the Paths"
                && inside[0].note === "~/notes" && inside[1].note === "~/elsewhere"
                && inside[3].copy === `${notes}/a.md\n${home}/elsewhere/old.md`,
            `${names(inside)} ${inside[0].note} ${inside[1].note} ${inside[3].copy}`);
        inside[0].run();
        inside = shown();
        expect("one unmarked there stays, unmarked", names(inside)
            === "a.md, old.md, Unmark All, Copy the Paths" && inside[0].verb === "mark"
            && rowsIn(notes)[0].side === "1", `${names(inside)} ${inside[0].verb}`);
        named(inside, "Unmark All").run();
        inside = shown();
        expect("with all unmarked, Marked still holds them while it's shown, with nothing to do",
            names(inside) === "a.md, old.md, Unmark All, Copy the Paths"
                && inside[2].still === true && inside[3].still === true
                && rowsIn(notes)[0].still === true, `${names(inside)} ${inside[2].still}`);
        inside[0].run();
        inside[1].run();
        launcher.held = launcher.held.slice(0, -1);
        expect("left, Marked is stepped into afresh, holding only what's marked",
            names(rowsIn(notes)[0].verbs()) === "a.md, old.md, Unmark All, Copy the Paths",
            names(rowsIn(notes)[0].verbs()));
        launcher.enter(rowsIn(notes)[0]);
        named(launcher.held[launcher.held.length - 1].rows, "old.md").run();
        launcher.held = launcher.held.slice(0, -1);
        expect("and one unmarked there is gone from it once it's left",
            names(rowsIn(notes)[0].verbs()) === "a.md, Unmark All, Copy the Paths",
            names(rowsIn(notes)[0].verbs()));
        named(rowsIn(notes), "old.md").run();
    }

    // The file kept elsewhere, as the case picking it last drew it.
    property var reportRow: null

    // Google Drive's rows, from home.
    function driveFiles() {
        const place = named(rowsIn(home), "Google Drive");
        launcher.enter(place);
        const mine = named(place.verbs(), "My Drive");
        launcher.enter(mine);
        return mine.verbs();
    }

    function elsewhere() {
        const rows = rowsIn(home);
        expect("a place kept elsewhere is listed after home's files, apart, then Order",
            names(rows).endsWith(", | Google Drive, | Order"), names(rows));
        const away = chooser.folderRow(notes).typed("drive");
        expect("typed in another folder, it's found", names(away) === "Google Drive", names(away));
        const there = chooser.folderRow(home).typed("drive");
        expect("typed at home, where it's listed, it isn't found twice", there.length === 0, names(there));
        const listing = named(named(rows, "Google Drive").verbs(), "Shared with Me");
        expect("a folder still being listed says so", listing.empty === "Listing it.", listing.empty);
        let told = false;
        const heard = () => {
            told = true;
        };
        chooser.changed.connect(heard);
        drive.asked.changed();
        chooser.changed.disconnect(heard);
        expect("a place saying its folder has come tells the launcher", told, "nothing said");
        const files = driveFiles();
        expect("a file there is a file, and one of Google's kinds the row it is everywhere",
            names(files) === "report.pdf, Plan" && files[0].component === "file"
                && files[0].key === "drive:report" && files[1].component === "document"
                && files[1].note === "As a .docx copy.",
            `${names(files)} ${files[0].component} ${files[1].component} ${files[1].note}`);
        const titles = launcher.held.slice(1).map(step => step.row.title);
        expect("held, every folder says the one held deepest", titles.every(title => title === "My Drive"),
            titles.join(", "));
        reportRow = files[0];
        expect("Return picks it by the program's word, keeping the launcher",
            reportRow.verb === "open" && reportRow.stays === true, `${reportRow.verb} ${reportRow.stays}`);
        reportRow.run();
    }

    function severalElsewhere() {
        const files = driveFiles();
        named(files, "Plan").run();
        named(files, "report.pdf").run();
        const lead = rowsIn(home)[0];
        expect("files kept elsewhere are marked as any is", lead.side === "2" && lead.stays === true,
            `${lead.side} ${lead.stays}`);
        lead.run();
    }

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
        launcher.enter(place);
        expect("saving, it holds only the folders a file can be saved in",
            names(place.verbs()) === "My Drive", names(place.verbs()));
        const mine = place.verbs()[0];
        launcher.enter(mine);
        const rows = mine.verbs();
        expect("it leads with the name, apart from its folders", names(rows) === "report.pdf, | Plans",
            names(rows));
        const asked = launcher.asked.length;
        rows[0].run();
        expect("a name already there asks once", rows[0].verb === "save"
            && launcher.asked.length === asked + 1
            && launcher.asked[asked].question === "Replace report.pdf? (y/n)",
            `${rows[0].verb} ${JSON.stringify(launcher.asked)}`);
        launcher.asked[asked].no();
        const plans = named(rows, "Plans");
        launcher.enter(plans);
        expect("there too, the line holds the name", plans.words() === "report.pdf"
            && plans.caretIn("report.pdf") === 6, plans.words());
        plans.worded("new.md");
        const typed = plans.finds("new.md");
        expect("the name typed is the file Return saves there, saying the folder",
            names(typed) === "new.md" && typed[0].verb === "save" && typed[0].side === "Plans"
                && typed[0].stays === true, `${names(typed)} ${typed[0]?.side} ${typed[0]?.stays}`);
        typed[0].run();
        expect("the launcher stays while the folder is made", launcher.open && answers.length === 8,
            `${launcher.open}, ${answers.length} answers`);
    }

    function savingForTheShell() {
        expect("asked for a local folder alone, no place kept elsewhere is offered",
            chooser.places.length === 0, names(chooser.places));
        const given = rowsIn(notes)[0];
        expect("asked from inside the shell, it leads with the name given, saved by Return",
            given.name === "copy.pdf" && given.verb === "save", `${given.name} ${given.verb}`);
        launcher.open = false;
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
