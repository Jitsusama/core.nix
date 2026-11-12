// The keys the launcher's Space ? lists in its drawer: every act each
// mode takes, from the table QueryKeys.js reads them by, each as the
// desktop draws a key, beside what it does.

// The keys as Space ? lists them, a section a mode, each named by its
// purpose (the vi-paradigm spec's key list), normal's first since the list
// is opened there, each in the table's order of use: each line
// what it does, named in words.json by its `name`, and the acts whose keys
// it shows in its mode, the first alone where `firsts` says, since
// Return's with Shift and Ctrl are told apart where they're done. A line's
// `also` are keys no act's row holds, as a count and the : line's. Escape
// isn't listed, since it always lets go: what's begun, then the launcher.
const normal = [
    { name: "rows", acts: ["next", "previous"] },
    { name: "ends", acts: ["first", "last"] },
    { name: "view", acts: ["viewTop", "viewMiddle", "viewBottom"] },
    { name: "half", acts: ["halfDown", "halfUp"] },
    { name: "screen", acts: ["pageDown", "pageUp"] },
    { name: "count", acts: [], also: ["3j", "12G"] },
    // Return opens a row or goes into it and - goes up, as his oil's; h
    // and l aren't listed, since on a row they only say so.
    { name: "openIn", acts: ["run"], firsts: true },
    { name: "up", acts: ["out"] },
    // Tab into what a row holds, as a program's page.
    { name: "into", acts: ["into"] },
    // Ctrl+] to the row's own place, where a row found elsewhere lives or
    // into what Return opens, as a mail's sender (VKCF3Z), and Ctrl+T back.
    { name: "follow", acts: ["follow", "tagBack"] },
    // Tab is Ctrl+I to a terminal, so it's listed once, as Ctrl+I.
    { name: "jumps", acts: ["jumpBack", "jumpForward"] },
    { name: "lastPlace", acts: ["lastPlace"] },
    { name: "beside", acts: ["nextFile", "previousFile"] },
    { name: "own", acts: ["openOwn"] },
    { name: "find", acts: ["freshFind", "freshFindBack"] },
    { name: "matches", acts: ["matchNext", "matchBack"] },
    { name: "letters", acts: ["toLetter", "toLetterBack", "letterAgain", "letterAgainBack"] },
    { name: "insert", acts: ["insert", "append", "insertStart", "appendEnd"] },
    { name: "change", acts: ["change"] },
    // d and y, over the row as dd and yy, or as far as a motion goes.
    { name: "ending", acts: ["ending"], also: ["dj"] },
    { name: "copyRow", acts: ["copy"], also: ["yG"] },
    // u takes back what d did, Ctrl+R does it again, and . does the last
    // d again.
    { name: "undo", acts: ["undoChange"] },
    { name: "redo", acts: ["redoChange"] },
    { name: "again", acts: ["again"] },
    // Vim's Ctrl+W l alone, as the way back names Ctrl+W h alone, so the
    // line sits whole at Joel's scale.
    { name: "intoDrawer", acts: ["drawer"], firsts: true },
    { name: "keys", acts: ["keys"] },
    { name: "numbers", acts: ["numbers"] },
    // q isn't listed, since it does nothing by itself but say how to
    // close (Joel, 2026-11-05).
    { name: "close", acts: ["close"], also: [":q"] }
];

// The find's, the line the launcher opens on: its rows and Return, the
// way its letters are edited being insert's, below, which it shares.
const find = [
    { name: "nextRow", acts: ["next"] },
    { name: "previousRow", acts: ["previous"] },
    { name: "screen", acts: ["pageDown", "pageUp"] },
    { name: "run", acts: ["run"], firsts: true },
    { name: "into", acts: ["into"] },
    { name: "follow", acts: ["follow"] },
    { name: "complete", acts: ["complete"] }
];

