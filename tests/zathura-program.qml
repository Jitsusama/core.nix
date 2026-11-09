// What zathura's provider in the launcher works out (modules/zathura/Zathura.js,
// for ZathuraProgram.qml), run in Qt's own JavaScript engine, the one the
// shell runs it in: open-now's lines read back as each zathura's
// document, page and page count, a line without a whole path passed over
// and a figure zathura didn't say read as none; the page said "p. 41 of
// 212" beside a document only one zathura has open, and nothing beside
// one two have open, or one whose page wasn't said; and its drawer's facts,
// a document each by its name, saying its page and how far through it is,
// or that it's open where its page wasn't said. The
// `the-launcher-says-the-page-zathura-is-on` check copies Zathura.js
// beside this file and runs `qml zathura-program.qml`.
import QtQuick
import "Zathura.js" as Zathura

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

    property int failed: 0

    function check(what, got, wanted) {
        if (JSON.stringify(got) !== JSON.stringify(wanted)) {
            const said = JSON.stringify(got);
            console.warn(`FAIL ${what}: got ${said}, wanted ${JSON.stringify(wanted)}`);
            failed += 1;
        }
    }

    function run() {
        const said = { page: "p. {page} of {pages}", through: "{share}% through", open: "open" };
        const lines = [
            "4242\t/home/joel/charts.pdf\t\t41\t212",
            "4343\t/home/joel/notes.pdf\t\t1\t1",
            "4444\t/home/joel/twice.pdf\t\t3\t9",
            "4545\t/home/joel/twice.pdf\t\t7\t9",
            "4646\t/home/joel/opening.pdf\t\t\t",
            "4747\trelative.pdf\t\t1\t2",
            ""
        ];
        const open = Zathura.opened(lines);
        check("each zathura's document, page and pages", open, [
            { pid: 4242, path: "/home/joel/charts.pdf", page: 41, pages: 212 },
            { pid: 4343, path: "/home/joel/notes.pdf", page: 1, pages: 1 },
            { pid: 4444, path: "/home/joel/twice.pdf", page: 3, pages: 9 },
            { pid: 4545, path: "/home/joel/twice.pdf", page: 7, pages: 9 },
            { pid: 4646, path: "/home/joel/opening.pdf", page: 0, pages: 0 }
        ]);
        check("the page said", Zathura.pageSaid(open[0], said), "p. 41 of 212");
        check("no page said where zathura said none", Zathura.pageSaid(open[4], said), "");
        check("each page beside a document one zathura has open", Zathura.through(open, said), {
            "/home/joel/charts.pdf": "p. 41 of 212",
            "/home/joel/notes.pdf": "p. 1 of 1"
        });
        check("how far through, to the page it's on", Zathura.shareSaid(open[0], said),
            "19% through");
        check("the last page all the way", Zathura.shareSaid(open[1], said), "100% through");
        const named = path => path.slice(path.lastIndexOf("/") + 1);
        check("its drawer's facts", Zathura.facts(open, said, named), [
            { label: "charts.pdf", value: "p. 41 of 212 \u00b7 19% through" },
            { label: "notes.pdf", value: "p. 1 of 1 \u00b7 100% through" },
            { label: "twice.pdf", value: "p. 3 of 9 \u00b7 33% through" },
            { label: "twice.pdf", value: "p. 7 of 9 \u00b7 78% through" },
            { label: "opening.pdf", value: "open" }
        ]);
        check("nothing open, nothing said", Zathura.facts([], said, named), []);
        if (failed > 0) {
            Qt.exit(1);
            return;
        }
        console.log("every case held");
        Qt.exit(0);
    }
}
