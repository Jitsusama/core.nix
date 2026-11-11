// What the shell says of the documents Joel looked at lately (src/drive.rs):
// which Google program each kind is, its words, and what its row and the
// drawer say, in the same words a mail's say when. Drive.qml makes each
// one's row from these, and the-desktop-reads-the-documents runs them in Qt's
// own engine (tests/drive.qml). It also says a Drive folder's files as the
// launcher's picker lists them.
.pragma library
.import "Mail.js" as Mail

// Each kind by the program that opens it and its words in the vocabulary.
const KINDS = {
    document: { program: "google-docs", thing: "docs:document", open: "verb:docs:open",
        link: "verb:docs:link" },
    spreadsheet: { program: "google-sheets", thing: "sheets:spreadsheet", open: "verb:sheets:open",
        link: "verb:sheets:link" },
    presentation: { program: "google-slides", thing: "slides:presentation",
        open: "verb:slides:open", link: "verb:slides:link" }
};

// Drive's own program opens every kind, so its Recent is the files of
// every kind Joel opened last (DriveProgram.qml), and a file no other
// program opens, a folder or a shared drive has its words, each opening
// in Drive's window, a folder or a drive wearing the folder's mark.
const EVERY = "every";
const DRIVE = { program: "google-drive", thing: "drive:file", open: "verb:drive:open",
    link: "verb:drive:link" };
const DRIVES = ["file", "folder", "other", "drive"];

function kind(document) {
    return KINDS[document?.kind] ?? (DRIVES.includes(document?.kind) ? DRIVE : null);
}

// The kind a program's desktop entry opens, EVERY for Drive's own, or ""
// for one that opens none.
function kindOf(program) {
    return program === DRIVE.program ? EVERY
        : Object.keys(KINDS).find(each => KINDS[each].program === program) ?? "";
}

// What its row says at the right: only when he last opened it, in the
// words a mail says when it came in (`said`'s), within `opened`'s, "opened
// {when}", since its drawer's Changed is another time (VKCF3Z,
// 2026-11-08). A stream's rows say only when, so a Recent reads down one
// column of times, and who owns it is its drawer's. One never opened says
// nothing there.
function beside(document, now, said, opened) {
    return document.viewed
        ? String(opened ?? "{when}").replace("{when}", Mail.came(document.viewed, now, said))
        : "";
}

// Its details, as the launcher's drawer shows them beside its row: its
// title, when it last changed and who owns it, what changes most first.
// When he last opened it is its row's side, so the drawer doesn't say it
// again.
function details(document, untitled, now, said) {
    const when = at => (at ? Mail.moment(at, now, said) : "");
    const facts = [
        { label: "Changed", value: when(document.changed) },
        { label: "Owner", value: document.owner ?? "" }
    ];
    return {
        title: document.title === "" ? untitled : document.title,
        facts: facts.filter(fact => fact.value !== ""),
        body: ""
    };
}

