// How a list the launcher shows takes what's changed under it while Joel
// looks at it: a session starting to work, a window renamed. Its rows
// stay in the order they're shown, each with what it says now, those new
// come at the end and those gone stay where they were, dimmed, until a key
// makes the list afresh, and the selection stays on its row, so nothing
// moves under the cursor (Joel, 2026-10-25: "it needs to be fucking stable
// and always make sense"). Only a search Joel types orders by rank
// (Rank.js). Apart from the launcher, so tests/steady.qml can hold it in
// Qt's own engine.
.pragma library

// The rows shown that may stand where they are as the list is said again:
// what ends a search comes after it all and is made again with it, so it's
// no row to hold in place (held, it stood dimmed above the one made
// again). An answer there only while nothing else was, as Ask's for two
// plain words, goes once it gives way (`gaveWay`) to what came after it,
// as the notices daemon's finds come a moment after the key, rather than
// standing dimmed above them with the selection on it.
function standing(shown, gaveWay) {
    return shown.filter(row => !row.endedBy && !(gaveWay && row.answered));
}

// The rows as they read now, in the order of those shown: each row shown
// as it is now, or as it was, `gone` and `went`, where it's no longer
// there, then each new one in its own order. A new one comes at the end,
// out of any place a tree would give it, so one that says how it reads
// ranked flat (`ranked`, as a session under its orchestrator does) reads so;
// a list shown afresh has nothing new, and keeps every row's own. `went`
// tells a row that left from one its source draws dimmed itself, gone but
// there, as Write in Slack stands dimmed until Slack answers. A row is
// known by `id`; rows alike in it pair off in their order, so two windows
// titled alike keep their places. The rows that wait unseen (laidAgain)
// come in once the list is shown afresh.
//
// What waits unseen comes in here, at the end, marked `unseen`, as kept
// makes a held step's rows again (Held.js's keep), the list shown laid from
// them (refreshed), so a key listing the step's rows (seen) leaves it out
// and counts it, rather than show it at the end as the last heading's.
function kept(shown, now, id) {
    const again = laidAgain(shown, now, id, false);
    return again.lit(again.rows);
}

// The rows as kept lays them, but for the new ones that wait unseen, and
// how many, counted in the foot as "1 new" (Launcher.qml's waitingRows):
// { rows, waiting }.
function refreshed(shown, now, id) {
    const again = laidAgain(shown, now, id, true);
    return { rows: again.lit(again.rows), waiting: again.late };
}

// A held step's rows as kept lays them, listed as they stand, as a key
// lists them: those that wait unseen left out, and how many still there,
// while what's left ends under a heading, as laidAgain would withhold
// them; once it doesn't, as a key forgetting the rows gone may leave it,
// they come in where they stand, seen: { rows, waiting }.
function seen(rows, id) {
    const shown = rows.filter(row => row.unseen !== true);
    if (endingHeading(shown, headingsUnder(shown, id).map(its => its === null)) === null) {
        return { rows: rows.filter(row => !(row.unseen === true && row.went)).map(asSeen),
            waiting: 0 };
    }
    return { rows: shown,
        waiting: rows.filter(row => row.unseen === true && row.went !== true).length };
}

// What a step lists of its rows, `offered`, with nothing typed, and how
// many it leaves out unseen: { rows, waiting }. Steadied, as a refresh
// lays them, it's every row, for the refresh to withhold and count, so
// `waiting` is null, the refresh's to say (Launcher.qml's steadied); else
// it's the rows as they stand (seen), one a search found this opening
// (`found`) counted seen (seenAmong).
function listed(offered, found, id, steadied) {
    return steadied ? { rows: offered.slice(), waiting: null }
        : seen(seenAmong(offered, found, id), id);
}

// What a search typed in a held step found, as it's shown: seen, so none
// keeps the mark the refresh after would read as never shown and move to
// the end, under the cursor and among rows held still.
function searched(rows) {
    return rows.map(asSeen);
}

// A held step's rows with nothing typed: one waiting unseen that a search
// found since it opened (its `id` in `found`, kept in a plain record, as
// Qt 6.12 walks a Set whole, Rank.js) is seen, not hidden again, counted
// new and losing the selection, however many lists came between.
function seenAmong(rows, found, id) {
    const ids = Object.assign(Object.create(null), ...found.map(one => ({ [one]: true })));
    return rows.map(row => row.unseen === true && ids[id(row)] === true ? asSeen(row) : row);
}

