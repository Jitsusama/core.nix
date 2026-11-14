// Says whether Super+S's line is up, in $XDG_RUNTIME_DIR/quickshell-talk-line, as the
// launcher's open mark does, so a switch restarting the shell waits while Joel is saying
// something (quickshell-takes-new-files): "open" or "closed", written as it changes and once
// as the shell starts; the unit removes it as the shell stops. Open is written off the
// shell's thread, so the line comes up unheld, and closed blocks, so it's said by the time
// the line has gone; a word wanted while another is still being written follows once that
// one is.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: mark

    // Whether the line is up.
    required property bool up

    property string says: ""
    property string writing: ""
    // One it couldn't write leaves a switch reading the last word, so that's said, once a
    // shell, since each time the line comes up would say it again.
    property bool failSaid: false

    function follow() {
        const word = up ? "open" : "closed";
        if (word === says)
            return;
        says = word;
        if (writing === "")
            write();
    }
    function write() {
        const blocking = says !== "open";
        file.blockWrites = blocking;
        writing = blocking ? "" : says;
        file.setText(`${says}\n`);
    }
    function failed(error) {
        if (!failSaid) {
            failSaid = true;
            console.warn(`talk line: couldn't write its mark: ${FileViewError.toString(error)}`);
        }
        written();
    }
    function written() {
        const was = writing;
        writing = "";
        if (was !== "" && was !== says)
            write();
    }

    onUpChanged: follow()
    Component.onCompleted: follow()

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quickshell-talk-line`
        printErrors: false
        onSaved: mark.written()
        onSaveFailed: error => mark.failed(error)
    }
}
