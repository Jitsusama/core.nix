// What Recent holds, the one meaning it has everywhere in the launcher: the
// last nine things Joel did with a thing, newest first, each once, leaving
// out what's open now, listed above it, and anything else already shown
// beside it (RSCH-20261030-GB56HX's verdict). Each kind says only what
// "did with it" means; a program's, a person's and Capture's Recent are
// all laid out by this. Apart from the views, so tests/recent.qml can hold
// it in Qt's own engine.
.pragma library
.import "Order.js" as Order

// Nine, so 1 to 9 pick one, and since the last ten things held most of what
// people went back to (Tauscher and Greenberg).
const MOST = 9;

// Between items said in one place, as every note and side joins them.
const BETWEEN = " \u00b7 ";

// What Recent lists, from what each of its sources kept, each a list of
// { key, at, row } in that source's own order, newest first: `key` the
// thing's identity (a file by its path, a site by its page), `at` when
// Joel last did something with it, in milliseconds, where the source says,
// and `row` whatever the caller makes its row from, given back untouched,
// so counting what Recent holds makes no rows.
// Rows with a time merge by it, newest first; a source that gives none
// keeps its own order after them, so a list nobody dated never cuts in
// among dated ones by guesswork. A thing is listed once, where it's
// newest, and never when its key is among those `shown` already above
// it. At most `most`, nine unless said.
function listed(sources, shown, most) {
    const seen = new Set(shown ?? []);
    const dated = [];
    const undated = [];
    for (const source of sources) {
        for (const one of source ?? []) {
            (Number.isFinite(one.at) ? dated : undated).push(one);
        }
    }
    // Recent is a stream, and things done at the same moment keep their
    // sources' order.
    const newest = Order.sorted(dated, Order.stream(one => one.at));
    const rows = [];
    for (const one of newest.concat(undated)) {
        if (rows.length >= (most ?? MOST)) {
            break;
        }
        if (seen.has(one.key)) {
            continue;
        }
        seen.add(one.key);
        rows.push(one);
    }
    return rows;
}

// What Recent's own row says of what it holds, given their names, newest
// first, whether a source it waits on hasn't answered yet, and its words:
// at its side how many, its words' `none` while it holds nothing, and an
// ellipsis while it's still waiting, so the row stands in its place from
// the first and never arrives late at the bottom. Nothing after its name,
// as no row says there: it's a place, stepped into and holding rows, so
// its names, `held`, go in its drawer as a place's do (the launcher's
// `heldNames`; VKCF3Z, dae7fb67, dafc2199).
function said(names, waiting, words) {
    const count = names.length;
    if (waiting && count === 0) {
        return { side: "\u2026", held: [] };
    }
    if (count === 0) {
        return { side: words?.none ?? "", held: [] };
    }
    return { side: String(count), held: names.slice() };
}

// What a thing taken says at its right in Capture's Recent: when it was
// taken (`when`, as a file's row says it), in the same spot on every row,
// as a stream's rows say theirs, and its folder after it only where that
// isn't the one most are in (`usual`), since a folder said on every row
// says nothing (VKCF3Z, 2026-11-08). Its folder is said once instead, in
// its drawer's eyebrow (`takenEyebrow`). Where when isn't known, the
// folder is all it says.
function takenSide(when, folder, usual) {
    if (when === "") {
        return folder;
    }
    return folder === usual || folder === "" ? when : `${when}${BETWEEN}${folder}`;
}

// Whether a source's `go` or `close` reaches each of a page's `tabs` by
// the id it gave them. A tab read from Chrome's session file has none,
// since a running Chrome may give that run's ids to other tabs, and a
// managed Chrome refusing the extension gives only those (2026-11-11).
// Without one Go To could bring only the page's window forward, which its
// window's row does already, so neither it nor Close It is offered.
function reaches(command, tabs) {
    return Boolean(command) && tabs.length > 0
        && tabs.every(tab => String(tab?.id ?? "") !== "");
}

// The eyebrow over a picture taken, as its drawer takes it: the folder
// it's in, as its row no longer says, then what kind of file it is, a
// picture or a video (`kind`), "in ~/Pictures/Screenshots \u00b7 picture". `words`
// gives how the folder is said, its `in`. It's set as written, since a
// folder is a name never re-cased (VKCF3Z, 2026-11-06). Short of room,
// what it is goes first, since the folder says it too, then the folder
// gives way from its middle, "in ~/\u2026/Screenshots", and it never goes
// (VKCF3Z, 2026-11-08).
function takenEyebrow(folder, kind, words) {
    const within = folder === "" ? "" : (words?.in ?? "{folder}").replace("{folder}", folder);
    const items = [
        { said: within, keep: 2, path: folder },
        { said: kind ?? "", keep: 1 }
    ].filter(item => item.said !== "");
    return {
        eyebrow: items.map(item => item.said).join(BETWEEN),
        eyebrowItems: items,
        eyebrowNames: true
    };
}