// The rows laid again, `rows`, with `lit(rows)` dimming each heading said
// again with no row of its own standing under it in `rows`, `late`, how
// many new ones wait to be shown afresh where `withholding`, and
// `putBack`, how many of `rows` the rows shown put back.
// Where a list has headings (Headings.js), as For You's cells do, what
// stands under a heading reads as its, so a new row of a heading that
// isn't the last above the end, dimmed or not, has it said again over it,
// the same heading in the same words, `repeated`; a heading new to the
// list comes in only so, over a new row. A new row of no heading there
// waits while the end reads as under a heading, by where each row stands
// (endingHeading), since there it would read as that heading's, unless
// it's `pressing`, as Agents Asking's row and a critical notice's are,
// since what's pressing never waits unseen; one that comes in ends the
// heading above it, so a row after it has its own said again. A row shown
// stays where it is, under the heading it was shown under, though its cell
// has changed. One said again stays where it was said, reading as its
// heading reads now, and is dimmed while no row of its stands under it, as
// a row is while gone; a list shown afresh says each heading once (Y2YBTS,
// 2026-11-22 [228cdeb9] and [4c5e154a]). One said again among `now`, as a
// held step's rows hand it back, is no row of the source's, and is said
// only as here, though it still says what follows it is its heading's.
// One that waits is withheld and counted in `late` where `withholding`,
// and else kept at the end marked `unseen`, as a row of no heading: it ends
// nothing, since it isn't shown, and once it's pressing, or has a heading
// or none above it, it comes in as any row does.
function laidAgain(shown, now, id, withholding) {
    // A list with no heading, as most are, has nothing to wait or say
    // again, so is spared the work of finding each row's.
    const headless = !shown.some(row => row.heading === true)
        && !now.some(row => row.heading === true);
    const owners = headless ? now.map(() => null) : headingsUnder(now, id);
    // Where each row shown stood: what the end reads as under goes by
    // that, not by its heading now, so one whose cell went still reads as
    // under the heading it stands under.
    const placed = headless ? shown.map(() => null) : headingsUnder(shown, id);
    const fresh = now.filter(row => !row.repeated);
    const under = owners.filter((_, at) => !now[at].repeated);
    // Each row's heading by its place in `out` (`lit` is given only it or
    // its first rows), not a Map, which Qt 6.12 walks whole (Rank.js).
    const owner = [];
    // Whether each row in `out` reads as under none where it stands.
    const none = [];
    const lit = headless ? rows => rows : rows => rows.map((row, at) => row.repeated
        && !row.went && !standsUnder(rows, at, owner, id) ? dimmed(row) : row);
    if (shown.length === 0) {
        return { rows: fresh.map(asSeen), late: 0, lit: rows => rows, putBack: 0 };
    }
    const headingsNow = fresh.filter(row => row.heading === true);
    const waiting = Object.create(null);
    fresh.forEach((row, at) => {
        const said = id(row);
        if (waiting[said] === undefined) {
            waiting[said] = [];
        }
        waiting[said].push(at);
    });
    const taken = fresh.map(() => false);
    const out = [];
    const put = (row, its, alone) => {
        owner[out.length] = its;
        none[out.length] = alone;
        out.push(row);
    };
    let late = 0;
    for (const [place, row] of shown.entries()) {
        if (row.repeated) {
            const again = headingsNow.find(one => id(one) === id(row));
            out.push(again ? Object.assign({}, again, { repeated: true }) : dimmed(row));
            continue;
        }
        const left = waiting[id(row)];
        // One that waits unseen was never shown, so it has no place to
        // keep: it stays waiting while it waits, and else comes in as a new
        // one does, with its heading said over it, or leaves with nothing
        // to dim.
        if (row.unseen === true) {
            const at = left && left.length > 0 ? left[0] : -1;
            if (at >= 0 && waits(fresh[at], under[at], endingHeading(out, none) !== null)) {
                left.shift();
                taken[at] = true;
                late += withholding ? 1 : 0;
                if (!withholding) {
                    out.push(asUnseen(fresh[at]));
                }
            }
            continue;
        }
        // One shown is seen, though `now` hands it back marked, as a held
        // step does one a search found: marked, it'd next move to the end.
        if (left && left.length > 0) {
            const at = left.shift();
            const again = asSeen(fresh[at]);
            taken[at] = true;
            put(again, under[at], placed[place] === null);
        } else {
            put(dimmed(row), undefined, placed[place] === null);
        }
    }
    const putBack = out.length;
    let last = endingHeading(out, none);
    fresh.forEach((row, at) => {
        // One that left before it was ever shown, as a held step's rows
        // hand it back, has nothing to dim, and never waits to be shown.
        if (taken[at] || row.heading === true || row.went) {
            return;
        }
        const its = under[at];
        if (waits(row, its, last !== null)) {
            late += withholding ? 1 : 0;
            if (!withholding) {
                out.push(asUnseen(row));
            }
            return;
        }
        if (its === null) {
            last = null;
        }
        if (its !== null && (last === null || id(last) !== id(its))) {
            last = Object.assign({}, its, { repeated: true });
            out.push(last);
        }
        const coming = asSeen(typeof row.ranked === "function" ? row.ranked() : row);
        put(coming, its, its === null);
    });
    return { rows: out, late, lit, putBack };
}

