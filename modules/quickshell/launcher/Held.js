// How the steps the launcher holds, each a row stepped into and the rows
// it offers, are made again as what they were made from changes: a place's
// members coming, a session's verbs as it starts working. Apart from the
// launcher, so tests/held.qml can hold it in Qt's own engine.
//
// Each takes `ways`, the launcher's own answers: `same(row, was)` whether a
// row offered now is the one a step holds, `into(thing, at, steps)` the
// rows a thing stepped into at `at` offers now, or null for none, as
// `offers` decides, `keep(step, into)` those rows kept in the order the
// step shows them, and `looked()` all a source searches (own's foundNow).
// Each gives `steps` made again, `leave`, the first step to leave or -1,
// and `all`, whether every step was made again; leaveTo says where that
// leave goes.
.pragma library

// Each step from `from` on made again from the rows the one above it
// offers now, `rows` for the first. A step entered from what was typed is
// made again from its own row, since what was typed found it, not the rows
// above. A step whose row is gone is kept as it was, dimmed where the list
// above shows it gone; one whose row is still offered but offers nothing
// now is left, as a session's verbs once it has ended, rather than kept
// showing what it offered before. A typed step whose own row offers
// nothing keeps its rows, each gone, as no row offered now says it's left.
function remade(steps, from, rows, ways) {
    const made = steps.slice();
    let offered = rows;
    for (let at = from; at < made.length; at++) {
        const step = made[at];
        // A row the step above keeps as gone, Steady.kept's copy marked
        // `went`, is the row gone; one its source serves dimmed, as an act
        // done with, is still the row.
        const now = offered.find(row => !row.went && ways.same(row, step.row));
        const thing = now ?? ((step.typed ?? "") !== "" ? step.row : undefined);
        const into = thing === undefined ? null : ways.into(thing, at, made);
        if (!into && (now !== undefined || thing === undefined)) {
            return { steps: made, leave: now === undefined ? -1 : at, all: false };
        }
        made[at] = Object.assign({}, step, { row: thing, rows: ways.keep(step, into ?? []) });
        offered = into ?? [];
    }
    return { steps: made, leave: -1, all: true };
}

// What a row offers a step into, from `rows`, what's inside it, or null
// for nothing: no rows is nothing, unless its vocabulary names `places`
// (Agents with no agent running still leads to its Quests), or it says
// what it holds when `empty`, as Waiting on You does. The launcher's step
// in and its held steps both ask this, so one is left exactly where the
// other is refused.
function offers(rows, places, empty) {
    if ((!rows || rows.length === 0) && places === 0 && !empty) {
        return null;
    }
    return rows ?? [];
}

// The step at `at`, stepped into from a source's own row, made again with
// that row as the source offers it now, among `offered`, everything it
// offers: kept as entered, its crumb and drawer said what it said then. It
// is the source's row as it is now, with only what the launcher gave it
// when found kept over it: `foundAs`, its side, the verbs it found and the
// rest, kept with it for the next time, or else `from`, the source that
// gave it, which the launcher finds the step by. What the search found,
// and where it lands, are each as the source offers them now (foundNow).
// It's never said in `given`, a row's way of asking for words. Nothing
// else the row held then survives, so a field the source no longer offers
// is gone. A row gone is kept, its rows made from it as it was, each gone
// where it now offers nothing, as a collection's own row, never among what
// it offers, is when its last row leaves, though what its search found is
// still each as offered now; one still offered that offers nothing now is
// left. `ways.looked` gives all the source searches, read only when a
// match isn't among `offered`. A row with no key, the step a place opened
// straight holds (Launcher.qml's placeStep), is the place itself, never
// one of its members, which the launcher's same would tell it from only by
// name: a session titled as the place's noun would take it over.
function own(steps, at, offered, ways) {
    const made = steps.slice();
    const step = made[at];
    const now = (step.row.key ?? "") === "" ? undefined
        : offered.find(one => ways.same(one, step.row));
    const foundAs = step.row.foundAs === undefined ? undefined
        : foundNow(step.row.foundAs, offered, ways);
    const kept = foundAs !== undefined ? Object.assign({}, foundAs, { foundAs })
        : step.row.from === undefined ? {} : { from: step.row.from };
    const row = Object.assign({}, now ?? step.row, kept);
    const into = ways.into(row, at, made);
    if (!into && now !== undefined) {
        return { steps: made, leave: at, all: false };
    }
    made[at] = Object.assign({}, step, { row, rows: ways.keep(step, into ?? []) });
    return { steps: made, leave: -1, all: true };
}

