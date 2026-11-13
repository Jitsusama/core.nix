// What a program's drawer says, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `a-program-says-all-it-knows` check copies
// launcher/Program.js beside this file and runs `qml -platform offscreen
// program-drawer.qml` over what the machine daemon says of its processes
// and the programs daemon of its desktop entry, made up here, shaped as
// the daemons write them.
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
        more: "{count} more",
        terminal: "a terminal"
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

        expect("a share, to three significant figures",
            [0.045, 0.37, 1].map(share => Program.percent(share)),
            ["4.50%", "37.0%", "100%"]);
        expect("none at all, as the machine says it", Program.percent(0), "0%");
        expect("a share, pointed as the launcher's marks say",
            Program.percent(0.045, { group: ".", point: "," }), "4,50%");
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

        // The programs daemon's record of its entry (modules/programs):
        // each kind it opens with its globs, heaviest first, and the links
        // it handles.
        const record = {
            opens: [
                { type: "application/pdf", globs: ["*.pdf"] },
                { type: "text/html", globs: ["*.html", "*.htm"] },
                { type: "image/png", globs: ["*.png", "*.PNG.bak"] },
                { type: "application/xhtml+xml", globs: ["*.xhtml"] },
                { type: "application/x-ext-weird", globs: [] }
            ],
            handles: ["http", "https"],
            keywords: ["web", "internet"],
            starts_in: "/srv/web",
            terminal: true,
            entry_path: "/usr/share/applications/google-chrome.desktop",
            window_class: "Google-chrome",
            package: { name: "google-chrome", version: "142.0.7444.59",
                store_path: "/nix/store/abc-google-chrome-142.0.7444.59" }
        };
        expect("what it opens and handles", Program.opens(record),
            { kinds: ["PDF", "HTML", "PNG", "XHTML", "ext-weird"], schemes: ["http", "https"] });
        expect("a kind's first plain glob names it", Program.kindName("image/x-thing",
            ["*.thing.gz", "*.thg"]), "THG");
        expect("a kind with no ending known", Program.kindName("application/x-ext-weird", []),
            "ext-weird");
        expect("each kind once", Program.opens({ opens: [
            { type: "text/html", globs: ["*.html"] }, { type: "text/html", globs: ["*.htm"] }] })
            .kinds, ["HTML"]);
        expect("nothing while the daemon says nothing", Program.opens(null),
            { kinds: [], schemes: [] });
        expect("categories as words, a dot apart", Program.categories(["Network",
            "WebBrowser", "X-Vendor", "Network"]), "Network \u00b7 Web Browser");

        const drawn = Program.drawer({
            name: "Google Chrome",
            places: [1, 4],
            usage: { processor: 0.25, memory: 741376000, count: 4 },
            playing: "Song of the test \u00b7 The Tester",
            given: [{ label: "Profiles", value: "2" }],
            record: record,
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
                { label: "[keywords]", value: "web \u00b7 internet",
                    valueItems: [{ said: "web" }, { said: "internet" }] },
                { label: "[starts-in]", value: "/srv/web",
                    valueItems: [{ said: "/srv/web", whole: true }] },
                { label: "[runs-in]", value: "a terminal" },
                { label: "[version]", value: "142.0.7444.59" },
                { label: "[package]", value: "google-chrome" },
                { label: "[command]", value: "google-chrome-stable %U",
                    valueItems: [{ said: "google-chrome-stable", whole: true },
                        { said: "%U", whole: true }],
                    between: " ", identifies: true },
                { label: "[window-class]", value: "Google-chrome", identifies: true },
                { label: "[desktop-id]", value: "google-chrome", identifies: true },
                { label: "[entry-file]",
                    value: "/usr/share/applications/google-chrome.desktop", identifies: true,
                    valueItems: [{ said: "/usr/share/applications/google-chrome.desktop",
                        whole: true }] },
                { label: "[store-path]", value: "/nix/store/abc-google-chrome-142.0.7444.59",
                    identifies: true, valueItems: [{
                        said: "/nix/store/abc-google-chrome-142.0.7444.59", whole: true }] }
            ]);
        expect("a command's arguments are shown whole, a path's too",
            Program.drawer({ name: "Firefox", command: ["/run/wrappers/bin/firefox", "-P"] },
                named, said).facts[0].valueItems,
            [{ said: "/run/wrappers/bin/firefox", whole: true }, { said: "-P", whole: true }]);
        const closed = Program.drawer({ name: "GIMP", places: [],
            usage: null, id: "gimp", command: ["gimp"] }, named, said);
        expect("a closed program says only what describes it",
            [closed.eyebrow].concat(closed.facts.map(fact => fact.label)),
            ["program", "[command]", "[desktop-id]"]);
        const classed = Program.drawer({ name: "GIMP", places: [], usage: null, id: "gimp",
            command: ["gimp"], record: { window_class: "gimp" } }, named, said);
        expect("a window class that's its entry's id is said once, as the id",
            classed.facts.map(fact => fact.label), ["[command]", "[desktop-id]"]);
        const windowed = Program.drawer({ name: "GIMP", places: [], usage: null, id: "gimp",
            command: ["gimp"], record: { terminal: false } }, named, said);
        expect("a program that needs no terminal says nothing of one",
            windowed.facts.map(fact => fact.label), ["[command]", "[desktop-id]"]);
        const asking = Program.drawer({ name: "GIMP", places: [2], usage: null, asking: true,
            id: "gimp", command: ["gimp"] }, named, said);
        expect("while its figures are asked, they hold their places",
            asking.facts.slice(0, 3), ["processor", "memory", "processes"]
                .map(id => ({ label: `[${id}]`, value: null })));

        // A making timed by its parts on a clock that moves only as each
        // part works, so each part's time is what it was given.
        let clock = 0;
        const now = () => clock;
        const spends = ms => () => {
            clock += ms;
            return ms;
        };
        const parts = [];
        expect("a timed part gives back its answer",
            Program.timed(parts, "kitty", now, spends(60)), 60);
        Program.timed(parts, "passwords", now, spends(4));
        Program.timed(parts, "kitty", now, spends(12));
        Program.timed(parts, "zathura", now, spends(0.2));
        expect("each part's time kept by its name, a name twice summed",
            parts.map(part => [part.name, Math.round(part.ms)]),
            [["kitty", 72], ["passwords", 4], ["zathura", 0]]);
        expect("nothing kept while no making is timed",
            Program.timed(null, "kitty", now, spends(3)), 3);
        expect("a slow making said by its parts, the dearest first, the rest last",
            Program.makingSaid("slack's drawer", 90.4, parts),
            "slack's drawer made in 90 ms: kitty 72, the rest 14, passwords 4");
        expect("a part under a millisecond left unsaid",
            Program.makingSaid("zathura's acts", 30, [{ name: "zathura", ms: 0.4 },
                { name: "neovim", ms: 29.6 }]), "zathura's acts made in 30 ms: neovim 30");
        expect("a making quick enough to feel nothing says nothing",
            Program.makingSaid("slack's drawer", 24, [{ name: "kitty", ms: 20 }]), "");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
