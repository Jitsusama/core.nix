// A line that's typed in, edited as Vim's command line and insert edit
// one: the find, the command line and a reply's prompt on the held notices
// (NoticeKeys.js reads which keys are a mode's own first), and where Vim's
// w, b and e go in its words. Apart from the shell, so tests/notice-keys.qml
// holds it in Qt's own engine.
.pragma library

const space = /\s/;
// The word before a caret and the space after it, which Ctrl+W takes.
const lastWord = /\S+\s*$|\s+$/;

// A line as it starts: empty, or holding what's given with the caret at
// its end. `register` says Ctrl+R waits for a register to put.
function line(text, caret) {
    return { text: text ?? "", caret: caret ?? (text ?? "").length, register: false };
}

// The line after an editing key: "leave" for a Backspace on an empty line,
// which only a fresh press does, so a held Backspace stops at empty;
// "paste" for Ctrl+R then + or *, the clipboard; otherwise no act, and
// the line as the key leaves it.
function edit(typed, key, repeated) {
    const { text, caret } = typed;
    const edited = (words, at) => ({ act: "", line: { text: words, caret: at, register: false } });
    if (typed.register) {
        return { act: key === "+" || key === "*" ? "paste" : "",
            line: { text, caret, register: false } };
    }
    switch (key) {
    case "Backspace":
    case "Ctrl+H":
        if (text === "") {
            return { act: repeated ? "" : "leave", line: typed };
        }
        return caret === 0 ? edited(text, caret)
            : edited(text.slice(0, caret - 1) + text.slice(caret), caret - 1);
    case "Delete":
        return edited(text.slice(0, caret) + text.slice(caret + 1), caret);
    case "Ctrl+W": {
        const before = text.slice(0, caret).replace(lastWord, "");
        return edited(before + text.slice(caret), before.length);
    }
    case "Ctrl+U":
        return edited(text.slice(caret), 0);
    case "Left":
        return edited(text, Math.max(caret - 1, 0));
    case "Right":
        return edited(text, Math.min(caret + 1, text.length));
    case "Home":
        return edited(text, 0);
    case "End":
        return edited(text, text.length);
    case "Ctrl+R":
        return { act: "", line: { text, caret, register: true } };
    case "Space":
        return edited(`${text.slice(0, caret)} ${text.slice(caret)}`, caret + 1);
    default:
        // A letter types itself; any other key changes nothing.
        return key.length === 1
            ? edited(text.slice(0, caret) + key + text.slice(caret), caret + 1)
            : { act: "", line: typed };
    }
}

// Where the next word starts after the caret, as Vim's w goes, or the
// line's end.
function nextWord(text, caret) {
    let at = caret;
    while (at < text.length && !space.test(text[at])) {
        at += 1;
    }
    while (at < text.length && space.test(text[at])) {
        at += 1;
    }
    return at;
}

// Where the word before the caret starts, as Vim's b goes.
function wordBefore(text, caret) {
    let at = caret - 1;
    while (at > 0 && space.test(text[at])) {
        at -= 1;
    }
    while (at > 0 && !space.test(text[at - 1])) {
        at -= 1;
    }
    return Math.max(at, 0);
}

// Where this word ends, or the next where the caret ends one, as Vim's e.
function wordEnd(text, caret) {
    let at = caret + 1;
    while (at < text.length && space.test(text[at])) {
        at += 1;
    }
    while (at + 1 < text.length && !space.test(text[at + 1])) {
        at += 1;
    }
    return at;
}

// Whether the caret stands on a space, or past the end.
function onSpace(text, caret) {
    return space.test(text[caret] ?? " ");
}
