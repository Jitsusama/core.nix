// A key pressed, named as the launcher's key table names keys
// (QueryKeys.js), from what the press says rather than a KeyEvent, so
// tests/query-keys.qml holds it in Qt's own engine.
.pragma library

// A key typing one of the letters, digits and marks the table names keys by.
const printable = /^[!-~]$/;

// A key pressed as the table names it, from what the press says
// (`pressed`): whether Ctrl, Shift and Alt were held, the text it typed,
// the letter its key is on the layout ("" for a key that isn't a letter's)
// and the name of a key that types nothing. With Ctrl a letter types a
// control character, so it goes by its key. "" for a key the table can't
// name, which is the field's own.
function nameOf(pressed) {
    const bare = !pressed.control && printable.test(pressed.text);
    const letter = bare ? pressed.text : pressed.control ? pressed.letter : "";
    const name = letter || pressed.named;
    if (name === "") {
        return "";
    }
    // Alt is named beside a key that types nothing, so Alt+Return is
    // never Return, and beside a letter it typed as the key's own, so
    // Alt+d is Vim's M-d and never d; a letter it shifts to another on the
    // layout goes by what it typed.
    const own = bare && pressed.letter !== "" && pressed.text.toLowerCase() === pressed.letter;
    return (pressed.control ? "Ctrl+" : "") + (pressed.alt && (!letter || own) ? "Alt+" : "")
        + (pressed.shift && !bare ? "Shift+" : "") + name;
}
