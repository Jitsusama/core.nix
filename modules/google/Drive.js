// What the shell says of the documents Joel looked at lately (src/drive.rs):
// which Google program each kind is, its words, and what its row and the
// drawer say, in the same words a mail's say when. Drive.qml makes each
// one's row from these, and the-shell-reads-the-documents runs them in Qt's
// own engine (tests/drive.qml).
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

function kind(document) {
    return KINDS[document?.kind] ?? null;
}

// The kind a program's desktop entry opens, or "" for one that opens none.
function kindOf(program) {
    return Object.keys(KINDS).find(each => KINDS[each].program === program) ?? "";
}

// What its row says at the right: who owns it and when he last opened it,
// as a mail says when it came, either alone when the other isn't known.
function beside(document, now) {
    return [document.owner ?? "", document.viewed ? Mail.came(document.viewed, now) : ""]
        .filter(part => part !== "").join(" \u00b7 ");
}

// Its details, as the launcher's drawer shows them beside its row: its
// title, who owns it, when he last opened it and when it last changed.
function details(document, untitled) {
    const facts = [
        { label: "Owner", value: document.owner ?? "" },
        { label: "Opened", value: document.viewed ? Mail.dated(document.viewed) : "" },
        { label: "Changed", value: document.changed ? Mail.dated(document.changed) : "" }
    ];
    return {
        title: document.title === "" ? untitled : document.title,
        facts: facts.filter(fact => fact.value !== ""),
        body: ""
    };
}
