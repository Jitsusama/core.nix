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
// and height, and `end`, the furthest its top goes; the pitch every line
// lies on, which the view moves by so the drawer's lines stay on the
// card's; `off`, the lines kept about the cursor, his scrolloff; and
// `xOf(line, letter)` and `letterAt(line, x)`, which ask the drawn lines
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

// How far from the view's edges the cursor keeps: `off` lines, but never
// past the view's middle, so a short view still has a place for it.
function margin(page) {
    const most = Math.floor((page.height - page.pitch) / 2 / page.pitch) * page.pitch;
    return Math.max(0, Math.min(page.off * page.pitch, most));
}

// Where a search lands, as { line, letter, want, top, wrapped }, or null
// where nothing is sought or it's found nowhere: on the next match's first
// letter past the cursor, or with `forward` false the one before it, a
// count going on that many, round past either end as Vim's wrapscan does,
// matched by smartcase as Search.js matches. Each line is searched as it's
// shown, so a match never runs across a line's end, and the view follows
// the cursor there as a move's does.
function sought(page, pattern, forward, count) {
    const matches = [].concat(...page.lines.map((line, at) => Search.found(pattern,
        line.text ?? "").map(([start]) => ({ line: at, letter: start }))));
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
    const wrapped = target < 0 || target >= matches.length;
    const landed = matches[((target % matches.length) + matches.length) % matches.length];
    return {
        line: landed.line,
        letter: landed.letter,
        want: page.xOf(landed.line, landed.letter),
        top: shown(page.lines[landed.line], page.top, page),
        wrapped
    };
}

// A view's top, held within the page and on the pitch.
function within(top, page) {
    return Math.max(0, Math.min(top, page.end));
}

// The view's top once it shows a line with room kept about it, from where
// it is: moved only as far as it must, up or down, never past either end.
function shown(line, top, page) {
    const room = margin(page);
    if (line.top - room < top) {
        return within(Math.floor((line.top - room) / page.pitch) * page.pitch, page);
    }
    if (line.top + line.height + room > top + page.height) {
        return within(Math.ceil((line.top + line.height + room - page.height) / page.pitch)
            * page.pitch, page);
    }
    return top;
}

// The lines a view from `top` holds with the margin kept: the margin at an
// edge the view can still move past, as Vim keeps scrolloff only where
// there's more to scroll to. With `whole`, every line wholly in view.
function inView(page, top, whole) {
    const room = margin(page);
    const from = top + (whole || top <= 0 ? 0 : room);
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
    // The view moved by so many pitches, the cursor carried within it.
    const scrolled = pitches => {
        const top = within(page.top + pitches * page.pitch, page);
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
        const pitches = (count > 0 ? count : Math.max(1, Math.floor(viewLines / 2)))
            * (down ? 1 : -1);
        const top = within(page.top + pitches * page.pitch, page);
        const line = kept(lineFrom(page, lines[page.line].top + pitches * page.pitch, down),
            page, top);
        return Object.assign(toLine(line, top), { top: shown(lines[line], top, page) });
    }
    case "pageDown":
    case "pageUp": {
        const down = act === "pageDown";
        const pitches = times * Math.max(1, viewLines - 2) * (down ? 1 : -1);
        const top = within(page.top + pitches * page.pitch, page);
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