// The command line's, whose keys are its own (CommandLine.qml), so none
// is a row of the table: a line's range, what it runs on, and its menu.
const commands = [
    { name: "row", acts: [], also: [":12"] },
    { name: "global", acts: [], also: [":g/Notes/q"] },
    // An option, the machine's, and that of what it's on, as Vim's
    // :setlocal is the buffer's.
    { name: "option", acts: [], also: [":set dnd"] },
    { name: "optionHere", acts: [], also: [":setl mute"] },
    { name: "menu", acts: [], also: ["Ctrl+N", "Ctrl+P"] },
    { name: "take", acts: [], also: ["Ctrl+Y"] },
    { name: "away", acts: [], also: ["Ctrl+E"] },
    { name: "commandAgain", acts: [], also: ["@:"] },
    { name: "history", acts: [], also: [":his"] }
];

// Visual's, on the rows, normal's moves extending it; gv and ip and ap
// are begun from normal, and an operator's, so no visual row holds them.
const visual = [
    { name: "chooseRows", acts: ["selectLines"] },
    { name: "reselect", acts: [], also: ["gv"] },
    { name: "otherEnd", acts: ["otherEnd"] },
    { name: "paragraph", acts: [], also: ["ip", "ap"] },
    { name: "endChosen", acts: ["ending"] },
    { name: "copyChosen", acts: ["copy"] },
    { name: "openChosen", acts: ["run"], firsts: true },
    { name: "commandChosen", acts: [], also: [":"] }
];

// Insert's, where words are typed, the find's and a prompt's alike.
const insert = [
    { name: "caret", acts: ["caretLeft", "caretRight"] },
    { name: "caretEnds", acts: ["caretStart", "caretEnd"] },
    { name: "letter", acts: ["deleteBack"] },
    { name: "word", acts: ["deleteWord"] },
    { name: "start", acts: ["deleteToStart"] }
];

// In the drawer, which the keys go into as a window, its page's moves.
// Its way back is Ctrl+W h alone, the first of its keys, and :q, since the
// rest make the line too long to sit whole.
const page = [
    { name: "lines", acts: ["next", "previous"] },
    { name: "along", acts: ["left", "right"] },
    { name: "ends", acts: ["first", "last"] },
    { name: "view", acts: ["viewTop", "viewMiddle", "viewBottom"] },
    { name: "scroll", acts: ["scrollDown", "scrollUp"] },
    { name: "half", acts: ["halfDown", "halfUp"] },
    { name: "screen", acts: ["pageDown", "pageUp"] },
    { name: "count", acts: [], also: ["3j", "12G"] },
    // / and ? open its search as : opens the command line, so no row holds
    // them.
    { name: "seek", acts: [], also: ["/", "?"] },
    { name: "seekOn", acts: ["seekNext", "seekPrevious"] },
    { name: "select", acts: ["select", "selectLines"] },
    { name: "reselect", acts: ["reselect"] },
    { name: "fold", acts: ["fold"] },
    { name: "keys", acts: ["keys"] },
    { name: "back", acts: ["rows"], firsts: true, also: [":q"] }
];

// Choosing in the drawer, by letters or lines: what it has of its own,
// the page's moves extending it as listed above.
const pageVisual = [
    { name: "otherEnd", acts: ["otherEnd"] },
    { name: "copy", acts: ["copy", "copyLines"] }
];

// A section's mode is the one its keys are pressed in, and names its
// heading in `modes`; the command line's keys are no mode's in the table.
const sections = [{ mode: "normal", lines: normal }, { mode: "find", lines: find },
    { mode: "command", lines: commands }, { mode: "visual", lines: visual },
    { mode: "insert", lines: insert }, { mode: "page", lines: page },
    { mode: "pageVisual", lines: pageVisual }];

// Whether a table's row is pressed in a mode, as QueryKeys.inMode says:
// "line" is the find's and insert's, "both" every mode's.
function pressedIn(one, mode) {
    return one.mode === mode || one.mode === "both"
        || (one.mode === "line" && (mode === "find" || mode === "insert"));
}

