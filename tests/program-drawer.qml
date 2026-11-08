// What a program's drawer says, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `a-program-says-all-it-knows` check copies
// launcher/Program.js beside this file and runs `qml -platform offscreen
// program-drawer.qml` over what the machine daemon says of its processes,
// a desktop entry and shared-mime-info's globs made up here, shaped as
// the daemon and the files write them.
import QtQuick
import "launcher/Program.js" as Program

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // The vocabulary's words for the values, as words.json's applications
    // gives them, and each fact named by its id, so a fact's order is read
    // off.
    readonly property var said: ({
        on: "on {places}",
        workspaces: "on {count} workspaces",
        program: "program",
        more: "{count} more"
    })

    function named(id) {
        return `[${id}]`;
    }

    property var failures: []
    property int asked: 0

    function expect(what, got, wanted) {
        asked += 1;
        if (JSON.stringify(got) !== JSON.stringify(wanted))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
    }

    function run() {

        expect("a share, to three significant figures", [0.045, 0.37, 1].map(Program.percent),
            ["4.50%", "37.0%", "100%"]);
        // Two windows' processes and all each started, as the daemon's
        // tree kind says them, and a window it hasn't said yet.
        expect("what its processes use, summed over its windows", Program.usage([
            { pid: 40, share_ratio: 0.2, resident_b: 641376000, processes: 3 },
            { pid: 77, share_ratio: 0.05, resident_b: 100000000, processes: 1 },
            null
        ]), { processor: 0.25, memory: 741376000, count: 4 });
        expect("nothing while the daemon says nothing", Program.usage([null, undefined]), null);
        // A window's kitty started from another window's, so in its tree.
        expect("a process another window's tree holds counted once", Program.usage([
            { pid: 2, share_ratio: 0.1, resident_b: 300, processes: 3, pids: [2, 3, 4] },
            { pid: 3, share_ratio: 0.05, resident_b: 200, processes: 2, pids: [3, 4] }
        ]), { processor: 0.1, memory: 300, count: 3 });
        expect("no share past the whole", Program.usage([{ share_ratio: 0.8 },
            { share_ratio: 0.7 }]).processor, 1);

        expect("windows on one workspace", Program.eyebrow([4, 4], "web browser", said),
            "on 4 \u00b7 web browser");
        expect("on several, how many", Program.eyebrow([3, 1, 2, 3], "terminal", said),
            "on 3 workspaces \u00b7 terminal");
        expect("with no window, what it is", Program.eyebrow([], "web browser", said),
            "web browser");

        const entry = "[Desktop Entry]\nName=Google Chrome\n"
            + "MimeType=application/pdf;text/html;image/png;application/xhtml+xml;"
            + "application/x-ext-weird;x-scheme-handler/http;x-scheme-handler/https;\n"
            + "[Desktop Action new-window]\nMimeType=text/plain;\n";
        const types = Program.mimeTypes(entry);
        expect("its kinds, from its own group", types.length, 7);
        const globs = "# generated\n80:text/html:*.html\n80:text/html:*.htm\n"
            + "50:application/pdf:*.pdf\n50:image/png:*.png\n"
            + "50:application/xhtml+xml:*.xhtml\n10:image/png:*.PNG.bak\n";
        const endings = Program.endings(globs);
        expect("each kind's ending", [endings["text/html"], endings["image/png"]],
            ["HTML", "PNG"]);
        expect("what it opens and handles", Program.opens(types, endings),
            { kinds: ["PDF", "HTML", "PNG", "XHTML", "ext-weird"], schemes: ["http", "https"] });
        expect("a kind with no ending known", Program.kindName("application/x-ext-weird", {}),
            "ext-weird");
        expect("each kind once", Program.opens(["text/html", "text/html"], endings).kinds,
            ["HTML"]);
        expect("categories as words, a dot apart", Program.categories(["Network",
            "WebBrowser", "X-Vendor", "Network"]), "Network \u00b7 Web Browser");

        const drawn = Program.drawer({
            name: "Google Chrome",
            places: [1, 4],
            usage: { processor: 0.25, memory: 741376000, count: 4 },
            playing: "Song of the test \u00b7 The Tester",
            given: [{ label: "Profiles", value: "2" }],
            types: types,
            endings: endings,
            categories: ["Network", "WebBrowser"],
            command: ["google-chrome-stable", "%U"],
            id: "google-chrome",
            comment: " Access the Internet "
        }, named, said);
        expect("a program's drawer", [drawn.eyebrow, drawn.title, drawn.body],
            ["on 2 workspaces \u00b7 program", "Google Chrome", "Access the Internet"]);
        expect("what moves first, what others say, what describes, then its identifiers",
            drawn.facts, [
                { label: "[processor]", value: "25.0%" },
                { label: "[memory]", value: "741\u00a0MB" },
                { label: "[processes]", value: "4" },
                { label: "[playing]", value: "Song of the test \u00b7 The Tester" },
                { label: "Profiles", value: "2" },
                { label: "[opens]",
                    value: "PDF \u00b7 HTML \u00b7 PNG \u00b7 XHTML \u00b7 ext-weird",
                    valueItems: ["PDF", "HTML", "PNG", "XHTML", "ext-weird"]
                        .map(kind => ({ said: kind })), tail: "{count} more" },
                { label: "[handles]", value: "http \u00b7 https" },
                { label: "[categories]", value: "Network \u00b7 Web Browser" },
                { label: "[command]", value: "google-chrome-stable %U",
                    valueItems: [{ said: "google-chrome-stable" }, { said: "%U" }],
                    between: " ", identifies: true },
                { label: "[desktop-id]", value: "google-chrome", identifies: true }
            ]);
        expect("a command's argument naming a path may give way from its middle",
            Program.drawer({ name: "Firefox", command: ["/run/wrappers/bin/firefox", "-P"] },
                named, said).facts[0].valueItems,
            [{ said: "/run/wrappers/bin/firefox", path: "/run/wrappers/bin/firefox" },
                { said: "-P" }]);
        const closed = Program.drawer({ name: "GIMP", places: [],
            usage: null, id: "gimp", command: ["gimp"] }, named, said);
        expect("a closed program says only what describes it",
            [closed.eyebrow].concat(closed.facts.map(fact => fact.label)),
            ["program", "[command]", "[desktop-id]"]);
        const asking = Program.drawer({ name: "GIMP", places: [2], usage: null, asking: true,
            id: "gimp", command: ["gimp"] }, named, said);
        expect("while its figures are asked, they hold their places",
            asking.facts.slice(0, 3), ["processor", "memory", "processes"]
                .map(id => ({ label: `[${id}]`, value: null })));

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
