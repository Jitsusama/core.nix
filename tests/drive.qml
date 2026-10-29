// The documents Joel looked at lately, as the shell reads and says them:
// read from the program's line, which program opens each kind, what a
// row says beside its title and what the drawer says, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-shell-reads-the-documents` check copies Drive.js, Mail.js,
// Calendar.js and Google.js beside this file and runs `qml -platform
// offscreen drive.qml`. Times are made on the local clock, so the cases
// hold in whatever zone the check runs in.
import QtQuick
import "Drive.js" as Drive
import "Google.js" as Google

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

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(`${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        // Wednesday 28 October 2026, 15:20 here.
        const at = (day, hour, min) => new Date(2026, 9, day, hour, min).getTime();
        const now = at(28, 15, 20);
        const plan = {
            id: "d1", title: "Quarterly Plan", kind: "document",
            link: "https://docs.google.com/document/d/d1/edit",
            viewed: at(28, 14, 2), changed: at(26, 9, 30), owner: "Ada Lovelace"
        };

        // Read from the program's line, and none from one before documents.
        check("the documents read", Google.heard(JSON.stringify({
            account: "ready", meetings: [], documents: [plan], heard: now
        })).documents, [plan]);
        check("none from a program before them", Google.heard(JSON.stringify({
            account: "ready", meetings: []
        })).documents, []);

        // Which program opens each kind, by its desktop entry.
        check("Docs opens documents", Drive.kindOf("google-docs"), "document");
        check("Sheets opens spreadsheets", Drive.kindOf("google-sheets"), "spreadsheet");
        check("Slides opens presentations", Drive.kindOf("google-slides"), "presentation");
        check("Mail opens none", Drive.kindOf("google-mail"), "");
        check("a presentation's words", Drive.kind({ kind: "presentation" }).open,
            "verb:slides:open");
        check("a kind it doesn't know has none", Drive.kind({ kind: "form" }), null);

        // What its row says beside the title.
        check("its owner and when, today", Drive.beside(plan, now), "Ada Lovelace \u00b7 14:02");
        check("yesterday", Drive.beside(Object.assign({}, plan, { viewed: at(27, 9, 0) }), now),
            "Ada Lovelace \u00b7 yesterday");
        check("one on a shared drive says only when",
            Drive.beside(Object.assign({}, plan, { owner: null }), now), "14:02");
        check("one never opened says only whose",
            Drive.beside(Object.assign({}, plan, { viewed: null }), now), "Ada Lovelace");

        // What the drawer says.
        check("its details in full", Drive.details(plan, "Untitled"), {
            title: "Quarterly Plan",
            facts: [
                { label: "Owner", value: "Ada Lovelace" },
                { label: "Opened", value: "Wednesday 28 October, 14:02" },
                { label: "Changed", value: "Monday 26 October, 09:30" }
            ],
            body: ""
        });
        check("an untitled one's, bare", Drive.details({ title: "", kind: "document" }, "Untitled"),
            { title: "Untitled", facts: [], body: "" });

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
