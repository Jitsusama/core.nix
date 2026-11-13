// What became of Joel's line to the void, as the Super+S line reads it
// (TalkLine.js): the talk his line began, the reply it gave, where the
// void says it went, and how a take-back stands. It carded his line once;
// Joel found notices popping up outside the line he typed in weird
// (2026-11-13 04:33), so the void has one place, the line.
.pragma library

// A line of the talk line holds about 36 of the alphabet's letters, room
// left for capitals and wide letters.
const LETTERS = 36;

const SPACES = /\s+/u;

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

// The void's reply says first where his line went, as "Your orchestrator
// has it · undo 07:43" (once "→ Delta · undo 22:54") when it can be
// undone, by the time its day file keeps the line at (8GYNJ6, 2026-11-12
// [e7d88599]; QCXZEB, 2026-11-13 [31907e55]); a to-do for him, or a line
// it kept, can't be.
const UNDOABLE = /^(.*?) · undo (\d{1,2}:\d{2})$/u;

// A reply as the line says it, where it went kept and the undo taken off
// it to be an act of its own (VKCF3Z, 2026-11-12 [244ed4fd]), and the
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

// How a take-back stands once the void answers it: taken, on its own
// "undone"; else told what it said instead, in the words a line holds, so
// it's never silent.
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
