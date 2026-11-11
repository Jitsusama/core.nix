// The documents Joel looked at lately, as the shell reads and says them:
// read from the program's state, which program opens each kind, what a
// row says beside its title and what the drawer says, and Drive's own
// places, which it lists and what each counts, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-desktop-reads-the-documents` check copies Drive.js, DrivePlaces.js,
// FileFacts.js, Mail.js, When.js, Calendar.js, Google.js and Saves.js beside this file
// and runs `qml
// -platform offscreen drive.qml`. Times are made on the local clock, so
// the cases hold in whatever zone the check runs in.
import QtQuick
import "Drive.js" as Drive
import "DrivePlaces.js" as Places
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
        // How a document says when, as a mail does (Mail.qml): the
        // vocabulary's words, and the locale's names, the check's being English.
        const said = {
            now: "just now",
            minutes: "{minutes} min ago",
            yesterday: "yesterday",
            date: (date, format) => Qt.formatDate(date, format)
        };
        const plan = {
            id: "d1", title: "Quarterly Plan", kind: "document",
            link: "https://docs.google.com/document/d/d1/edit",
            viewed: at(28, 14, 2), changed: at(26, 9, 30), owner: "Ada Lovelace"
        };

        // Read from the program's state, and none from one saying none.
        check("the documents read", Google.stateOf({
            account: "ready", meetings: [], documents: [plan], heard: now
        }).documents, [plan]);
        check("none from a program before them", Google.stateOf({
            account: "ready", meetings: []
        }).documents, []);

        // Which program opens each kind, by its desktop entry.
        check("Docs opens documents", Drive.kindOf("google-docs"), "document");
        check("Sheets opens spreadsheets", Drive.kindOf("google-sheets"), "spreadsheet");
        check("Slides opens presentations", Drive.kindOf("google-slides"), "presentation");
        check("Mail opens none", Drive.kindOf("google-mail"), "");
        check("Drive opens every kind", Drive.kindOf("google-drive"), Drive.EVERY);
        check("a presentation's words", Drive.kind({ kind: "presentation" }).open,
            "verb:slides:open");
        check("a kind it doesn't know has none", Drive.kind({ kind: "form" }), null);
        check("every kind no other program opens opens in Drive",
            ["file", "folder", "other", "drive"].map(kind => Drive.kind({ kind }).open),
            ["verb:drive:open", "verb:drive:open", "verb:drive:open", "verb:drive:open"]);
        check("and copies its link by Drive's words", Drive.kind({ kind: "file" }).link,
            "verb:drive:link");

        // What its row says beside the title: only when he opened it, as a
        // stream's rows say only when, whose it is being the drawer's, and
        // that it's when he opened it, the drawer saying when it changed.
        const opened = "opened {when}";
        check("when, today", Drive.beside(plan, now, said, opened), "opened 14:02");
        check("minutes ago",
            Drive.beside(Object.assign({}, plan, { viewed: at(28, 15, 5) }), now, said, opened),
            "opened 15 min ago");
        check("yesterday",
            Drive.beside(Object.assign({}, plan, { viewed: at(27, 9, 0) }), now, said, opened),
            "opened yesterday");
        check("one never opened says nothing",
            Drive.beside(Object.assign({}, plan, { viewed: null }), now, said, opened), "");

        // What the drawer says: when it changed, said as the launcher says
        // when with its time, then whose it is, and never when he opened it,
        // which its side says.
        check("its details in full", Drive.details(plan, "Untitled", now, said), {
            title: "Quarterly Plan",
            facts: [
                { label: "Changed", value: "Monday 09:30" },
                { label: "Owner", value: "Ada Lovelace" }
            ],
            body: ""
        });
        check("an untitled one's, bare",
            Drive.details({ title: "", kind: "document" }, "Untitled", now, said),
            { title: "Untitled", facts: [], body: "" });

        // Drive's own places, read from the program's state, and none from
        // one before them.
        const menuFile = {
            id: "pdf1", title: "menu.pdf", kind: "file", mime: "application/pdf",
            link: "https://drive.google.com/file/d/pdf1/view", viewed: at(28, 9, 5),
            changed: at(27, 9, 0), owner: "Ada Lovelace", size: 2400000,
            created: at(20, 8, 0), changedBy: "Grace Hopper", changedByMe: at(26, 10, 0),
            sharedWithMe: at(21, 8, 0), sharedBy: "Ada Lovelace", shared: true,
            starred: true, drive: "0Team", about: "What's on, and what it costs.",
            across: null, down: null, lasts: null
        };
        const team = {
            id: "0Team", title: "Team", kind: "drive", mime: "",
            link: "https://drive.google.com/drive/folders/0Team", created: at(5, 12, 0),
            shared: true, starred: false, drive: "0Team"
        };
        const places = {
            recent: [menuFile, plan],
            starred: { files: [menuFile], more: false },
            shared: { files: [menuFile, plan], more: true },
            drives: { files: [team], more: false }
        };
        check("the places read", Google.stateOf({
            account: "ready", meetings: [], drive: { places }
        }).drive.places, places);
        check("none from a program before them", Google.stateOf({
            account: "ready", meetings: [], drive: { listings: [] }
        }).drive.places, {
            recent: [], starred: { files: [], more: false }, shared: { files: [], more: false },
            drives: { files: [], more: false }
        });
        check("a place said in part holds what it says", Google.stateOf({
            account: "ready", meetings: [], drive: { places: { starred: { files: [plan] } } }
        }).drive.places.starred, { files: [plan], more: false });

        // Which places Google Drive lists, in Drive's order, and the count
        // each says: how many, or over that where Drive holds more.
        const over = key => key === "drive:place:shared" ? "over {count}" : "past {count}";
        check("each place with its count", Places.listed(places, over)
            .map(place => [place.key, place.count, place.files.length]), [
            ["drive:place:starred", "1", 1],
            ["drive:place:shared", "over 2", 2],
            ["drive:place:drives", "1", 1]
        ]);
        check("a place holding none isn't listed", Places.listed({
            recent: [], starred: { files: [], more: false },
            shared: { files: [plan], more: false }, drives: { files: [], more: true }
        }, over).map(place => place.key), ["drive:place:shared"]);
        check("no places said, none listed", Places.listed(undefined, over), []);

        // A Drive file's kind, as its drawer's eyebrow says it, each part an
        // item it gives way by.
        const named = {
            document: "Document", spreadsheet: "Spreadsheet", presentation: "Presentation",
            folder: "Folder", drive: "Shared Drive", file: "File"
        };
        check("each kind as its drawer says it", [
            Places.kindOf(menuFile, named),
            Places.kindOf({ title: "notes", kind: "file" }, named),
            Places.kindOf({ title: "Plan", kind: "document" }, named),
            Places.kindOf({ title: "Trips", kind: "folder" }, named),
            Places.kindOf(team, named),
            Places.kindOf({ title: "Survey", kind: "other",
                mime: "application/vnd.google-apps.form" }, named),
            Places.kindOf({ title: "x", kind: "other" }, named)
        ], [["PDF"], ["File"], ["Document"], ["Folder"], ["Shared Drive"], ["Form"], ["File"]]);
        check("a picture's size and a video's length", [
            Places.kindOf({ title: "view.png", kind: "file", across: 2400, down: 1600 }, named),
            Places.kindOf({ title: "talk.mp4", kind: "file", across: 1920, down: 1080,
                lasts: 221400 }, named),
            Places.kindOf({ title: "day.mp4", kind: "file", lasts: 3723000 }, named)
        ], [["PNG", "2400 \u00d7 1600"], ["MP4", "1920 \u00d7 1080", "3:41"], ["MP4", "1:02:03"]]);

        // Where it is, as its drawer's eyebrow says it before its kind: the
        // shared drive it's on, by name once the drives are read, else
        // Shared with Me for another's, else My Drive, never a folder, whose
        // name the program doesn't read.
        const where = {
            in: "in {place}", onDrive: "{drive} (shared drive)", mine: "My Drive",
            sharedWithMe: "Shared with Me", drives: "Shared Drives"
        };
        check("where each is", [
            Places.where(menuFile, [team], where),
            Places.where(menuFile, [], where),
            Places.where({ sharedWithMe: at(21, 8, 0) }, [team], where),
            Places.where({ sharedWithMe: at(21, 8, 0), drive: "0Team" }, [team], where),
            Places.where(team, [team], where),
            Places.where(plan, [team], where)
        ], [
            "in Team (shared drive)", "in Shared Drives", "in Shared with Me",
            "in Team (shared drive)", "", "in My Drive"
        ]);

        // Its drawer, with every figure the program read: what changes most
        // first, said as the launcher says when with its time, and its id
        // last, as an identifier.
        const around = { drives: [team], kinds: named, said: where, now, dates: said };
        check("a file's drawer in full", Places.drawn(
            Drive.details(menuFile, "Untitled", now, said), menuFile, around), {
            title: "menu.pdf",
            facts: [
                { label: "Changed", value: "yesterday 09:00" },
                { label: "Changed By", value: "Grace Hopper" },
                { label: "Owner", value: "Ada Lovelace" },
                { label: "You Changed", value: "Monday 10:00" },
                { label: "Created", value: "20 Oct 08:00" },
                { label: "Shared", value: "21 Oct 08:00" },
                { label: "Shared By", value: "Ada Lovelace" },
                { label: "Size", value: "2.4 MB" },
                { label: "ID", value: "pdf1", identifies: true }
            ],
            body: "What's on, and what it costs.",
            eyebrow: "in Team (shared drive) \u00b7 PDF",
            eyebrowItems: [{ said: "in Team (shared drive)", keep: 2 }, { said: "PDF", keep: 1 }],
            eyebrowNames: true
        });
        check("a shared drive's, its kind and when it was made", Places.drawn(
            Drive.details(team, "Untitled", now, said), team, around), {
            title: "Team",
            facts: [
                { label: "Created", value: "5 Oct 12:00" },
                { label: "ID", value: "0Team", identifies: true }
            ],
            body: "",
            eyebrow: "Shared Drive",
            eyebrowItems: [{ said: "Shared Drive", keep: 1 }],
            eyebrowNames: true
        });
        const bare = Places.drawn(Drive.details(plan, "Untitled", now, said), plan,
            Object.assign({}, around, { drives: [] }));
        check("a document from before the rest were read, in My Drive with its kind",
            [bare.eyebrow, bare.facts.map(fact => fact.label)],
            ["in My Drive \u00b7 Document", ["Changed", "Owner", "ID"]]);
        // Its eyebrow gives way from its end, as every eyebrow's items do
        // (Reading.wholeItems): what's measured goes before the kind, and
        // where it is never.
        check("a video's eyebrow keeps where it is longest", Places.drawn(
            { title: "talk.mp4", facts: [], body: "" },
            { id: "v1", title: "talk.mp4", kind: "file", across: 1920, down: 1080, lasts: 221400 },
            around).eyebrowItems.map(item => [item.said, item.keep]), [
            ["in My Drive", 4], ["MP4", 3], ["1920 \u00d7 1080", 2], ["3:41", 1]
        ]);

        // The same file as the google daemon serves it ready
        // (src/document_rows.rs): its row, its acts and its drawer read as
        // the shell made them from the file, only its times worded here.
        const ready = {
            key: "row:documents:pdf1", thing: "gdoc:pdf1", place: "documents", kind: "file",
            name: "menu.pdf", glyph: "file", hue: "program", depth: 0,
            side: [{ at: menuFile.viewed, as: "opened", keep: 1 }],
            drawer: "drawer:documents:pdf1",
            acts: [{ key: "verb:drive:open", ask: { open: { link: menuFile.link } } },
                { key: "verb:drive:link", ask: { copy: menuFile.link } }],
            held: { link: menuFile.link, viewed: menuFile.viewed, mime: menuFile.mime }, order: 1024
        };
        check("a ready row's document", Drive.readyDocument(ready), {
            id: "pdf1", title: "menu.pdf", kind: "file", link: menuFile.link,
            viewed: menuFile.viewed, mime: "application/pdf"
        });
        check("says when it was opened as a document's row does",
            Drive.readySide(ready, now, said, opened), Drive.beside(menuFile, now, said, opened));
        check("and nothing when never opened",
            Drive.readySide(Object.assign({}, ready, { side: [] }), now, said, opened), "");
        check("its acts, opening and copying its link, in the daemon's order",
            Drive.readyActs(ready), [{ key: "verb:drive:open", open: menuFile.link },
                { key: "verb:drive:link", copy: menuFile.link }]);
        check("an act asked at its top level reads alike",
            Drive.readyActs({ acts: [{ key: "verb:docs:open", open: "https://d/1" },
                { key: "verb:docs:link", copy: "https://d/1" }] }),
            [{ key: "verb:docs:open", open: "https://d/1" },
                { key: "verb:docs:link", copy: "https://d/1" }]);
        const timed = (name, when) => ({ name, at: when, as: "moment" });
        check("its drawer reads as the shell's own", Drive.readyDrawer({
            eyebrow: "in Team (shared drive) \u00b7 PDF",
            title: "menu.pdf",
            facts: [
                timed("Changed", menuFile.changed),
                { name: "Changed By", said: "Grace Hopper" },
                { name: "Owner", said: "Ada Lovelace" },
                timed("You Changed", menuFile.changedByMe),
                timed("Created", menuFile.created),
                timed("Shared", menuFile.sharedWithMe),
                { name: "Shared By", said: "Ada Lovelace" },
                { name: "Size", said: "2.4 MB" },
                { name: "ID", said: "pdf1", identifies: true }
            ],
            body: "What's on, and what it costs."
        }, now, said), Places.drawn(Drive.details(menuFile, "Untitled", now, said), menuFile,
            around));
        check("a drawer with no eyebrow has no items",
            Drive.readyDrawer({ title: "Team", facts: [] }, now, said).eyebrowItems, []);

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
        const nowhere = {
            recent: [], starred: { files: [], more: false }, shared: { files: [], more: false },
            drives: { files: [], more: false }
        };
        const unkept = { held: 0, most: 0 };
        check("a folder listed and the downloads, read", Google.stateOf({
            account: "ready", meetings: [], drive
        }).drive, Object.assign({}, drive, { uploads: [], places: nowhere, kept: unkept }));
        check("no Drive said, none held", Google.stateOf({
            account: "ready", meetings: []
        }).drive, { listings: [], downloads: [], uploads: [], places: nowhere, kept: unkept });

        // What Drive's copies here take, as Google Drive's drawer says it.
        const kept = { held: 412_000_000, most: 2_000_000_000 };
        check("what the copies take, read", Google.stateOf({
            account: "ready", meetings: [], drive: { kept }
        }).drive.kept, kept);
        check("a figure not a number is none", Google.stateOf({
            account: "ready", meetings: [], drive: { kept: { held: "lots", most: -1 } }
        }).drive.kept, unkept);
        const keptSaid = { of: "{held} of {most}" };
        check("kept here", Places.kept(kept, keptSaid), "412\u00a0MB of 2.00\u00a0GB");
        check("none kept, nothing said", Places.kept(unkept, keptSaid), "");
        check("from a program before it, nothing said", Places.kept(undefined, keptSaid), "");
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
        check("a file says what a folder's read would, its size and when it changed, and its owner",
            Drive.placed(menu), {
                key: "drive:pdf1", name: "menu.pdf", note: "",
                entry: { name: "menu.pdf", folder: false, size: 2400000, modified: at(28, 9, 5) },
                facts: [{ label: "Owner", value: "Ada Lovelace" }]
            });
        check("Google's own says what it comes as, and no size Drive doesn't give",
            [Drive.placed(sheet).note, Drive.placed(sheet).entry],
            ["as .xlsx", { name: "Numbers", folder: false, modified: at(26, 9, 30) }]);
        check("a folder's row, keyed as one, saying nothing Drive doesn't give",
            Drive.placed(trips), { key: "drive:folder:dir1", name: "Trips", note: "",
                entry: { name: "Trips", folder: true }, facts: [] });
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

        // Saves into Drive: read from the program's state, and none from one
        // before them; asked again by the line Try Again writes.
        const notes = {
            path: "/s/google/drive-saves/f1/Notes.txt", folder: "f1", name: "Notes.txt",
            asked: now, id: "n1", link: "https://drive.google.com/file/d/n1/view", failed: null
        };
        check("the saves read", Google.stateOf({
            account: "ready", meetings: [], drive: { listings: [], downloads: [], uploads: [notes] }
        }).drive.uploads, [notes]);
        check("none from a program before them", Google.stateOf({
            account: "ready", meetings: [], drive: { listings: [], downloads: [] }
        }).drive.uploads, []);
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
        // Back to the program that saved it, only while it's open.
        const unread = Object.assign({}, pdf, { failed: "unread" });
        check("a copy that couldn't be read, its program open",
            Saves.notice(unread, [], "Text Editor").acts,
            [{ key: "go", name: "Go To Text Editor" }]);
        check("only a copy that couldn't be read goes back to it",
            Saves.notice(Object.assign({}, pdf, { failed: "gone" }), [], "Text Editor").acts
                .map(act => act.name), ["Save a Copy"]);

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
