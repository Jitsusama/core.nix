// What normal's keys do to a prompt's words, as Vim's w b e 0 $ x dw and
// cw edit a line (VKCF3Z's vi-paradigm spec, Insert and The File Picker).
// QueryKeys.js reads which act a key is, and Typing.js what a word is;
// this says what the act does to the words. Plain JavaScript, held to it
// by tests/query-keys.qml in Qt's own engine.
.pragma library
.import "Typing.js" as Typing

const space = /\s/;

// Where the next word starts after the caret, as Vim's w goes: past the
// rest of the word it's on, or of the run of other letters, then past the
// spaces after it; the line's end if no word follows.
function nextWordStart(text, at) {
    let place = at;
    if (place < text.length && !space.test(text[place])) {
        const kind = Typing.wordly(text[place]);
        while (place < text.length && !space.test(text[place])
            && Typing.wordly(text[place]) === kind) {
            place += 1;
        }
    }
    while (place < text.length && space.test(text[place])) {
        place += 1;
    }
    return place;
}

// Where the word the caret's on ends, the place after its last letter, as
// Vim's cw changes up to; on a space, the next word's start, as dw takes.
function runEnd(text, at) {
    if (at >= text.length || space.test(text[at])) {
        return nextWordStart(text, at);
    }
    const kind = Typing.wordly(text[at]);
    let place = at;
    while (place < text.length && !space.test(text[place])
        && Typing.wordly(text[place]) === kind) {
        place += 1;
    }
    return place;
}

// The last letter of the word after the caret, as Vim's e goes: on at
// least one letter, past any spaces, to the end of the run it lands in.
function wordEndAfter(text, at) {
    let place = at + 1;
    while (place < text.length && space.test(text[place])) {
        place += 1;
    }
    if (place >= text.length) {
        return Math.max(0, text.length - 1);
    }
    return runEnd(text, place) - 1;
}

// Takes a step count times, each from where the last left off.
function stepped(step, at, times) {
    let place = at;
    for (let made = 0; made < times; made += 1) {
        place = step(place);
    }
    return place;
}

// What a words act in normal does to a prompt's line: { text, at }, the
// words after it and the caret's place, as Vim's h l w b e 0 $ x dw and cw do,
// a count taking it that many times. The caret rests on a letter, but for
// cw's, which goes into insert where the word was.
function edited(act, text, at, count) {
    const times = Math.max(1, count);
    const put = (words, caret) => ({ text: words, at: Typing.resting(words, caret) });
    const without = (from, to) => text.slice(0, from) + text.slice(to);
    switch (act) {
    case "letterBack":
        return put(text, at - times);
    case "letterNext":
        return put(text, at + times);
    case "wordNext":
        return put(text, stepped(place => nextWordStart(text, place), at, times));
    case "wordBack":
        return put(text, stepped(place => Typing.wordStart(text, place), at, times));
    case "wordEnd":
        return put(text, stepped(place => wordEndAfter(text, place), at, times));
    case "lineStart":
        return put(text, 0);
    case "lineEnd":
        return put(text, text.length);
    case "deleteLetter":
        return put(without(at, at + times), at);
    case "deleteWordOn":
        return put(without(at, stepped(place => nextWordStart(text, place), at, times)), at);
    case "changeWord": {
        // Each word after the first is counted from past the spaces before
        // it, so c2w takes two words and the space between, not the next.
        const end = stepped(place => runEnd(text, place < text.length
            && place > at ? nextWordStart(text, place - 1) : place), at, times);
        return { text: without(at, end), at };
    }
    default:
        return { text, at };
    }
}
