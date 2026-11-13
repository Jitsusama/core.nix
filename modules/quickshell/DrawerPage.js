// The launcher's drawer gone into, as a page in normal mode with a cursor
// on its lines, as his Neovim's windows have one: where each key
// takes the cursor and the view, read apart from the drawing (Drawer.qml)
// so tests/query-keys.qml can hold it in Qt's own engine.
//
// A page is its lines, each { top, height, length, text } in pixels down
// the drawer's content, letters along it and the words it says, as Qt laid
// them out, in order; the
// cursor's line and letter, and `want`, how far along the line it wants to
// be, as Vim's curswant keeps the column j and k go back to; the view's top
// and height, and `end`, the furthest its top goes; the pitch its body's
// lines lie on, which keeps them on the card's rows; `off`, the lines kept
// about the cursor, his scrolloff; `start`, its view's top at rest, and
// `phase`, where on the page the pitch of its lines is counted from, both
// the page's top unless it says; `tops`, if it has them, the tops its
// view may scroll to (tops, below); and `xOf(line, letter)` and `letterAt(line, x)`, which ask the drawn lines
// where a letter is and which letter is nearest a place.
.pragma library
.import "Search.js" as Search

// What visual takes, from where it began to the cursor, either way round:
// each line it touches in order, { line, from, to }, letters `from` up to
// `to`. By "letters" it takes from the first end's letter through the
// other's, both their own, as Vim's inclusive selection does, and every
// letter of the lines between; by "lines", every letter of each.
function chosen(began, cursor, kind, lines) {
    const after = (one, other) => one.line > other.line
        || (one.line === other.line && one.letter > other.letter);
    const [first, last] = after(began, cursor) ? [cursor, began] : [began, cursor];
    const taken = [];
    for (let line = first.line; line <= last.line; line += 1) {
        const { length } = lines[line];
        const whole = kind === "lines";
        const from = whole || line !== first.line ? 0 : Math.min(first.letter, length);
        const to = whole || line !== last.line ? length : Math.min(last.letter + 1, length);
        taken.push({ line, from, to });
    }
    return taken;
}

// How many paragraphs words hold, as the drawer's foot counts those it
// hides, never their lines (VKCF3Z, 2026-11-05). Each line another program
// broke is one, which Qt keeps in a pre-wrap block as a line separator, and
// so is each a styled text breaks; a line Qt wraps is part of its own, and
// a blank one is none.
const PARAGRAPH_BREAK = /[\n\u2028\u2029]/;
function paragraphs(text) {
    return text.split(PARAGRAPH_BREAK).filter(paragraph => paragraph.trim() !== "").length;
}

// How far from the view's edges the cursor keeps: `off` lines, but never
// past the view's middle, so a short view still has a place for it.
function margin(page) {
    const most = Math.floor((page.height - page.pitch) / 2 / page.pitch) * page.pitch;
    return Math.max(0, Math.min(page.off * page.pitch, most));
}

// Every match of a search in lines as they're shown, as { line, letter },
// in order down the page, matched by smartcase as Search.js matches.
function matchesIn(lines, pattern) {
    return [].concat(...lines.map((line, at) => Search.found(pattern, line.text ?? "")
        .map(([start]) => ({ line: at, letter: start }))));
}

// Which match the cursor stands on, of how many, as { at, of } counting
// from one, or null where it stands on none: the foot counts it as
// Neovim's search count does.
function found(lines, pattern, line, letter) {
    const matches = matchesIn(lines, pattern);
    const at = matches.findIndex(match => match.line === line && match.letter === letter);
    return at < 0 ? null : { at: at + 1, of: matches.length };
}