// What the launcher gave a row a search found, with each row the search
// found, and the one it lands in, as the source offers it now among
// `offered`, or else among all it searches, `ways.looked`, as Sound's
// found by ear are: an ask hidden as a share begins is then listed hidden,
// never as it was found. A match ranked flat, as Agents' search gives its
// sessions, is ranked as it stands, but only where ranking gives the same
// row: a copy under a key of its own, as For You's want row copies an
// Agents session, would rank to the session it was copied from. One found
// in neither is kept as found, never dropped, since a source can find past
// both: Notices past its page, People with a search Joel has typed past,
// or Agents inside a fold.
function foundNow(foundAs, offered, ways) {
    let lookedNow = null;
    const inLooked = was => {
        lookedNow = lookedNow ?? ways.looked?.() ?? [];
        return lookedNow.find(one => ways.same(one, was));
    };
    const asNow = was => offered.find(one => ways.same(one, was)) ?? inLooked(was);
    const ranked = one => {
        const flat = typeof one.ranked === "function" ? one.ranked() : one;
        return flat?.key === one.key ? flat : one;
    };
    const found = typeof foundAs.verbs !== "function" ? null
        : foundAs.verbs().map(was => { const now = asNow(was);
            return now === undefined ? was : ranked(now); });
    return Object.assign({}, foundAs, found === null ? {} : { verbs: () => found },
        foundAs.landsIn === undefined ? {} : { landsIn: asNow(foundAs.landsIn)
            ?? foundAs.landsIn });
}

// Where the launcher goes once `made` (remade's or own's answer) says a
// step offers nothing now: `keep`, how many steps it keeps, back to the
// step above the one to leave, or -1 to leave none, with `steps` as
// they're shown. A thing landed in from a search, `through`, is left back
// to the search, past the collection it was found in, as Joel's - leaves
// it, but never past a dialog's own collection, `how.outermost`, for the
// home it never showed. That collection is itself never left: it's a
// place's own step, with no key, which own keeps with its rows each gone.
function leaveTo(made, how) {
    const steps = made.steps.slice();
    if (made.leave < 0) {
        return { steps, keep: -1 };
    }
    const left = made.leave > 0 && steps[made.leave].through === true ? made.leave - 1
        : made.leave;
    return { steps, keep: how.outermost && left < made.leave ? Math.max(left, 1) : left };
}

// What every history forgets once `made` says a step offers nothing now,
// for afterLeave, or null where none does: `left`, the places through
// that step, whether it's left or left past the collection a search
// found it in; and `landed`, where Joel is once `leaving` (leaveTo's
// answer) has gone, whose last places go. Each
// is read as `held` read it before the remaking and as `steps` read it
// after, since a row remade as offered now may be renamed so. Only the
// spent step's places go, never its collection's, whose other rows still
// offer what they did.
function forgets(made, leaving, held, steps) {
    if (made.leave < 0) {
        return null;
    }
    const through = made.leave + 1;
    return {
        landed: [held.slice(0, leaving.keep), steps.slice(0, leaving.keep)],
        left: [held.slice(0, through), steps.slice(0, through)]
    };
}

// How many keys Joel had pressed when a step left by itself landed on a
// row that isn't `from`, the step's own record of the row he stepped in
// from (its key, or "" for none), or -1 where it landed on that row by
// its key. `landed` is the key of the row selected once it landed, or
// "" for none. A row gone from the rows the leave searched again, as an
// ask a share hides, leaves the selection where it stood, on a row Joel
// never chose (returnUnder).
function guessedAt(from, landed, keys) {
    const key = from?.key ?? "";
    return key !== "" && key === landed ? -1 : keys;
}

// A landing a step that left by itself had to guess: the keys the field
// had taken then (`keys`, or -1 for none guessed), when a Return was
// last held under it (`at`, or 0), how deep the launcher stood once it
// landed (`depth`, or -1 for any), and whether the leave threw partway
// (`broken`), its selection half made.
function unguessed() {
    return { keys: -1, at: 0, depth: -1, broken: false };
}

// A guess made before the field's key `keys` and not yet answered by a
// key: the one a leave keeps, as a second leave with no key between
// leaves the Return still meant for the first step.
function pending(guess, keys) {
    return guess.keys >= 0 && guess.keys === keys && guess.at === 0;
}

// The guess that stands while a leave runs, at the field's key `keys`,
// in a launcher `open` or not: broken until the leave says where it
// landed, so one that throws partway holds the next Return too.
function leaving(keys, open) {
    return open ? { keys, at: 0, depth: -1, broken: true } : unguessed();
}

// The guess once a leave is done, from `before`, the guess as it stood
// before the leave; `from`, the step's record of the row Joel stepped in
// from; `landed`, the key of the row selected once it landed ("" for
// none, null where nothing was left); and `now`, the field's `keys`,
// whether the launcher is `open`, and its `depth`. A landing by its key
// keeps only a guess still pending from an earlier leave.
function landedAfter(before, from, landed, now) {
    const kept = pending(before, now.keys);
    if (!now.open) {
        return unguessed();
    }
    if (landed !== null && guessedAt(from, landed, now.keys) >= 0) {
        return { keys: now.keys, at: 0, depth: now.depth, broken: false };
    }
    return kept ? Object.assign({}, before, { depth: now.depth, broken: false }) : unguessed();
}