// Google's own kinds, as the office program's file each is fetched as
// for the program that asked the picker (src/drive.rs's export).
const EXPORTED = {
    document: { ending: ".docx",
        mime: "application/vnd.openxmlformats-officedocument.wordprocessingml.document" },
    spreadsheet: { ending: ".xlsx",
        mime: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" },
    presentation: { ending: ".pptx",
        mime: "application/vnd.openxmlformats-officedocument.presentationml.presentation" }
};

// A Drive file as the program that asked the picker gets it: its name and
// kind, Google's own kinds as their office program's file.
function pickedAs(file) {
    const exported = EXPORTED[file.kind];
    return exported
        ? { name: `${file.name}${exported.ending}`, mime: exported.mime }
        : { name: file.name, mime: file.mime ?? "" };
}

// A Drive file or folder's row in the picker, drawn as a folder here
// draws one (open-with's FileChooser.qml, by FileFacts.js): its key, its
// name, what a folder's read would say of it (`entry`: whether it's a
// folder, its size and when it changed), and what more Drive says of it
// (`facts`), its owner, after the Size, Changed and Where every file's
// plate says. Drive says a size only of a file it holds as it is, and
// counts no folder's items, so neither is said where it isn't given. One
// of Google's own kinds is the row it is everywhere instead (asDocument),
// noting what it comes as, since that's the file a program is handed.
function placed(file) {
    const exported = EXPORTED[file.kind];
    const folder = file.kind === "folder";
    const entry = { name: file.name, folder };
    if (typeof file.size === "number") {
        entry.size = file.size;
    }
    if (typeof file.changed === "number") {
        entry.modified = file.changed;
    }
    return {
        key: folder ? `drive:folder:${file.id}` : `drive:${file.id}`,
        name: file.name,
        note: exported ? `as ${exported.ending}` : "",
        entry,
        facts: file.owner ? [{ label: "Owner", value: file.owner }] : []
    };
}

// One of Google's own kinds as a document Joel looked at, so the picker
// can adopt its row (Drive.qml's row); null for any other file.
function asDocument(file) {
    if (!EXPORTED[file.kind]) {
        return null;
    }
    return {
        id: file.id, title: file.name, kind: file.kind, link: file.link ?? "",
        viewed: null, changed: file.changed ?? null, owner: file.owner ?? ""
    };
}

// Why a file wasn't fetched, as a short sentence the picker shows.
const UNFETCHED = {
    gone: "It's no longer in Google Drive.",
    runs: "It's a program or a script.",
    unreachable: "Google Drive couldn't be reached.",
    unwritten: "It couldn't be written down."
};

function unfetched(failed) {
    return UNFETCHED[failed] ?? UNFETCHED.unreachable;
}

// What a Drive folder with no rows says in the picker: that it's being
// listed, or why it couldn't be; nothing once it's listed, so the picker
// says it's empty as it says of any folder.
const UNLISTED = {
    listing: "Listing it.",
    gone: "It's no longer in Google Drive.",
    unreachable: "Google Drive couldn't be reached."
};

function unlisted(standing) {
    return standing === "" ? "" : UNLISTED[standing] ?? UNLISTED.unreachable;
}

// What the google daemon serves of a document Joel looked at lately, as
// its ready row (src/document_rows.rs) says it, worded here only where it
// says a time, by the clock, so no row changes on the daemon's side as a
// minute turns (Y2YBTS's ready rows, "Times").

// The document a ready row is of, as `documents` says one, for what opens
// it and what Saves and a program's Recent read of it.
function readyDocument(row) {
    const held = row.held ?? {};
    return {
        id: String(row.thing ?? "").replace(/^gdoc:/, ""),
        title: String(row.name ?? ""),
        kind: String(row.kind ?? ""),
        link: String(held.link ?? ""),
        viewed: held.viewed ?? null,
        mime: String(held.mime ?? "")
    };
}

// What its row says at the right, as `beside` says it: the time it was
// opened, the one time a document's side holds, or what it says.
function readySide(row, now, said, opened) {
    return (row.side ?? []).map(item => (item.at !== undefined
        ? beside({ viewed: item.at }, now, said, opened) : String(item.said ?? "")))
        .filter(part => part !== "").join(" \u00b7 ");
}

// Its acts, in the daemon's order, each `{ key, open }` with the link to
// open or `{ key, copy }` with what to copy, however the daemon asks them.
function readyActs(row) {
    return (row.acts ?? []).map(act => {
        const copy = act.copy ?? act.ask?.copy;
        return copy !== undefined ? { key: String(act.key), copy: String(copy) }
            : { key: String(act.key), open: String(act.open ?? act.ask?.open?.link ?? "") };
    });
}

// Its drawer as Drawer.qml reads one, from the record the daemon made
// ready: each fact `{ label, value }`, a time worded as a mail's moment,
// one that identifies it said so, and its eyebrow giving way by whole
// items from its end, where it is last to go, as DrivePlaces.js's drawn
// gives it.
function readyDrawer(drawer, now, said) {
    const facts = (drawer.facts ?? []).map(fact => {
        const told = {
            label: String(fact.name ?? ""),
            value: fact.at !== undefined ? Mail.moment(fact.at, now, said)
                : String(fact.said ?? "")
        };
        if (fact.identifies) {
            told.identifies = true;
        }
        return told;
    }).filter(fact => fact.value !== "");
    const parts = String(drawer.eyebrow ?? "").split(" \u00b7 ").filter(part => part !== "");
    return {
        title: String(drawer.title ?? ""),
        facts,
        body: String(drawer.body ?? ""),
        eyebrow: parts.join(" \u00b7 "),
        eyebrowItems: parts.map((part, at) => ({ said: part, keep: parts.length - at })),
        eyebrowNames: true
    };
}
