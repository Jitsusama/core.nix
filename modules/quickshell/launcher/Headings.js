// A list's headings, which name the group of rows under them and are never
// a row themselves, as Networks' Repositories and Projects are (VKCF3Z,
// 2026-11-15 [90904822]). A move counts rows alone, so 5j goes five rows
// whatever headings it passes, gg and G land on a row, and the gutter's
// numbers count rows; the selection never rests on one; and a heading
// never stands alone at the list's foot, its rows below it out of view.
// Apart from the launcher, so tests/query-keys.qml holds it in Qt's own
// engine.
.pragma library
.import "../Operators.js" as Operators

// How far a row may stand past the view and still be wholly in it, as
// the list reads its rows: half a pixel, the rounding of where Qt lays them.
const LEEWAY = 0.5;

// Whether a row is a heading rather than a row.
function isHeading(row) {
    return row?.heading === true;
}

// Where each heading stands among the rows, in order: none, for nearly
// every list, which every reader here answers as it always has.
function placesOf(rows) {
    const found = [];
    for (let each = 0; each < rows.length; each += 1) {
        if (isHeading(rows[each])) {
            found.push(each);
        }
    }
    return found;
}

// How many rows stand above `place`, headings left out: a row's place
// among the rows alone, and a heading's the row's under it.
function rank(headings, place) {
    let above = 0;
    while (above < headings.length && headings[above] < place) {
        above += 1;
    }
    return place - above;
}

// A place's rank, or a heading's the row's above it.
function rankAbove(headings, place) {
    return rank(headings, place) - (headings.includes(place) ? 1 : 0);
}

// Where the row of rank `ranked` stands among them all.
function placeOf(headings, ranked) {
    let place = ranked;
    for (const heading of headings) {
        if (heading > place) {
            break;
        }
        place += 1;
    }
    return place;
}

// Where a move lands (Operators.moved), counted in rows alone: the place
// as the list has it, read as ranks and moved there, so a count, the
// view's ends and a run's ends never count a heading. Above the rows (-1,
// a headline) stays where it is.
function moved(act, count, place, headings) {
    if (headings.length === 0) {
        return Operators.moved(act, count, place);
    }
    const ranked = ranks(place, headings);
    if (ranked.last < 0) {
        return place.at;
    }
    const to = Operators.moved(act, count, ranked);
    return to < 0 ? to : placeOf(headings, to);
}

// The rows an operator takes (Operators.spanned), counted in rows alone,
// so 3dd and d2j take three rows whatever headings stand between: the
// span read in ranks, then as the list has it, the headings within it
// included, which `taken` leaves out.
function spanned(count, motion, place, headings) {
    if (headings.length === 0) {
        return Operators.spanned(count, motion, place);
    }
    const ranked = ranks(place, headings);
    const span = Operators.spanned(count, motion, ranked);
    if (span.to < span.from) {
        return span;
    }
    const back = one => one < 0 ? one : placeOf(headings, one);
    return { from: back(span.from), to: back(span.to) };
}

// The rows a span takes (Operators.taken), no heading among them, since
// a heading has nothing to end, copy or run.
function taken(rows, span, headline) {
    return Operators.taken(rows, span, headline).filter(one => !isHeading(one.row));
}

// A place as the list has it, read in ranks: where the selection is, the
// view's ends, the last row and each run's ends, each counting rows alone.
// Above the rows (-1, a headline) stays where it is.
function ranks(place, headings) {
    const below = one => one < 0 ? one : rank(headings, one);
    const above = one => one < 0 ? one : rankAbove(headings, one);
    const runOf = place.runOf ? ranked => {
        const run = place.runOf(placeOf(headings, ranked));
        return { from: below(run.from), to: above(run.to) };
    } : undefined;
    return {
        at: below(place.at),
        highest: place.highest,
        last: place.last - headings.length,
        top: below(place.top),
        bottom: Math.max(below(place.top), above(place.bottom)),
        screen: place.screen,
        runOf
    };
}

// The row the n-th the gutter numbers stands at, as :5 goes to, within
// the rows.
function nth(headings, number, last) {
    const rows = last + 1 - headings.length;
    return placeOf(headings, Math.max(0, Math.min(number - 1, rows - 1)));
}