// How a key is drawn, as the desktop draws one (modules/theme/keys.nix):
// its glyph where it has one, its name where not, a letter with a
// modifier as a capital, joined by +.
const faces = {
    Shift: "\u21e7", Return: "\u21b5", Up: "\u2191", Down: "\u2193", Left: "\u2190",
    Right: "\u2192", Escape: "Esc", PageUp: "PgUp", PageDown: "PgDn"
};

function face(key) {
    const parts = key.split("+");
    const last = parts.pop();
    const letter = parts.length > 0 && last.length === 1 ? last.toUpperCase() : last;
    return parts.map(part => faces[part] ?? part).concat([faces[letter] ?? letter]).join("+");
}

// A sequence of letters reads as Vim writes it, gg; any other a space
// apart, Space ?. Keys that do the same are two spaces apart, so a
// sequence's space never reads as between two of them.
function sequenceFace(keys) {
    return sequenceParts(keys).join(" ");
}

// A sequence's keys as the drawer draws them, each a run of its own, the
// tie rung apart, so Space Tab reads as one key; letters typed together,
// as gg, stay one run, as they're typed.
function sequenceParts(keys) {
    const sequence = Array.isArray(keys) ? keys : [keys];
    return sequence.every(key => key.length === 1) ? [sequence.map(face).join("")]
        : sequence.map(face);
}

// Vim's own keys first, as his Neovim's help gives them, then those with
// Ctrl, then the named keys: j k \u2193 \u2191, not \u2193 j \u2191 k.
function rank(keys) {
    const sequence = Array.isArray(keys) ? keys : [keys];
    if (sequence.every(key => key.length === 1)) {
        return 0;
    }
    return sequence[0].startsWith("Ctrl+") ? 1 : 2;
}

// An act's keys as the list names them: those of its first rank, as the
// notices' list names each act's first key, so an arrow is named only
// where an act has nothing better, as the find's caret, Joel keeping to
// Vim's keys ("rely only on vi right feeling things", 2026-11-05).
// The drawer's names stay short enough to sit whole beside what they do
// in the narrowest plate, so PgDn, Backspace and Space w, which do what a
// listed key does, are left out.
function firstRank(keys) {
    const best = Math.min(...keys.map(rank));
    return keys.filter(key => rank(key) === best);
}

// A line of a mode's section: its acts' keys in that mode alone, so no
// key is listed where it does nothing.
function keyLine(table, mode, line, words) {
    // Qt's engine has no flatMap, so each act's keys are joined by hand.
    const keys = [].concat(...line.acts.map(act => {
        const all = [].concat(...table
            .filter(one => one.act === act && pressedIn(one, mode))
            .map(one => one.keys));
        return line.firsts ? all.slice(0, 1) : firstRank(all);
    })).filter(key => !(line.without ?? []).includes(key));
    const ranked = keys.map((key, at) => ({ key, at, rank: rank(key) }))
        .sort((a, b) => a.rank - b.rank || a.at - b.at);
    const said = ranked.map(one => sequenceFace(one.key)).concat(line.also ?? []);
    // Its chords each a run of their own, which the drawer sets the apart
    // rung from each other, and each chord's keys the tie rung; the label
    // joins them for what reads it whole.
    const keyed = ranked.map(one => sequenceParts(one.key))
        .concat((line.also ?? []).map(also => [also]));
    return { label: said.join("  "), chords: said, keys: keyed, value: words[line.name] ?? "" };
}

// Every line Space ? lists, as the drawer's facts, from QueryKeys' table:
// each mode's heading, from `modes`, then its lines, the keys as the name,
// muted, since a chord says how, and what they do, from `words`, as the
// value. Listed from the drawer, `only` names the sections it lists, its
// own, since the rows' keys do nothing while its keys are in it.
function keyList(table, words, modes, only = null) {
    const listed = sections.filter(section => only === null || only.includes(section.mode));
    return [].concat(...listed.map(section => [{ heading: modes[section.mode] ?? "" }]
        .concat(section.lines.map(line => keyLine(table, section.mode, line, words)))));
}
