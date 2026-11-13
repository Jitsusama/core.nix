// What the card of Joel's line to the one standing at Agents' top says,
// the void above all, and when it changes (VKCF3Z, 2026-11-12 [a17c2491]):
// sent the moment he presses Return, then that same card answered, in
// place, once the talk is replied to. Told.qml sends it with notify-send;
// the notices daemon words "sent" and "answered" from its key and signs it
// by the name it gives.
.pragma library

// A card holds about 40 of the alphabet's average letters a line, 6 lines
// (Notifications.qml's letters, NoticeRow's maximumLineCount). 36 leaves
// room for capitals and wide letters, which Qt's own wrap would push onto
// a seventh line it elides with a mark Joel's words never use.
const LETTERS = 36;
const LINES = 6;

const SPACES = /\s+/u;

// Whether Told makes a card at all. It makes none, sent, answered or taken
// back: Joel found notices popping up outside the line he typed in weird
// (2026-11-13 04:33), so the void gets one place, the Super+S line, whose
// answers are to show inside it. Until they do, his answer reads on the
// void's row in Agents. Set it true to card them again.
const CARDED = false;

// Whether a card is made for Joel's line, as CARDED says.
function carded() {
    return CARDED;
}

// A reply as the card says it: whole, when it fits, else its words to the
// card's last line and that line ending "... more in the talk", never a
// word cut (VKCF3Z, 2026-11-12 [24c972ba]). The whole stays in the talk.
function cut(reply, more) {
    const words = String(reply ?? "").trim().split(SPACES).filter(word => word !== "");
    const lines = laid(words);
    if (lines.length <= LINES) {
        return words.join(" ");
    }
    const ending = `... ${more}`;
    const kept = [].concat(...lines.slice(0, LINES - 1));
    const last = lines[LINES - 1].slice();
    while (last.length > 0 && `${last.join(" ")} ${ending}`.length > LETTERS) {
        last.pop();
    }
    return kept.concat(last, [ending]).join(" ");
}

// Words laid greedily into lines of LETTERS, as Qt wraps at their
// boundaries; one longer than a line takes the lines it fills, broken
// where Qt breaks it.
function laid(words) {
    const lines = [];
    let line = [];
    for (const word of words) {
        if (line.concat([word]).join(" ").length <= LETTERS) {
            line.push(word);
        } else {
            if (line.length > 0) {
                lines.push(line);
            }
            const spans = Math.ceil(word.length / LETTERS);
            for (let filled = 1; filled < spans; filled += 1) {
                lines.push(filled === 1 ? [word] : []);
            }
            line = spans > 1 ? [] : [word];
        }
    }
    if (line.length > 0) {
        lines.push(line);
    }
    return lines;
}

// The void's reply says first where his line went, as "→ Delta · undo
// 22:54" when it can be undone, by the time its day file keeps the line
// at (8GYNJ6, 2026-11-12 [e7d88599]); a to-do for him, or a line it kept,
// can't be.
const UNDOABLE = /^(→ .*?) · undo (\d{1,2}:\d{2})$/u;

// A reply as the card says it, where it went kept and the undo taken off
// it to be an act of the card's (VKCF3Z, 2026-11-12 [244ed4fd]), and the
// time the void undoes it by, or "" where it can't.
function went(reply) {
    const lines = String(reply ?? "").split("\n");
    const first = lines[0].trim().match(UNDOABLE);
    if (first === null) {
        return { reply: String(reply ?? ""), undo: "" };
    }
    return { reply: [first[1]].concat(lines.slice(1)).join("\n"), undo: first[2] };
}

// What the void is told, as Joel's own line, to take his line back.
function undoing(time) {
    return `undo ${time}`;
}

const UNDONE = /^undone\.?$/iu;

// How the card stands once the void answers being asked to take it back:
// taken, on its own "undone"; else told what it said instead, in the
// words a side holds, so it's never a silent card.
function takenBy(answer) {
    const said = String(answer ?? "").trim();
    if (UNDONE.test(said)) {
        return { said: "taken", told: "" };
    }
    const words = said.split(SPACES).filter(word => word !== "");
    const line = laid(words)[0] ?? [];
    const told = line.join(" ");
    return { said: "told", told: line.length < words.length ? `${told}...` : told };
}

// The harness stamps delivery by its own clock, which may run a little
// behind the shell's.
const SLACK_MS = 2000;

// The talk Joel's line began, as Orchestrators.qml folds a session's
// talks: the one saying his words, delivered once he'd sent them; or null
// before it's delivered.
function talkOf(talks, line, since) {
    return (talks ?? []).find(talk => talk.words === line
        && Date.parse(talk.at) >= since - SLACK_MS) ?? null;
}

// The reply a talk has given, once it's given one whole, or "": one still
// being written isn't a reply yet.
function replyOf(talk) {
    if (talk === null || talk.reply === "") {
        return "";
    }
    return ["replied", "over", "read"].includes(talk.stage) ? talk.reply : "";
}

const AMPERSANDS = /&/gu;
const OPENINGS = /</gu;
const CLOSINGS = />/gu;

// What a notice's body holds of words as the bus takes them, which reads
// markup.
function plain(text) {
    return text.replace(AMPERSANDS, "&amp;").replace(OPENINGS, "&lt;").replace(CLOSINGS, "&gt;");
}

// notify-send's command for the card as it stands: from the name it signs
// by, his line, how it stands and, once answered, the reply cut to the
// card; low, so it never takes his eye or the keys, and in place of the
// card it was given the id of. It waits for its default action, which
// opens the talk, and for Take It Back while the void can undo it, so it
// runs while the card is up. Told, it carries what the void said.
function command(card) {
    return ["notify-send", "--print-id", "--app-name=pi", "--urgency=low"]
        .concat(card.id > 0 ? [`--replace-id=${card.id}`] : [])
        .concat([`--hint=string:x-from:${card.from}`, `--hint=string:x-said:${card.said}`])
        .concat(card.said === "told" ? [`--hint=string:x-told:${card.told}`] : [])
        .concat([`--action=default=${card.open}`])
        .concat((card.undo ?? "") === "" ? [] : [`--action=take=${card.takes}`])
        .concat(["--", card.line])
        .concat(card.reply === "" ? [] : [plain(card.reply)]);
}
