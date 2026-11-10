// What key-counts has tallied of a keyboard (modules/key-counts), for its
// drawer (Keyboards.qml): how many keys it has pressed and held, which most,
// on which layers and under which modifiers, as the machine daemon sums
// them from the counts file and tells them (modules/machine/src/counts.rs),
// worded here in plain JavaScript so tests/answers.qml holds it in Qt's own
// engine.
.pragma library

// How many layers and modifiers it names beside their whole.
const SHARES = 3;

// A key as a chord draws it, from the key map's spelling of it, as the
// daemon tells it: its face in the theme's table (Shortcuts.qml's faces),
// else a letter as a capital and any other word with one.
function face(key, faces) {
    const own = faces[key];
    if (own !== undefined) {
        return own.text;
    }
    return key.length === 1 ? key.toUpperCase() : key.charAt(0).toUpperCase() + key.slice(1);
}

// What one board has tallied, from its record as the daemon tells it
// (`counts:<board>`, summed and ranked there), or null for one it has
// nothing of: { presses, holds, most: [{ key, presses }], layers: [{ name,
// share }], modified: share, modifiers: [{ name, share }] }, a share of one.
function board(told, faces) {
    const presses = Number(told?.presses ?? 0);
    if (!(presses > 0)) {
        return null;
    }
    const share = one => ({ name: one.name, share: one.presses / presses });
    return {
        presses,
        holds: Number(told.holds ?? 0),
        most: (told.most ?? []).map(one => ({ key: face(one.name, faces), presses: one.presses })),
        layers: (told.layers ?? []).map(share),
        modified: Number(told.modified ?? 0) / presses,
        modifiers: (told.modifiers ?? []).map(share)
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
