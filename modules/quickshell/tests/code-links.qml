// Where a mycelium:repository/ link lands in Code, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-repository-link-lands-on-its-row` check copies
// modules/quickshell/launcher/CodeLinks.js and tests/lib/Expect.js beside
// this file and runs `qml -platform offscreen code-links.qml`. A link
// picks the row its whole key names and no other; a key missing from rows
// read whole in the watch now running picks none, unlisted; and judged by
// rows held from an earlier watch, which may be from before a clone, or by
// none, it picks the row to land on as it comes, and is said missed once
// the watch answers whole without it. It exits 0 once every case holds,
// and otherwise 1, saying which didn't.
import QtQuick
import "CodeLinks.js" as Links
import "Expect.js" as Expect

QtObject {
    Component.onCompleted: Expect.run(cases)

    function cases() {
        // The keys Code's rows are served by, a repository's files row
        // among them, as the daemon serves them.
        const served = ["row:code:github:someone/thing", "row:code:github:someone/thing:files",
            "row:code:local:~/scratch/sketch", "row:code:gitlab:group/sub/other"];
        const said = ["github:someone/thing", "  local:~/scratch/sketch ", "github:someone/thin",
            "github:someone", "thing", "github:someone/thing:files x", ""];
        Expect.same("read whole, a link picks the row its whole key names, and a miss none",
            said.map(one => Links.linked(served, one, true)), [
                { key: "github:someone/thing", row: "row:code:github:someone/thing",
                    unlisted: false },
                { key: "local:~/scratch/sketch", row: "row:code:local:~/scratch/sketch",
                    unlisted: false },
                { key: "github:someone/thin", row: null, unlisted: true },
                { key: "github:someone", row: null, unlisted: true },
                { key: "thing", row: null, unlisted: true },
                { key: "github:someone/thing:files x", row: null, unlisted: true },
                { key: "", row: null, unlisted: true }]);

        // Rows held from an earlier watch may be from before a clone the
        // page already shows, so a key missing from them isn't known
        // missing: it's landed on once its row comes, never said unlisted.
        const thing = { key: "github:someone/thing", row: "row:code:github:someone/thing",
            unlisted: false };
        Expect.same("judged by rows held from an earlier watch, a missing key waits for its row",
            Links.linked(["row:code:gitlab:group/sub/other"], "github:someone/thing", false),
            thing);
        Expect.same("before Code reads its rows a link waits for its row",
            Links.linked([], "github:someone/thing", undefined), thing);
        Expect.same("an empty link picks nothing however the rows were read",
            Links.linked([], "  ", false), { key: "", row: null, unlisted: true });

        // A link landed on before the rows were read whole is judged once
        // the watch answers whole: missed only then, only if its row isn't
        // among them, and never once judged.
        const waiting = "row:code:github:nobody/thing";
        Expect.same("a link waiting is said missed once rows read whole lack it, and only then",
            [Links.missed(waiting, served, true), Links.missed(waiting, served, false),
                Links.missed(waiting, served, undefined),
                Links.missed("row:code:github:someone/thing", served, true),
                Links.missed(null, served, true), Links.missed("", served, true)],
            [true, false, false, false, false, false]);

        // Among drawn rows, a link lands on its repository's alone, never one
        // whose name or a row inside it matches.
        const rows = [
            { name: "thing", servedKey: "row:code:github:someone/thing" },
            { name: "Open a File by Name", servedKey: "row:code:github:someone/thing:files" },
            { name: "thing", servedKey: "row:code:github:another/thing" },
            { name: "keyless" }];
        const all = served.concat("row:code:github:another/thing");
        Expect.same("a link lands on its repository's row and no other, and a miss on none",
            ["github:someone/thing", "github:another/thing", "github:nobody/thing"]
                .map(one => rows.filter(Links.lands(Links.linked(all, one, true).row))
                    .map(row => row.servedKey)),
            [["row:code:github:someone/thing"], ["row:code:github:another/thing"], []]);
    }
}
