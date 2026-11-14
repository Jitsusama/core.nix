.pragma library
.import "Order.js" as Order

// When an open launcher next refreshes a source that said it changed. The
// first change is taken at once, and those after it within the pace
// together, since each refresh joins the whole corpus again and searches
// what's shown: 86 of Slack's answers, one a tenth of a second just after a
// restart, held the open launcher for a minute (2026-10-30, 19:14).
//
// How long a change waits is sized by what the last refreshes cost: three
// times the median of the last three, so the shell spends about a third
// of its time refreshing at most, and never more than the pace, `every`.
// A quarter second after every refresh held Joel's windows changes up to
// 190 ms behind other sources' refreshes of a few milliseconds while his
// opening woke them (2026-11-12, 10:16); a dear refresh, as Slack's
// answers were, still waits the pace. Five times the last one alone held
// sound's changes, which come from no key, 58 ms after refreshes of 12,
// and 255 after a single dear one (Y2YBTS, 2026-11-12, c1591663), so one
// stall no longer holds the next change, though two of three do.
//
// A pace is when the last refresh began, `at`, what the last three cost,
// `costs`, oldest first, their median `cost`, undefined until one is
// known, and when a paced one is due, `due`, or null when none waits. The
// launcher's timer rings at `due`.

const share = 3;
const costsKept = 3;
// Two refreshes within one frame would draw only the second.
const aFrame = 16;

// A launcher that hasn't refreshed yet.
function rested() {
    return { at: 0, due: null, cost: undefined, costs: [] };
}

