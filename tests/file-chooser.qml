// Drives the launcher's chooser of files (modules/open-with/
// FileChooser.qml) through Quickshell itself, as open-with asks it, over
// folders the check makes, with the launcher stood in and a socket standing
// in for open-with's. Asked to open, it opens in the folder asked for, every
// folder on the way held above it and each saying the one held deepest, so
// the path is one folder; it lists the files opened lately
// elsewhere first, then its folders, then the files the filter passes, by
// name, then the program's filters, each group apart; a file is one row,
// though it was opened lately too, and the hidden ones show once a "." is
// typed; a kind the filter names passes its narrower kinds too; another
// filter picked shows what it passes; and Return on a file answers its
// path and the filter in use. Asked to save, it leads with the name the
// program gave, which, being there already, steps in to Replace It first;
// what's typed names it instead, saved in the folder it's typed in. Asked
// for a folder, it lists no files and each folder leads with Open This
// Folder. Closing the launcher answers nothing, a request that isn't JSON
// is refused at once, and a withdrawn one closes the launcher. Where several
// may be opened, Open the Marked leads from the start, saying none and
// opening nothing; Return marks a file, the launcher staying, and its row
// wears the mark, says what Return does now and holds Marked, yes; Return
// again unmarks it; and Open the Marked answers every one marked, in the
// order they were, wherever each was. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
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

        function into(name) {
            open = true;
            into_ = name;
            held = [{ row: { key: "", from: chooser }, rows: [] }];
        }

        function enter(row) {
            held = held.concat([{ row: row, rows: row.verbs() }]);
            return true;
        }
    }

    Launcher.FileChooser {
        id: chooser

        launcher: launcher
        settings: ({
            name: "fileChooser",
            recent: [Quickshell.env("RECENT_LIST")],
            kinds: Quickshell.env("KINDS")
        })
    }

    // A socket of open-with's, by its number: each answer it's given.
    Variants {
        model: [1, 2, 3, 4, 5, 6, 7]

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
    }

    // Each case, once what it waits on is there.
    function step() {
        try {
            if (stage === 0 && ready(["/", notes].concat(chooser.onTheWay(notes)))) {
                stage = 1;
                opening();
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
                chooser.chooseToSave({ name: "copy.pdf", folder: notes, title: "Save a Copy" },
                    root.heardFromTheShell);
            } else if (stage === 13 && chooser.shown.startsWith("shell:") && ready([notes])) {
                stage = 14;
                savingForTheShell();
            } else if (stage === 14 && shellPicked !== null) {
                stage = 15;
                expect("an ask from inside the shell is answered with the path, never on a socket",
                    JSON.stringify(shellPicked) === JSON.stringify([`${notes}/copy.pdf`])
                        && answers.length === 6, `${JSON.stringify(shellPicked)}, ${answers.length} answers`);
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
        const titles = launcher.held.slice(1).map(step => step.row.title);
        expect("the path says one folder, the one held deepest",
            titles.every(title => title === "~/notes"), titles.join(" "));
        let rows = rowsIn(notes);
        expect("lately elsewhere, then folders, then files the filter passes, then the filters",
            names(rows) === "old.md, | sub, a.md, notes.txt, | Markdown (chosen), All Files", names(rows));
        expect("a file opened lately elsewhere says where", rows[0].side === "~/elsewhere"
            && rows[0].key === `file:${home}/elsewhere/old.md`, `${rows[0].side} ${rows[0].key}`);
        expect("each file is drawn as every file is", rows.every(row => row.component
            || row.key.startsWith("file-chooser:")), rows.map(row => row.key).join(" "));
        const typed = launcher.held[launcher.held.length - 1].row.typed(".");
        expect("the hidden ones show once a dot is typed", names(typed) === ".secret, .hidden.txt",
            names(typed));
        named(rows, "All Files").run();
        rows = rowsIn(notes);
        expect("another filter picked shows what it passes",
            names(rows) === "old.md, | sub, a.md, b.png, notes.txt, | Markdown, All Files (chosen)",
            names(rows));
        const a = named(rows, "a.md");
        expect("Return on a file opens it by the program's word", a.verb === "upload"
            && a.verbs()[0].name === "Upload", `${a.verb} ${a.verbs()[0].name}`);
        stage = 2;
        launcher.open = false;
        a.run();
    }

    function save() {
        const rows = rowsIn(notes);
        expect("saving leads with the name given, then the folders lately, then the rest",
            names(rows) === "a.md, elsewhere, | sub, b.png, notes.txt, | Encoding", names(rows));
        const there = rows[0];
        expect("a name already there steps in to Replace It first", there.verb === "more"
            && names(there.verbs()) === "Replace It, Copy the Path", `${there.verb}: ${names(there.verbs())}`);
        named(rows, "Encoding").verbs()[1].run();
        const sub = named(rows, "sub");
        const typed = sub.typed("new.md");
        expect("what's typed names the file Return saves", names(typed) === "new.md"
            && typed[0].verb === "save" && typed[0].side === "~/notes/sub", `${names(typed)} ${typed[0]?.side}`);
        launcher.open = false;
        typed[0].run();
    }

    function folder() {
        const rows = rowsIn(notes);
        expect("asked for a folder, it lists no files", names(rows) === "elsewhere, | sub", names(rows));
        const inside = named(rows, "sub").verbs();
        expect("each folder leads with Open This Folder", inside[0].name === "Open This Folder",
            names(inside));
        launcher.open = false;
        inside[0].run();
    }

    function marking() {
        let rows = rowsIn(notes);
        expect("Open the Marked leads from the start, then lately, folders and files",
            names(rows) === "Open the Marked, | old.md, | sub, a.md, b.png, notes.txt", names(rows));
        const lead = rows[0];
        expect("with none marked, it says none and why, and Return keeps the launcher",
            lead.side === "none" && lead.note !== "" && lead.stays === true,
            `${lead.side} ${lead.note} ${lead.stays}`);
        lead.run();
        expect("with none marked, Open the Marked opens nothing", answers.length === 5
            && chooser.asking?.reply.endsWith("/7.sock"), `${answers.length} answers`);
        const a = named(rows, "a.md");
        const unmarked = a.glyph;
        expect("Return marks a file, the launcher staying", a.verb === "mark" && a.stays === true,
            `${a.verb} ${a.stays}`);
        a.run();
        expect("a file marked wears the mark and says Return unmarks it", a.glyph !== unmarked
            && a.verb === "unmark", `${a.glyph} ${a.verb}`);
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
        expect("Open the Marked says how many are", rows[0].name === "Open the Marked"
            && rows[0].side === "2" && rows[0].note === "", `${rows[0].name} ${rows[0].side}`);
        launcher.open = false;
        rows[0].run();
    }

    // What the shell's own ask was answered with, none until it is.
    property var shellPicked: null

    // The shell's own ask answered: kept, and the next case looked for, as
    // an answer on a socket is.
    function heardFromTheShell(paths) {
        shellPicked = paths;
        Qt.callLater(root.step);
    }

    function savingForTheShell() {
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
        onTriggered: root.fail(`stopped at stage ${root.stage}`)
    }
}
