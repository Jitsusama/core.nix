// Whether a key the prompt took acted, from what the key table read it
// as (QueryKeys.read) rather than from its name, so the launcher times a
// source changing beside a key from the last key that did something
// (Launcher.qml's keyHeardAt), and tests/query-keys.qml holds it in Qt's
// own engine.
.pragma library
.import "QueryKeys.js" as QueryKeys

// The line's own edits and caret moves, and Ctrl+Y's completion, which
// change only what's typed and where its caret stands, as a letter typed
// does; but for Ctrl+Y on a command's row in a place, which puts its start
// in the field out at the top (`fillLeaves`), leaving the place.
const typing = ["caretLeft", "caretRight", "caretStart", "caretEnd", "deleteBack", "deleteWord",
    "deleteToStart", "complete"];

// Normal's edits and caret moves on a prompt's words, read only in normal
// (QueryKeys.js, `words`), which change what's typed as insert's do. Not
// cw, which goes into insert as i, a and S do, so acts as they do.
const editing = ["letterBack", "letterNext", "wordNext", "wordBack", "wordEnd", "lineStart",
    "lineEnd", "deleteLetter", "deleteWordOn", "undo"];

// The keys that only say a word: h and l on a row saying the way out and
// in, q how to close, and a page's change keys Vim's E21, which say only
// that on the page: while the drawer chooses, each ends its choice too
// (QueryKeys.modeAfter), so acts there (`saysOnly`).
const saying = ["intoSaid", "outSaid", "quitSaid", "cannotChange"];

function saysOnly(act, line) {
    return saying.includes(act) && !(act === "cannotChange" && QueryKeys.chosen(line.mode));
}

// Whether gv has something to take back: on the rows a selection
// (Launcher.qml's lastVisual, `rows`), and in the drawer the kind it last
// chose by (Prompt.qml's lastChosen, `chosen`); with neither it does
// nothing (Drawer.qml's reselect).
function reselectHere(mode, rows, chosen) {
    return QueryKeys.paged(mode) ? chosen !== "" : rows;
}

// The acts that do nothing where the launcher has nowhere to take them,
// each by the line's word for whether it has: Space or Ctrl+W l into the
// drawer with none out, Tab in normal into a row with no parts, which only
// says so (Launcher.qml's tabbedIn), - with nothing above, and gv on the
// rows with no selection to take back. Where Tab goes is asked of Tab
// alone (`intoHere`), since the launcher runs a search held back first.
function nowhere(act, line) {
    return (act === "drawer" && !line.drawerHere)
        || (act === "into" && line.mode !== "find" && !line.intoHere(line.mode))
        || (act === "out" && !line.upHere)
        || (act === "reselect" && !line.reselectHere);
}

// Whether a key that acted only moved through the rows, so the launcher
// tells it apart (Prompt.qml's keyMoved): the table's moves, and the
// find's Tab where it has no row to go into, which goes on to the next
// row as Down does (Launcher.qml's tabbedIn), asked of Tab alone, in the
// mode it came in (`intoHere`).
function moves(act, line) {
    return QueryKeys.repeats(act)
        || (act === "into" && line.mode === "find" && !line.intoHere(line.mode));
}

// Whether a key acted, given what QueryKeys.read made of it (`read`) and
// how the line stood as it came (`line`): a modifier pressed `alone`, as
// the Shift before a capital, acts on nothing, nor does Ctrl+R's wait for
// a register, or its letter, which types (`registering`), nor the operator
// pressed again after visual's, which is swallowed (`operated`). A key
// while an ask stands (`answering`) acts where the ask takes it as its
// answer (`answerTaken`, Confirm.answer), never as it repeats, even
// visual's operator again, and a colon opening the command
// line (`commands`) acts. Otherwise a key the field types itself, as a
// letter, Shift+Left or Alt+a, a count or a sequence begun, Escape at rest
// and a key meaning nothing where it was pressed act on nothing; nor does
// a key that only says a word, nor one with nowhere to go (`nowhere`), nor
// an edit or a caret move on the line or on a prompt's words, but for
// Backspace on an empty find, which leaves it for normal. Every other act
// acts, moves through the rows too, which the prompt says apart (its
// keyMoved). A key meaning nothing that the selected row takes as its own
// act, Escape at rest backing out of a list or an ask, and a secret's and
// the command line's keys, which the line never reads, act after all: the
// launcher times those itself (Launcher.qml's keyActs).
function acts(read, line) {
    // In the order Prompt.qml's modalKey hands a key on: a register's key,
    // then the ask, which never takes a held key's repeat (`repeated`),
    // then visual's operator.
    if (line.alone || line.registering) {
        return false;
    }
    if (line.answering) {
        return !line.repeated && line.answerTaken;
    }
    if (line.operated) {
        return false;
    }
    if (line.commands) {
        return true;
    }
    if (!read.ours || read.act === "" || saysOnly(read.act, line) || nowhere(read.act, line)) {
        return false;
    }
    if (line.mode === "normal" && editing.includes(read.act)) {
        return false;
    }
    if (read.act === "complete" && line.fillLeaves()) {
        return true;
    }
    const emptied = read.act === "deleteBack" && line.mode === "find" && line.text === "";
    return emptied || !typing.includes(read.act);
}