// Whether a row new to a list waits unseen: one of no heading (`its`)
// where the end reads as under one (`endsHeaded`), and not pressing.
function waits(row, its, endsHeaded) {
    return its === null && endsHeaded && !pressing(row);
}

// A row as it's shown, no longer marked as waiting unseen, or so marked:
// a copy, so the row given keeps its mark, laid from its descriptors as
// Member.js's copied lays one, a getter handed on unread and unfrozen.
function asSeen(row) {
    return row.unseen === true ? marked(row, {}) : row;
}
function asUnseen(row) {
    return marked(row, { unseen: { value: true, enumerable: true, writable: true } });
}
function marked(row, laid) {
    const own = Object.getOwnPropertyDescriptors(row);
    return Object.defineProperties({}, Object.keys(own).filter(name => name !== "unseen")
        .reduce((all, name) => Object.assign(all, { [name]: own[name] }), laid));
}

// The heading the end of `rows` reads as under: the last one, unless a row
// that reads as under none where it stands (`none`) comes after it, as a
// pressing one that came in does, or To Look At after the cells, which
// ends it however long ago it came.
function endingHeading(rows, none) {
    let last = null;
    for (const [at, row] of rows.entries()) {
        if (row.heading === true) {
            last = row;
        } else if (none[at] === true) {
            last = null;
        }
    }
    return last;
}

// Whether a row is pressing, so it never waits unseen: its own word, or
// what it says while asked, as a notices row is while it holds a critical
// one (Now.qml's latelyRows).
function pressing(row) {
    return (typeof row.pressing === "function" ? row.pressing() : row.pressing) === true;
}

// A row as it stands once it's left: dimmed where it was, `gone` and `went`.
function dimmed(row) {
    return row.went ? row : Object.assign({}, row, { gone: true, went: true });
}

// Whether a row that hasn't left, and is the heading's at `at` in `rows`
// (`owner`, each row's heading), stands under it, before the next heading.
function standsUnder(rows, at, owner, id) {
    for (let next = at + 1; next < rows.length && rows[next].heading !== true; next += 1) {
        const its = owner[next] ?? null;
        if (!rows[next].went && its !== null && id(its) === id(rows[at])) {
            return true;
        }
    }
    return false;
}

// The heading each row is under among `rows`, or null for a heading itself
// and a row under none. A heading that names the rows it's `holding`, by
// key, as For You's cells' do (ForYouCells.js's laidOut), has those
// wherever they stand, so what follows them, To Look At and History, is
// under none, and a row that came in at the end is still its cell's; one
// that doesn't has every row to the next. One said again stands for the
// heading it repeats.
function headingsUnder(rows, id) {
    const holderOf = Object.create(null);
    for (const one of rows.filter(row => row.heading === true && !row.repeated
        && Array.isArray(row.holding))) {
        // A row with no key holds no place, so no row of none is a cell's.
        for (const key of one.holding.filter(key => typeof key === "string")) {
            holderOf[key] = holderOf[key] ?? one;
        }
    }
    let above = null;
    return rows.map(row => {
        if (row.heading === true) {
            above = row.repeated ? rows.find(one => one.heading === true && !one.repeated
                && id(one) === id(row)) ?? row : row;
            return null;
        }
        const holder = holderOf[row.key];
        if (holder !== undefined) {
            return holder;
        }
        return above !== null && !Array.isArray(above.holding) ? above : null;
    });
}

// The rows shown, held still as kept holds them, with no new one among
// them: while visual, a pending operator or the command line holds the
// rows, what arrives waits, counted, and comes in at the end once it ends
// (VKCF3Z's spec). { rows, waiting }. A heading is never a row, so none
// is counted among what waits; what waits to be shown afresh is. A
// heading said again is dimmed by the rows held, not by one that waits.
function held(shown, now, id) {
    const again = laidAgain(shown, now, id, true);
    return { rows: again.lit(again.rows.slice(0, again.putBack)),
        waiting: again.rows.slice(again.putBack).filter(row => row.heading !== true).length
            + again.late };
}