// Where a search lands, as { line, letter, want, top, wrapped, at, of },
// or null where nothing is sought or it's found nowhere, `at` being which
// match of the `of` it found: on the next match's first
// letter past the cursor, or with `forward` false the one before it, a
// count going on that many, round past either end as Vim's wrapscan does,
// `wrapped` naming the end it went round, "bottom" or "top", as the rows'
// Finding.matchTo does, else ""; matched by smartcase as Search.js
// matches. Each line is searched as it's shown, so a match never runs
// across a line's end, and the view follows the cursor there as a move's
// does.
function sought(page, pattern, forward, count) {
    const matches = matchesIn(page.lines, pattern);
    if (matches.length === 0) {
        return null;
    }
    const past = match => match.line > page.line
        || (match.line === page.line && match.letter > page.letter);
    const before = match => match.line < page.line
        || (match.line === page.line && match.letter < page.letter);
    const times = Math.max(1, count) - 1;
    let target;
    if (forward) {
        const first = matches.findIndex(past);
        target = (first < 0 ? matches.length : first) + times;
    } else {
        let last = -1;
        matches.forEach((match, at) => {
            if (before(match)) {
                last = at;
            }
        });
        target = last - times;
    }
    const wrapped = target >= matches.length ? "bottom" : target < 0 ? "top" : "";
    const which = ((target % matches.length) + matches.length) % matches.length;
    const landed = matches[which];
    return {
        line: landed.line,
        letter: landed.letter,
        want: page.xOf(landed.line, landed.letter),
        top: shown(page.lines[landed.line], page.top, page),
        wrapped,
        at: which + 1,
        of: matches.length
    };
}

// Whether a view's top cuts a line partway, which Vim's never does, its
// topline always a whole line. An eyebrow or a title isn't a pitch tall,
// so a pitch can fall inside one (VKCF3Z, 2026-11-05: "Deploy log" as
// slivers at the top). A line's box holds its letters' descenders and no
// more, so a top even two pixels inside one shows their tips; only half a
// pixel, a box's rounding, is let go.
function cut(page, top) {
    return page.lines.some(line => top > line.top + 0.5 && top < line.top + line.height - 0.5);
}

// The tops a view may have: its rest, then each on the pitch from the
// page's phase, so the drawer's lines stay on the card's rows and each
// scrolled to takes the head's baseline, that cuts no line (as cut
// judges), up to the first at or past the page's end, so the last line
// shows whole too. Past a rest above the phase, the first is the phase
// itself, the body's first line: a top in the room between the head and
// the body would leave the view's first rows bare, though a blank line
// the page sets between groups is a line, as Vim's is. One pass
// down the rows and the lines in step, carrying the furthest any line
// begun above a row reaches: a row is cut just when that lies below it.
// A page that brings its own, as the drawer's does, worked out once its
// lines are laid, is taken at its word: on two thousand lines the pass
// takes some 2 ms, half a key's budget, where a look takes nothing.
function tops(page) {
    if (page.tops) {
        return page.tops;
    }
    // The drawer lays its lines down the page in order, so they're sorted
    // only when they come otherwise.
    const ordered = page.lines.every((line, at) => at === 0
        || page.lines[at - 1].top <= line.top);
    const lines = ordered ? page.lines : page.lines.slice().sort((one, other) => one.top
        - other.top);
    let bottom = 0;
    for (const line of lines) {
        bottom = Math.max(bottom, line.top + line.height);
    }
    const start = page.start ?? 0;
    const phase = page.phase ?? 0;
    // The first top on the pitch past the rest, from which the rest go on.
    const past = phase > start ? phase
        : phase + (Math.floor((start - phase) / page.pitch) + 1) * page.pitch;
    const all = [];
    let next = 0;
    let reach = -Infinity;
    for (let top = start; top <= Math.max(page.end, bottom);
        top = top === start ? past : top + page.pitch) {
        for (; next < lines.length && lines[next].top + 0.5 < top; next++) {
            reach = Math.max(reach, lines[next].top + lines[next].height - 0.5);
        }
        if (reach <= top) {
            all.push(top);
            if (top >= page.end) {
                break;
            }
        }
    }
    return all.length === 0 ? [start] : all;
}

