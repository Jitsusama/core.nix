.pragma library
.import "Sprouting.js" as Sprouting

// Where Start New Work's words can be said, each one of its In rows: the
// folders his open quests work in (modules/quests/src/folders.rs), then
// the places the realm's node says work can begin in (mycelium's Places,
// asked through the agents daemon's places.rs: a big repository's zones by
// name, checkouts, folders lately begun in), so typing "pond" finds
// //system/pond (Joel, 24 November: "the list of things to start in
// was shit"). The node's words are drawn, and a zone's name is said to
// the node as where to start, so each is read again here.

// The most places listed, so a node naming thousands can't slow each
// letter Joel types in Start New Work; the agents daemon passes no more.
const MOST_PLACES = 2000;

// The longest name listed, as Sprouting keeps offered words short: a
// repository's own files name its zones.
const MOST_NAMED = 200;

// What the node calls a big repository's zone, which it starts by name,
// and one of its checkouts.
const ZONE = "zone";
const CHECKOUT = "checkout";

// How a zone's name reads (mycelium's places.rs): opening with //, so the
// node takes it as a name, never a folder under where it's asked from
// (its sprout.rs), and one word, so what's started is what's drawn.
const ZONE_NAMED = /^\/\/\S+$/u;

// Why a refusal that gives none is journalled, so each still says one,
// and why no answer at all is: the daemon gone, or never there.
const NO_WHY = "the agents daemon gave no reason";
const NO_ANSWER = "the agents daemon isn't answering";

// Whether `text` is a plain, non-empty line of at most MOST_NAMED
// characters: nothing a control, format or blank character would hide,
// turn or join.
function plain(text) {
    return typeof text === "string" && text.trim() !== "" && text.length <= MOST_NAMED
        && Sprouting.uncontrolled(text) === text && Sprouting.unformatted(text) === text;
}

// The node's `places` that can be listed, read once as they come
// (heard) rather than at each letter typed: of the first MOST_PLACES,
// each whose folder is a whole path and whose words are plain.
function listable(places) {
    return (Array.isArray(places) ? places : []).slice(0, MOST_PLACES).filter(one =>
        one !== null && typeof one === "object" && plain(one.folder)
        && one.folder.startsWith("/") && plain(one.name));
}

// What the agents daemon's `answer` to the places ask numbered `asked`
// changes, the last asked being `latest` and why the last went unlisted
// `unlisted` (Agents.qml's askPlaces): null for an answer to an earlier
// ask, come after a later one's; else `{ places, unlisted, journal }`,
// `places` the new list or null to keep the last, and `journal` why to
// journal, said once as each new why comes, or null.
function heard(latest, asked, unlisted, answer) {
    if (asked !== latest) {
        return null;
    }
    if (Array.isArray(answer?.places)) {
        return { places: listable(answer.places), unlisted: "", journal: null };
    }
    const why = answer === null || answer === undefined ? NO_ANSWER
        : String(answer?.error?.message ?? "").trim() || NO_WHY;
    return { places: null, unlisted: why, journal: why === unlisted ? null : why };
}

// Whether the places `now` differ from those listed `before`, so a list
// landing while Joel is inside Start New Work draws its rows again
// (Agents.qml's askPlaces) and one the same as the last costs nothing.
function differs(before, now) {
    return JSON.stringify(before) !== JSON.stringify(now);
}

// The checkout of the node's `checkouts` a `folder` is in, the innermost,
// or null.
function checkoutOf(checkouts, folder) {
    let found = null;
    for (const one of checkouts) {
        if (folder.startsWith(`${one.folder}/`)
            && (found === null || one.folder.length > found.folder.length)) {
            found = one;
        }
    }
    return found;
}

// The rows' places, `{ folder, name, start }`, each start once (a row's
// act is keyed by it, Agents.qml's newWorkAct), so each folder once, and a
// quest's first: each of the `quests`' folders ({ folder, name }), then
// each of the node's `places`, as listable() left them ({ folder, name,
// kind, out }), none read again here. `start` is what a Sprout names as
// its folder: a zone's name (one the node calls a zone, named as one, the
// first it names at a folder), a quest's folder that is a zone's included,
// since one not checked out has no folder here yet and the node starts it
// by its name; else the folder, as for a checked-out zone whose name two
// checkouts share, which its name alone wouldn't say, so its row is named
// by where it is, its checkout's name then the rest of its folder. A zone
// not checked out that two checkouts share is one row: the node starts
// either by the one name. A zone not checked out whose name isn't one the
// node starts by is left out: its folder isn't there yet, so the node
// would refuse it.
function wheres(quests, places) {
    const named = Array.isArray(places) ? places : [];
    const zones = new Map();
    const folders = new Map();
    for (const one of named) {
        if (one.kind === ZONE && ZONE_NAMED.test(one.name) && !zones.has(one.folder)) {
            zones.set(one.folder, one);
            folders.set(one.name, (folders.get(one.name) ?? new Set()).add(one.folder));
        }
    }
    const checkouts = named.filter(one => one.kind === CHECKOUT);
    const shared = zone => zone.out !== true && folders.get(zone.name).size > 1;
    const listed = [];
    const seen = new Set();
    const rowAt = (folder, name) => {
        const zone = zones.get(folder);
        if (zone === undefined) {
            return { folder, name, start: folder };
        }
        if (shared(zone)) {
            return { folder, name: wherever(checkouts, folder), start: folder };
        }
        return { folder, name, start: zone.name };
    };
    const add = (folder, name) => {
        const row = rowAt(folder, name);
        if (!seen.has(row.start)) {
            seen.add(row.start);
            listed.push(row);
        }
    };
    for (const one of Array.isArray(quests) ? quests : []) {
        add(one.folder, one.name);
    }
    for (const one of named) {
        if (!(one.kind === ZONE && one.out === true && !zones.has(one.folder))) {
            add(one.folder, one.name);
        }
    }
    return listed;
}

// A `folder`'s name by where it is: its checkout's name, of `checkouts`,
// then the rest of the folder, or the folder whole where none holds it.
function wherever(checkouts, folder) {
    const checkout = checkoutOf(checkouts, folder);
    return checkout === null ? folder : checkout.name + folder.slice(checkout.folder.length);
}