// A key with no name, as Shift or Ctrl pressed alone, chose nothing, so
// the Return after it is still the first key after the leave.
function passedOver(guess, name) {
    return name === "" && guess.keys >= 0
        ? Object.assign({}, guess, { keys: guess.keys + 1 }) : guess;
}

// Whether a Return holds still, doing nothing, under `guess`, and what's
// guessed after it. `now` is the field's keys, this Return counted, as
// it counts a key before it acts; the time in ms; the launcher's depth;
// and whether words are asked for. Only the first key after the leave
// holds, at the depth it landed, so a second Return meant for the step
// that left, as an ask's Release It, never acts on the row the selection
// stands on instead; and the Return straight after a held one, within
// `wait` ms, holds once more, as that Return doubled, a key bounced or
// pressed twice for one. After a leave that threw, every Return holds
// until another key. Any other key first, as a move or a word typed,
// lets the next Return act, and words asked for are given, ending it.
function returnUnder(guess, now, wait) {
    const holds = !now.asking && guess.keys >= 0 && now.keys === guess.keys + 1
        && (guess.depth < 0 || now.depth === guess.depth)
        && (guess.at === 0 || guess.broken || now.at - guess.at < wait);
    return {
        holds,
        next: holds && (guess.at === 0 || guess.broken)
            ? Object.assign({}, guess, { keys: now.keys, at: now.at }) : unguessed()
    };
}

// Where a list of steps goes, the one way the launcher tells places
// apart, for its histories as they're kept and as they forget: a row by
// its key, as the launcher's sameRow matches a row remade, so a refresh
// renaming a held row is no step to a new place; and a row with none,
// as an answer or a place's own step, by what it is and its name
// together, since every answer is the same "answer". An answer's name is
// the figure it says now, which each re-answer changes, so it's named by
// the words asked (its chip), which again() keeps.
function placeOf(steps) {
    return steps.map(step => {
        const row = step.row ?? {};
        const name = row.is === "answer" ? row.chip ?? row.name : row.name;
        return (row.key ?? "") !== "" ? `key\u0000${row.key}`
            : `is\u0000${row.is ?? ""}\u0000${name ?? ""}`;
    }).join("\u0001");
}

// What the launcher's remaking answers its caller, from `made`: "made"
// when every step was made again, "kept" when one was kept as it was,
// and "left" when steps were left.
function answer(made) {
    if (made.leave < 0) {
        return made.all ? "made" : "kept";
    }
    return "left";
}

// A throw caught as the launcher refreshes a source, as one journal line:
// its kind and where it threw first, never its message, which could
// carry what a source served, nor every frame, which would bury the
// journal. The kind always ends "Error:", as the VM tests' error nets
// look for, a thrown value that isn't an Error, or one named otherwise,
// said as "Error"; and a first line not shaped wholly as a frame, as
// another engine's "Error: message" is, is left out, so no message can
// come in through it.
const errorKind = /^[A-Za-z]*Error$/;
const frameLine = /^[^\s@]*@[^\s@]+:\d+(:\d+)?$/;
function thrown(error) {
    const real = error instanceof Error;
    const kind = real && errorKind.test(`${error.name}`) ? error.name : "Error";
    const first = real ? `${error.stack ?? ""}`.split("\n")[0] : "";
    const where = frameLine.test(first) ? ` at ${first}` : "";
    return `${kind}: ${real ? "" : `${typeof error} `}thrown${where}`;
}

// What each history keeps once a step left by itself has landed where
// Joel stepped in from, which is now where he is: the way back
// (`went.jumps`, Ctrl+O), the places ahead (`went.ahead`, Ctrl+I), the
// tags (`went.tags`, Ctrl+T), each a list of places with their steps
// in `held`, and the place used last (`went.last`, Tab, or null). Every
// place through the step left goes, wherever it is, since each would
// show that spent step's rows as live; then each list's last places
// where he landed go, as the place used last does there, since each
// would go nowhere. `at.landed` and `at.left` hold the steps of where
// he is and of the step left, as forgets gives them; `where` says
// where a list of steps goes, as placeOf above does.
function afterLeave(went, at, where) {
    const landed = at.landed.map(where);
    const left = at.left.map(steps => ({ depth: steps.length, place: where(steps) }));
    const through = one => left.some(step =>
        one.held.length >= step.depth && where(one.held.slice(0, step.depth)) === step.place);
    // Home is no place a leave strands an entry at: going back to it
    // brings back what was typed there, which may be another search.
    const there = one => one.held.length > 0 && landed.includes(where(one.held));
    const kept = places => {
        const away = places.filter(one => !through(one));
        let end = away.length;
        while (end > 0 && there(away[end - 1])) {
            end -= 1;
        }
        return away.slice(0, end);
    };
    const last = went.last;
    return {
        jumps: kept(went.jumps),
        ahead: kept(went.ahead),
        tags: kept(went.tags),
        last: last === null || through(last) || there(last) ? null : last
    };
}
