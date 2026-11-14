// How the launcher asks once before what costs a lot and can't be put
// back: what it asks, which keys answer, and how soon. It's answered as
// an armed row is (Arming.js): the Return or d that asked was the first
// press, a second Return does it, and any other key lets it go, never
// y or n (Joel, 2026-11-13: "having to say y/n for launcher items when
// others allow a double enter is stupid"). Where the ask is drawn is the
// launcher's to say. Read apart from it so tests/query-keys.qml can hold
// it in Qt's own engine.
.pragma library

// An ask standing: what it asks, whole and in its parts (questionParts),
// what Return does and what letting it go does, and when it was put, in
// ms. `asks` is its words, or its parts where it says what stays.
function asked(asks, yes, no, now) {
    const whole = typeof asks === "string" ? { head: asks, clause: "", tail: "" } : asks;
    return { question: joined(whole), parts: whole, yes, no, at: now };
}

// An ask's parts said whole, a space between each there is.
function joined(ask) {
    return [ask.head, ask.clause, ask.tail].filter(part => part !== "").join(" ");
}

// An ask as it fits where `fits(words)` says it's drawn: whole, else its
// clause saying what stays giving way from its end a whole word at a time
// behind an ellipsis, so what it does, to what, and its keys are always
// read, as "Close TESTQC? Its folder\u2026" (VKCF3Z, 2026-11-07). One
// with no clause is said whole, and its line cuts it.
function fitted(ask, fits) {
    const whole = joined(ask);
    if (ask.clause === "" || fits(whole)) {
        return whole;
    }
    const words = ask.clause.split(" ");
    for (let count = words.length - 1; count > 0; count -= 1) {
        const shorter = `${ask.head} ${words.slice(0, count).join(" ")}\u2026${ask.tail}`;
        if (fits(shorter)) {
            return shorter;
        }
    }
    return `${ask.head} \u2026${ask.tail}`;
}

// The keys that answer yes: Return, the main one or the keypad's.
const YES = ["Return", "Enter"];

// What a key says to an ask: "early" in its first `guard` ms, so the
// Return that asked, doubled or bounced, and a key typed on from it
// neither answer nor reach the line, or for no key at all, as a modifier
// alone; "yes" for Return; "stands" for y, which weeks of (y/n) left in
// the hand, so a press from habit leaves the ask standing rather than
// losing the act (VKCF3Z, 2026-11-14; y goes back to "no" on
// 2026-12-14); and "no" for any other, Escape and - among them, as any
// other key lets an armed row go.
function answer(ask, key, now, guard) {
    if (key === "" || now - ask.at < guard) {
        return "early";
    }
    if (key === "y") {
        return "stands";
    }
    return YES.includes(key) ? "yes" : "no";
}

// What d asks over the rows it would end, each { ends, name, kind }: the
// one row by its name, as "Close Notes?", and more by their count and
// kind, each verb with its own in the order they come, as "Close 2
// windows and forget 1 copy?", so Joel needn't look to see what they are
// (VKCF3Z, 2026-11-05). One row that says what its ending leaves
// (`leaves`) says it after, as "Close Lamplight? Its worktree stays.", so
// ending a session never reads as losing its work, begun as the sentence
// it is. One row with words of its own for the ask (`asked`) is asked in
// them whole: the act on its thing, then what it ends, as "Reboot the
// Machine, Closing 12 Windows?", never a place's name joined to the act's
// (VKCF3Z, 2026-11-08). The foot says what Return does, as an armed
// row's does.
function question(rows, words) {
    return joined(questionParts(rows, words));
}

// What an act asked from its own row is an act of, for its ask: the
// thing holding it, as Disconnect's is the Keychron or Close's a window,
// but never a place, so Power's Reboot asks of itself rather than
// "Reboot Power" (VKCF3Z, 2026-11-08). `holderKind` is the holder's kind,
// as the catalogue says it; the : line and launcher do both ask so.
function actOf(holder, act, holderKind) {
    return holder && holderKind !== "place" ? holder : act;
}

// What a served act's ask says of it, where its daemon serves that: whom
// it names (`askedAs`) and what stays once it's done (`leaves`), as
// Agents' Close says "Close 8IDU4E? Its worktree and 1 to-do stay."; and
// whether it costs enough to ask at all (`costly`, Operators.asks), read
// once a daemon serves it, as agentsd does of a Close by its work in
// flight. Nothing for one that serves none of
// them (ReadyRows.qml).
function asking(act) {
    const leaves = String(act?.leaves ?? "");
    const costly = typeof act?.costly === "boolean" ? { costly: act.costly } : {};
    return Object.assign(leaves === "" ? {} : { askedAs: String(act.askedAs ?? ""), leaves },
        costly);
}

// The same ask in its parts: its head, what it does and to what; the
// clause saying what stays, where one does; and the keys after it, so
// fitted can give the clause way alone.
function questionParts(rows, words) {
    const left = rows.length === 1 ? String(rows[0].leaves ?? "") : "";
    const clause = left.charAt(0).toUpperCase() + left.slice(1);
    const capital = text => text.charAt(0).toUpperCase() + text.slice(1);
    if (rows.length !== 1) {
        const many = (words.asks ?? "").replace("{what}", listed(parts(rows, words), words));
        return { head: capital(many), clause: "", tail: "" };
    }
    const own = String(rows[0].asked ?? "");
    const filled = text => text
        .replace("{ends}", own !== "" || saysItself(rows[0], words) ? "" : verbOf(rows[0], words))
        .replace("{name}", own !== "" ? own : rows[0].name ?? "").trim();
    if (clause === "") {
        return { head: capital(filled(words.ask ?? "")), clause: "", tail: "" };
    }
    const [before, after] = (words.askLeaves ?? "{leaves}").split("{leaves}");
    return { head: capital(filled(before)), clause, tail: filled(after ?? "") };
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