// The rows without those that left since they were shown, as a key makes
// the list afresh; a row its source draws dimmed stays where it stands.
function forgotten(rows) {
    return rows.filter(row => !row.went);
}

// The rows without those that left, and where the selection goes in them
// from `at`: { rows, at }. A share's start takes them off the screen at
// once (Launcher.qml's untold), since a copy kept from before it names
// what the share hides, and the cursor keeps to its row as a refresh
// keeps it; on a row that left, it stays where it stood.
function forgottenAt(rows, at, id) {
    const left = forgotten(rows);
    if (left.length === rows.length || at < 0) {
        return { rows: left, at };
    }
    const on = at < rows.length && !rows[at].went ? id(rows[at]) : "";
    const nth = on === "" ? 0 : alikeAbove(left, left.indexOf(rows[at]), id);
    return { rows: left, at: selection(left, { row: on, at, lowest: 0, nth }, id) };
}

// What a share's start leaves of what the launcher holds, from `was`:
// { rows, at, back, ahead, tags, last }, the rows shown and where the
// selection is, Ctrl+O's and Ctrl+I's places, Ctrl+T's, and Tab's last.
// The rows that left go (forgottenAt), and every way back goes with them,
// since a place held before the share may be an ask's step, whose name
// the crumb draws; none is kept by telling which, so one never told
// apart can't come back (Launcher.qml's untold).
function shareStarts(was, id) {
    const shown = forgottenAt(was.rows, was.at, id);
    return { rows: shown.rows, at: shown.at, back: [], ahead: [], tags: [], last: null };
}

// What's left of the ways back, `ways` as { back, ahead, tags, last },
// once every place goes that holds a row made showing by a source that
// hides now (its `shownBy`), as a step or among a step's rows, which the
// place draws as they were: one recorded between a share's start and its
// source's rows being remade holds the name from before it, and a source
// that never says a share began forgets nothing at its start
// (Launcher.qml's jump, toLastPlace and tagBack). The row says it rather
// than its step's source, since For You holds an ask under Now; a place
// whose rows were made hidden stays.
function hiddenGone(ways) {
    const bare = row => row?.shownBy?.hiding === true;
    const shown = place => !place.held.some(step => bare(step.row)
        || (step.rows ?? []).some(bare));
    return { back: ways.back.filter(shown), ahead: ways.ahead.filter(shown),
        tags: ways.tags.filter(shown),
        last: ways.last !== null && shown(ways.last) ? ways.last : null };
}

// The ways back a share has let go and their keys haven't yet said so, as
// { back, ahead, tags, last }: none as the launcher opens.
function noneLetGo() {
    return { back: false, ahead: false, tags: false, last: false };
}

// Which ways back stand let go once a share's letting go moves each from
// `had` places to `left` (counts by way, the last place 1 or 0), from the
// `marks` before it: a way it emptied is let go, as untold empties every
// way at a share's start and hiddenGone some at a press, and a way that
// holds a place again is not, its key having somewhere to go; any other
// write since is wayWritten's. Each way
// keeps its own mark, so a press that empties all four leaves the other
// three keys to say so at their own first press.
function waysLetGo(marks, had, left) {
    const now = {};
    for (const way of Object.keys(noneLetGo())) {
        now[way] = left[way] === 0 && (marks[way] === true || had[way] > 0);
    }
    return now;
}

// The marks once `way` is written by anything but a share's letting go (a
// step, a jump, Ctrl+], a step leaving by itself): no longer the share's
// to explain, though no key looked while it held a place, as one refilled
// and emptied again would otherwise blame the share. One not let go comes
// back as it was, so nothing that reads the marks moves.
function wayWritten(marks, way) {
    if (marks[way] !== true) {
        return marks;
    }
    const after = Object.assign({}, marks);
    after[way] = false;
    return after;
}

// What a press of `way`'s key, finding it empty, says: whether the share
// let it go (`say`), as the key had somewhere to go until then, so its
// silence or Ctrl+T's E73 would read as the key broken, and the marks
// after, that way's spent, so its next press says what Vim says
// (Launcher.qml's jump, toLastPlace and tagBack).
function letGoTold(marks, way) {
    const after = Object.assign({}, marks);
    after[way] = false;
    return { say: marks[way] === true, marks: after };
}

