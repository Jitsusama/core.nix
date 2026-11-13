// Drives a program's step in the launcher (modules/quickshell/launcher/
// Applications.qml) through Quickshell itself, over desktop entries written
// beside this, with niri, the desktop's shortcuts, the notices' history and
// what's playing (launcher/Playing.qml) stood in. A running program with a
// player, a stream and notices holds, in order, Go To the window used
// last, New window, New window below, its desktop entry's own actions (one
// shortened, one left to New window), its other window by title, Recent,
// saying how many and the first one's name, Notices, Quit, then Media and Sound, each doing what it says: Quit asks once
// and closes each window, the media and sound acts reach the player and
// the program's streams only, and its figures follow what they did. A
// program with nothing of the sort holds only New window and New window
// below, and one whose player is found by its identity holds Media with
// only what that player can do, and Sound with only its Microphone while
// it records and plays nothing, Mute there reaching what it records alone.
// A program with keys of its own holds them as its Shortcuts. One that
// keeps what it had open lately holds Recent, each path by its name beside
// its folder, opened by its start command while none of its windows is
// open and through the process behind its last window while one is, and
// read again as its file changes, nine at most, a program held then drawn
// again; a change while the launcher is closed is read only as it opens.
// A program's files on the
// list programs share are its Recent too, found by its desktop entry's id
// or name, and opened by its own command when it takes files, else as the
// desktop opens them. A program whose sites are read only as it's stepped
// into reads them then, once each time the launcher opens, each site by
// its title beside its host, opened by its own command, and Copy the Link
// last, its Recent standing and waiting until they're first read. What it
// has open comes before its other windows and Recent, in the step itself,
// and a window titled by a page it has open in front isn't listed again:
// every page open, one row a page newest
// looked at first, saying first that it's open and in how many tabs, then
// its host, so a side cut short keeps the state, Return going to
// it through its source's command and its window, Close It closing each of
// its tabs and leaving the page where it was, dimmed, until the launcher
// opens again; a page open is never listed again as closed. A tab the
// source leaves out has no row but still finds the window it's in front
// of, and a source that only says what's open, as Chrome's session file
// does, gives Go To bringing the window forward and no Close It. A file
// source saying what's open, as mpv's does, comes first the same way, one row a
// file only while its process has a window, Return going to that window,
// Close It closing it and leaving the file dimmed, the rest of its Recent
// still the list programs share; a file open in two windows says so, and
// one in a window named by niri's id, as Neovim's in kitty's, goes there
// through its source's command too, with no Close It. A program whose
// entry hides itself is listed only when another provider gives it acts,
// and then steps in on Return rather than starting. An act found from the
// top that asks for a choice goes to its list inside its program. The list
// programs share, heard again as it was, changes nothing, and an act
// another provider gives to be found from the top makes its own row and
// no program's own again, and none again as the same list is given anew
// or what its acts read changes. A step asked again, as a held one is at each
// change, makes none of its Recent's rows again. Typed inside Programs,
// the rows holding its letters stand at once and the programs daemon's
// answer takes its own order; where the daemon can't search, typing on
// looks only through what the last key found, and finds what looking
// through everything would. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher
import "HeardWindows.js" as HeardWindows

