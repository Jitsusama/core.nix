// What On the Selection works out, apart from the place that draws it
// (OnSelection.qml), so Qt's engine can try each case.
.pragma library

// Runs of white space, which a head says as one space.
const spaces = /\s+/gu;
// What a head given way at a word leaves at its end, dropped before its
// mark.
const trailing = /[\s,;:.]+$/u;
// The first sentence of an answer: up to a stop that ends a word.
const sentence = /^[\s\S]*?[.!?](?=\s|$)/u;

// The selected text as the place's head says it: on one line, its spaces
// run together, given way at the last whole word that fits in `most`
// characters, with a mark that more was selected (VKCF3Z, e684d7ae: "the
// head one line giving way at a whole word, never a wrap"). A first word
// longer than `most` is cut there, since a head of only the mark says
// nothing.
function headOf(text, most) {
    const line = String(text ?? "").replace(spaces, " ").trim();
    if (line.length <= most) {
        return line;
    }
    const cut = line.slice(0, most + 1);
    const at = cut.lastIndexOf(" ");
    return `${(at > 0 ? cut.slice(0, at) : line.slice(0, most)).replace(trailing, "")}\u2026`;
}

// An answer's first sentence, which names its row while the whole of it
// is read in the drawer, as Ask's reply does.
function firstSentence(whole) {
    const text = String(whole ?? "").trim();
    return (text.match(sentence) ?? [text])[0];
}

// The daemon's records in the order it served them: those of its whole
// answer as it listed them, then any it told of since, in the order told.
function served(order, held) {
    const listed = order.filter(key => held[key] !== undefined);
    const told = Object.keys(held).filter(key => !order.includes(key));
    return listed.concat(told).map(key => held[key]);
}

// How an answer stands, by Ask's stage, as its row's note names it from
// the words: `said` holds asking, answering, answered, copied and failed.
function noted(stage, result, said) {
    if (stage === "answered") {
        return result === "replace" ? said.copied ?? "" : said.answered ?? "";
    }
    if (stage === "failed" || stage === "stopped") {
        return said.failed ?? "";
    }
    return stage === "answering" ? said.answering ?? "" : said.asking ?? "";
}