// A view's top on a whole line's: going down, the first at or past where
// it would be; going up, the last at or before it.
function onLine(top, page, down) {
    const all = tops(page);
    if (down) {
        return all.find(one => one >= top) ?? all[all.length - 1];
    }
    return all.filter(one => one <= top).pop() ?? all[0];
}

// The view's top so many lines on from where it is, or back, never past
// either end, as Vim scrolls by lines.
function stepped(page, steps) {
    const all = tops(page);
    const at = Math.max(0, all.filter(one => one <= page.top).length - 1);
    return all[Math.max(0, Math.min(at + steps, all.length - 1))];
}

// The view's top once it shows a line with room kept about it, from where
// it is: moved only as far as it must, up or down, never past either end.
function shown(line, top, page) {
    const room = margin(page);
    if (line.top - room < top) {
        return onLine(line.top - room, page, false);
    }
    if (line.top + line.height + room > top + page.height) {
        return onLine(line.top + line.height + room - page.height, page, true);
    }
    return top;
}

// The lines a view from `top` holds with the margin kept: the margin at an
// edge the view can still move past, as Vim keeps scrolloff only where
// there's more to scroll to. With `whole`, every line wholly in view.
function inView(page, top, whole) {
    const room = margin(page);
    const from = top + (whole || top <= (page.start ?? 0) ? 0 : room);
    const to = top + page.height - (whole || top >= page.end ? 0 : room);
    const held = [];
    page.lines.forEach((line, at) => {
        if (line.top >= from && line.top + line.height <= to) {
            held.push(at);
        }
    });
    return held;
}

// The line nearest the cursor's that a view from `top` holds with its
// margin, so a view scrolled under the cursor carries it along, as Vim's
// does; the cursor's own when it's held already.
function kept(line, page, top) {
    const held = inView(page, top, false);
    if (held.length === 0 || held.includes(line)) {
        return line;
    }
    return line < held[0] ? held[0] : held[held.length - 1];
}

// The first line from a place down, going down, or the last from it up,
// going up, or the page's last or first where none is.
function lineFrom(page, y, down) {
    const lines = page.lines;
    if (down) {
        const at = lines.findIndex(line => line.top >= y);
        return at < 0 ? lines.length - 1 : at;
    }
    for (let at = lines.length - 1; at >= 0; at--) {
        if (lines[at].top <= y) {
            return at;
        }
    }
    return 0;
}

