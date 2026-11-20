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

// The rows as they read now, in the order of those shown: each row shown
// as it is now, or as it was, `gone` and `went`, where it's no longer
// there, then each new one in its own order. A new one comes at the end,
// out of any place a tree would give it, so one that says how it reads
// ranked flat (`ranked`, as a session under its orchestrator does) reads so;
// a list shown afresh has nothing new, and keeps every row's own. `went`
// tells a row that left from one its source draws dimmed itself, gone but
// there, as Write in Slack stands dimmed until Slack answers. A row is
// known by `id`; rows alike in it pair off in their order, so two windows
// titled alike keep their places.
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

function kept(shown, now, id) {
    const waiting = {};
    now.forEach((row, at) => {
        const said = id(row);
        if (waiting[said] === undefined) {
            waiting[said] = [];
        }
        waiting[said].push(at);
    });
    const taken = now.map(() => false);
    const out = [];
    for (const row of shown) {
        const left = waiting[id(row)];
        if (left && left.length > 0) {
            const at = left.shift();
            taken[at] = true;
            out.push(now[at]);
        } else {
            out.push(row.went ? row : Object.assign({}, row, { gone: true, went: true }));
        }
    }
    now.forEach((row, at) => {
        if (!taken[at]) {
            out.push(shown.length > 0 && typeof row.ranked === "function" ? row.ranked() : row);
        }
    });
    return out;
}

// The rows shown, held still as kept holds them, with no new one among
// them: while visual, a pending operator or the command line holds the
// rows, what arrives waits, counted, and comes in at the end once it ends
// (VKCF3Z's spec). { rows, waiting }.
function held(shown, now, id) {
    const out = kept(shown, now, id);
    return { rows: out.slice(0, shown.length), waiting: out.length - shown.length };
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
// the rows it's sized for: `rows` rows `rowTall` high, and `gap` more where
// one of those it shows first stands apart from the row above, as a group
// of Network's does. One rule for every place the card is sized, home as
// found among them, so none can size it from another list's gaps.
function tallFor(shown, rows, rowTall, gap) {
    return rows * rowTall + (shown.slice(0, rows).some(row => row.gap) ? gap : 0);
}

// How tall one row stands in the list, at `at` among them: its band,
// `sizes.row`, with `sizes.gap` above one that stands apart; a heading
// its line, `sizes.heading`, with the groups' rung, `sizes.rung`, above it
// but for the list's first, which stands where the list starts (VKCF3Z,
// 2026-11-15 [90904822]). Whether it's a heading the caller says, as
// Headings.js decides it. A row's gap is its own, whatever list it comes
// from, and the same value tallFor counts. The two agree for as many rows
// as the card has room for, with no heading and at most one gap among them;
// tallFor sizes a shorter list for the room, counts one gap however many
// stand apart, and a heading as a row, which is left to its own change.
function tallOf(heading, gap, at, sizes) {
    if (heading) {
        return (at === 0 ? 0 : sizes.rung) + sizes.heading;
    }
    return sizes.row + (gap ? sizes.gap : 0);
}
