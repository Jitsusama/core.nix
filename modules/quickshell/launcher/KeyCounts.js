// What key-counts has tallied of a keyboard (modules/key-counts), for its
// drawer (Keyboards.qml): how many keys it has pressed and held, which most,
// on which layers and under which modifiers, read from the counts file it
// writes, in plain JavaScript so tests/answers.qml holds it in Qt's own
// engine. The file is a header and a row for each board, layer, modifiers
// held and key: `ergodox	coal	ctrl	a	4355	12`, "-" for no modifiers.
.pragma library

// How many keys the drawer names as most pressed.
const MOST = 5;
// How many layers and modifiers it names beside their whole.
const SHARES = 3;

// A key as a chord draws it (modules/theme/keys.nix, home.nix's
// stepFaces), from the kernel's name for it: its glyph where it has one,
// one name for a key there are two of, a letter as a capital, and any other
// word with one.
const FACES = {
    leftshift: "\u21e7", rightshift: "\u21e7", enter: "\u21b5", kpenter: "\u21b5",
    up: "\u2191", down: "\u2193", left: "\u2190", right: "\u2192", escape: "Esc",
    pageup: "PgUp", pagedown: "PgDn", leftctrl: "Ctrl", rightctrl: "Ctrl",
    leftalt: "Alt", rightalt: "Alt", leftmeta: "Super", rightmeta: "Super",
    capslock: "Caps Lock", numlock: "Num Lock", scrolllock: "Scroll Lock",
    // The key the counter knows only by its number: F23, the Copilot key's.
    code193: "F23", code70: "Scroll Lock"
};

// A number pad's key, by the kernel's name for it, kp7.
const PAD = /^kp(\d)$/;

function face(key) {
    const own = FACES[key];
    if (own !== undefined) {
        return own;
    }
    // The number pad's keys read as what they type, as the board's
    // numbers layer sends them.
    const pad = PAD.exec(key);
    if (pad !== null) {
        return pad[1];
    }
    return key.length === 1 ? key.toUpperCase() : key.charAt(0).toUpperCase() + key.slice(1);
}

// The counts file's rows, each { board, layer, modifiers, key, presses,
// holds }, leaving out its header and any row it can't read.
function rows(text) {
    return String(text ?? "").split("\n").map(line => line.split("\t"))
        .filter(cells => cells.length === 6 && cells[0] !== "board")
        .map(([named, layer, modifiers, key, presses, holds]) => ({
            board: named, layer, modifiers, key,
            presses: Number(presses) || 0,
            holds: Number(holds) || 0
        }));
}

// Each key's presses summed over the layers and modifiers it was pressed
// under, keyed by how it's drawn, so the two shifts are one.
function tallied(each, by) {
    const sums = new Map();
    for (const row of each) {
        const key = by(row);
        sums.set(key, (sums.get(key) ?? 0) + row.presses);
    }
    return Array.from(sums, ([name, presses]) => ({ name, presses }))
        .filter(one => one.presses > 0)
        .sort((a, b) => b.presses - a.presses || a.name.localeCompare(b.name));
}

// What one board has tallied, or null for one it has nothing of:
// { presses, holds, most: [{ key, presses }], layers: [{ name, share }],
// modified: share, modifiers: [{ name, share }] }, a share of one.
function board(text, name) {
    const own = rows(text).filter(row => row.board === name);
    const presses = own.reduce((sum, row) => sum + row.presses, 0);
    if (presses === 0) {
        return null;
    }
    const share = one => ({ name: one.name, share: one.presses / presses });
    const modified = own.filter(row => row.modifiers !== "-");
    return {
        presses,
        holds: own.reduce((sum, row) => sum + row.holds, 0),
        most: tallied(own, row => face(row.key)).slice(0, MOST)
            .map(one => ({ key: one.name, presses: one.presses })),
        layers: tallied(own, row => row.layer).map(share),
        modified: modified.reduce((sum, row) => sum + row.presses, 0) / presses,
        modifiers: tallied(modified, row => row.modifiers).map(share)
    };
}

// A count in full, its thousands apart: 90,432.
function whole(count) {
    return String(Math.round(count)).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

// A count short, as a key's beside it: 9.1k, 12k, 870.
function short(count) {
    if (count < 1000) {
        return String(Math.round(count));
    }
    const thousands = count / 1000;
    return thousands < 10 ? `${Math.round(thousands * 10) / 10}k` : `${Math.round(thousands)}k`;
}

// A share of one as a percentage, to a place under ten: 62%, 4.1%.
function percent(share) {
    const value = share * 100;
    return value < 10 ? `${Math.round(value * 10) / 10}%` : `${Math.round(value)}%`;
}

// The first few of a share's parts, after their whole where it has one:
// "coal 62% · base 30% · numb 6%".
function shares(parts) {
    return parts.slice(0, SHARES).map(part => `${part.name} ${percent(part.share)}`)
        .join(" \u00b7 ");
}

// When counting began, as the vocabulary fills a count's start: "since 5
// Nov", from the seconds key-counts wrote beside its counts when it first
// ran, or its words for a start it never wrote (`said.began`).
function since(seconds, said, today) {
    const at = Number(String(seconds ?? "").trim());
    if (!(at > 0)) {
        return said.began;
    }
    const when = new Date(at * 1000);
    const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
        "Nov", "Dec"];
    const year = (today ?? new Date()).getFullYear() === when.getFullYear() ? ""
        : ` ${when.getFullYear()}`;
    return said.since.split("{day}").join(`${when.getDate()} ${months[when.getMonth()]}${year}`);
}

// A board's counts as its drawer's facts, named by id (`named`): its
// presses since counting began, how many were held, the layers they were
// on and how many had a modifier held. None where it has counted nothing.
function facts(tally, named, began) {
    if (tally === null) {
        return [];
    }
    const out = [
        { label: named("presses"), value: `${whole(tally.presses)} ${began}` }
    ];
    if (tally.holds > 0) {
        out.push({ label: named("held"), value: whole(tally.holds) });
    }
    if (tally.layers.length > 1) {
        out.push({ label: named("layers"), value: shares(tally.layers) });
    }
    if (tally.modified > 0) {
        out.push({ label: named("modified"), value: [percent(tally.modified),
            shares(tally.modifiers)].join(" \u00b7 ") });
    }
    return out;
}

// The keys it pressed most, under a heading, each drawn as a chord's key
// is, with its count. A heading holds every fact after it, so these close
// the drawer, as the keys' list does. None where it has counted nothing.
function most(tally, named) {
    if (tally === null) {
        return [];
    }
    return [{ heading: named("most") }].concat(tally.most.map(one => ({
        keys: [[one.key]],
        value: short(one.presses)
    })));
}
