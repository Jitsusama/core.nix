// What a program's drawer says, run in Qt's own JavaScript engine, the one
// the shell runs it in. The `a-program-says-all-it-knows` check copies
// launcher/Program.js beside this file and runs `qml -platform offscreen
// program-drawer.qml` over a desktop entry and shared-mime-info's globs
// made up here, shaped as the files write them.
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
        and: "and",
        program: "program",
        kind: "1 kind",
        kinds: "{count} kinds"
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

        expect("a share", [0.045, 0.37, 0].map(Program.percent), ["4.5%", "37%", "0%"]);

        expect("its windows' workspaces", Program.eyebrow([4, 1, 4], "web browser", said),
            "on 1 and 4 \u00b7 web browser");
        expect("three workspaces", Program.eyebrow([3, 1, 2], "terminal", said),
            "on 1, 2 and 3 \u00b7 terminal");
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
        expect("what it opens and handles", Program.opens(types, endings, said),
            { kinds: "5 kinds \u00b7 PDF, HTML, PNG", schemes: "http, https" });
        expect("a kind with no ending known", Program.kindName("application/x-ext-weird", {}),
            "ext-weird");
        expect("one kind", Program.opens(["text/html"], endings, said).kinds,
            "1 kind \u00b7 HTML");
        expect("categories as words", Program.categories(["Network", "WebBrowser",
            "X-Vendor", "Network"]), "Network, Web Browser");

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
            ["on 1 and 4 \u00b7 program", "Google Chrome", "Access the Internet"]);
        expect("what moves first, what others say, what describes, then its identifiers",
            drawn.facts, [
                { label: "[processor]", value: "25%" },
                { label: "[memory]", value: "741\u00a0MB" },
                { label: "[processes]", value: "4" },
                { label: "[playing]", value: "Song of the test \u00b7 The Tester" },
                { label: "Profiles", value: "2" },
                { label: "[opens]", value: "5 kinds \u00b7 PDF, HTML, PNG" },
                { label: "[handles]", value: "http, https" },
                { label: "[categories]", value: "Network, Web Browser" },
                { label: "[command]", value: "google-chrome-stable %U" },
                { label: "[desktop-id]", value: "google-chrome" }
            ]);
        const closed = Program.drawer({ name: "GIMP", places: [],
            usage: null, id: "gimp", command: ["gimp"] }, named, said);
        expect("a closed program says only what describes it",
            [closed.eyebrow].concat(closed.facts.map(fact => fact.label)),
            ["program", "[command]", "[desktop-id]"]);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
