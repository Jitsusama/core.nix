// What Recent holds, the one meaning it has everywhere in the launcher: the
// last nine things Joel did with a thing, newest first, each once, leaving
// out what's open now, listed above it, and anything else already shown
// beside it (RSCH-20261030-GB56HX's verdict). Each kind says only what
// "did with it" means; a program's, a person's and Capture's Recent are
// all laid out by this. Apart from the views, so tests/recent.qml can hold
// it in Qt's own engine.
.pragma library

// Nine, so 1 to 9 pick one, and since the last ten things held most of what
// people went back to (Tauscher and Greenberg).
const MOST = 9;

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
    // Array.prototype.sort is stable, so things done at the same moment keep
    // their sources' order.
    dated.sort((a, b) => b.at - a.at);
    const rows = [];
    for (const one of dated.concat(undated)) {
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

// How many characters Recent's note may take beside its name and side on
// the launcher's card, where a provider can't measure the row it makes.
const ROOM = 40;

// What Recent's own row says of what it holds, given their names, newest
// first, whether a source it waits on hasn't answered yet, its words, and
// the room its note has: at its side how many, its words' `none` while it
// holds nothing, and an ellipsis while it's still waiting, so the row
// stands in its place from the first and never arrives late at the
// bottom; as its note, the names, as many whole as fit, then how many
// more by its words' `more` ("Wikipedia, Kayak", "Wikipedia and 8
// more"; VKCF3Z, 2026-11-07). The first is always named, cut by the row
// where it's longer than the room.
function said(names, waiting, words, room) {
    const count = names.length;
    if (waiting && count === 0) {
        return { side: "\u2026", note: "" };
    }
    if (count === 0) {
        return { side: words?.none ?? "", note: "" };
    }
    return { side: String(count), note: noted(names, words, room ?? ROOM) };
}

// The names that fit in `room`, whole, and how many more were left out.
function noted(names, words, room) {
    const more = left => ` ${(words?.more ?? "and {more} more").replace("{more}", left)}`;
    let note = names[0];
    for (let at = 1; at < names.length; at++) {
        const next = `${note}, ${names[at]}`;
        const last = at === names.length - 1;
        if (next.length + (last ? 0 : more(names.length - at - 1).length) > room) {
            return `${note}${more(names.length - at)}`;
        }
        note = next;
    }
    return note;
}
