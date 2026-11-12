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

// A reply as the card says it: whole, when it fits, else its words to the
// card's last line and that line ending "... more in the talk", never a
// word cut (VKCF3Z, 2026-11-12 [24c972ba]). The whole stays in the talk.
function cut(reply, more) {
    const words = String(reply ?? "").trim().split(/\s+/).filter(word => word !== "");
    const lines = laid(words);
    if (lines.length <= LINES)
        return words.join(" ");
    const ending = `... ${more}`;
    const kept = [].concat(...lines.slice(0, LINES - 1));
    const last = lines[LINES - 1].slice();
    while (last.length > 0 && `${last.join(" ")} ${ending}`.length > LETTERS)
        last.pop();
    return kept.concat(last, [ending]).join(" ");
}

// Words laid greedily into lines of LETTERS, as Qt wraps at their
// boundaries; one longer than a line takes the lines it fills, broken
// where Qt breaks it.
function laid(words) {
    const lines = [];
    let line = [];
    for (const word of words) {
        const next = line.concat([word]).join(" ");
        if (next.length <= LETTERS) {
            line.push(word);
            continue;
        }
        if (line.length > 0)
            lines.push(line);
        line = [word];
        for (let spans = Math.ceil(word.length / LETTERS); spans > 1; spans--) {
            lines.push(line);
            line = [];
        }
    }
    if (line.length > 0)
        lines.push(line);
    return lines;
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
    if (talk === null || talk.reply === "")
        return "";
    return ["replied", "over", "read"].includes(talk.stage) ? talk.reply : "";
}

// What a notice's body holds of words as the bus takes them, which reads
// markup.
function plain(text) {
    return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// notify-send's command for the card as it stands: from the name it signs
// by, his line, how it stands and, once answered, the reply cut to the
// card; low, so it never takes his eye or the keys, and in place of the
// card it was given the id of. It waits for its default action, which
// opens the talk, so it runs while the card is up.
function command(card) {
    return ["notify-send", "--print-id", "--app-name=pi", "--urgency=low"]
        .concat(card.id > 0 ? [`--replace-id=${card.id}`] : [])
        .concat([`--hint=string:x-from:${card.from}`, `--hint=string:x-said:${card.said}`,
            `--action=default=${card.open}`, "--", card.line])
        .concat(card.reply === "" ? [] : [plain(card.reply)]);
}
