// Where a mycelium:repository/ link lands in Code, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-repository-link-lands-on-its-row` check copies
// modules/quickshell/launcher/CodeLinks.js beside this file and runs
// `qml -platform offscreen code-links.qml`. A link lands on the row its
// whole key names and no other; a key no row has lands on none, and says
// so; before Code has read any row it lands on the row as it comes. It
// exits 0 once every case holds, and otherwise 1, saying which didn't.
import QtQuick
import "CodeLinks.js" as Links

QtObject {
    Component.onCompleted: {
        try {
            run();
            console.log("every repository link holds");
            Qt.exit(0);
        } catch (error) {
            console.warn(`FAIL ${error}`);
            Qt.exit(1);
        }
    }

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function run() {
        // The keys Code's rows are served by, a repository's files row
        // among them, as the daemon serves them.
        const served = ["row:code:github:someone/thing", "row:code:github:someone/thing:files",
            "row:code:local:~/scratch/sketch", "row:code:gitlab:group/sub/other"];
        const linked = ["github:someone/thing", "  local:~/scratch/sketch ", "github:someone/thin",
            "github:someone", "thing", "github:someone/thing:files x", ""]
            .map(said => Links.linked(served, said));
        expect("a link lands on the row its whole key names, and a miss on none, saying so",
            JSON.stringify(linked) === JSON.stringify([
                { key: "github:someone/thing", row: "row:code:github:someone/thing",
                    unlisted: false },
                { key: "local:~/scratch/sketch", row: "row:code:local:~/scratch/sketch",
                    unlisted: false },
                { key: "github:someone/thin", row: null, unlisted: true },
                { key: "github:someone", row: null, unlisted: true },
                { key: "thing", row: null, unlisted: true },
                { key: "github:someone/thing:files x", row: null, unlisted: true },
                { key: "", row: null, unlisted: true }]), JSON.stringify(linked));

        // Before any row's read, a key isn't known missing, so it's landed
        // on once its row comes, never said unlisted.
        const unread = Links.linked([], "github:someone/thing");
        expect("before Code reads its rows a link waits for its row",
            JSON.stringify(unread) === JSON.stringify({ key: "github:someone/thing",
                row: "row:code:github:someone/thing", unlisted: false }), JSON.stringify(unread));
        // Rows of another place served beside them don't count as Code's read.
        const elsewhere = Links.linked(["row:machines:anchor"], "github:someone/thing");
        expect("only Code's own rows say it has read them",
            elsewhere.unlisted === false, JSON.stringify(elsewhere));

        // Among drawn rows, a link lands on its repository's alone, never one
        // whose name or a row inside it matches.
        const rows = [
            { name: "thing", servedKey: "row:code:github:someone/thing" },
            { name: "Open a File by Name", servedKey: "row:code:github:someone/thing:files" },
            { name: "thing", servedKey: "row:code:github:another/thing" },
            { name: "keyless" }];
        const landed = ["github:someone/thing", "github:another/thing", "github:nobody/thing"]
            .map(said => rows.filter(Links.lands(Links.linked(served.concat(
                "row:code:github:another/thing"), said).row)).map(row => row.servedKey));
        expect("a link lands on its repository's row and no other, and a miss on none",
            JSON.stringify(landed) === JSON.stringify([["row:code:github:someone/thing"],
                ["row:code:github:another/thing"], []]), JSON.stringify(landed));
    }
}
