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
// they're shown. A dialog's own collection, `how.outermost`, is never
// left, for the home it never showed: it stays with its rows each gone,
// as no row offered now says it's left, rather than showing what it
// offered before, its row as it was entered. A thing landed in from a
// search, `through`, is left back to the search, past the collection it
// was found in, as Joel's - leaves it.
function leaveTo(made, how, ways) {
    const steps = made.steps.slice();
    if (made.leave < 0) {
        return { steps, keep: -1 };
    }
    const left = made.leave > 0 && steps[made.leave].through === true ? made.leave - 1
        : made.leave;
    const keep = Math.max(left, how.outermost ? 1 : 0);
    if (keep > made.leave) {
        steps[made.leave] = Object.assign({}, steps[made.leave], {
            rows: ways.keep(steps[made.leave], [])
        });
    }
    return { steps, keep: keep < steps.length ? keep : -1 };
}

// What every history forgets once `made` says a step offers nothing now,
// for afterLeave, or null where none does: `left`, the places through
// that step, whether it's left, left past the collection a search found
// it in, or kept as a dialog's outermost; and `landed`, where Joel is
// once `leaving` (leaveTo's answer) has gone, whose last places go. Each
// is read as `held` read it before the remaking and as `steps` read it
// after, since a row remade as offered now may be renamed so. Only the
// spent step's places go, never its collection's, whose other rows still
// offer what they did.
function forgets(made, leaving, held, steps) {
    if (made.leave < 0) {
        return null;
    }
    const through = made.leave + 1;
    const at = leaving.keep < 0 ? steps.length : leaving.keep;
    return {
        landed: [held.slice(0, at), steps.slice(0, at)],
        left: [held.slice(0, through), steps.slice(0, through)]
    };
}

// Where a list of steps goes, for telling the histories' places apart:
// each step's row by its key, or what it is, as the launcher's sameRow
// matches a row remade, and only a row with neither by its name. So a
// place recorded under a name its row had before a refresh renamed it
// is still the same place.
function placeOf(steps) {
    return steps.map(step => {
        const row = step.row ?? {};
        if ((row.key ?? "") !== "") {
            return `key\u0000${row.key}`;
        }
        return (row.is ?? "") !== "" ? `is\u0000${row.is}` : `name\u0000${row.name ?? ""}`;
    }).join("\u0001");
}

// What the launcher's remaking answers its caller, from `made` and where
// `leaving` (leaveTo's answer) goes: "made" when every step was made
// again, or the outermost alone kept with its rows gone; "kept" when one
// was kept as it was; and "left" when steps were left.
function answer(made, leaving) {
    if (made.leave < 0) {
        return made.all ? "made" : "kept";
    }
    return leaving.keep < 0 ? "made" : "left";
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
    const there = one => landed.includes(where(one.held));
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
