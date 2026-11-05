// How the launcher asks once before what can't be put back, as Vim's
// confirm() asks: what it asks, which keys answer, and how soon. Where the
// ask is drawn is the launcher's to say. Read apart from it so
// tests/query-keys.qml can hold it in Qt's own engine.
.pragma library

// An ask standing: what it asks, what y does and what any other key
// does, and when it was put, in ms.
function asked(asks, yes, no, now) {
    return { question: asks, yes, no, at: now };
}

// What a key says to an ask: "early" in its first `guard` ms, so a key
// typed on from what asked can't answer it, or for no key at all, as a
// modifier alone; "yes" for y alone; "no" for any other, as Escape, since
// confirm()'s answer is no unless it's yes.
function answer(ask, key, now, guard) {
    if (key === "" || now - ask.at < guard) {
        return "early";
    }
    return key === "y" ? "yes" : "no";
}

// What d asks over the rows it would end, each { ends, name }: the one
// row by its name, more by their count, the verb the one they share or
// the words' own where they differ, as "Close Notes? (y/n)".
function question(rows, words) {
    const shared = rows.every(row => row.ends === rows[0].ends) && (rows[0].ends ?? "") !== "";
    const said = (rows.length === 1 ? words.ask ?? "" : words.asks ?? "")
        .replace("{ends}", shared ? rows[0].ends : words.ends ?? "")
        .replace("{name}", rows[0].name ?? "")
        .replace("{count}", String(rows.length));
    return said.charAt(0).toUpperCase() + said.slice(1);
}