// Where the selection goes in them, from where it `was`: { row, at,
// lowest, nth }, the id of the row it was on ("" for none), its place, and
// how many rows alike in that id stood above it. It goes onto that row, the
// nth of those alike, as kept pairs them off in their order, so the cursor
// on the second of two windows titled alike, or of two rows one key lists
// twice, stays there rather than going up to the first (Joel, 2026-11-18:
// "it goes up when i scroll down to it"). With fewer alike now it goes onto
// the first; that one gone, it stays where it was, no lower than `lowest`
// (-1 when an answer's headline stands above the rows) and no further than
// the last row.
// How many rows alike in id to the one at `at` stand above it, the `nth`
// selection takes: none where nothing is selected.
function alikeAbove(rows, at, id) {
    if (at < 0 || at >= rows.length) {
        return 0;
    }
    const was = id(rows[at]);
    return rows.slice(0, at).filter(row => id(row) === was).length;
}

function selection(rows, was, id) {
    let first = -1;
    let above = was.nth ?? 0;
    for (let at = 0; was.row !== "" && at < rows.length; at++) {
        if (id(rows[at]) !== was.row) {
            continue;
        }
        if (above === 0) {
            return at;
        }
        first = first >= 0 ? first : at;
        above -= 1;
    }
    if (first >= 0) {
        return first;
    }
    return Math.max(was.lowest, Math.min(was.at, rows.length - 1));
}

// A landing asked for, as Super+N's on the notice in front (Launcher.qml's
// intoOn), taking what came: `waiting`, the row it wants (a test of a row)
// or null, and `came`, the rows listed now ({ rows }) or a key pressed
// ({ key: true }). It lands on the first row it wants once one is listed,
// and goes: { at, waiting }, `at` -1 where it lands nowhere. A key pressed
// first lets it go, since the cursor is Joel's from then: landing after his
// j, it put him back on the row he'd left, so j j stood on the second row
// (a gate under load, 2026-11-15).
function landing(waiting, came) {
    if (waiting === null || came.key === true) {
        return { at: -1, waiting: null };
    }
    const at = came.rows.findIndex(row => waiting(row));
    return { at, waiting: at < 0 ? waiting : null };
}

// How tall the list stands while the launcher is open, sized from `shown`,
// the rows it's sized for: as tall as its first `rows` rows stand by
// tallOf, and the headings among them, so the last row it shows ends at
// its foot, every gap among them counted however many stand apart. A heading is never a row (Headings.js), so it adds its
// line and not to the count, which `heading` tells it of. It never passes
// `sizes.room`, the screen's room for the list, stopping at the last row
// that fits in it, and a heading is never left alone at its foot. A list
// with fewer rows is sized for the room, a band for each row it hasn't,
// so a short place opens as tall as a long one. One rule for every place
// the card is sized, home as found among them, so none can size it from
// another list's gaps.
function tallFor(shown, rows, sizes, heading) {
    let tall = 0;
    let counted = 0;
    let at = 0;
    // How tall the headings since the last row stand, taken back should
    // the room end the card under them.
    let trailing = 0;
    for (; at < shown.length && counted < rows; at += 1) {
        const headed = heading(shown[at]);
        const one = tallOf(headed, shown[at]?.gap, at, sizes);
        if (tall + one > sizes.room) {
            return tall - trailing;
        }
        tall += one;
        trailing = headed ? trailing + one : 0;
        counted += headed ? 0 : 1;
    }
    for (; counted < rows && tall + sizes.row <= sizes.room; counted += 1) {
        tall += sizes.row;
    }
    return tall;
}

// How tall an open card's list stands once the screen's room for it has
// changed: as `tall` while the room still holds it, so its rows never move
// as Joel reads them, however much the room grows, and sized again by
// tallFor's rule for `by`, its `shown`, `rows`, `sizes` and `heading`, once
// the room drops below it, as a new scale leaves it. The net's
// Networks card stood 493.5 tall, its top 53 above a screen 388 high, kept
// from the room it opened in (2026-11-19, 20).
function refitted(tall, by) {
    return tall > by.sizes.room ? tallFor(by.shown, by.rows, by.sizes, by.heading) : tall;
}

// How tall one row stands in the list, at `at` among them: its band,
// `sizes.row`, with `sizes.gap` above one that stands apart; a heading
// its line, `sizes.heading`, with the groups' rung, `sizes.rung`, above it
// but for the list's first, which stands where the list starts (VKCF3Z,
// 2026-11-15 [90904822]). Whether it's a heading the caller says, as
// Headings.js decides it. A row's gap is its own, whatever list it comes
// from. tallFor sums it over the rows a card shows, so the two can't part
// but for a list shorter than the card, which is sized for the room.
function tallOf(heading, gap, at, sizes) {
    if (heading) {
        return (at === 0 ? 0 : sizes.rung) + sizes.heading;
    }
    return sizes.row + (gap ? sizes.gap : 0);
}
