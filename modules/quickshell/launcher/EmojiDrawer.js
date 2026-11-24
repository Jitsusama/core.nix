// An emoji's drawer, made by the shell from the row the clipboard's history
// serves, as it's drawn: its group over its name, set as Unicode writes it,
// its keywords as words where it has any, and its code points as Unicode
// writes them, raw, folded behind za. The row says only `drawer: true`
// (clipboard/src/emoji.rs), since it holds all the drawer says: served in
// every row, the 1,914 drawers were 1.26 of the 4.34 MB the shell's heap
// kept live for Emoji's records, for the one drawer shown at a time (the
// heap lab over Joel's answer, 2026-11-23). In plain JavaScript, so a check
// runs it in Qt's own engine, as the shell does.
.pragma library

// The base a code point is written in, and the fewest digits it's given.
const HEX = 16;
const DIGITS = 4;

// The drawer of `row`, its lines named by `words`, `{ keywords, code }`.
function made(row, words) {
    const keywords = Array.isArray(row?.keywords) ? row.keywords : [];
    return {
        eyebrow: row?.held?.group ?? "",
        eyebrowNames: true,
        title: row?.name ?? "",
        facts: keywords.length === 0 ? []
            : [{ name: words.keywords, said: keywords.join(", "), words: true }],
        nerd: [{ label: words.code, value: points(row?.held?.emoji ?? "") }]
    };
}

// Every code point of `emoji`, as Unicode writes them, a space apart, so a
// joined or toned emoji keeps every one.
function points(emoji) {
    const each = [];
    for (const one of emoji) {
        const hex = one.codePointAt(0).toString(HEX).toUpperCase();
        each.push(`U+${"0".repeat(Math.max(0, DIGITS - hex.length))}${hex}`);
    }
    return each.join(" ");
}
