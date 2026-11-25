// Whether a key the prompt took acted, from what the key table read it
// as (QueryKeys.read) rather than from its name, so the launcher times a
// source changing beside a key from the last key that did something
// (Launcher.qml's keyHeardAt), and tests/query-keys.qml holds it in Qt's
// own engine.
.pragma library

// The line's own edits and caret moves, which change only what's typed
// and where its caret stands, as a letter typed does.
const typing = ["caretLeft", "caretRight", "caretStart", "caretEnd", "deleteBack", "deleteWord",
    "deleteToStart"];

// Whether a key acted, given what QueryKeys.read made of it (`read`) and
// how the line stood as it came (`line`): a modifier pressed `alone`, as
// the Shift before a capital, acts on nothing, nor does Ctrl+R's wait for
// a register, or its letter, which types (`registering`). A key answering
// what the line asks (`answering`), the operator pressed again after
// visual's (`operated`) and a colon opening the command line (`commands`)
// act. Otherwise a key the field types itself, as a letter, Shift+Left or
// Alt+a, a count or a sequence begun, Escape at rest and a key meaning
// nothing where it was pressed act on nothing; nor does the drawer's key
// with no drawer out (`drawerHere`), nor an edit or a caret move on the
// line, but for Backspace on an empty find, which leaves it for normal.
// Every other act acts, moves through the rows too, which the prompt says
// apart (its keyMoved). Tab into a row with no parts errs the other way,
// read as acting though it opens nothing.
function acts(read, line) {
    if (line.alone || line.registering) {
        return false;
    }
    if (line.answering || line.operated || line.commands) {
        return true;
    }
    if (!read.ours || read.act === "" || (read.act === "drawer" && !line.drawerHere)) {
        return false;
    }
    const emptied = read.act === "deleteBack" && line.mode === "find" && line.text === "";
    return emptied || !typing.includes(read.act);
}
