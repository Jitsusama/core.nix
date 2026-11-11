// Drives the programs' own acts on a copy and on the repositories under
// ~/src through Quickshell itself (Copied.qml, launcher/Repositories.qml,
// and each program's provider: Neovim's NeovimProgram.qml, Chrome's
// ChromeProgram.qml, the reader's ReaderProgram.qml, zathura's
// ZathuraProgram.qml and GIMP's CopiedPicture.qml), with niri and the
// clipboard's history stood in and the copy set as the clipboard's
// watcher sets it. A path is acted on only
// while its file is there, which its folder says, and goes as the file
// does. Each act on a copy is listed in its program, and found from the
// top, only while the clipboard holds what it takes, says what it takes
// beside it, and opens it: Neovim in the kitty used last at the line and
// column copied, the reader the markdown by the reader's own Return verb,
// zathura the document at the page copied after it, Chrome the link in a
// new window, and GIMP the picture in a file of its own, copied from the history's or written by wl-paste where the history
// has none. The repositories are read once for both acts that list them,
// by the real programs daemon over its socket, as the shell starts, and
// each is listed only while a read found one it
// can open: Open a File by Name lists them, the one whose git last wrote
// first, and a repository's files as it's stepped into, hidden ones but
// not what git ignores, each beside its folder there, opening the one
// picked; Open a Repository on GitHub lists only those with a remote
// there, by their owners and names, opening each one's page. A
// repository's files are forgotten as the launcher closes, and as it
// opens they're all read again, so a folder holding none takes both acts
// away. Chrome's downloads are read only once Chrome is stepped into, by
// the real programs daemon running a stand-in for chrome-recent-sites,
// and the step starts no process of the shell's: each is then a file's row in its
// Recent, dated, its size and when beside it and its drawer saying where
// it is then its kind, a picture previewed, and where it came from, and
// Open the Latest Download is listed and found from the
// top, opening the newest as the desktop opens a file, standing in its
// place saying an ellipsis until the first reading answers. And what each
// Neovim holds, as a stand-in open-now says it: each file open once,
// the Neovim looked at last first, said modified and by its tab before its
// folder, Return going to its buffer through open-now and to its window
// through niri, each file's drawer saying where it is and its buffer;
// and the program's drawer counting them, while the acts found from the
// top ask open-now nothing and make no file's row. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import qs.launcher as Launcher
import "Frames.js" as Frames

