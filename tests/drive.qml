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
import "Saves.js" as Saves

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

        // Drive's folders and downloads, as the picker and Files read them.
        // Two folders asked together, both written before the shell read.
        const listings = [
            { folder: "root", asked: now + 2, files: [{ id: "f1" }], more: false, failed: null },
            { folder: "drives", asked: now + 1, files: [], more: false, failed: null }
        ];
        const downloads = [
            { file: "f1", how: "pick", asked: now + 5, path: null, failed: null },
            { file: "f2", how: "open", asked: now + 3, path: null, failed: "runs" },
            { file: "f1", how: "pick", asked: now + 1, path: "/run/f1/old.pdf", failed: null }
        ];
        const drive = { listings, downloads };
        check("a folder listed and the downloads, read", Google.heard(JSON.stringify({
            account: "ready", meetings: [], drive
        })).drive, drive);
        check("no Drive said, none held", Google.heard(JSON.stringify({
            account: "ready", meetings: []
        })).drive, { listings: [], downloads: [] });
        check("each folder asked together is found, none asked before", [
            Google.listing(drive, "drives", now),
            Google.listing(drive, "root", now),
            Google.listing(drive, "root", now + 3),
            Google.listing(drive, "shared", now)
        ], [listings[1], listings[0], null, null]);
        check("the line listing a folder", Google.list("shared"),
            '{"list":{"folder":"shared"}}\n');
        check("the lines downloading, a save saying where and no other", [
            Google.download("f1", "pick", "/home/a/x.pdf"),
            Google.download("f1", "save", "/home/a/x.pdf"),
            Google.download("f1", "save", "")
        ], [
            '{"download":{"file":"f1","how":"pick"}}\n',
            '{"download":{"file":"f1","how":"save","to":"/home/a/x.pdf"}}\n',
            '{"download":{"file":"f1","how":"save"}}\n'
        ]);
        check("a download stands by the newest asked since", [
            Google.downloading(drive, "f1", null),
            Google.downloading(drive, "f1", now),
            Google.downloading(drive, "f1", now + 6),
            Google.downloading(drive, "f2", now),
            Google.downloading({
                downloads: [{ file: "f1", asked: now + 5, path: "/run/f1/new.pdf", failed: null }]
            }, "f1", now + 2)
        ], [
            { state: "", path: "", failed: "" },
            { state: "fetching", path: "", failed: "" },
            { state: "fetching", path: "", failed: "" },
            { state: "failed", path: "", failed: "runs" },
            { state: "done", path: "/run/f1/new.pdf", failed: "" }
        ]);

        // Drive's files as the picker lists them.
        const menu = {
            id: "pdf1", name: "menu.pdf", kind: "file", mime: "application/pdf", size: 2400000,
            changed: at(28, 9, 5), owner: "Ada Lovelace", link: "https://x/pdf1", runs: false
        };
        const sheet = {
            id: "sheet1", name: "Numbers", kind: "spreadsheet",
            mime: "application/vnd.google-apps.spreadsheet", size: null, changed: at(26, 9, 30),
            owner: "", link: "https://docs.google.com/spreadsheets/d/sheet1/edit", runs: false
        };
        const trips = { id: "dir1", name: "Trips", kind: "folder", size: null, changed: null };
        check("a file picked as it is, Google's own as its office file",
            [menu, sheet].map(Drive.pickedAs), [
                { name: "menu.pdf", mime: "application/pdf" },
                { name: "Numbers.xlsx",
                    mime: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" }
            ]);
        check("a file's row says its size and when it changed", Drive.placed(menu, now), {
            key: "drive:pdf1", name: "menu.pdf", note: "2.4 MB \u00b7 09:05",
            drawer: { title: "menu.pdf", facts: [
                { label: "Owner", value: "Ada Lovelace" },
                { label: "Changed", value: "Wednesday 28 October, 09:05" },
                { label: "Size", value: "2.4 MB" }
            ], body: "" }
        });
        check("Google's own says what it comes as", Drive.placed(sheet, now).note, "as .xlsx");
        check("a folder's row, keyed as one", Drive.placed(trips, now),
            { key: "drive:folder:dir1", name: "Trips", note: "",
                drawer: { title: "Trips", facts: [], body: "" } });
        check("Google's own adopts its document row, no other file",
            [Drive.asDocument(sheet), Drive.asDocument(menu)], [{
                id: "sheet1", title: "Numbers", kind: "spreadsheet",
                link: "https://docs.google.com/spreadsheets/d/sheet1/edit", viewed: null,
                changed: at(26, 9, 30), owner: ""
            }, null]);
        check("why a file or a folder wasn't had, as a sentence", [
            Drive.unfetched("gone"), Drive.unfetched("unwritten"), Drive.unfetched("strange"),
            Drive.unlisted("listing"), Drive.unlisted("unreachable"), Drive.unlisted("")
        ], [
            "It's no longer in Google Drive.", "It couldn't be written down.",
            "Google Drive couldn't be reached.", "Listing it.", "Google Drive couldn't be reached.",
            ""
        ]);

        // Saves into Drive: read from the program's line, and none from one
        // before them; asked again by the line Try Again writes.
        const notes = {
            path: "/s/google/drive-saves/f1/Notes.txt", folder: "f1", name: "Notes.txt",
            asked: now, id: "n1", link: "https://drive.google.com/file/d/n1/view", failed: null
        };
        check("the saves read", Google.heard(JSON.stringify({
            account: "ready", meetings: [], drive: { listings: [], downloads: [], uploads: [notes] }
        })).drive.uploads, [notes]);
        check("none from a program before them", Google.heard(JSON.stringify({
            account: "ready", meetings: [], drive: { listings: [], downloads: [] }
        })).drive.uploads, []);
        check("Try Again's line", Google.upload(notes.path),
            '{"upload":{"path":"/s/google/drive-saves/f1/Notes.txt"}}\n');

        // Which to tell: settled, asked since the shell started, not told,
        // oldest first.
        const going = Object.assign({}, notes, { path: "/s/a", id: null, link: null });
        const before = Object.assign({}, notes, { path: "/s/b", asked: now - 60000 });
        const pdf = Object.assign({}, notes, {
            path: "/s/google/drive-saves/root/Menu.pdf", folder: "root", name: "Menu.pdf",
            asked: now + 5, id: null, link: null, failed: "replaceNotGranted"
        });
        check("the saves to tell", Saves.untold([pdf, going, notes, before], now, [])
            .map(upload => upload.name), ["Notes.txt", "Menu.pdf"]);
        check("none told twice", Saves.untold([pdf, notes], now, [Saves.key(notes)])
            .map(upload => upload.name), ["Menu.pdf"]);

        // Where it went, as Joel knows the folder.
        const shown = [{ folder: "root", files: [{ id: "f1", name: "Plans", kind: "folder" }] }];
        check("its folder, by a listing's name for it", Saves.where(notes, shown), "Plans");
        check("My Drive's top", Saves.where(pdf, []), "My Drive");
        check("a folder no listing showed", Saves.where(notes, []), "");

        // What each is told as.
        check("one sent", Saves.notice(notes, shown), {
            summary: "Saved to Google Drive", body: "Notes.txt \u00b7 Plans",
            acts: [{ key: "open", name: "Open in Drive" }], answers: true
        });
        const failed = why => Saves.notice(Object.assign({}, pdf, { failed: why }), []);
        check("signed out", failed("signedOut"), {
            summary: "Not Saved to Google Drive", body: "Menu.pdf: Google signed out",
            acts: [{ key: "sign-in", name: "Sign In" }, { key: "copy", name: "Save a Copy" }],
            answers: false
        });
        check("Drive out of reach", failed("unreachable").acts.map(act => act.name),
            ["Try Again", "Save a Copy"]);
        check("one it may not replace", [failed("replaceNotGranted").body,
            failed("replaceNotGranted").acts.map(act => act.name)], [
            "Menu.pdf: Drive holds one you made elsewhere, which this can't replace",
            ["Save a Copy"]
        ]);
        check("a folder gone", failed("gone").body, "Menu.pdf: that folder is gone");
        check("a copy that couldn't be read", [failed("unread").body, failed("unread").acts],
            ["Menu.pdf couldn't be read, so nothing was saved", []]);

        // A failure's notice, found to take back by its file alone.
        const told = { app: "Google Drive", summary: "Not Saved to Google Drive",
            body: "Menu.pdf: that folder is gone" };
        check("its failure's notice", [
            Saves.failedOf(told, pdf.path),
            Saves.failedOf(told, "/s/google/drive-saves/root/Menu.pdf.bak"),
            Saves.failedOf(Object.assign({}, told, { summary: "Saved to Google Drive" }), pdf.path)
        ], [true, false, false]);

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
