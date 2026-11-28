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
.import "Thrown.js" as Thrown

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

// Whether a step left by itself landed on the row Joel stepped in from,
// by its key alone: `from` is the step's own record of that row (its key,
// or "" for none) and `landed` the key of the row selected once it landed
// ("" for none). Where the selection stands says nothing, since a row
// clicked into is stepped into without being selected, so a step's record
// can stand on the headline under another row's key; a headline a
// re-answer renamed is then guessed, and holds one Return. A row gone
// from the rows the leave searched again, as an ask a share hides, leaves
// the selection where it stood, on a row Joel never chose (returnUnder).
function onItsRow(from, landed) {
    const key = from?.key ?? "";
    return key !== "" && key === landed;
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
// in a launcher `open` or not, from `before`, the guess standing: broken
// until the leave says where it landed, so one that throws partway holds
// the next Return too. A broken guess standing at most one key back is
// kept, at its own keys, so a leave that throws again on each refresh of
// a busy source never holds the Return after the one key that ended the
// first, as its foot says; a throw once Joel has pressed more is new.
function whileLeaving(keys, open, before) {
    if (!open) {
        return unguessed();
    }
    return before.broken && before.keys >= 0 && keys - before.keys <= 1
        ? before : { keys, at: 0, depth: -1, broken: true };
}

// The guess once a leave is done, from `before`, the guess as it stood
// before the leave; `from`, the step's record of the row Joel stepped in
// from; `landed`, the key of the row selected once it landed ("" for
// none, null where nothing was left); and `now`, the field's `keys`,
// whether the launcher is `open`, its `depth`, and whether what left
// was Joel's last key's act (`byKey`), as keyed says. A leave his key
// caused is one he watched, so it guesses nothing new, though a guess
// still pending from an earlier leave stands. A landing back on its row
// keeps only such a pending guess, unbroken, since this one closed
// whole.
function landedAfter(before, from, landed, now) {
    const kept = pending(before, now.keys);
    if (!now.open) {
        return unguessed();
    }
    if (!now.byKey && landed !== null && !onItsRow(from, landed)) {
        return { keys: now.keys, at: 0, depth: now.depth, broken: false };
    }
    return kept ? Object.assign({}, before, { depth: now.depth, broken: false }) : unguessed();
}

// Whether the key the clock is timed from only moved, from `word`, as it
// stood, once the prompt says it `moved` with `keys` taken (its
// keysTaken), the clock's key having been the `heard`th. The prompt
// clears its word as each key comes, before counting it, and says a move
// once the key is read, so only a move said while the clock's own key
// is the last counted is that key's: a Shift after j clears the word
// without moving the clock, and read then, j's move was forgotten and a
// share's change beside j taken for an act. A held key's repeat is
// counted once read, so its word is the prompt's as its clock moves.
function movedSaid(word, heard, keys, moved) {
    return moved && keys === heard ? true : word;
}

// What a source's change does to the acts waiting on its refresh, as the
// launcher hears it: `ago`, ms after the work of the key the clock is
// timed from (Pacing.sinceKey); whether that key only `moved`; `within`,
// keyedWithin; whether the change was to a place `shown` on screen in an
// open launcher (Pacing.keyed's own word); and whether the source was
// already `waiting` on a refresh, on the pace or a fill's find. Heard
// within `within` of a key that acted, a change is that key's, whoever
// made it: shown, it's the key's "act", or a "joined act" when what
// already waits will be taken into the same refresh; not shown, it's
// "kept", recording nothing and leaving the key's acts be. Any other
// change, a daemon's answer past `within` too, "joins" each act of its
// source (mixIn), so the leave that refresh makes is guessed. What
// waits isn't asked who queued it, so Joel's own key just before, its
// refresh not yet run, makes his next key's act joined too: a Return
// held that needn't have been, the side this errs on. It answers what
// it `says`, whether the change `joins` and whether an act it records
// is `mixed`, so the launcher matches no words.
function changeDoes(change) {
    const said = says => ({ says, joins: says === "joins", mixed: says === "joined act" });
    if (change.moved || change.ago >= change.within) {
        return said("joins");
    }
    if (!change.shown) {
        return said("kept");
    }
    return said(change.waiting ? "joined act" : "act");
}

// What a leave of `source` was, by `acts`, the launcher's keyedActs, and
// `heard`, when the key the clock is timed from was taken: "a key's act"
// when that key's change to the source is still waiting on the refresh
// that shows it; "a key's act, joined" when a change it doesn't own was
// taken into the same refresh (changeDoes); "a key's act, another key
// since" when the act was an earlier key's; and "no key's act" when none
// waits, as when a share's change or a daemon's answer comes alone. The
// journal says which, so a held Return can be told from one let through.
function actOf(acts, source, heard) {
    const its = acts.filter(one => one.source === source);
    const last = its.find(one => one.heard === heard);
    if (last) {
        return last.mixed ? "a key's act, joined" : "a key's act";
    }
    return its.length > 0 ? "a key's act, another key since" : "no key's act";
}

// Whether a leave of `source` was the act of Joel's last key, alone, so
// one he watched (actOf).
function keyed(acts, source, heard) {
    return actOf(acts, source, heard) === "a key's act";
}

// Marks each of `acts` waiting on a refresh of `source` as joined by a
// change its key doesn't own (changeDoes), so the leave that refresh
// makes is guessed (keyed). Marked in place, as the launcher times each
// act by the very record.
function mixIn(acts, source) {
    for (const one of acts) {
        if (one.source === source) {
            one.mixed = true;
        }
    }
}

// What the journal says of a leave once landedAfter has judged it: what
// it was (`act`, as actOf says), and whether `guess`, the guess standing
// after it, holds the Return pressed next, as returnUnder would answer
// it, since a key's act may still leave one an earlier leave left
// pending, and one by no key that lands back on its row holds nothing.
// It's a forecast for a Return pressed next with no words asked for; one
// pressed where the line asks for words is answered, and never held.
function leaveSaid(act, guess) {
    const next = { keys: guess.keys + 1, at: 0, depth: guess.depth, asking: false };
    const holds = returnUnder(guess, next, 0).holds;
    return `${act}; ${holds ? "a Return next holds" : "nothing holds"}`;
}

// A modifier pressed `alone`, as Shift or Ctrl, chose nothing, so the
// Return after it is still the first key after the leave. A key with no
// name isn't one, as é or Delete typed into the find moves the rows.
function passedOver(guess, alone) {
    return alone && guess.keys >= 0
        ? Object.assign({}, guess, { keys: guess.keys + 1 }) : guess;
}

// Whether a Return holds still, doing nothing, under `guess`, and what's
// guessed after it. `now` is the field's keys, this Return counted, as
// it counts a key before it acts; the time in ms; the launcher's depth;
// and whether words are asked for. Only the first key after the leave
// holds, however late, at the depth it landed, so a second Return meant
// for the step that left, as an ask's Release It, never acts on the row
// the selection stands on instead: one Return more, its foot saying why,
// costs less than a row Joel never chose acting. The Return straight after
// a held one, within `wait` ms of when it was pressed, holds once more, as
// that Return doubled, a key bounced or pressed twice for one. After a
// leave that threw, every Return holds until another key, since the
// selection is half made. Any other key first, as a move or a word typed,
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

// What a call for the launcher's state answers: what read gives, or,
// where it threw, JSON naming what threw as Thrown.js says it, told to said
// as well for the journal, since a call once answered an empty line and
// kept nothing of why (vm-desktop-looks, 22 Nov). The desktop tests read
// "threw" (tests/desktop/heard.py's shown); the state itself has no key
// of that name.
function answered(read, said) {
    try {
        return read();
    } catch (error) {
        const threw = Thrown.shown(error);
        said(threw);
        return JSON.stringify({ threw });
    }
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
