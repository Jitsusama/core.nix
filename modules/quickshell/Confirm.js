// How the launcher asks once before what can't be put back, as Vim's
// ask_yesno asks: what it asks, which keys answer, and how soon. Where the
// ask is drawn is the launcher's to say. Read apart from it so
// tests/query-keys.qml can hold it in Qt's own engine.
.pragma library

// An ask standing: what it asks, what y does and what n does, and when it
// was put, in ms.
function asked(asks, yes, no, now) {
    return { question: asks, yes, no, at: now };
}

// The keys that answer no, as Vim's ask_yesno takes them.
const NO = ["n", "Escape", "Ctrl+c"];

// What a key says to an ask: "early" in its first `guard` ms, so a key
// typed on from what asked can't answer it, or for no key at all, as a
// modifier alone; "yes" for y alone; "no" for n, Escape and Ctrl+C; and
// "nothing" for any other, which ask_yesno lets by, so a stray key never
// answers.
function answer(ask, key, now, guard) {
    if (key === "" || now - ask.at < guard) {
        return "early";
    }
    if (key === "y") {
        return "yes";
    }
    return NO.includes(key) ? "no" : "nothing";
}

// What d asks over the rows it would end, each { ends, name, kind }: the
// one row by its name, as "Close Notes? (y/n)", and more by their count
// and kind, each verb with its own in the order they come, as "Close 2
// windows and forget 1 copy? (y/n)", so Joel needn't look to see what
// they are (VKCF3Z, 2026-11-05). One row that says what its ending
// leaves (`leaves`) says it before y/n, as "Close Lamplight? Its worktree
// stays. (y/n)", so ending a session never reads as losing its work,
// begun as the sentence it is.
function question(rows, words) {
    const left = rows.length === 1 ? String(rows[0].leaves ?? "") : "";
    const leaves = left.charAt(0).toUpperCase() + left.slice(1);
    const said = rows.length === 1
        ? (leaves !== "" ? words.askLeaves ?? "" : words.ask ?? "")
            .replace("{ends}", saysItself(rows[0], words) ? "" : verbOf(rows[0], words))
            .replace("{name}", rows[0].name ?? "").replace("{leaves}", leaves).trim()
        : (words.asks ?? "").replace("{what}", listed(parts(rows, words), words));
    return said.charAt(0).toUpperCase() + said.slice(1);
}

// Whether a row's name already begins with what's done to it, as Forget
// the Clipboard does, so the ask names it alone rather than "Forget
// Forget the Clipboard".
function saysItself(row, words) {
    const verb = verbOf(row, words).toLowerCase();
    const first = String(row.name ?? "").split(" ")[0].toLowerCase();
    return verb !== "" && first === verb;
}

// What a row's ending does, or the words' own where it says none.
function verbOf(row, words) {
    return (row.ends ?? "") !== "" ? row.ends : words.ends ?? "";
}

// The rows by what's done to them and their kind, each as "close 2
// windows", in the order each first comes; a row of no kind is a thing.
function parts(rows, words) {
    const groups = [];
    for (const row of rows) {
        const ends = verbOf(row, words);
        const kind = (row.kind ?? "") !== "" ? row.kind : words.thing ?? "";
        const group = groups.find(one => one.ends === ends && one.kind === kind);
        if (group) {
            group.count += 1;
        } else {
            groups.push({ ends, kind, count: 1 });
        }
    }
    return groups.map(group => (words.part ?? "").replace("{ends}", group.ends)
        .replace("{count}", String(group.count))
        .replace("{kind}", group.count === 1 ? group.kind : plural(group.kind, words)));
}

// A kind's word for more than one, as the words have it, or with an s.
function plural(kind, words) {
    return words.many?.[kind] ?? `${kind}s`;
}

// Parts said as a list: commas between, and before the last.
function listed(said, words) {
    if (said.length < 2) {
        return said.join("");
    }
    return `${said.slice(0, -1).join(", ")} ${words.and ?? ""} ${said[said.length - 1]}`;
}
