// What the launcher's field's keys do to the line itself, apart from which
// key does what (QueryKeys.js): a search typed on it, the word Ctrl+W takes,
// and where the caret goes as a mode is entered and where it rests in
// normal, each as Vim does it. Read apart from the field (Prompt.qml) so
// tests/query-keys.qml can hold it in Qt's own engine.
.pragma library

// A search being typed, as { pattern, at }, the caret's place in it, after
// an act of the search's, or the letters a key typed where it ends none:
// the line after it, and whether it ends the typing, "found" to run it or
// "cancelled" to put back what the field held, as Vim's / does with
// Return, and with Escape or a Backspace with nothing left to take.
function typedInSearch(line, act, letters) {
    const { pattern, at } = line;
    const put = (text, caret) => ({ pattern: text, at: caret, ends: "" });
    switch (act) {
    case "find":
        return { pattern, at, ends: "found" };
    case "cancel":
        return { pattern, at, ends: "cancelled" };
    case "deleteBack":
        if (pattern === "") {
            return { pattern, at, ends: "cancelled" };
        }
        return at === 0 ? put(pattern, at)
            : put(pattern.slice(0, at - 1) + pattern.slice(at), at - 1);
    case "deleteWord": {
        const start = wordStart(pattern, at);
        return put(pattern.slice(0, start) + pattern.slice(at), start);
    }
    case "deleteToStart":
        return put(pattern.slice(at), 0);
    case "caretLeft":
        return put(pattern, Math.max(0, at - 1));
    case "caretRight":
        return put(pattern, Math.min(pattern.length, at + 1));
    case "caretStart":
        return put(pattern, 0);
    case "caretEnd":
        return put(pattern, pattern.length);
    case "":
        // A key that types a character, as a letter or Space does; one
        // that types nothing, or a control character, changes nothing.
        if (letters === "" || letters < " " || letters === "\u007f") {
            return put(pattern, at);
        }
        return put(pattern.slice(0, at) + letters + pattern.slice(at), at + letters.length);
    default:
        return put(pattern, at);
    }
}

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