ShellRoot {
    id: root

    property string waiting: "the programs"

    readonly property string home: Quickshell.env("HOME")
    // What quiet had open lately, one path a line, as the check writes it.
    readonly property string recentFile: `${home}/recent`

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
        // The row chosen, whose drawer shows; Applications ends a program's
        // watch once it's off the program's row.
        property var selected: null
        property var held: []
        property string typed: ""
        property var belowChord: []
        // What each program's step lists from other places, and the one
        // listed under its row, which a program's note reads: none here.
        property var partsOfRows: ({})
        property string underListed: ""
        property var sources: [notices, keys, giver]
        property var entered: null
        property var went: null

        // Nothing is drawn here, so what waits on a frame runs at once.
        function afterFrame(then) {
            then();
        }

        function enter(row) {
            entered = row;
        }

        function goTo(spot) {
            went = spot;
        }

        // Each list shown again in its rows' own order, as a search's
        // answer asks (Launcher.qml's reshow).
        property var reshown: []

        function reshow(count, ordered) {
            reshown = reshown.concat([`${count} ${ordered}`]);
        }
    }

    // Another provider giving a program's Recent its own things, as Docs'
    // documents are given, and the acts of a program whose entry hides
    // itself, as Tuple's are.
    QtObject {
        id: giver

        signal changed

        property var hiddenPrograms: ["pair"]
        // The acts it gives that are found from the top too, as Meet's
        // Join by Code is, none until a case gives one.
        property var foundActs: []

        // How many times its acts were asked, so a case sees that the acts
        // found from the top never ask them.
        property int askedActs: 0

        function programActs(program) {
            askedActs += 1;
            return program === "tester" ? [
                { key: "tester:link", name: "Copy a New Log's Link", verb: "copy" }
            ] : program !== "pair" ? [] : [
                { key: "pair:link", name: "Copy the Room's Link", verb: "copy" },
                { key: "pair:room", name: "Open the Room", verb: "open" }
            ];
        }

        // Who's in the room, which its acts read as swayimg's read its
        // galleries, so a case sees it change remake none of them.
        property int seated: 0

        // The acts found from the top alone, as Neovim's are (Provider.qml),
        // never asking every act it gives.
        function foundActsOf(program) {
            return program !== "pair" ? [] : [
                { key: "pair:room", name: "Open the Room", verb: "open", seated: giver.seated }
            ];
        }

        // How many times it was asked a program's Recent and what a program
        // has open, so a case sees a program's note asks each once.
        property int askedRecent: 0
        property int askedOpen: 0

        function programOpen(program) {
            askedOpen += 1;
            return [];
        }

        function recentOf(program) {
            askedRecent += 1;
            return program === "tester" ? [{ key: "gdoc:plan", name: "Plan Doc", at: 200000 }]
                : program === "pair" ? [{ key: "gdoc:notes", name: "Pair Notes" }] : [];
        }
    }

    // Shortcuts as a program's row asks it: Quiet has keys of its own.
    QtObject {
        id: keys

        function layerOf(app) {
            return app !== "quiet" ? null : {
                key: "shortcut-level:shortcuts:quiet",
                name: "Quiet",
                count: "Volume",
                side: "4",
                verbs: () => []
            };
        }
    }

    Launcher.PastNotices {
        id: notices

        launcher: theLauncher
        settings: ({ name: "notices" })
        onChanged: root.ready()
    }

    // The notices daemon, serving the history's rows, the tester's
    // notices naming its desktop entry, as the daemon finds it.
    NoticesDaemon {
        notices: Notices.history.map(one => Object.assign({
            entry: one.app.toLowerCase() === "tester" ? "tester" : "" }, one))
    }

    Launcher.Applications {
        id: applications

        launcher: theLauncher
        settings: ({
            name: "applications",
            recents: [{
                app: "quiet",
                kind: "folder",
                watch: root.recentFile,
                list: [Quickshell.env("RECENT_LIST")],
                open: ["quiet", "@", "--to", "unix:@quiet-{pid}", "launch", "{path}"],
                start: ["quiet", "--directory", "{path}"]
            }, {
                // The list programs share, as recent-files prints it.
                watch: `${root.home}/shared`,
                list: [Quickshell.env("SHARED_LIST")],
                dated: true
            }, {
                // Sites, read as the browser is stepped into.
                app: "browser",
                kind: "site",
                asked: true,
                list: [Quickshell.env("SITES_LIST")],
                opened: [Quickshell.env("OPENED_LIST")],
                go: ["tabs", "go", "{id}"],
                close: ["tabs", "close", "{id}"]
            }, {
                // What the viewer has open, as mpv's source says, and
                // nothing else, so the rest stays the shared list's.
                app: "org.example.viewer",
                kind: "file",
                opened: [Quickshell.env("VIEWING_LIST")],
                go: ["editor", "go", "{id}"]
            }]
        })
        // Desktop entries and what quiet had open are read in the
        // background, so the cases wait for all three and both lists.
        onThingsChanged: root.ready()
    }

    Connections {
        target: applications.recents

        function onLatelyChanged() {
            if (root.waiting === "a change while closed")
                root.fail("quiet's list was read while the launcher was closed");
            else if (root.waiting === "read as it opens")
                root.readOnOpening();
            else if (root.waiting === "the recents to change")
                root.changed();
            else if (root.waiting === "the sites")
                root.browsed(1);
            else if (root.waiting === "the sites again")
                root.browsed(2);
            else
                root.ready();
        }

        function onSharedChanged() {
            root.sharedSaid += 1;
            root.ready();
        }

        function onOpenNowChanged() {
            if (root.waiting === "the sites")
                root.browsed(1);
            else if (root.waiting === "the viewer")
                root.viewed();
        }
    }

    // How often the list programs share has changed, as everything that
    // reads it reads it again.
    property int sharedSaid: 0

    // Writes quiet's list anew, as the program would.
    FileView {
        id: writer

        path: root.recentFile
        printErrors: false
    }

    function ready() {
        if (root.waiting === "the programs"
                && ["tester", "quiet", "radio", "browser", "pair", "org.example.viewer"]
                    .every(id => root.program(id) !== undefined)
                && applications.recents.of("quiet").length === 2
                && applications.recents.of("tester").length === 3
                && applications.recents.of("radio").length === 1
                && notices.everything().length === 3)
            Qt.callLater(root.cases);
    }

    // How often the launcher heard the programs changed, as it would redraw.
    property int redrawn: 0

    Connections {
        target: applications

        function onChanged() {
            root.redrawn += 1;
        }
    }

    // Once its file changes, Recent says what's in it now, nine at most.
    function changed() {
        const now = applications.recents.of("quiet");
        if (now[0] !== `${root.home}/folder 2`)
            return;
        try {
            const recent = program("quiet").verbs()[2];
            same("Recent follows its file, nine at most", `${recent.side} ${names(recent.targets())}`,
                "9 folder 2, folder 3, folder 4, folder 5, folder 6, folder 7, folder 8, folder 9, "
                    + "folder 10");
            same("each in the folder it's in", recent.targets()[0].side, "~");
            // A step held is drawn again at each change of the provider it
            // came from, so one that said its Recent anew made every row of
            // it again each time (Joel's shell, 2026-11-10: Chrome's step
            // made again as its downloads were read, 136 ms).
            const before = recent.targets();
            const again = program("quiet").verbs()[2];
            const made = applications.tally.rows;
            const held = again.targets();
            expect("asked again, its Recent makes none of its rows again",
                applications.tally.rows === made && held.every((row, at) => row === before[at]),
                applications.tally.rows - made);
            // Read once every handler of the change has run.
            Qt.callLater(() => {
                try {
                    expect("a program held as its list changes is drawn again",
                        root.redrawn > 0, root.redrawn);
                    root.redrawn = 0;
                    giver.changed();
                    Qt.callLater(() => {
                        try {
                            expect("a program held as another provider's Recent for it changes is drawn again",
                                root.redrawn > 0, root.redrawn);
                            root.browse();
                        } catch (error) {
                            fail(error.message);
                        }
                    });
                } catch (error) {
                    fail(error.message);
                }
            });
        } catch (error) {
            fail(error.message);
        }
    }

    // A program whose sites are read as it's stepped into: not before,
    // once an opening, and again the next time the launcher opens.
    function browse() {
        // Two of its windows, each titled by the tab in front of it.
        Niri.windows = Object.assign({}, Niri.windows, {
            "21": { id: 21, app_id: "browser", title: "Open A - Browser", workspace_id: 1,
                focus_timestamp: { secs: 300, nanos: 0 } },
            "22": { id: 22, app_id: "browser", title: "Another - Browser", workspace_id: 1,
                focus_timestamp: { secs: 250, nanos: 0 } },
            "23": { id: 23, app_id: "browser", title: "Left out - Browser", workspace_id: 1,
                focus_timestamp: { secs: 200, nanos: 0 } }
        });
        windowsChanged();
        theLauncher.open = false;
        theLauncher.open = true;
        same("sites aren't read before their program is stepped into",
            applications.recents.of("browser").length, 0);
        root.waiting = "the sites";
        const asking = program("browser").verbs().find(row => row.name === "Recent");
        same("Recent stands while its sites are first read, waiting",
            asking ? `${asking.side} ${asking.empty}` : "no Recent", "\u2026 \u2026");
    }

    function browsed(reading) {
        if (applications.recents.of("browser").length === 0
                || applications.recents.openIn("browser").length === 0)
            return;
        try {
            const rows = program("browser").verbs();
            same("stepped into, its open tabs stand as one row before its other windows and "
                + "Recent, a window its front tab stands for not listed again", names(rows),
                "Go To, New Window, New Window Below, Tabs, Left out, Recent, Quit");
            same("Tabs says how many tabs, and is stepped into",
                `${rows[3].side} ${rows[3].nested} ${rows[3].verb}`, "5 tabs true more");
            same("its pages are its verbs and its targets alike",
                names(rows[3].verbs()), names(rows[3].targets()));
            const visited = rows[5].targets();
            const sites = rows[3].targets().concat(visited);
            expect("asked again, the sites it visited are the rows made before",
                program("browser").verbs()[5].targets().every((row, at) => row === visited[at]),
                names(visited));
            same("what's open, one row a page, newest looked at first, then in Recent what was "
                + "open, never a page open now nor one left out", names(sites),
                `Open A, Older page, Another, Behind, Filed, Page ${reading}`);
            same("Recent says how many, nothing after its name, and their names for its drawer",
                `${rows[5].side} ${rows[5].count ?? ""}|${rows[5].heldNames}`,
                `1 |Page ${reading}`);
            same("each saying it's open and in how many tabs, then its host",
                sites.map(row => row.side).join(" | "),
                "open in 2 tabs \u00b7 open.example.com | open \u00b7 other.example.net "
                + "| open \u00b7 another.example.com | open \u00b7 behind.example.com "
                + "| open \u00b7 filed.example.com "
                + "| example.org");
            same("each made as every site is, keyed by the page",
                sites.map(row => `${row.component} ${row.key}`).join(" "),
                "site site:https://open.example.com/a site site:https://other.example.net/a "
                + "site site:https://another.example.com/b site site:https://behind.example.com/ "
                + "site site:https://filed.example.com/ "
                + `site site:https://example.org/${reading}`);
            same("Return goes to one open, and opens one that was",
                sites.map(row => row.verb).join(" "), "go to go to go to go to go to open");
            const inside = sites[0].verbs();
            same("one open holds Go To, Close It, its window, then Copy the Link", names(inside),
                "Go To, Close It, Open A, Copy the Link");
            same("Go To says the window it goes to", inside[0].side, "Open A");
            Niri.acted = [];
            Niri.spawns = [];
            sites[0].run();
            same("Return goes to the tab looked at last, and to its window",
                `${JSON.stringify(Niri.spawns)} ${acted()}`,
                '[["tabs","go","7"]] [{"FocusWindow":{"id":21}}]');
            Niri.acted = [];
            sites[1].run();
            same("a tab behind another goes to the window titled by the one in front", acted(),
                '[{"FocusWindow":{"id":21}}]');
            Niri.acted = [];
            sites[2].run();
            same("and one in front of the other window to that one", acted(),
                '[{"FocusWindow":{"id":22}}]');
            Niri.acted = [];
            sites[3].run();
            same("a tab behind one left out goes to the window titled by that one", acted(),
                '[{"FocusWindow":{"id":23}}]');
            // A tab its source said with no id, as Chrome's session file
            // says each, has nothing to close it or bring it forward by, so
            // its window's row leads rather than a Go To doing only that.
            same("a tab with no id holds neither Go To nor Close It", names(sites[4].verbs()),
                "Another, Copy the Link");
            Niri.acted = [];
            Niri.spawns = [];
            sites[4].run();
            same("and Return brings its window forward, asking the program nothing",
                `${JSON.stringify(Niri.spawns)} ${acted()}`, '[] [{"FocusWindow":{"id":22}}]');
            same("Close It keeps the launcher, saying how many tabs it closes",
                `${inside[1].stays} ${inside[1].note}`, "true closes 2 tabs");
            Niri.spawns = [];
            inside[1].run();
            same("Close It closes each of its tabs", JSON.stringify(Niri.spawns),
                '[["tabs","close","7"],["tabs","close","9"]]');
            const shut = program("browser").verbs()[3].targets()[0];
            same("the page stays where it was, dimmed and said closed, holding only its link",
                `${shut.key} ${shut.gone} ${shut.side} | ${names(shut.verbs())}`,
                "site:https://open.example.com/a true closed \u00b7 open.example.com | Copy the Link");
            const copied = inside[3];
            same("Copy the Link copies its address", `${copied.verb} ${copied.copy}`,
                "copy https://open.example.com/a");
            same("one that was open holds Open, then Copy the Link", names(sites[5].verbs()),
                "Open, Copy the Link");
            same("read once an opening, however often it's stepped into",
                JSON.stringify(applications.recents.asked), '{"browser":true}');
            if (reading === 2) {
                // A source that only says what's open, as Chrome's session
                // file does, with nothing to bring a tab forward or close it.
                const source = applications.settings.recents.find(one => one.app === "browser");
                delete source.go;
                delete source.close;
                const bare = program("browser").verbs()[3].targets()
                    .find(row => row.key === "site:https://another.example.com/b");
                same("with only what's open said, a page holds its window, then Copy the Link",
                    names(bare.verbs()), "Another, Copy the Link");
                Niri.acted = [];
                Niri.spawns = [];
                bare.run();
                same("and Return brings its window forward, asking the program nothing",
                    `${JSON.stringify(Niri.spawns)} ${acted()}`, '[] [{"FocusWindow":{"id":22}}]');
                root.view();
                return;
            }
            sites[5].run();
            same("Return opens the page by the program's own command", JSON.stringify(Niri.spawned),
                JSON.stringify(["browser", "https://example.org/1"]));
            theLauncher.open = false;
            same("closing the launcher lets the next opening read them again",
                JSON.stringify(applications.recents.asked), "{}");
            theLauncher.open = true;
            root.waiting = "the sites again";
            program("browser").verbs();
        } catch (error) {
            fail(error.message);
        }
    }

    // The viewer's windows, each a process with a file open, and the
    // viewer stepped into, which reads what it has open.
    function view() {
        Niri.windows = Object.assign({}, Niri.windows, {
            "31": { id: 31, app_id: "org.example.viewer", pid: 5151, title: "open.pdf",
                workspace_id: 1, focus_timestamp: { secs: 400, nanos: 0 } },
            "32": { id: 32, app_id: "org.example.viewer", pid: 5252, title: "paper.pdf",
                workspace_id: 1, focus_timestamp: { secs: 350, nanos: 0 } },
            "33": { id: 33, app_id: "org.example.viewer", pid: 5353, title: "open.pdf",
                workspace_id: 1, focus_timestamp: { secs: 100, nanos: 0 } },
            "34": { id: 34, app_id: "org.example.terminal", pid: 6060, title: "notes",
                workspace_id: 1, focus_timestamp: { secs: 300, nanos: 0 } },
            "35": { id: 35, app_id: "org.example.terminal", pid: 6060, title: "more notes",
                workspace_id: 1, focus_timestamp: { secs: 200, nanos: 0 } }
        });
        windowsChanged();
        root.waiting = "the viewer";
        program("org.example.viewer").verbs();
        // What it has open was read as it was stepped into before, and
        // the daemon says a list again only once it changes, so it may be
        // here already rather than heard.
        Qt.callLater(root.viewed);
    }

    function viewed() {
        if (root.waiting !== "the viewer"
                || applications.recents.openIn("org.example.viewer").length === 0)
            return;
        root.waiting = "the viewer's files";
        try {
            const rows = program("org.example.viewer").verbs();
            same("what it has open comes before Recent, no window it stands for listed again",
                names(rows), "Go To, New Window, New Window Below, open.pdf, paper.pdf, notes.md, "
                + "Recent, Quit");
            const recent = rows[6];
            const files = rows.slice(3, 6).concat(recent.targets());
            same("what it has open, one row a file while its window is open, newest "
                + "looked at first, then in Recent the list programs share, never a file open now",
                names(files), "open.pdf, paper.pdf, notes.md, old.pdf");
            // An open file in the home leaves its folder out, a lone "~"
            // telling nothing beside how it stands (VKCF3Z, 2026-11-09).
            same("each made as every file is, saying it's open, and in how many, then its folder",
                files.map(row => `${row.component} ${row.key} ${row.side}`).join(" | "),
                `file file:${root.home}/open.pdf open in 2 windows | file `
                + `file:${root.home}/paper.pdf open | file file:${root.home}/notes.md `
                + `open in 2 windows | file file:${root.home}/old.pdf ~`);
            same("Return goes to one open, and opens one that was",
                files.map(row => row.verb).join(" "), "go to go to go to open");
            same("one in a window niri names holds Go To, then Copy the Path, and no Close It",
                names(files[2].verbs()), "Go To, Copy the Path");
            Niri.acted = [];
            Niri.spawns = [];
            files[2].run();
            same("it goes there inside the window through its source, then to the window looked "
                + "at last", `${JSON.stringify(Niri.spawns)} ${acted()}`,
                '[["editor","go","10:6"]] [{"FocusWindow":{"id":34}}]');
            const inside = files[0].verbs();
            same("one open holds Go To, Close It, then Copy the Path", names(inside),
                "Go To, Close It, Copy the Path");
            Niri.acted = [];
            files[0].run();
            same("Return goes to the window looked at last of those that have it", acted(),
                '[{"FocusWindow":{"id":31}}]');
            Niri.acted = [];
            files[1].run();
            same("and another to its own", acted(), '[{"FocusWindow":{"id":32}}]');
            Niri.acted = [];
            same("Close It keeps the launcher", inside[1].stays, true);
            inside[1].run();
            same("Close It closes that window", acted(), '[{"CloseWindow":{"id":31}}]');
            const shut = program("org.example.viewer").verbs()[3];
            same("the file stays where it was, dimmed and said closed, holding only its path",
                `${shut.key} ${shut.gone} ${shut.side} | ${names(shut.verbs())}`,
                `file:${root.home}/open.pdf true closed | Copy the Path`);
            Qt.exit(0);
        } catch (error) {
            fail(error.message);
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    // niri's windows changed, told to Applications as the windows daemon
    // tells it.
    function windowsChanged() {
        HeardWindows.told(applications.windowsDaemon, Niri.windows,
            id => DesktopEntries.heuristicLookup(id), Niri.current);
    }

    function program(id) {
        return applications.things.find(row => row.key === `program:${id}`);
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function near(value, wanted) {
        return Math.abs(value - wanted) < 1e-9;
    }

    // What a case found, as said when it isn't what was wanted.
    function same(what, got, wanted) {
        expect(what, got === wanted, got);
    }

    function acted() {
        return JSON.stringify(Niri.acted);
    }

    function cases() {
        if (root.waiting !== "the programs")
            return;
        root.waiting = "the cases";
        // The windows are matched to programs as niri's change, and these
        // were there before the entries were read.
        const redrawn = root.redrawn;
        windowsChanged();
        try {
            same("the programs' windows heard draw their rows again", root.redrawn - redrawn, 1);
            windowsChanged();
            same("and the same windows again don't", root.redrawn - redrawn, 1);
            // The windows daemon gone a second is said on Programs' row,
            // naming it, since the row is Programs'; not before, since a
            // restart answers within it.
            applications.windowsDaemon.gone();
            same("a windows daemon just gone isn't said yet", applications.noted(), "");
            applications.windowsQuietFor();
            same("gone a second, Programs says so", applications.noted(),
                "windows not answering");
            applications.windowsDaemon.said(JSON.stringify({ id: "hello" }));
            same("and no more once it answers", applications.noted(), "");
            // A step into Programs reads the rows the programs daemon
            // serves, placed, counted and worded there
            // (modules/programs/src/program_rows.rs): the shell makes each
            // the program's own row, going to the window the daemon says
            // was last looked at, or starting it where none is open.
            const open = applications.served({ key: "row:programs:tester",
                thing: "program:tester", id: "tester", name: "Tester", what: "logbook",
                count: "logbook \u00b7 2 windows", verb: "go to", window: 32 });
            same("a served row is the program's", open.key, "program:tester");
            // Its say counts the pages it has open, which says it's open
            // already, so only what it is follows its name.
            same("and says what's served, its windows left to its say",
                `${open.verb}|${open.count}`, "go to|logbook");
            // Ghost has no list of its own, so its say is nothing.
            const ghost = applications.served({ key: "row:programs:ghost",
                thing: "program:ghost", id: "ghost", name: "Ghost", what: "files",
                count: "files \u00b7 1 window", verb: "go to", window: 33 });
            same("one holding nothing counts its windows as served",
                `${ghost.count}|${applications.holding(DesktopEntries.byId("ghost"))}`,
                "files \u00b7 1 window|");
            Niri.acted = [];
            open.run();
            same("its Return goes to the window served", acted(), '[{"FocusWindow":{"id":32}}]');
            const closed = applications.served({ key: "row:programs:tester",
                thing: "program:tester", id: "tester", name: "Tester", what: "logbook",
                count: "logbook", verb: "start", window: null });
            Niri.acted = [];
            closed.run();
            same("a closed one's starts it", `${closed.verb} ${acted()} ${Niri.spawned}`,
                "start [] tester");
            same("a row served for no program here is none",
                applications.served({ key: "row:programs:gone", id: "gone" }), null);
            // The launcher says Programs was stepped into, as it does.
            applications.opened();
            const tester = program("tester").verbs();
            same("in the program's order, each part where it applies, what's given it to copy last",
                names(tester),
                "Go To, New Window, New Window Below, Private window, Open the logbook, "
                + "Draft, Recent, Notices, Quit, Media, Sound, Copy a New Log's Link");
            const [switching, , , secret, logbook, draft, shared, told, quit, media, sound] = tester;
            same("Recent says how many it holds, and their names for its drawer",
                `${shared.side} ${shared.count ?? ""}|${shared.heldNames.join(", ")}`,
                "4 |notes.md, Plan Doc, plan.md, shot.png");
            const files = shared.targets();
            same("its files from the list programs share, by its name or its id, and what another "
                + "provider gives it, merged by when each was last used, newest first",
                `${names(files)} ${files.map(row => row.key).join(" ")}`,
                `notes.md, Plan Doc, plan.md, shot.png file:${root.home}/notes.md gdoc:plan `
                + `file:${root.home}/plan.md file:${root.home}/Pictures/shot.png`);
            // A file from the list programs share is said as a download
            // is (ChromeProgram.qml): when it was last used at its right,
            // with its folder after but the home's, a lone "~" saying
            // nothing (VKCF3Z, 2026-11-09); a picture's mark or a video's,
            // as files wear them; and a drawer saying where it is and what
            // kind, its name, and a picture shown as it is.
            const [notes, , , shot] = files;
            same("each says when it was last used, then its folder but the home's",
                `${notes.side} | ${shot.side}`,
                "1\u00a0Jan\u00a01970 | 1\u00a0Jan\u00a01970 \u00b7 ~/Pictures");
            same("a picture wears a picture's mark, and any other file a file's",
                `${shot.glyph === Theme.glyph.picture} ${notes.glyph === Words.glyph("path:file")}`,
                "true true");
            const plate = typeof shot.drawer === "function" ? shot.drawer() : shot.drawer;
            same("its drawer says where it is and what kind, its name, and shows the picture",
                `${plate?.eyebrow} | ${plate?.title} | ${plate?.preview?.picture}`,
                `in ~/Pictures \u00b7 PNG | shot.png | ${root.home}/Pictures/shot.png`);
            same("one not a picture shows none",
                (typeof notes.drawer === "function" ? notes.drawer() : notes.drawer)?.preview,
                undefined);
            files[0].run();
            same("one opens by the program's own command", JSON.stringify(Niri.spawned),
                JSON.stringify(["tester", `${root.home}/notes.md`]));
            same("a hidden program nobody gives acts to isn't listed", program("ghost"), undefined);
            const pair = program("pair");
            same("a hidden program another gives acts to steps in rather than starting, "
                + "what it gives to copy after everything",
                `${pair.verb} ${pair.stays} ${names(pair.verbs())} | ${names(pair.targets())}`,
                "more true Open the Room, Recent, Copy the Room's Link "
                + "| Open the Room, Recent, Copy the Room's Link");
            const spawnedBefore = JSON.stringify(Niri.spawned);
            pair.run();
            expect("and Return steps in, starting nothing", theLauncher.entered === pair
                && JSON.stringify(Niri.spawned) === spawnedBefore, JSON.stringify(Niri.spawned));
            same("one closed still starts", program("radio").verb, "start");
            same("Go To names the window used last", switching.side, "Logbook");
            switching.run();
            same("Go To goes to it", acted(), '[{"FocusWindow":{"id":11}}]');
            same("the other window says where it is", draft.side, "workspace 2");
            same("an action is run", `${secret.verb} ${logbook.verb}`, "run run");
            expect("its full name still finds it", secret.keywords !== null, secret.keywords);
            secret.run();
            same("an action runs as its entry says", JSON.stringify(Niri.spawned),
                '["tester","--private"]');

            const sure = quit.targets();
            same("Quit asks once, saying what it closes", names(sure),
                "Quit Now, Closing 2 Windows");
            same("as an alert", sure[0].hue, "alert");
            Niri.acted = [];
            sure[0].run();
            same("Quit closes each window through niri", acted(),
                '[{"CloseWindow":{"id":11}},{"CloseWindow":{"id":12}}]');

            const player = Launcher.Playing.players[0];
            same("Media holds more, saying how it stands", `${media.verb} ${media.side}`,
                "more playing");
            // Its drawer is the player's, named by the vocabulary (Media.js).
            const drawn = media.drawer();
            same("Media's drawer is its player's", `${drawn.eyebrow}: ${drawn.title}`,
                "on The Tester \u00b7 track: Song of the test");
            same("where it is leading", `${drawn.facts[0].label} ${drawn.facts[0].value}`,
                "Position 1:38 of 6:43");

            // A program's drawer, its entry the programs daemon's once it's
            // asked (Program.js), where its windows are leading. An entry is
            // read only while the launcher is open, as a drawer is seen.
            theLauncher.open = true;
            program("tester").drawer();
            root.kept = { told, media, sound, player };
            root.waiting = "the tester's entry";
            entryAsked.start();
        } catch (error) {
            fail(error.message);
        }
    }

    // Each tenth of a second until the daemon has said the tester's entry.
    Timer {
        id: entryAsked

        interval: 100
        repeat: true
        onTriggered: {
            if (applications.entryOf("tester") === null)
                return;
            entryAsked.stop();
            root.described();
        }
    }

    // The tester's rows the cases go on with once its entry is said.
    property var kept: ({})

    function described() {
        try {
            const { told, media, sound, player } = root.kept;
            const described = program("tester").drawer();
            same("a program's drawer says where and what it is",
                `${described.eyebrow}: ${described.title}`,
                "on 2 workspaces \u00b7 program: Tester");
            same("what it plays, then opens, then how it starts",
                described.facts.map(fact => fact.label).join(", "),
                "Playing, Opens, Handles, Categories, Command, Desktop ID, Entry File");
            same("what it opens, from its desktop file", described.facts[1].value, "HTML");
            same("its categories, as words", described.facts[3].value,
                "Office \u00b7 Text Editor");
            // Closed again, as the cases after it began, which lets the
            // entry go.
            theLauncher.open = false;
            same("closing lets the entry go", applications.entryWanted, "");
            // Its processes are watched only while a program's row is the
            // one selected (Applications.qml).
            applications.treesWanted = [5151];
            theLauncher.selected = program("tester");
            same("a program's row keeps its processes watched",
                applications.treesWanted.join(", "), "5151");
            theLauncher.selected = { is: "media:playing" };
            same("moving off it ends the watch", applications.treesWanted.join(", "), "");
            theLauncher.selected = null;
            const played = media.targets();
            same("Media's acts, then what's playing", names(played),
                "Pause, Next, Previous, Song of the test");
            same("what's playing says by whom", played[3].side, "The Testers");
            same("as a figure", played[3].figure, true);
            same("Next keeps the launcher", played[1].stays, true);
            same("saying what's on", played[1].state(),
                "Song of the test \u00b7 The Testers");
            played[1].run();
            played[2].run();
            same("Next and Previous reach its player", `${player.nexts} ${player.previouses}`,
                "1 1");
            played[0].run();
            same("Pause pauses it", `${player.isPlaying} ${media.side}`, "false paused");

            const [first, second, filter, other] = Launcher.Playing.streams;
            const volumes = () => [first, second, filter, other]
                .map(node => node.audio.volume.toFixed(2)).join(" ");
            same("Sound holds more, saying how loud", `${sound.verb} ${sound.side}`, "more 40%");
            expect("as a bar", near(sound.graph.values[0], 0.4), sound.graph.values[0]);
            const heard = sound.targets();
            same("Sound's acts, then its volume", names(heard), "Muted, Louder, Quieter, Volume");
            same("Muted a switch saying no", heard[0].side, "no");
            const volume = heard[3];
            same("its volume is a figure", `${volume.side} ${volume.figure}`, "40% true");
            same("with a bar", volume.graph.shape, "bar");
            same("Louder keeps the launcher", heard[1].stays, true);
            same("saying how loud", heard[1].state(), "now 40%");
            heard[1].run();
            same("Louder turns up its streams together, and no one else's", volumes(),
                "0.45 0.45 0.90 0.70");
            same("its volume follows", `${volume.side} ${heard[1].state()}`, "45% now 45%");
            heard[2].run();
            heard[2].run();
            same("Quieter turns it down", volumes(), "0.35 0.35 0.90 0.70");
            heard[0].run();
            same("Mute mutes its streams, and no one else's",
                [first, second, other].map(node => node.audio.muted).join(" "), "true true false");
            same("its volume says so", `${volume.side} ${sound.graph.values[0]}`, "muted 0");
            same("then Muted, keeping its name, says yes",
                `${sound.targets()[0].name} ${sound.targets()[0].side}`, "Muted yes");
            // Its own options, which :setl sets, as Vim's :setlocal sets the
            // buffer's (VKCF3Z, 2026-11-05): mute and volume, on its streams
            // alone, never the machine's output.
            const local = sound.options ?? {};
            same("Sound's options are its mute and volume", Object.keys(local).sort().join(" "),
                "mute volume");
            same("each reading its streams", `${local.mute?.now()} ${local.volume?.now()}`,
                "true 35");
            local.mute?.set(false);
            same("nomute unmutes its streams, and no one else's",
                [first, second, other].map(node => node.audio.muted).join(" "),
                "false false false");
            local.volume?.set(60, { option: "volume", how: "set", value: 60 });
            same("volume=60 sets its streams, and no one else's", volumes(),
                "0.60 0.60 0.90 0.70");
            local.mute?.set(true);
            same("mute mutes them again", `${first.audio.muted} ${second.audio.muted}`,
                "true true");

            same("Notices counts its own, at its right", told.side, "2 \u00b7 1 unread");
            same("and holds more", told.nested, true);
            const sent = told.targets();
            // Each is named by its title alone, as every notice row is (VKCF3Z,
            // 2026-11-08); what it said is its note.
            // Each as the notices daemon serves it, its drawer and its acts
            // the daemon's (modules/notices' row.rs), done as every served
            // row's are (tests/ready-rows.qml).
            same("its notices, newest first", names(sent), "Newest, Oldest");
            // Ctrl+] on a notice goes to its sender where it lives, as Vim
            // follows a tag, by the program's row among Programs.
            same("Ctrl+] follows each to the program that sent it",
                sent.map(row => row.follows).join(" "), "program:tester program:tester");
            same("which is that program's own row", program("tester").key, "program:tester");

            const quiet = program("quiet").verbs();
            same("a program with keys of its own holds them, and only its new windows besides",
                names(quiet), "New Window, New Window Below, Recent, Shortcuts");
            same("its Shortcuts counts its keys", quiet[3].side, "4");

            const recent = quiet[2];
            same("Recent holds more, counting what it had open",
                `${recent.verb} ${recent.side} ${recent.nested}`, "more 2 true");
            const lately = recent.targets();
            same("each by its name, in the order it keeps them", names(lately), "core.nix, etc");
            same("beside the folder it's in", lately.map(row => row.side).join(" "), "~/src /");
            same("Return opens it", lately.map(row => row.verb).join(" "), "open open");
            same("each made as every folder is, keyed by itself",
                `${lately[0].component} ${lately[0].key}`, `folder folder:${root.home}/src/core.nix`);
            same("a folder holds a window there, and Copy the Path last",
                names(lately[0].verbs()), "New Window Here, Copy the Path");
            same("Copy the Path copies its whole path", lately[0].verbs()[1].copy,
                `${root.home}/src/core.nix`);
            lately[0].run();
            same("with none of its windows open, by starting it there",
                JSON.stringify(Niri.spawned),
                JSON.stringify(["quiet", "--directory", `${root.home}/src/core.nix`]));
            const before = Niri.windows;
            Niri.windows = Object.assign({}, before, {
                "13": { id: 13, app_id: "quiet", pid: 4242, title: "Quiet", workspace_id: 1,
                    focus_timestamp: { secs: 50, nanos: 0 } }
            });
            windowsChanged();
            lately[0].run();
            same("with one open, through the process behind it", JSON.stringify(Niri.spawned),
                JSON.stringify(["quiet", "@", "--to", "unix:@quiet-4242", "launch",
                    `${root.home}/src/core.nix`]));
            Niri.windows = before;
            windowsChanged();

            const radio = program("radio").verbs();
            same("a player found by its identity, and a program recording", names(radio),
                "New Window, New Window Below, Recent, Media, Sound");
            const song = radio[2].targets()[0];
            same("a file made as every file is, opened first and its path copied last",
                `${song.component} ${song.key} ${names(song.verbs())}`,
                `file file:${root.home}/song.ogg Open, Copy the Path`);
            song.run();
            same("a file given to a program that opens none opens as the desktop opens it",
                JSON.stringify(Niri.spawned), JSON.stringify(["xdg-open", `${root.home}/song.ogg`]));
            same("Media holds only what the player can do", names(radio[3].targets()), "Play");
            const viewed = program("org.example.viewer").verbs().find(row => row.name === "Recent");
            same("a program the list names by its command, as recent-files does",
                viewed ? names(viewed.targets()) : "no Recent", "paper.pdf, old.pdf");
            same("by its command, the entry shown rather than a hidden one running it too",
                applications.recents.entryOf("viewer", [
                    { id: "org.example.viewer-pdf", name: "Viewer PDF", noDisplay: true, command: ["viewer"] },
                    { id: "org.example.viewer", name: "Viewer", noDisplay: false, command: ["/bin/viewer"] }
                ]), "org.example.viewer");
            same("by a keyword, where its entry goes by a short name the list doesn't know",
                applications.recents.entryOf("Long Name of the Painter", [
                    { id: "org.example.other", name: "Other", keywords: ["painting"],
                        command: ["other"] },
                    { id: "org.example.painter", name: "Painter",
                        keywords: ["Long Name of the Painter", "painting"], command: ["painter"] }
                ]), "org.example.painter");
            const recording = radio[4];
            same("Sound says it records, with nothing to draw",
                `${recording.side} ${recording.graph}`, "recording undefined");
            const [microphone] = recording.targets();
            same("its Microphone alone, as it plays nothing", names(recording.targets()),
                "Microphone");
            same("saying it's heard", microphone.side, "recording");
            const [hush] = microphone.targets();
            same("its Microphone holds Muted, saying no", `${hush.name} ${hush.side}`,
                "Muted no");
            hush.run();
            const [listening, meter] = Launcher.Playing.recorders;
            same("Mute mutes what it records, and not a level meter",
                `${listening.muted} ${meter.muted}`, "true false");
            same("its Microphone says so", `${microphone.side} ${recording.side}`, "muted muted");
            same("then Muted says yes",
                `${microphone.targets()[0].name} ${microphone.targets()[0].side}`, "Muted yes");
            const heldMic = microphone.options ?? {};
            same("its Microphone's option is its mute alone, reading what it records",
                `${Object.keys(heldMic).join(" ")} ${heldMic.mute?.now()}`, "mute true");
            heldMic.mute?.set(false);
            same("nomute unmutes what it records, and not a level meter",
                `${listening.muted} ${meter.muted}`, "false false");

            // Where the daemon can't search, typing on looks only through
            // what the last key found (Provider.qml's poolFor), and finds
            // just what looking through everything would; going back, or
            // another word, looks through everything again. Inside
            // Programs only its programs are searched, not the acts found
            // from the top.
            same("a program's acts are found from the top, not inside Programs",
                applications.things.length > applications.programs.length, true);
            // One found from the top that asks for a choice, as Neovim's Open
            // a File by Name does, goes to its list inside its program on
            // Return, as one asking for words does.
            const choosing = applications.foundAct({ id: "tester", name: "Tester" },
                { key: "tester:pick", name: "Pick a Log", verb: "open", targets: () => [] });
            choosing.run();
            same("a found act asking for a choice goes to its list inside its program",
                `${JSON.stringify(choosing.trail)} ${theLauncher.went === choosing}`,
                '["collection:applications","program:tester","tester:pick"] true');
            const found = typed => names(applications.matched(typed).rows);
            const whole = typed => {
                applications.matchedIn(null, "", []);
                return found(typed);
            };
            applications.matchedIn(null, "", []);
            for (const [typed, narrowed] of [["r", false], ["ra", true], ["rad", true],
                ["ra", false], ["q", false], ["qu", true], ["qu r", true]]) {
                const pool = applications.poolFor(applications.programs, typed);
                same(`"${typed}" looked ${narrowed ? "through the last key's" : "whole"}`,
                    pool !== applications.programs, narrowed);
                const got = found(typed);
                same(`"${typed}" found as looking whole does`, got, whole(typed));
                found(typed);
            }
            // A program held open as its entries are read again keeps the
            // one that's gone, its id and name with it: it offers nothing,
            // and doesn't throw.
            same("a program whose entry is gone offers nothing",
                applications.controls({}, () => {}, () => {}).length, 0);
            // The daemon says every list again as the launcher opens, and
            // the list programs share, heard again as it was, changes
            // nothing that reads it: on Joel's shell it remade every
            // program's acts and every gallery twice an opening (2026-11-09).
            const sharedBefore = root.sharedSaid;
            applications.recents.took(["recent:shared"]);
            same("the list programs share, heard again unchanged, is the same list",
                root.sharedSaid - sharedBefore, 0);
            // An act another provider gives that's found from the top is
            // made as it comes, alone: no program's own act is made again.
            const madeBefore = applications.tally.rows;
            const askedBefore = giver.askedActs;
            giver.foundActs = [{ program: "pair", key: "pair:room" }];
            same("an act given to be found from the top makes its own row and no other",
                applications.tally.rows - madeBefore, 1);
            // It's asked of the acts found from the top alone: asking every
            // act there let Neovim ask open-now inside the binding, a loop
            // on Joel's shell at each opening and closing (2026-11-09).
            same("and it asks none of the giver's other acts", giver.askedActs - askedBefore, 0);
            same("and it's found there", applications.things.filter(row => row.key
                === "program-act:pair:pair:room").length, 1);
            // The same acts given anew, as a binding makes its list again,
            // or what they read changing, makes none of them again: Joel's
            // shut launcher made every act found from the top again about
            // every 2.5 s, 1,216 rows in 2 minutes (2026-11-12).
            const againBefore = applications.tally.rows;
            giver.foundActs = [{ program: "pair", key: "pair:room" }];
            giver.seated += 1;
            same("the same acts given anew, or what they read changing, make none again",
                applications.tally.rows - againBefore, 0);
            giver.foundActs = [];
            // A program's note, read with its row, asks each other provider
            // for its Recent and what it has open once: it asked each twice,
            // and a row that doesn't step in reads its note for its count
            // too, on every row a step into Programs draws (2026-11-10).
            const recentAsked = giver.askedRecent;
            const openAsked = giver.askedOpen;
            same("a program's note counts what another provider gives its Recent",
                program("pair").note, "1 recent");
            same("asking that provider for its Recent once", giver.askedRecent - recentAsked, 1);
            same("and for what it has open once", giver.askedOpen - openAsked, 1);
            // A closed program's count says what it is alone, having no
            // windows to count, so it asks no provider what it holds: a
            // step into Programs asked for every row it drew (2026-11-10).
            const countAsked = giver.askedRecent + giver.askedOpen;
            const closedCount = program("radio").count;
            same("a closed program's count asks no provider what it holds",
                `${closedCount}|${giver.askedRecent + giver.askedOpen - countAsked}`, "|0");
            // Typed inside Programs, the rows holding its letters stand at
            // once, and the daemon's answer, asked as the key comes, takes
            // its own order, shown again with the selection kept.
            theLauncher.open = true;
            theLauncher.typed = "ra";
            theLauncher.held = [{ row: { from: applications }, rows: [] }];
            same("a key inside Programs shows the rows holding its letters at once",
                names(applications.search("ra").rows), "Pair, Radio");
            root.waiting = "the programs daemon's search";
            searchAnswered.start();
        } catch (error) {
            fail(error.message);
        }
    }

    // Each tenth of a second until the daemon has answered what's typed.
    Timer {
        id: searchAnswered

        interval: 100
        repeat: true
        onTriggered: {
            if (theLauncher.reshown.length === 0)
                return;
            searchAnswered.stop();
            root.searchedOn();
        }
    }

    function searchedOn() {
        try {
            same("the answer shows Programs again in its own order",
                theLauncher.reshown.join(", "), "1 true");
            same("and it's the daemon's, which finds the letters together",
                names(applications.search("ra").rows), "Radio");
            // Typed on, its rows are the last answer's that hold them.
            same("a key typed on narrows the last answer, in its order",
                names(applications.search("rad").rows), "Radio");
            theLauncher.typed = "";
            theLauncher.held = [];
            // Written while the launcher is closed, quiet's list isn't read
            // until it opens, however long that is.
            root.waiting = "a change while closed";
            theLauncher.open = false;
            writer.setText(Array.from({ length: 10 }, (_, n) => `${root.home}/folder ${n + 1}`)
                .join("\n") + "\n");
            whileClosed.start();
        } catch (error) {
            fail(error.message);
        }
    }

    // Long enough for a list read at the change to have come.
    Timer {
        id: whileClosed

        interval: 1000
        onTriggered: {
            root.waiting = "read as it opens";
            theLauncher.open = true;
        }
    }

    // The change made while it was closed, read as it opened; then, held
    // inside a program, as Joel would be, its list changes again.
    function readOnOpening() {
        try {
            same("a change while it was closed is read as the launcher opens",
                applications.recents.of("quiet")[0], `${root.home}/folder 1`);
            root.waiting = "the recents to change";
            theLauncher.held = [{ row: program("quiet"), rows: program("quiet").verbs() }];
            root.redrawn = 0;
            // Its first nine move, so what Recent shows changes: a list
            // whose nine stay as they were changes nothing it shows.
            writer.setText(Array.from({ length: 11 }, (_, n) => `${root.home}/folder ${n + 2}`)
                .join("\n") + "\n");
        } catch (error) {
            fail(error.message);
        }
    }
}