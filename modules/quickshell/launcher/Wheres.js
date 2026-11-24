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

// What the node calls a big repository's zone, which it starts by name.
const ZONE = "zone";

// Whether `text` is a plain, non-empty line: nothing a control, format or
// blank character would hide, turn or join.
function plain(text) {
    return typeof text === "string" && text.trim() !== ""
        && Sprouting.uncontrolled(text) === text && Sprouting.unformatted(text) === text;
}

// The node's `places` that can be listed, read once as they come
// (Agents.qml's askPlaces) rather than at each letter typed: each whose
// folder is a whole path and whose words are plain, at most MOST_PLACES.
function listable(places) {
    return (Array.isArray(places) ? places : []).filter(one =>
        one !== null && typeof one === "object" && plain(one.folder)
        && one.folder.startsWith("/") && plain(one.name)).slice(0, MOST_PLACES);
}

// The rows' places, `{ folder, name, start }`, each folder once and a
// quest's first: each of the `quests`' folders ({ folder, name }), then
// each of the node's `places`, as listable() left them ({ folder, name,
// kind }), none read again here. `start` is what a Sprout names as its folder: a zone's name,
// since one not checked out has no folder here yet and the node starts it
// by its name, a quest's folder that is a zone's included; else the
// folder, as for a zone whose name two checkouts share, which its name
// alone wouldn't say.
function wheres(quests, places) {
    const named = Array.isArray(places) ? places : [];
    const zones = new Map();
    const folders = new Map();
    for (const one of named.filter(place => place.kind === ZONE)) {
        zones.set(one.folder, one.name);
        folders.set(one.name, (folders.get(one.name) ?? 0) + 1);
    }
    const startOf = folder => {
        const zone = zones.get(folder);
        return zone === undefined || folders.get(zone) > 1 ? folder : zone;
    };
    const listed = [];
    const seen = new Set();
    const add = (folder, name) => {
        if (!seen.has(folder)) {
            seen.add(folder);
            listed.push({ folder, name, start: startOf(folder) });
        }
    };
    for (const one of Array.isArray(quests) ? quests : []) {
        add(one.folder, one.name);
    }
    for (const one of named) {
        add(one.folder, one.name);
    }
    return listed;
}
