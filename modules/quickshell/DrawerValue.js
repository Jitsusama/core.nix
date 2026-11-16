// How a drawer's fact lays its value on lines: items as many to a line as
// fit, an identifier broken at its own marks, a list giving way to a tail,
// and a lead's one line giving way after what it keeps, read apart from the
// drawing (Drawer.qml) so tests/query-keys.qml can hold it in Qt's own
// engine.
.pragma library
.import "Identifier.js" as Identifier
.import "Reading.js" as Reading

// What a dotted list's items run on.
const DOTTED = " \u00b7 ";

// A value given as items, each `{ said, path, whole }`, as lines no wider
// than `wide` by `width(text)`: as many items to a line as fit, `between`
// them (a command's arguments a space apart, a list's a dot), the dot
// going with a break, so no line ends or starts on one, as a plain run's
// does (DrawerFigure.runLines; VKCF3Z, 2026-11-09), and never one broken
// across two while it fits a line alone. An item too wide for a line of its
// own that's `whole`, an identifier, is shown whole on as many lines as
// it needs, broken at its own marks, and a short last one joins the line
// before rather than stand alone (Identifier.js). Any other that names its
// `path` gives way from the path's middle, "/nix/\u2026/firefox", its last
// part whole, as a folder does (Reading.pathTries); one still too wide, or
// naming none, gives way from its own middle. Nothing draws past its frame
// (VKCF3Z, 2026-11-08).
function valueLines(items, between, wide, width) {
    const saying = items.filter(item => item.said !== "");
    const lines = [];
    for (const item of saying) {
        const last = lines.length - 1;
        const said = item.whole ? item.said : fitted(item, wide, width);
        if (last >= 0 && width(`${lines[last]}${between}${said}`) <= wide) {
            lines[last] = `${lines[last]}${between}${said}`;
        } else {
            lines.push(...(item.whole && width(said) > wide
                ? Identifier.wholeLines(said, wide, width) : [said]));
        }
    }
    const final = saying[saying.length - 1];
    return final?.whole && lines.length > 1 && lines[lines.length - 1] === final.said
        && Identifier.short(final.said)
        ? Identifier.joinedDown(lines, between, wide, width) : lines;
}

// An identifier's value (`identifies: true`) as lines no wider than
// `wide`: whole on one where it fits, else a dotted run's items a dot
// apart as valueLines lays them, each item whole on as many lines as it
// needs, broken at its own marks (Identifier.js), so a desktop entry's
// "com.onepassword.OnePassword" never draws past the frame (VKCF3Z,
// 2026-11-09).
function identifierLines(said, wide, width) {
    const whole = String(said);
    return width(whole) <= wide ? [whole]
        : valueLines(whole.split(" \u00b7 ").map(item => ({ said: item, whole: true })),
            " \u00b7 ", wide, width);
}

// The words an identifier's value is read as, for the drawer to say
// whether each stands whole (Identifier.wordsOf).
function identifierWords(said) {
    return Identifier.wordsOf(said);
}

// How many of a drawer's acts, `widths` wide, stand on each line of `wide`
// with `apart` between two: as few lines as hold them in their order, none
// broken inside, split as evenly by width as they go. A lone act on a line
// is a gross wrap, "Close" under "↵Talk Go To Pause Stop" (VKCF3Z,
// 448f6cc6), so none stands alone where another split has none; then no
// line is wider than the one above, Return's act leading the longer; then
// the widest line is as short as it can be. One wider than the line stands
// on its own, as does one that can share it with neither neighbour, and a
// line of one act is never held wider than the line above, since it
// stands as it must. Of splits that rank alike, the one whose first lines
// hold fewest is taken.
//
// Trying every split took 117 ms for 25 acts, 812 for 30 and 4.7 s for 35
// in Qt's engine, as many as kitty holds with 25 windows open, and the
// shell's thread starved as the drawer was laid again and again
// (vm-desktop-launcher-moves-windows, 2026-11-16). So each rank is found
// in turn over what's left from each act on: the fewest lone acts and
// whether a line rises are a sum and an either over the lines, so the
// best of what's left after a line is the best overall; the widest line
// is the narrowest bound under which those two still hold; and the split
// is then read off with each line as short as that leaves it. Only the
// lines still to lay that the acts left can fill are tried, a count or
// two from each act: trying every count took 12.8 s for 120 narrow acts.
function actLines(widths, wide, apart) {
    const acts = actsOf(widths, wide, apart);
    if (acts.count === 0) {
        return [];
    }
    const wanted = rankedUnder(acts, Number.POSITIVE_INFINITY)(-1, 0, acts.fewest);
    const most = boundFor(acts, wanted);
    return readOff(acts, wanted, rankedUnder(acts, most), most);
}