// Where a key's act takes the cursor and the view, as
// { line, letter, want, top }: a count moves that many times, or with gg
// and G goes to that line, counted from 1. j and k keep where along the
// line the cursor wants to be; gg and G go to a line's start. A scroll
// moves the view and carries the cursor with it where it would leave the
// margin; Ctrl+D and Ctrl+U move both as far, as Vim's do, and Ctrl+F and
// Ctrl+B a view less two lines, the cursor at the new view's top or
// bottom, or at the page's end where the view can go no further. Every
// move stops at the first and last lines, never going round.
function moved(act, count, page) {
    const lines = page.lines;
    const still = { line: page.line, letter: page.letter, want: page.want, top: page.top };
    if (lines.length === 0) {
        return still;
    }
    const times = Math.max(1, count);
    const last = lines.length - 1;
    const clamp = at => Math.max(0, Math.min(at, last));
    // To a line, the letter nearest where the cursor wants to be, with the
    // view brought round it.
    const toLine = (line, top) => ({ line, letter: page.letterAt(line, page.want),
        want: page.want, top: shown(lines[line], top, page) });
    // To a line's start, as gg and G go.
    const toStart = line => ({ line, letter: 0, want: 0, top: shown(lines[line], page.top, page) });
    // The view moved by so many lines, the cursor carried within it.
    const scrolled = steps => {
        const top = stepped(page, steps);
        const line = kept(page.line, page, top);
        return line === page.line ? Object.assign(still, { top }) : Object.assign(toLine(line,
            top), { top });
    };
    const viewLines = Math.floor(page.height / page.pitch);
    switch (act) {
    case "next":
        return toLine(clamp(page.line + times), page.top);
    case "previous":
        return toLine(clamp(page.line - times), page.top);
    case "left":
    case "right": {
        const most = Math.max(0, lines[page.line].length - 1);
        const letter = Math.max(0, Math.min(page.letter + (act === "right" ? times : -times),
            most));
        return Object.assign(still, { letter, want: page.xOf(page.line, letter) });
    }
    case "first":
        return toStart(count > 0 ? clamp(count - 1) : 0);
    case "last":
        return toStart(count > 0 ? clamp(count - 1) : last);
    case "scrollDown":
        return scrolled(times);
    case "scrollUp":
        return scrolled(-times);
    case "halfDown":
    case "halfUp": {
        const down = act === "halfDown";
        const steps = (count > 0 ? count : Math.max(1, Math.floor(viewLines / 2)))
            * (down ? 1 : -1);
        const top = stepped(page, steps);
        const line = kept(lineFrom(page, lines[page.line].top + steps * page.pitch, down),
            page, top);
        return Object.assign(toLine(line, top), { top: shown(lines[line], top, page) });
    }
    case "pageDown":
    case "pageUp": {
        const down = act === "pageDown";
        const steps = times * Math.max(1, viewLines - 2) * (down ? 1 : -1);
        const top = stepped(page, steps);
        if (top === page.top) {
            return toLine(down ? last : 0, top);
        }
        const held = inView(page, top, false);
        const line = held.length === 0 ? lineFrom(page, top, down)
            : down ? held[0] : held[held.length - 1];
        return Object.assign(toLine(line, top), { top: shown(lines[line], top, page) });
    }
    case "viewTop":
    case "viewBottom": {
        const held = inView(page, page.top, false);
        if (held.length === 0) {
            return still;
        }
        const step = Math.min(times, held.length) - 1;
        return toLine(act === "viewTop" ? held[step] : held[held.length - 1 - step], page.top);
    }
    case "viewMiddle": {
        const held = inView(page, page.top, true);
        return held.length === 0 ? still
            : toLine(held[Math.floor((held.length - 1) / 2)], page.top);
    }
    default:
        return still;
    }
}

// A line already on a beat, to within the sums' rounding, keeps it rather
// than going a beat on, as Drawer.qml's onBeat does.
const BEAT_ROUNDING = 0.001;

// How tall a preview stands, in a column that sets `grid.pitch` after it:
// a picture so what follows starts on the first of the card's beats at
// least the groups' `grid.gap` below it, never a band rounded up to the
// pitch (VKCF3Z, 2026-11-08), and never off the beat the facts keep
// (2026-11-05); lines whole lines of the pitch; nothing while a picture's
// size isn't known yet.
function previewHeight(picture, lines, grid) {
    if (picture <= 0) {
        return lines * grid.pitch;
    }
    const beats = Math.ceil((Math.ceil(picture) + grid.gap) / grid.beat - BEAT_ROUNDING);
    return beats * grid.beat - grid.pitch;
}

// The face a fact's value is set in, given the script its plate is in, ""
// for the theme's own: "name" for an identifier (`identifies: true`), a
// node, a serial or a bus name, which the drawer lists last in the Name
// face so it reads as something to copy rather than a figure (VKCF3Z,
// 2026-11-07); "words" for a sentence (`words: true`); "latin" for Latin
// letters on a script's plate (`latin: true`), since the script's cut
// draws them in another's; and "value", at Figure, for the rest.
function faceOf(fact, script) {
    if (fact?.identifies === true) {
        return "name";
    }
    if (fact?.words === true) {
        return "words";
    }
    return script !== "" && fact?.latin === true ? "latin" : "value";
}