// Where the selection rests, never on a heading: past it the way it was
// going from `from`, or the other way where no row is that way.
function off(rows, place, from) {
    if (!isHeading(rows[place])) {
        return place;
    }
    const way = place >= from ? 1 : -1;
    for (const step of [way, -way]) {
        for (let each = place + step; each >= 0 && each < rows.length; each += step) {
            if (!isHeading(rows[each])) {
                return each;
            }
        }
    }
    return place;
}

// Where the list stands to show the row at `place` from where it stands,
// `from`, moving as little as it can, as the launcher's list.showing does
// without headings: a row's heading comes into view with it when the list
// rises to it, and no heading is left alone at the foot. `view` says the
// list's height and how many it holds, where each stands (`top`) and how
// tall it is (`tall`), and whether one's a heading.
function showing(place, from, view) {
    const headed = place > 0 && view.heading(place - 1) ? view.top(place - 1) : view.top(place);
    if (headed < from) {
        return footed(place, headed, view);
    }
    const bottom = view.top(place) + view.tall(place);
    return footed(place, bottom > from + view.height ? bottom - view.height : from, view);
}

// The view from `to`, moved so the last it shows is no heading alone: down
// to show the heading's first row while `place` stays wholly in view, and
// otherwise up until the heading is just below it.
function footed(place, to, view) {
    let last = place;
    while (last + 1 < view.count && view.top(last + 1) < to + view.height) {
        last += 1;
    }
    if (!view.heading(last) || last + 1 >= view.count) {
        return to;
    }
    const under = view.top(last + 1) + view.tall(last + 1) - view.height;
    if (under <= view.top(place) + LEEWAY) {
        return under;
    }
    return Math.min(to, view.top(last) - view.height);
}

// Where a view moved to `to` rests, so no row is cut at its top: on the
// top of the first row at or below it, which keeps whatever `to` showed
// whole at the top, the rest moved up by less than a row (VKCF3Z,
// 2026-11-15 [c09d8222]). No row's top is past the row `showing` brought
// into view, so that row stays whole. Past the last row's top it stays.
function rested(to, view) {
    for (let each = 0; each < view.count; each += 1) {
        const top = view.top(each);
        if (top >= to - LEEWAY) {
            return top;
        }
    }
    return to;
}

// Whether a row is drawn in a view: only with what it draws wholly in it,
// so the view never shows a row cut at its foot, and a heading only with
// its first row whole too, so none stands alone there. A row draws its
// ground only while selected, and otherwise just its line, so `row` says
// where what it draws starts and ends (`from`, `to`), the same of the row
// under it (`under`, null for none), and whether it's a heading (VKCF3Z,
// 2026-11-15 [889f9a7d]); `view` says where the view starts (`from`) and
// how tall it is.
function drawn(row, view) {
    const whole = span => span.from >= view.from - LEEWAY
        && span.to <= view.from + view.height + LEEWAY;
    return whole(row) && !(row.heading && row.under !== null && !whole(row.under));
}

// Whether a list's view rests whole: on a row's top, drawing only the rows
// wholly in it, the room left under the last less than a row's and bare. A
// list with headings does, and so does a place's own list with a gap, as
// the home network's eeros each have, since its rows' bands then fall off
// the pitch of home's card, whose foot would cut its last row (Y2YBTS,
// 2026-11-20 [95c35e88]). A find's rows rest as they always have: they
// never have headings, and a gap they carry from their place shouldn't
// change how the list rests as one is found and the next letter loses it.
// Spaces alone, `typed`, are no find: the search's words drop them, and it
// lists the place whole.
function restsWhole(rows, typed) {
    const found = typed.trim() !== "";
    return rows.some(isHeading) || (!found && rows.some(row => Boolean(row?.gap)));
}

// The rows with no heading, as a search shows them: its ranking mixes the
// groups, so no heading names what's under it. The same list where none
// is, so what reads it sees nothing new.
function without(rows) {
    return rows.some(isHeading) ? rows.filter(row => !isHeading(row)) : rows;
}
