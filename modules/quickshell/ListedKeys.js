// The keys the launcher's Space ? lists in its drawer: every act each
// mode takes, from the table QueryKeys.js reads them by, each as the
// desktop draws a key, beside what it does.

// The keys as Space ? lists them, a section a mode, normal's first since
// the list is opened there, each in the table's order of use: each line
// what it does, named in words.json by its `name`, and the acts whose keys
// it shows in its mode, the first alone where `firsts` says, since
// Return's with Shift and Ctrl are told apart where they're done. A line's
// `also` are keys no act's row holds, as a count and the : line's. Escape
// isn't listed, since it always steps back.
const normal = [
    { name: "rows", acts: ["next", "previous"] },
    { name: "ends", acts: ["first", "last"] },
    { name: "view", acts: ["viewTop", "viewMiddle", "viewBottom"] },
    { name: "half", acts: ["halfDown", "halfUp"] },
    { name: "screen", acts: ["pageDown", "pageUp"] },
    { name: "count", acts: [], also: ["3j", "12G"] },
    { name: "row", acts: [], also: [":12"] },
    { name: "inOut", acts: ["into", "out"] },
    { name: "run", acts: ["run"], firsts: true },
    { name: "lastPlace", acts: ["lastPlace"] },
    { name: "beside", acts: ["nextFile", "previousFile"] },
    { name: "own", acts: ["openOwn"] },
    { name: "insert", acts: ["insert", "append", "insertStart", "appendEnd"] },
    { name: "change", acts: ["change"] },
    // d and y, over the row as dd and yy, or as far as a motion goes.
    { name: "ending", acts: ["ending"], also: ["dj"] },
    { name: "copy", acts: ["copy"], also: ["yG"] },
    { name: "restore", acts: ["restore"] },
    { name: "drawer", acts: ["drawerDown", "drawerUp"] },
    { name: "keys", acts: ["keys"] },
    // q isn't listed, since Joel wants it to do nothing by itself and :q
    // to quit (2026-11-05); his respec takes its binding out.
    { name: "close", acts: ["close"], without: ["q"], also: [":q"] }
];

const insert = [
    { name: "nextRow", acts: ["next"] },
    { name: "previousRow", acts: ["previous"] },
    { name: "screen", acts: ["pageDown", "pageUp"] },
    { name: "run", acts: ["run"], firsts: true },
    { name: "complete", acts: ["complete"] },
    { name: "caret", acts: ["caretLeft", "caretRight"] },
    { name: "caretEnds", acts: ["caretStart", "caretEnd"] },
    { name: "letter", acts: ["deleteBack"] },
    { name: "word", acts: ["deleteWord"] },
    { name: "start", acts: ["deleteToStart"] },
    { name: "drawer", acts: ["drawerDown", "drawerUp"] }
];

const sections = [{ mode: "normal", lines: normal }, { mode: "insert", lines: insert }];

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
    const sequence = Array.isArray(keys) ? keys : [keys];
    return sequence.map(face).join(sequence.every(key => key.length === 1) ? "" : " ");
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

// The arrows, which Joel presses rather than hjkl ("support hjkl even
// though i only use arrows"), so the list never hides them.
const arrows = ["Up", "Down", "Left", "Right"];

// An act's keys as the list names them: those of its first rank, as the
// notices' list names each act's first key, and its arrow beside them.
// The drawer's names stay short enough to sit whole beside what they do
// in the narrowest plate, so PgDn, Backspace and Space w, which do what a
// listed key does, are left out.
function firstRank(keys) {
    const best = Math.min(...keys.map(rank));
    return keys.filter(key => rank(key) === best || arrows.includes(key));
}

// A line of a mode's section: its acts' keys in that mode alone, so no
// key is listed where it does nothing.
function keyLine(table, mode, line, words) {
    // Qt's engine has no flatMap, so each act's keys are joined by hand.
    const keys = [].concat(...line.acts.map(act => {
        const all = [].concat(...table
            .filter(one => one.act === act && (one.mode === mode || one.mode === "both"))
            .map(one => one.keys));
        return line.firsts ? all.slice(0, 1) : firstRank(all);
    })).filter(key => !(line.without ?? []).includes(key));
    const ranked = keys.map((key, at) => ({ key, at, rank: rank(key) }))
        .sort((a, b) => a.rank - b.rank || a.at - b.at);
    const said = ranked.map(one => sequenceFace(one.key)).concat(line.also ?? []);
    return { label: said.join("  "), value: words[line.name] ?? "" };
}

// Every line Space ? lists, as the drawer's facts, from QueryKeys' table:
// each mode's heading, from `modes`, then its lines, the keys as the name,
// muted, since a chord says how, and what they do, from `words`, as the
// value.
function keyList(table, words, modes) {
    return [].concat(...sections.map(section => [{ heading: modes[section.mode] ?? "" }]
        .concat(section.lines.map(line => keyLine(table, section.mode, line, words)))));
}