// The median of these costs, undefined while there are none.
function median(costs) {
    if (costs.length === 0) {
        return undefined;
    }
    const sorted = costs.slice().sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

// How long after the last refresh began the next may, by its cost.
function apart(cost, every) {
    return cost === undefined ? every : Math.min(every, Math.max(aFrame, cost * share));
}

// A source has changed at `now`: whether it's refreshed at once, and the
// pace after it, holding when the waiting refresh is due.
function heard(pace, now, every) {
    if (pace.due !== null) {
        return { pace, now: false };
    }
    const due = pace.at + apart(pace.cost, every);
    if (due <= now) {
        return { pace, now: true };
    }
    return { pace: { at: pace.at, due, cost: pace.cost, costs: pace.costs }, now: false };
}

// A refresh began at `now`, however it was asked for, so it takes the one
// that waited with it: a key's own change or a daemon's answer refreshed
// at once, or the timer rang late and another change past the pace came
// beside it, and a refresh left due would follow within the pace. It keeps
// the last `pace`'s costs until its own is known, as a source may change
// from within it.
function took(now, pace) {
    return pace === undefined ? { at: now, due: null, cost: undefined, costs: [] }
        : { at: now, due: null, cost: pace.cost, costs: pace.costs };
}

// The refresh that began at the pace's `at` cost `ms` of the shell's work.
function costed(pace, ms) {
    const costs = pace.costs.concat([ms]).slice(-costsKept);
    return { at: pace.at, due: pace.due, cost: median(costs), costs };
}

// Whether a change heard `ago` ms after the last key is that key's own,
// refreshed at once on its frame: only after a key that acted, within
// `within`. A key that only `moved` changed nothing, so a change beside it,
// as Now's each second, waits on the pace like any other (Joel,
// 2026-11-08: "weird lag can happen"). Nor is a change to a place none of
// whose rows are on screen as it comes (`shown`, a held step's or those
// listed), which nothing needs in the key's frame: Runs changing just
// after a key in Emoji was timed as its act, and its refresh, drawing
// nothing, waited 631 ms for a frame drawn for something else (Joel's
// journal, 2026-11-11). A place a key brings into view, as Backspace to
// home brings Windows', is on screen by the time its change is heard.
function keyed(ago, moved, within, shown) {
    return shown && !moved && ago < within;
}

// How long after the last key's work, taken at `taken` and done at `done`,
// a change came at `now`: none while the work is under way, as the act it
// does changes a source from within it. Counted from the key's press
// instead, a handler a loaded machine slowed past `within` before its act
// left the change it made to the pace, and dd on a notice never said it was
// shown (draws-still at 550a45788, cd92d7660 and 2101c92b6, 2026-11-09).
function sinceKey(now, taken, done) {
    return done < taken ? 0 : now - done;
}

// The timer rang: the refresh it waited for is taken.
function rang(pace) {
    return { at: pace.at, due: null, cost: pace.cost, costs: pace.costs };
}

// A key's acts on their way to the screen, each shown by the first frame
// begun after its refresh: `waiting`, refreshed with no frame begun since,
// and `framing`, in a frame begun after it, so a swap of one already under
// way when an act's refresh ran isn't taken for it.
function unframed() {
    return { waiting: [], framing: [] };
}

// The acts whose refresh has just run wait for the next frame.
function refreshedActs(acts, refreshed) {
    return { waiting: acts.waiting.concat(refreshed), framing: acts.framing };
}

// A frame began, so it draws every act that waited.
function frameBegun(acts) {
    return { waiting: [], framing: acts.framing.concat(acts.waiting) };
}

// A frame was swapped onto the screen: the acts it drew are `shown`, and
// those refreshed while it was drawn wait on for the next. Held back with
// them, as one flag for every act once held them, dd on a notice was never
// said shown: a source no row showed changed just after it, as its own
// refresh's frame began, was taken for the key's, and its refresh drew no
// frame to follow (draws-still at 7b2252da0, 2026-11-10).
function frameSwapped(acts) {
    return { shown: acts.framing, acts: { waiting: acts.waiting, framing: [] } };
}

// A drawer coming the frame after its key's waited one whole refresh for
// its swap, niri's callback for the key's frame: 16 ms of every drawer's
// median of 69 from the key, against Joel's 50 (2026-11-12), where its
// making took a median of 2. So one whose kind made in under 3 ms of late
// comes in the key's own frame, decided from the makings kept before it's
// made, never by timing it there; a kind not yet made, or dearer, comes
// the frame after, as does any while a key is held, so the highlight never
// waits on it (VKCF3Z, 2026-11-12, d4c660a6). `makings` holds each kind's
// last eight, in ms, and is never changed.
const drawerMakingsKept = 8;
const drawerJoinsUnder = 3;

function drawerMade(makings, kind, making) {
    const kept = Object.assign({}, makings);
    kept[kind] = (makings[kind] ?? []).concat([making]).slice(-drawerMakingsKept);
    return kept;
}

// What a drawer's makings are kept under: its place and what its row is,
// by its key's start, since a place holds rows of many kinds. Kept by place
// alone, home's people and commands, made in a millisecond, brought a
// program's into its key's frame, Slack's after 49 ms making, and 7 of
// the 15 that came there that day made in over 3 ms (2026-11-12).
function drawerKind(place, row) {
    const key = String(row?.key ?? "");
    const at = key.indexOf(":");
    return at > 0 ? `${place} ${key.slice(0, at)}` : place;
}

function drawerJoinsKey(makings, kind, held) {
    const made = makings[kind] ?? [];
    if (held || made.length === 0) {
        return false;
    }
    return median(made) < drawerJoinsUnder;
}

// How long letters rest before a drawer one of them brought is made, in
// ms: longer than the 100 to 150 ms Joel types letters apart, since each
// letter that moved the selection had its drawer made right after its
// frame and the next letter waited behind it, "calend" 272 ms behind
// Google Calendar's first (2026-11-13, 06:14). Each letter restarts it.
const typingRests = 160;

// The rest itself, for the timer that waits it out (Launcher.qml).
function lettersRest() {
    return typingRests;
}

// How long the selection rests on a row, in ms, before what a place there
// holds watches its daemon ahead of a step in (Seen.js): as long as letters
// rest, so an arrow tapped past a place, 100 to 150 ms apart, or held,
// starts and stops no watch.
function selectionRest() {
    return typingRests;
}

// Whether the letters are still coming, the last typed at `typedAt` on
// KeyTimes's clock (0 when an arrow has moved since), as `now` reads it.
function lettersComing(typedAt, now) {
    return typedAt > 0 && now - typedAt < typingRests;
}

// Which frame a drawer came in, as its journal line says it, so the lines
// show whether coming in the key's frame pays, and how long it rested for
// the letters first, apart from its making.
function drawerCame(joined, restedFor) {
    if (restedFor > 0) {
        return `after resting ${Math.round(restedFor)} ms for the letters`;
    }
    return joined ? "in the key's frame" : "the frame after";
}

// The room the foot holds for Return's widest verb, as `kept` weighed it
// for these words in this face, or weighed again by `weigh` once either
// has changed: the words are the file as it was read, and the face what
// the foot measured its keys in, each a new object when it changes. On
// 10 November every opening walked 1,803 things' verbs again, its slowest
// part after the first.
function verbRoom(kept, words, face, weigh) {
    if (kept !== null && kept.words === words && kept.face === face) {
        return kept;
    }
    return { words, face, widest: weigh(words) };
}

// The rows' keys as the foot weighed them, kept for what's typed, the steps
// and the mode, `key`, in these words and the face the foot measured them
// in: a row is the same object for as long as it says the same, so one
// weighed before says the same keys at the same widths. Weighed afresh each
// time, one act in For You's 115 rows weighed every row again, 7 to 46 ms of
// the 11 to 52 its rows took to be shown, where one row had changed
// (2026-11-10). Each weighing holds only the rows the one before it saw
// (`seen`, carried as `rows`), so a row replaced goes at the next: home's
// key stays the same across every opening, and a Map of every row it
// ever weighed held each replaced row for the life of the shell (Y2YBTS,
// 2026-11-11). Not a WeakMap, which crashed the shell once its rows were
// collected.
function weighing(kept, key, words, face) {
    const same = kept !== null && kept.key === key && kept.words === words && kept.face === face;
    return { key, words, face, rows: same ? kept.seen : new Map(), seen: new Map(),
        weighed: same ? kept.weighed : 0 };
}

// How many places' weighings are held at once, the one shown included.
const placesWeighed = 4;

// The weighings `held` for what was shown last, newest first, with the one
// for `key` made from its own: going home and back weighed every one of
// Windows' rows again on each kept opening, 3.5 to 4.8 ms of it over 40
// windows on a builder (ZINAWZ, 2026-11-11). Only a place as it's shown
// (`place`) is held past the next key, at most `placesWeighed` of them,
// never a typed search, which a reopening never shows again (Y2YBTS,
// 2026-11-11). A row that changed is another object, so it's weighed
// again, and a held place lets go of rows it no longer lists as `weighing`
// does. `measure` says the words and the face the foot measured in.
function weighingAmong(held, key, measure, place) {
    const before = held.find(one => one.key === key) ?? null;
    const now = Object.assign(weighing(before, key, measure.words, measure.face), { place });
    const rest = held.filter(one => one.key !== key && one.place);
    return [now].concat(rest.slice(0, placesWeighed - (place ? 1 : 0)));
}

// The rows the foot weighs keys for, of `rows` listed: each, but only the
// first of a run that says its keys are alike by the word they share
// (`keysAlike`), since the rest weigh the same, and the `selected`, which
// says more. The room is then what every row would give, so it never
// widens as the selection walks them. Each of Emoji's 1,924 rows weighed as
// it opened took 129 to 202 ms of its 245 to 373 ms opening on a builder
// (2026-11-11).
function toWeigh(rows, selected) {
    return weighedWith(rows, weighedAt(rows), selected);
}

// Where in `rows` the rows weighed with no selection stand. Only the row
// before says whether one is, so a selection adds itself and moves none,
// and a list shown again keeps these whatever is selected in it.
function weighedAt(rows) {
    const at = [];
    let alike = "";
    rows.forEach((row, place) => {
        const like = row.keysAlike ?? "";
        if (like === "" || like !== alike) {
            at.push(place);
        }
        alike = like;
    });
    return at;
}

// The rows standing `at` in `rows`, and the `selected` in its place among
// them where it's listed.
function weighedWith(rows, at, selected) {
    const chosen = selected ? rows.indexOf(selected) : -1;
    const places = chosen < 0 || at.includes(chosen) ? at
        : at.filter(place => place < chosen).concat([chosen], at.filter(place => place > chosen));
    return places.map(place => rows[place]);
}

// A row's keys and widths, as `kept` holds them or as `weigh` weighs them
// now, kept for the next time.
function weighedRow(kept, row, weigh) {
    const known = kept.seen.get(row) ?? kept.rows.get(row);
    if (known !== undefined) {
        kept.seen.set(row, known);
        return known;
    }
    const made = weigh(row);
    kept.seen.set(row, made);
    kept.weighed += 1;
    return made;
}

// Keeps a row weighed before for the next weighing, as the selected one is
// while it's weighed afresh, so it isn't weighed again once it's left.
function keptRow(kept, row) {
    const known = kept.rows.get(row);
    if (known !== undefined) {
        kept.seen.set(row, known);
    }
}

// What an opening's own work went on, from each part's milliseconds by its
// name, said slowest first, since the slowest is what its line is read for,
// and parts as slow A to Z. A part under a millisecond is left out, and an
// opening with none left says so.
function openingParts(parts) {
    const said = Order.sorted(Object.keys(parts)
        .map(name => ({ name, ms: Math.round(parts[name]) }))
        .filter(part => part.ms >= 1),
        Order.then(Order.larger(part => part.ms), Order.byName(part => part.name)));
    if (said.length === 0) {
        return "none over a millisecond";
    }
    return said.map(part => `${part.name} ${part.ms}`).join(", ");
}

// When an opening became usable, from a timer ticking every `every` ms
// after its first frame: the start of the first `calmFor` ms in which no
// tick came more than `within` ms late, so the shell's thread was free to
// draw what niri asked and to answer a key. Its first frame alone wasn't
// what Joel saw: his openings logged 437 ms to their frame and showed him
// the launcher about 7 s after his chord, when the thread came free (Joel,
// 2026-11-10: "It takes 7 seconds for me to see the launcher window").
// `stall` is the longest gap between ticks, begun at `stallAt`, and
// `usable` is -1 until then.
function feltBegun(now) {
    return { last: now, calm: now, stall: 0, stallAt: now, usable: -1 };
}

function feltTicked(felt, now, pace) {
    if (felt.usable >= 0) {
        return felt;
    }
    const gap = now - felt.last;
    const calm = gap - pace.every > pace.within ? now : felt.calm;
    const longer = gap > felt.stall;
    return { last: now, calm, stall: longer ? gap : felt.stall,
        stallAt: longer ? felt.last : felt.stallAt,
        usable: now - calm >= pace.calmFor ? calm : -1 };
}

// Each frame's own work while an opening is felt, as spans for
// stallSpent. Its rows are laid out as ListView polishes it, from the
// first row begun to the last made. Its sync runs from its animating,
// inside Qt's polishAndSync, until the shell's thread runs again: the
// wait for the render thread to take the frame, the sync, the animations
// advanced and incubation. What the thread runs after that is its own;
// Joel's openings named all of it drawing, 23 ms of a stall, which
// couldn't say what to cut (2026-11-14). `what` is "row", from `from` to
// `to`, or "animating" or "synced" at `from`.
function feltUnframed() {
    return { rows: null, syncFrom: -1 };
}

function feltFramed(frame, what, from, to) {
    if (what === "row") {
        const rows = frame.rows === null ? { from, to } : { from: frame.rows.from, to };
        return { frame: { rows, syncFrom: frame.syncFrom }, spans: [] };
    }
    if (what === "animating") {
        const spans = frame.rows === null ? []
            : [{ name: "laying out", from: frame.rows.from, to: frame.rows.to }];
        return { frame: { rows: null, syncFrom: from }, spans };
    }
    const spans = frame.syncFrom < 0 ? [] : [{ name: "syncing", from: frame.syncFrom, to: from }];
    return { frame: { rows: frame.rows, syncFrom: -1 }, spans };
}

// What of `from` to `to` each named span of work ran into, most first,
// then what none did: Joel's kept openings drew in a median 48 ms and
// then stalled a median 73 ms, over a fill, a frame and more the journal
// didn't name (2026-11-13). Each span is `{ name, from, to }`; where one
// runs inside another, the one begun last has the time, as it's the work
// then running, so none is counted twice.
function stallSpent(spans, from, to) {
    const edges = [from, to];
    for (const span of spans) {
        if (span.to > from && span.from < to) {
            edges.push(Math.max(span.from, from), Math.min(span.to, to));
        }
    }
    edges.sort((a, b) => a - b);
    const spent = new Map();
    let unnamed = 0;
    // Two edges at one time hold no time between them.
    const segments = edges.slice(0, -1).map((start, at) => [start, edges[at + 1]])
        .filter(([start, end]) => end > start);
    for (const [start, end] of segments) {
        const running = spans.filter(span => span.from <= start && span.to >= end)
            .reduce((last, span) => last === null || span.from >= last.from ? span : last, null);
        if (running === null) {
            unnamed += end - start;
        } else {
            spent.set(running.name, (spent.get(running.name) ?? 0) + end - start);
        }
    }
    // Ties go to the one that ran first, as Qt's sort may not keep order.
    const said = [...spent].map(([name, ms], first) => [name, Math.round(ms), first])
        .filter(([, ms]) => ms > 0).sort((a, b) => b[1] - a[1] || a[2] - b[2])
        .map(([name, ms]) => `${name} ${ms}`);
    if (Math.round(unnamed) > 0) {
        said.push(`${Math.round(unnamed)} unnamed`);
    }
    return said.join(", ");
}