ShellRoot {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string src: `${home}/src`

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // Each step runs once what it waits for holds, in order.
    property var steps: []
    property int at: 0
    property string waiting: "the start"
    // What each provider has said changed, counted.
    property int heard: 0
    // What the repositories have said changed, counted.
    property int repositoriesSaid: 0

    Connections {
        target: Launcher.Repositories

        function onChanged() {
            root.repositoriesSaid += 1;
        }
    }

    function tick() {
        if (at >= steps.length)
            return;
        const step = steps[at];
        try {
            if (step.until && !step.until()) {
                waiting = step.what;
                return;
            }
            at += 1;
            step.then();
            if (at >= steps.length) {
                console.log(`every case held, ${steps.length} steps`);
                Qt.exit(0);
            }
        } catch (error) {
            at = steps.length;
            fail(`${step.what}: ${error.message}`);
        }
    }

    Timer {
        interval: 20
        repeat: true
        running: true
        onTriggered: root.tick()
    }

    Timer {
        interval: 30000
        running: true
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    // Takes away the markdown file copied, as Joel might after copying it.
    Process {
        id: removing

        command: ["rm", `${root.src}/beta/README.md`]
    }

    // The clipboard's history, holding a picture newest.
    QtObject {
        id: copies

        property string what: "copy"
        property var all: []
    }

    QtObject {
        id: theLauncher

        property bool open: true
        property var held: []
        property var sources: [copies]

        // What waits for the next frame, as the launcher's afterFrame
        // keeps it (tests/lib/Frames.js), run as a frame is drawn.
        readonly property var frames: Frames.kept()
        function afterFrame(then) {
            frames.due.push(then);
        }
        function drawn() {
            Frames.drawn(frames);
        }

        function enter(row) {
        }
    }

    Launcher.NeovimProgram {
        id: neovim

        launcher: theLauncher
        settings: ({ folder: root.src })
        onChanged: root.heard += 1
    }

    // Neovim given what reads what's open now.
    Launcher.NeovimProgram {
        id: held

        launcher: theLauncher
        settings: ({
            folder: root.src,
            openNow: Quickshell.env("OPEN_NOW"),
            open: "listed:nvim:open"
        })
    }

    // Neovim previewing, through the programs daemon's file-preview, stood
    // in, which says which process answered each file's drawer.
    Launcher.NeovimProgram {
        id: previewed

        launcher: theLauncher
        settings: ({
            folder: root.src,
            openNow: Quickshell.env("OPEN_NOW"),
            open: "listed:nvim:open",
            preview: true
        })
    }

    // What file-preview said of each file's drawer, by its path.
    function previewsSaid() {
        return previewed.previews.previews ?? previewed.previews;
    }

    // A file of Neovim's open now, its row as Neovim lists it.
    function openFile(provider, at) {
        return provider.programActs("nvim").filter(row => String(row.key).startsWith("file:"))[at];
    }

    Launcher.ChromeProgram {
        id: chrome

        launcher: theLauncher
        settings: ({ folder: root.src, command: ["chrome", "--new-window"],
            downloads: "listed:google-chrome:downloads" })
        onChanged: root.heard += 1
    }

    Launcher.ReaderProgram {
        id: reader

        launcher: theLauncher
        settings: ({ reader: "reader" })
    }

    Launcher.ZathuraProgram {
        id: zathura

        launcher: theLauncher
        settings: ({ zathura: "zathura", opened: "" })
    }

    Launcher.CopiedPicture {
        id: gimp

        launcher: theLauncher
        settings: ({ gimp: "gimp", paste: "wl-paste" })
    }

    // A copy as the watcher sets it.
    function copy(types, text) {
        Copied.types = types;
        Copied.text = text;
    }

    function keys(rows) {
        return rows.map(row => row.key);
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function spawned() {
        return JSON.stringify(Niri.spawned);
    }

    // Neovim's files open now and its drawer, as the stand-in open-now says
    // them: window 5 was looked at
    // after window 6, and both are one kitty's.
    function neovimsOpen() {
        Niri.windows = ({
            "5": { id: 5, app_id: "kitty", pid: 41, focus_timestamp: { secs: 20, nanos: 0 } },
            "6": { id: 6, app_id: "kitty", pid: 41, focus_timestamp: { secs: 10, nanos: 0 } }
        });
        const open = held.programActs("nvim").filter(row => String(row.key).startsWith("file:"));
        expect("each file once, the Neovim looked at last first, the file used last first",
            names(open) === "notes.md, main.rs, .env, README.md", names(open));
        const sides = open.map(row => row.side).join(" | ");
        expect("modified, then its tab or hidden or shown, then how many windows, then its folder",
            sides === "modified \u00b7 tab 2 \u00b7 ~/src/alpha/docs | modified \u00b7 tab 1"
                + " \u00b7 in 2 windows \u00b7 ~/src/alpha | hidden \u00b7 ~/src/alpha"
                + " | shown \u00b7 ~/src/beta", sides);
        expect("Go To, then Copy the Path, and no Close It", names(open[0].verbs())
            === "Go To, Copy the Path", names(open[0].verbs()));
        // Its drawer says where it is, its name, and what only Neovim knows:
        // when it was last in a window, then its buffer. With no file-preview
        // given, nothing reads the file, so it says no kind or lines.
        const plate = open[0].drawer();
        const labels = plate.facts.map(fact => fact.label).join(", ");
        expect("a file's drawer, where then its name", plate.eyebrow === "in ~/src/alpha/docs"
            && plate.title === "notes.md" && plate.preview === undefined,
            JSON.stringify(plate));
        expect("then when it was last in a window and its buffer, last",
            labels === "Last Used, Buffer" && plate.facts[1].value === "2",
            JSON.stringify(plate.facts));
        open[0].run();
        expect("its buffer gone to through open-now", spawned() === JSON.stringify([
            Quickshell.env("OPEN_NOW"), "neovim", "go", "41:7", "/run/n1", "2"]), spawned());
        expect("and its window through niri", JSON.stringify(Niri.acted[Niri.acted.length - 1])
            === JSON.stringify({ FocusWindow: { id: 5 } }), JSON.stringify(Niri.acted));
        const facts = held.programFacts("nvim").map(fact => `${fact.label} ${fact.value}`)
            .join(", ");
        expect("the drawer counts them", facts === "Files 4 in 2 windows, Modified 2", facts);
        const given = held.programOpen("nvim").join(", ");
        expect("Programs is told each by its file's key, so Recent leaves it out",
            given === `file:${src}/alpha/docs/notes.md, file:${src}/alpha/main.rs,`
                + ` file:${src}/alpha/.env, file:${src}/beta/README.md`, given);
        expect("and nothing for another program", held.programOpen("vim").length === 0,
            held.programOpen("vim").join(", "));

        // Programs asks the acts found from the top inside a binding
        // (Applications.qml's optedActs), so asking them makes no open
        // file's row: Neovim doing that, and asking open-now, there looped
        // that binding on Joel's shell at each opening and closing
        // (2026-11-09), and made every act found from the top again as a
        // window was focused. What's open is the programs daemon's now, so
        // nothing found asks it.
        const made = held.tally.rows;
        const found = keys(held.foundActsOf("nvim"));
        expect("found from the top, it gives only the acts it lists there",
            JSON.stringify(found) === JSON.stringify(held.foundActs.map(one => one.key)),
            JSON.stringify(found));
        expect("making no file's row", held.tally.rows - made === found.length,
            String(held.tally.rows - made));
        expect("and nothing for another program", held.foundActsOf("vim").length === 0, "");
    }

    Component.onCompleted: {
        const readme = `${src}/beta/README.md`;
        const code = `${src}/alpha/main.rs`;
        steps = [{
            what: "the repositories and their remotes read as the shell starts",
            until: () => Launcher.Repositories.read && Launcher.Repositories.remotesRead,
            then: () => {
                const repositories = Launcher.Repositories;
                expect("read once for both", repositories.folder === root.src
                    && repositories.remoting, `${repositories.folder} ${repositories.remoting}`);
                for (const [program, provider] of [["nvim", neovim], ["google-chrome", chrome],
                    ["reader", reader], ["org.pwmt.zathura", zathura], ["gimp", gimp]]) {
                    const acts = provider.programActs(program);
                    expect(`${program} offers nothing on an empty clipboard`, acts.every(act =>
                        !["verb:nvim:copied", "verb:chrome:link", "verb:reader:markdown",
                            "verb:zathura:copied", "verb:gimp:picture"].includes(act.key)),
                        keys(acts).join(", "));
                }
                expect("only the acts on repositories found from the top",
                    JSON.stringify(neovim.foundActs.concat(chrome.foundActs, reader.foundActs,
                        zathura.foundActs, gimp.foundActs).map(one => one.key))
                        === JSON.stringify(["verb:nvim:find", "verb:chrome:repository"]),
                    JSON.stringify(neovim.foundActs.concat(chrome.foundActs)));
                // Open the Latest Download waits in its place, Chrome not yet
                // stepped into, and is found from the top only once read.
                expect("and inside their programs", keys(neovim.programActs("nvim")
                    .concat(chrome.programActs("google-chrome"))).join(", ")
                    === "verb:nvim:find, verb:chrome:repository, verb:chrome:download", "");
                expect("another program given none", neovim.programActs("kitty").length === 0
                    && chrome.programActs("nvim").length === 0, "");
                root.copy(["text/plain"], `${code}:12:5\n`);
            }
        }, {
            what: "the copied path's file found",
            until: () => Copied.place !== null,
            then: () => {
                expect("its line and column", Copied.place.line === 12
                    && Copied.place.column === 5, JSON.stringify(Copied.place));
                const acts = neovim.programActs("nvim");
                expect("Neovim's acts in order", keys(acts).join(", ")
                    === "verb:nvim:find, verb:nvim:copied", keys(acts).join(", "));
                expect("its path said", acts[1].side === "~/src/alpha/main.rs:12:5", acts[1].side);
                expect("found from the top", neovim.foundActs.map(one => one.key)
                    .includes("verb:nvim:copied"), JSON.stringify(neovim.foundActs));
                expect("not markdown, so the reader offers nothing",
                    reader.programActs("reader").length === 0, "");
                acts[1].run();
                expect("opened in the kitty used last at its line and column", spawned()
                    === JSON.stringify(["kitty", "@", "--to", "unix:@kitty-41", "launch",
                        "--type=os-window", "--cwd", `${src}/alpha`, "nvim",
                        "+call cursor(12, 5)", code]), spawned());
                root.copy(["text/plain"], `${src}/alpha/gone.rs:3`);
            }
        }, {
            what: "a path whose file isn't there left alone",
            until: () => Copied.named?.name === "gone.rs",
            then: () => {
                expect("no place", Copied.place === null, JSON.stringify(Copied.place));
                expect("no act", neovim.programActs("nvim").length === 1, "");
                root.copy(["text/plain", "UTF8_STRING"], `file://${readme}`);
            }
        }, {
            what: "the copied markdown's file found",
            until: () => Copied.place?.name === "README.md",
            then: () => {
                const acts = reader.programActs("reader");
                expect("the reader's act", keys(acts).join(", ") === "verb:reader:markdown",
                    keys(acts).join(", "));
                expect("its name, and the verb Return has on the reader's files",
                    acts[0].name === "Read the Copied Markdown" && acts[0].verb === "open",
                    `${acts[0].name} ${acts[0].verb}`);
                expect("its path said", acts[0].side === "~/src/beta/README.md", acts[0].side);
                expect("found from the top", reader.foundActs.length === 1, "");
                acts[0].run();
                expect("read in the reader", spawned() === JSON.stringify(["reader", readme]),
                    spawned());
                expect("no document, so zathura offers nothing",
                    zathura.programActs("org.pwmt.zathura").length === 0
                        && zathura.foundActs.length === 0, "");
                expect("Neovim offers it too", neovim.programActs("nvim").length === 2, "");
                // Neovim's opens in a new kitty while none runs.
                Niri.windows = ({});
                neovim.programActs("nvim")[1].run();
                expect("in a new kitty while none runs", spawned() === JSON.stringify(["kitty",
                    "--directory", `${src}/beta`, "nvim", readme]), spawned());
            }
        }, {
            what: "the file removed",
            then: () => {
                removing.running = true;
            }
        }, {
            what: "its act gone with it",
            until: () => Copied.place === null,
            then: () => {
                expect("no reader act", reader.programActs("reader").length === 0, "");
                // As pdfgrep says a page a match is on.
                root.copy(["text/plain"], `${home}/report.pdf:12:`);
            }
        }, {
            what: "the copied document's file found",
            until: () => Copied.place?.name === "report.pdf",
            then: () => {
                const acts = zathura.programActs("org.pwmt.zathura");
                expect("zathura's act", keys(acts).join(", ") === "verb:zathura:copied",
                    keys(acts).join(", "));
                expect("its name and verb", acts[0].name === "Read the Copied Document"
                    && acts[0].verb === "open", `${acts[0].name} ${acts[0].verb}`);
                expect("its path and page said", acts[0].side === "~/report.pdf:12", acts[0].side);
                expect("found from the top", zathura.foundActs.length === 1, "");
                expect("not offered elsewhere", zathura.programActs("reader").length === 0, "");
                acts[0].run();
                expect("read in zathura at its page", spawned() === JSON.stringify(["zathura",
                    "--page=12", `${home}/report.pdf`]), spawned());
                root.copy(["text/plain"], `file://${home}/report.pdf`);
            }
        }, {
            what: "the copied document's file found with no page",
            until: () => Copied.place?.name === "report.pdf" && Copied.place.line === 0,
            then: () => {
                zathura.programActs("org.pwmt.zathura")[0].run();
                expect("read where it was left, with no page copied", spawned()
                    === JSON.stringify(["zathura", `${home}/report.pdf`]), spawned());
                root.copy(["text/plain"], "  https://github.com/Jitsusama/core.nix/pull/12 \n");
                expect("a web address left alone",
                    zathura.programActs("org.pwmt.zathura").length === 0, "");
            }
        }, {
            what: "the copied link",
            until: () => chrome.link !== "",
            then: () => {
                const acts = chrome.programActs("google-chrome");
                expect("Chrome's acts in order", keys(acts).join(", ")
                    === "verb:chrome:repository, verb:chrome:link, verb:chrome:download",
                    keys(acts).join(", "));
                expect("its address said",
                    acts[1].side === "github.com/Jitsusama/core.nix/pull/12", acts[1].side);
                acts[1].run();
                expect("opened in a new window", spawned() === JSON.stringify(["chrome",
                    "--new-window", "https://github.com/Jitsusama/core.nix/pull/12"]), spawned());
                expect("found from the top", chrome.foundActs.length === 2, "");
                root.copy(["text/plain"], "see https://github.com/Jitsusama");
                expect("no act for a link among words",
                    chrome.programActs("google-chrome").length === 2, "");
                root.copy(["image/png", "image/jpeg"], "");
                copies.all = [{ picture: "/run/clipboard/7.jpg", across: 1920, down: 1080,
                    about: "JPEG \u00b7 1.2 MB" }];
                const pictured = gimp.programActs("gimp");
                expect("GIMP's act", keys(pictured).join(", ") === "verb:gimp:picture",
                    keys(pictured).join(", "));
                expect("its picture said", pictured[0].side
                    === "1920 \u00d7 1080 \u00b7 JPEG \u00b7 1.2 MB", pictured[0].side);
                pictured[0].run();
                // [sh, -c, its script, $0, the type, the file, GIMP, the
                // history's file, wl-paste]
                const copied = Niri.spawned;
                expect("the history's file copied, by its own type, and opened",
                    copied[0] === "sh" && copied[4] === "image/png"
                    && /\/runtime\/gimp\/Copied [0-9-]+ [0-9.]+\.jpg$/.test(copied[5])
                    && copied[6] === "gimp" && copied[7] === "/run/clipboard/7.jpg", spawned());
                copies.all = [{ text: "words" }];
                expect("its type alone said", gimp.programActs("gimp")[0].side === "PNG",
                    gimp.programActs("gimp")[0].side);
                gimp.programActs("gimp")[0].run();
                const asked = Niri.spawned;
                expect("pasted to a file and opened", asked[0] === "sh" && asked[4] === "image/png"
                    && /\/gimp\/Copied [0-9-]+ [0-9.]+\.png$/.test(asked[5])
                    && asked[6] === "gimp" && asked[7] === "" && asked[8] === "wl-paste",
                    spawned());
                root.copy([], "");
                expect("nothing once it's gone", gimp.programActs("gimp").length === 0
                    && gimp.foundActs.length === 0, "");
            }
        }, {
            what: "the repositories, as Open a File by Name is stepped into",
            then: () => {
                const finding = neovim.programActs("nvim")[0];
                expect("its name and ask", finding.name === "Open a File by Name"
                    && finding.ask === "which repository?" && finding.nested === true,
                    `${finding.name} ${finding.ask}`);
                expect("and what it would say of none",
                    finding.empty === "No repositories under ~/src.", finding.empty);
                const rows = finding.targets();
                expect("the one whose git wrote last first, none past four folders or hidden",
                    names(rows) === "alpha, beta, gamma", names(rows));
                expect("how many said", finding.side === "3", finding.side);
                expect("when its git wrote said", /ago|just now/.test(rows[0].side), rows[0].side);
                const read = Object.keys(Launcher.Repositories.files);
                expect("no repository's files read before it's stepped into", read.length === 0,
                    read);
                rows[0].verbs();
            }
        }, {
            what: "a repository's files read",
            until: () => Launcher.Repositories.files[`${src}/alpha`] !== undefined,
            then: () => {
                const alpha = neovim.programActs("nvim")[0].targets()[0];
                const inside = alpha.verbs();
                const files = inside.filter(row => String(row.key).startsWith("file:"));
                expect("the one changed last first, hidden ones but not what git ignores",
                    names(files) === "main.rs, .env, notes.md, .gitignore", names(files));
                expect("each beside its folder in the repository", files.map(row => row.side)
                    .join(",") === ",,docs,", files.map(row => row.side).join(","));
                expect("Copy the Path last", inside[inside.length - 1].name === "Copy the Path",
                    names(inside));
                Niri.windows = ({ "5": { id: 5, app_id: "kitty", pid: 41,
                    focus_timestamp: { secs: 1, nanos: 0 } } });
                files[0].run();
                expect("opened in its repository", spawned() === JSON.stringify(["kitty", "@",
                    "--to", "unix:@kitty-41", "launch", "--type=os-window", "--cwd",
                    `${src}/alpha`, "nvim", code]), spawned());
                const opening = chrome.programActs("google-chrome")[0];
                const pages = opening.targets();
                expect("only those on GitHub, by owner and name, the newest first",
                    names(pages) === "Jitsusama/alpha, NixOS/gamma", names(pages));
                expect("how many on GitHub said", opening.side === "2", opening.side);
                expect("and what it would say of none on GitHub",
                    opening.empty === "No repository under ~/src has a remote on GitHub.",
                    opening.empty);
                expect("each the site it is",
                    pages[0].key === "site:https://github.com/Jitsusama/alpha"
                    && names(pages[0].verbs()) === "Open, Copy the Link", names(pages[0].verbs()));
                pages[1].run();
                expect("its page opened", spawned() === JSON.stringify(["chrome", "--new-window",
                    "https://github.com/NixOS/gamma"]), spawned());
                theLauncher.open = false;
                const kept = Object.keys(Launcher.Repositories.files);
                expect("a repository's files forgotten as the launcher closes", kept.length === 0,
                    kept);
                expect("the repositories kept", Launcher.Repositories.listed.length === 3, "");
                root.repositoriesSaid = 0;
                theLauncher.open = true;
            }
        }, {
            what: "the repositories asked again as the launcher opens, as they were",
            until: () => Launcher.Repositories.reading === "",
            then: () => {
                // Each act reading them is drawn again at a change, which
                // held kitty's page past its frame when none had come.
                expect("no change said where none came", root.repositoriesSaid === 0,
                    String(root.repositoriesSaid));
                theLauncher.open = false;
                // As if every repository went, so the next read finds none.
                Launcher.Repositories.folder = `${root.home}/nowhere`;
                theLauncher.open = true;
            }
        }, {
            what: "the repositories read again as the launcher opens",
            until: () => Launcher.Repositories.listed.length === 0,
            then: () => {
                const waiting = chrome.programActs("google-chrome");
                expect("neither act listed once none is there", keys(neovim.programActs("nvim")
                    .concat(waiting)).join(", ") === "verb:chrome:download"
                    && neovim.foundActs.length === 0 && chrome.foundActs.length === 0,
                    JSON.stringify(neovim.foundActs.concat(chrome.foundActs)));
                expect("no download read before Chrome is stepped into",
                    chrome.downloads.length === 0 && chrome.recentOf("google-chrome").length === 0,
                    JSON.stringify(chrome.downloads));
                // So it's in its place from the first frame, never joining the
                // page's end once read.
                expect("Open the Latest Download waits in its place until a reading answers",
                    waiting[0].side === "\u2026", waiting[0].side);
                theLauncher.held = [{ row: { key: "program:google-chrome" } }];
                // The programs daemon reads them, so a step's frame never
                // waits on starting a process, nor on anything after it.
                expect("its downloads asked of the daemon, nothing left for the frame",
                    theLauncher.frames.due.length === 0, theLauncher.frames.due.length);
                theLauncher.drawn();
            }
        }, {
            what: "Chrome's downloads read as it's stepped into",
            until: () => chrome.downloads.length > 0,
            then: () => {
                const report = `${root.home}/Downloads/report.pdf`;
                const acts = chrome.programActs("google-chrome");
                expect("Open the Latest Download last", keys(acts).join(", ")
                    === "verb:chrome:download", keys(acts).join(", "));
                expect("its name and the latest's", acts[0].name === "Open the Latest Download"
                    && acts[0].side === "report.pdf", `${acts[0].name} ${acts[0].side}`);
                expect("found from the top", JSON.stringify(chrome.foundActs.map(one => one.key))
                    === JSON.stringify(["verb:chrome:download"]), JSON.stringify(chrome.foundActs));
                acts[0].run();
                expect("opened as the desktop opens it", spawned()
                    === JSON.stringify(["xdg-open", report]), spawned());
                const rows = chrome.recentOf("google-chrome");
                const chart = `${root.home}/Downloads/tide-chart.png`;
                expect("each a file's row, keyed by its path, newest first", keys(rows).join(", ")
                    === `file:${report}, file:${root.home}/Downloads/undated.zip, file:${chart}`,
                    keys(rows).join(", "));
                expect("dated as Chrome finished it, or undated", rows[0].at === 1800000100000
                    && rows[1].at === null, `${rows[0].at} ${rows[1].at}`);
                const sized = "2.05\u00a0kB \u00b7 ";
                expect("its size and when beside it", rows[0].side.startsWith(sized)
                    && rows[1].side === "10\u00a0B", `${rows[0].side} | ${rows[1].side}`);
                const drawer = rows[0].drawer;
                expect("its drawer where it is then its kind, and where it came from",
                    drawer.eyebrow === "in ~/Downloads \u00b7 PDF" && drawer.title === "report.pdf"
                    && JSON.stringify(drawer.facts)
                        === JSON.stringify([{ label: "From", value: "docs.example.com" }])
                    && drawer.preview === undefined,
                    JSON.stringify(drawer));
                expect("none said for one from nowhere", rows[1].drawer.facts.length === 0
                    && rows[1].drawer.eyebrow === "in ~/Downloads \u00b7 ZIP",
                    JSON.stringify(rows[1].drawer));
                const pictured = rows[2].drawer;
                expect("a picture previewed as a file's drawer previews one",
                    pictured.eyebrow === "in ~/Downloads \u00b7 PNG"
                    && JSON.stringify(pictured.preview)
                        === JSON.stringify({ picture: chart, across: 0, down: 0 }),
                    JSON.stringify(pictured));
                expect("Return opens it", rows[0].verb === "open"
                    && names(rows[0].verbs()) === "Open, Copy the Path", names(rows[0].verbs()));
                expect("no other program given them", chrome.recentOf("nvim").length === 0, "");
            }
        }, {
            what: "what each Neovim holds, read ahead",
            until: () => held.neovims.length === 2,
            then: () => root.neovimsOpen()
        }, {
            // Each file's drawer is asked of one file-preview, kept, rather
            // than one started for each drawer shown (Y2YBTS, 2026-11-10).
            what: "the Neovim with a file-preview read ahead",
            until: () => previewed.neovims.length === 2,
            then: () => root.openFile(previewed, 0).drawer()
        }, {
            what: "file-preview's answer for the first file's drawer",
            until: () => Object.keys(root.previewsSaid()).length === 1,
            then: () => root.openFile(previewed, 1).drawer()
        }, {
            what: "file-preview's answer for the second file's drawer",
            until: () => Object.keys(root.previewsSaid()).length === 2,
            then: () => {
                const by = Object.values(root.previewsSaid()).map(shown => shown?.by);
                expect("both asked of one file-preview, kept",
                    by[0] !== undefined && by[0] === by[1], JSON.stringify(by));
            }
        }];
    }
}