// What laying a drawer's acts reads: how many, how wide those from one to
// another stand on a line, the fewest lines that hold them, and where a
// line from one may end.
function actsOf(widths, wide, apart) {
    const count = widths.length;
    const sums = [0];
    for (const width of widths) {
        sums.push(sums[sums.length - 1] + width);
    }
    const across = (from, to) => sums[to] - sums[from] + apart * (to - from - 1);
    // Where a line from `from` may end, none wider than `most`: those
    // that fit the line, or one act alone however wide, each wider than
    // the last, so the first too wide ends them.
    const reach = (from, most) => {
        const found = [];
        for (let end = from + 1; end <= count; end += 1) {
            const width = across(from, end);
            if ((end - from > 1 && width > wide) || width > most) {
                break;
            }
            found.push(end);
        }
        return found;
    };
    // A line of acts from `from` to `to`, after one begun at `before` (-1
    // for none), ranked with the rank of what follows it.
    const ranked = (before, from, to, after) => {
        if (after === null) {
            return null;
        }
        const many = to - from > 1;
        const rises = before >= 0 && many && across(from, to) > across(before, from) ? 1 : 0;
        return [after[0] + (many ? 0 : 1), Math.max(after[1], rises)];
    };
    // The fewest lines the acts from each on can be laid on: a line ending
    // as far on as it fits leaves fewest.
    const need = Array(count + 1).fill(0);
    for (let from = count - 1; from >= 0; from -= 1) {
        const far = reach(from, Number.POSITIVE_INFINITY);
        need[from] = 1 + need[far[far.length - 1]];
    }
    // Where a line may end with `left` lines still to lay, this one
    // among them: each line after it holds an act at least, and the acts
    // after it must fit on those lines, so an end leaving too few acts or
    // too many is passed by.
    const ends = (from, left, most) => reach(from, most)
        .filter(end => count - end >= left - 1 && need[end] <= left - 1);
    return { count, across, fewest: need[0], reach, ends, ranked };
}

// The least [lone acts, whether a line rises] of laying the acts from
// `from` on on `left` lines no wider than `most`, after a line begun at
// `before`, or null where they can't be, each answer kept once found.
function rankedUnder(acts, most) {
    const kept = new Map();
    const least = (before, from, left) => {
        const key = ((before + 1) * (acts.count + 1) + from) * (acts.fewest + 1) + left;
        if (kept.has(key)) {
            return kept.get(key);
        }
        let found = left === 0 && from === acts.count ? [0, 0] : null;
        if (left > 0) {
            for (const end of acts.ends(from, left, most)) {
                const one = acts.ranked(before, from, end, least(from, end, left - 1));
                if (one !== null && (found === null || lower(one, found))) {
                    found = one;
                }
            }
        }
        kept.set(key, found);
        return found;
    };
    return least;
}

