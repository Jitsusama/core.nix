.pragma library
.import "Sprouting.js" as Sprouting

// Where Start New Work's words can be said, each one of its In rows: the
// folders his open quests work in (modules/quests/src/folders.rs), then
// the places the realm's node says work can begin in (mycelium's Places,
// asked through the agents daemon's places.rs: a big repository's zones by
// name, checkouts, folders lately begun in), so typing "pond" finds
// //system/pond (Joel, 24 November: "the list of things to start in
// was shit"). The node's words are drawn, so each is read again here as
// Sprouting reads a page's.

// The most places listed, so a node naming thousands can't slow each
// letter Joel types in Start New Work.
const MOST_PLACES = 2000;

// What opens a zone's name, which the node starts it by.
const ZONE = "//";

// Whether `text` is a plain, non-empty line: nothing a control, format or
// blank character would hide, turn or join.
function plain(text) {
    return typeof text === "string" && text.trim() !== ""
        && Sprouting.uncontrolled(text) === text && Sprouting.unformatted(text) === text;
}

// The rows' places, `{ folder, name, start }`, each folder once and a
// quest's first: each of the quests' `folders` ({ folder, name }), then
// each of the node's `places` ({ folder, name }) whose folder is a whole
// path and whose words are plain, at most MOST_PLACES of them. `start`
// is what a Sprout names as its folder: a zone's name, since one not
// checked out has no folder here yet and the node starts it by its name,
// else the folder.
function wheres(folders, places) {
    const listed = [];
    const seen = new Set();
    const add = row => {
        if (!seen.has(row.folder)) {
            seen.add(row.folder);
            listed.push(row);
        }
    };
    for (const one of Array.isArray(folders) ? folders : []) {
        add({ folder: one.folder, name: one.name, start: one.folder });
    }
    const named = (Array.isArray(places) ? places : []).filter(one =>
        one !== null && typeof one === "object" && plain(one.folder)
        && one.folder.startsWith("/") && plain(one.name));
    for (const one of named.slice(0, MOST_PLACES)) {
        add({ folder: one.folder, name: one.name,
            start: one.name.startsWith(ZONE) ? one.name : one.folder });
    }
    return listed;
}
