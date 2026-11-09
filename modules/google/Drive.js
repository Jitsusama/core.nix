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
// words a mail says when it came in (`said`'s). A stream's rows say only
// when, so a Recent reads down one column of times (VKCF3Z, 2026-11-08),
// and who owns it is its drawer's. One never opened says nothing there.
function beside(document, now, said) {
    return document.viewed ? Mail.came(document.viewed, now, said) : "";
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

// A Drive file or folder's row in the picker, as QGB4ON's chooser draws it
// (open-with's FileChooser.qml): its key, name, what it says beside it
// and its details. Beside a file are its size and when it changed; beside
// one of Google's own kinds, what it comes as.
function placed(file, now, said) {
    const exported = EXPORTED[file.kind];
    const folder = file.kind === "folder";
    // Drive says a size only of a file it holds as it is.
    const size = typeof file.size === "number" ? Mail.weighed(file.size) : "";
    const noted = exported ? [`as ${exported.ending}`]
        : [size, file.changed ? Mail.came(file.changed, now, said) : ""];
    const facts = [
        { label: "Owner", value: file.owner ?? "" },
        { label: "Changed", value: file.changed ? Mail.dated(file.changed, now, said) : "" },
        { label: "Size", value: size }
    ];
    return {
        key: folder ? `drive:folder:${file.id}` : `drive:${file.id}`,
        name: file.name,
        note: noted.filter(part => part !== "").join(" \u00b7 "),
        drawer: { title: file.name, facts: facts.filter(fact => fact.value !== ""), body: "" }
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
