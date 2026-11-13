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