// The narrowest widest line under which the acts still rank as `wanted`,
// one of the widths a line can stand at.
function boundFor(acts, wanted) {
    const bounds = [];
    for (let from = 0; from < acts.count; from += 1) {
        for (const end of acts.reach(from, Number.POSITIVE_INFINITY)) {
            bounds.push(acts.across(from, end));
        }
    }
    bounds.sort((one, other) => one - other);
    let low = 0;
    let high = bounds.length - 1;
    while (low < high) {
        const middle = Math.floor((low + high) / 2);
        const found = rankedUnder(acts, bounds[middle])(-1, 0, acts.fewest);
        if (found !== null && !lower(wanted, found)) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    return bounds[low];
}

// The split read off line by line, each the shortest whose split can
// still rank as wanted: what's laid so far, this line and the best of the
// rest, whole, since once a line rises a rest that rises too ranks no
// worse.
function readOff(acts, wanted, least, most) {
    const counts = [];
    let alone = 0;
    let rose = 0;
    for (let before = -1, from = 0, left = acts.fewest; left > 0; left -= 1) {
        let to = -1;
        for (const end of acts.ends(from, left, most)) {
            const one = acts.ranked(before, from, end, least(from, end, left - 1));
            if (one !== null && !lower(wanted, [alone + one[0], Math.max(rose, one[1])])) {
                to = end;
                break;
            }
        }
        const line = acts.ranked(before, from, to, [0, 0]);
        alone += line[0];
        rose = Math.max(rose, line[1]);
        counts.push(to - from);
        before = from;
        from = to;
    }
    return counts;
}

// Whether one rank is lower than another, by its first figure that differs.
function lower(one, other) {
    const at = one.findIndex((figure, which) => figure !== other[which]);
    return at >= 0 && one[at] < other[at];
}

// A list alone in a value, as its one line no wider than `wide`: every
// item a dot apart where they fit, else whole items giving way from the
// end to `tail`, "{count} more", saying how many went, "PDF \u00b7 HTML \u00b7
// 87 more" (VKCF3Z, 2026-11-08); the tail alone when not one fits beside it.
function valueLine(items, tail, wide, width) {
    const saying = items.map(item => item.said).filter(said => said !== "");
    const more = count => tail.replace("{count}", String(count));
    for (let whole = saying.length; whole > 0; whole--) {
        const gone = saying.length - whole;
        const line = saying.slice(0, whole).concat(gone > 0 ? [more(gone)] : []).join(DOTTED);
        if (width(line) <= wide) {
            return [line];
        }
    }
    return saying.length === 0 ? [] : [fitted({ said: more(saying.length) }, wide, width)];
}

// A lead's one line no wider than `wide`: its kept items a dot apart,
// which never give way, then `gives` after a dot, as many of its words as
// fit with an ellipsis where any went, so a person's unanswered message
// reads "in Slack \u00b7 14:02 \u00b7 Can you look\u2026" (VKCF3Z, 6ea9cd26);
// the kept items alone where not one word fits, past the frame if need be.
function leadLine(kept, gives, wide, width) {
    const held = kept.map(item => item.said).filter(said => said !== "").join(DOTTED);
    const words = String(gives).split(" ").filter(word => word !== "");
    const after = said => held === "" ? said : `${held}${DOTTED}${said}`;
    if (words.length === 0 || width(after(words.join(" "))) <= wide) {
        return [words.length === 0 ? held : after(words.join(" "))];
    }
    for (let whole = words.length - 1; whole > 0; whole--) {
        const line = after(`${words.slice(0, whole).join(" ")}\u2026`);
        if (width(line) <= wide) {
            return [line];
        }
    }
    return [held];
}

// What of a fact's value stands beside its name, measured to choose
// whether a drawer reads as a table: a list with a `tail` is the one line
// valueLine draws in `room`, never every item it holds, so a browser
// opening dozens of kinds keeps its table (Y2YBTS, 2026-11-08); any other
// its value whole. `value` stays what's copied either way.
function besideText(fact, room, width) {
    if (fact.valueItems && fact.gives !== undefined) {
        return leadLine(fact.valueItems, fact.gives, room, width).join("");
    }
    if (fact.valueItems && fact.tail) {
        return valueLine(fact.valueItems, fact.tail, room, width).join("");
    }
    return String(fact.value);
}

// An item's words no wider than `room`: whole where they fit, else its
// path from the path's middle, else its narrowest form from its middle.
function fitted(item, room, width) {
    if (width(item.said) <= room) {
        return item.said;
    }
    const ways = item.path ? Reading.pathTries(item.path, true)
        .map(way => item.said.replace(item.path, way)) : [];
    const narrowest = ways.reduce((least, way) => (width(way) < width(least) ? way : least),
        item.said);
    return ways.find(way => width(way) <= room) ?? middled(narrowest, room, width);
}

// Words given way from their middle, "--a-ver\u2026ument", keeping as much of
// each end as fits in `room`.
function middled(words, room, width) {
    // By characters, never halves of one, so an emoji is kept whole or not:
    // spread, since Qt's Array.from takes a string's UTF-16 units.
    const each = [...words];
    for (let keep = each.length - 1; keep > 0; keep--) {
        const head = Math.ceil(keep / 2);
        const tried = `${each.slice(0, head).join("")}\u2026`
            + each.slice(each.length - (keep - head)).join("");
        if (width(tried) <= room) {
            return tried;
        }
    }
    return "\u2026";
}
