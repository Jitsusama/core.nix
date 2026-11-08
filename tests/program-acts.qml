// Drives the programs' own acts on a copy and on the repositories under
// ~/src through Quickshell itself (Copied.qml, launcher/Repositories.qml,
// and each program's provider: Neovim's NeovimProgram.qml, Chrome's
// ChromeProgram.qml, the reader's ReaderProgram.qml and GIMP's
// CopiedPicture.qml), with niri and the clipboard's history stood in and
// the copy set as the clipboard's watcher sets it. A path is acted on only
// while its file is there, which its folder says, and goes as the file
// does. Each act on a copy is listed in its program, and found from the
// top, only while the clipboard holds what it takes, says what it takes
// beside it, and opens it: Neovim in the kitty used last at the line and
// column copied, the reader the markdown by the reader's own Return verb,
// Chrome the link in a new window, and GIMP the picture in a file of its
// own, copied from the history's or written by wl-paste where the history
// has none. The repositories are read once for both acts that list them,
// as the shell starts, and each is listed only while a read found one it
// can open: Open a File by Name lists them, the one whose git last wrote
// first, and a repository's files as it's stepped into, hidden ones but
// not what git ignores, each beside its folder there, opening the one
// picked; Open a Repository on GitHub lists only those with a remote
// there, by their owners and names, opening each one's page. A
// repository's files are forgotten as the launcher closes, and as it
// opens they're all read again, so a folder holding none takes both acts
// away. It exits 0 once every case holds, and otherwise 1, saying which
// didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import qs.launcher as Launcher

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
        id: launcher

        property bool open: true
        property var held: []
        property var sources: [copies]

        function enter(row) {
        }
    }

    Launcher.NeovimProgram {
        id: neovim

        launcher: launcher
        settings: ({ folder: root.src })
        onChanged: root.heard += 1
    }

    Launcher.ChromeProgram {
        id: chrome

        launcher: launcher
        settings: ({ folder: root.src, command: ["chrome", "--new-window"] })
        onChanged: root.heard += 1
    }

    Launcher.ReaderProgram {
        id: reader

        launcher: launcher
        settings: ({ reader: "reader" })
    }

    Launcher.CopiedPicture {
        id: gimp

        launcher: launcher
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
                    ["reader", reader], ["gimp", gimp]]) {
                    const acts = provider.programActs(program);
                    expect(`${program} offers nothing on an empty clipboard`, acts.every(act =>
                        !["verb:nvim:copied", "verb:chrome:link", "verb:reader:markdown",
                            "verb:gimp:picture"].includes(act.key)), keys(acts).join(", "));
                }
                expect("only the acts on repositories found from the top",
                    JSON.stringify(neovim.foundActs.concat(chrome.foundActs, reader.foundActs,
                        gimp.foundActs).map(one => one.key))
                        === JSON.stringify(["verb:nvim:find", "verb:chrome:repository"]),
                    JSON.stringify(neovim.foundActs.concat(chrome.foundActs)));
                expect("and inside their programs", keys(neovim.programActs("nvim")
                    .concat(chrome.programActs("google-chrome"))).join(", ")
                    === "verb:nvim:find, verb:chrome:repository", "");
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
                root.copy(["text/plain"], "  https://github.com/Jitsusama/core.nix/pull/12 \n");
            }
        }, {
            what: "the copied link",
            until: () => chrome.link !== "",
            then: () => {
                const acts = chrome.programActs("google-chrome");
                expect("Chrome's acts in order", keys(acts).join(", ")
                    === "verb:chrome:repository, verb:chrome:link", keys(acts).join(", "));
                expect("its address said",
                    acts[1].side === "github.com/Jitsusama/core.nix/pull/12", acts[1].side);
                acts[1].run();
                expect("opened in a new window", spawned() === JSON.stringify(["chrome",
                    "--new-window", "https://github.com/Jitsusama/core.nix/pull/12"]), spawned());
                expect("found from the top", chrome.foundActs.length === 2, "");
                root.copy(["text/plain"], "see https://github.com/Jitsusama");
                expect("no act for a link among words",
                    chrome.programActs("google-chrome").length === 1, "");
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
                expect("the one whose git wrote last first, none past three folders", names(rows)
                    === "alpha, beta, gamma", names(rows));
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
                launcher.open = false;
                const kept = Object.keys(Launcher.Repositories.files);
                expect("a repository's files forgotten as the launcher closes", kept.length === 0,
                    kept);
                expect("the repositories kept", Launcher.Repositories.listed.length === 3, "");
                // As if every repository went, so the next read finds none.
                Launcher.Repositories.folder = `${root.home}/nowhere`;
                launcher.open = true;
            }
        }, {
            what: "the repositories read again as the launcher opens",
            until: () => Launcher.Repositories.listed.length === 0,
            then: () => {
                expect("neither act listed once none is there", keys(neovim.programActs("nvim")
                    .concat(chrome.programActs("google-chrome"))).join(", ") === ""
                    && neovim.foundActs.length === 0 && chrome.foundActs.length === 0,
                    JSON.stringify(neovim.foundActs.concat(chrome.foundActs)));
            }
        }];
    }
}
