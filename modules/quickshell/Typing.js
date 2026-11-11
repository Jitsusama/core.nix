// What the launcher's field's keys do to the line itself, apart from which
// key does what (QueryKeys.js): the word Ctrl+W takes, and where the caret
// goes as a mode is entered and where it rests in normal, each as Vim does
// it; and the note the line says after what's typed. Read apart from the
// field (Prompt.qml) so tests/query-keys.qml can hold it in Qt's own engine.
.pragma library

// What Vim's iskeyword counts as a word's: letters, digits, the underscore
// and the accented letters past them.
const wordLetter = /[0-9A-Za-z_\u00c0-\u024f]/;
const space = /\s/;
const notSpace = /\S/;

function wordly(letter) {
    return wordLetter.test(letter);
}

// Where the word before the caret starts, as Vim's Ctrl+W deletes back to:
// past the spaces before it, then a run of a word's letters, or else a run
// of anything else but spaces.
function wordStart(text, at) {
    let start = at;
    while (start > 0 && space.test(text[start - 1])) {
        start -= 1;
    }
    if (start === 0) {
        return 0;
    }
    const kind = wordly(text[start - 1]);
    while (start > 0 && !space.test(text[start - 1]) && wordly(text[start - 1]) === kind) {
        start -= 1;
    }
    return start;
}

// Where the caret goes as a mode is entered, as Vim moves it: one left
// leaving insert, since normal's block sits on a letter and insert's bar
// between two; at it, after it, at the first letter or at the end, going
// into insert.
function caretAfter(act, text, at) {
    switch (act) {
    case "normal":
        return Math.max(0, at - 1);
    case "append":
        return Math.min(text.length, at + 1);
    case "insertStart": {
        const first = text.search(notSpace);
        return first < 0 ? text.length : first;
    }
    case "appendEnd":
        return text.length;
    default:
        return at;
    }
}

// Where the caret may rest in normal: on a letter, never past the last.
function resting(text, at) {
    return Math.max(0, Math.min(at, text.length - 1));
}

// The note on the line's baseline after what's typed: how the place it's
// in stands where that presses, as "not answering", or else how far a find
// narrowed the rows where that's said (`narrowing`). Pressing comes first,
// since a found count from a place that can't answer would claim more than
// it knows (VKCF3Z, 2026-11-10).
function noteOf(pressing, narrowed, narrowing) {
    if (pressing !== "") {
        return pressing;
    }
    return narrowing ? narrowed : "";
}
