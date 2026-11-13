// Joel's words in the launcher's talk fields, kept in the shell's state
// folder so a shell started again, as a switch does, gives them back
// (Joel, 2026-11-12 16:00: "had it disappear through a quickshell switch
// losing what i was typing"). launcher/Drafts.js says what each change
// makes of them; this holds them and writes them.
//
// The file is read once, as the shell starts, and written whole: a quarter
// of a second after the last change, so typing writes nothing per key; at
// once when words are sent or the field is put away; and, blocking, as the
// shell stops. Each write but the last is off the shell's thread, and one
// wanted while another is still going follows once that one is saved, so
// the last state is always the last written.
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Drafts.js" as Drafts

Scope {
    id: root

    property var state: Drafts.empty()

    // What the field for `key` opens with: { text, caret }.
    function kept(key) {
        return Drafts.kept(state, key, Date.now());
    }

    // The field for `key` now reads `text`, its caret at `caret`.
    function typed(key, text, caret) {
        state = Drafts.typed(state, key, { text, caret }, Date.now());
        settling.restart();
    }

    // Return sent `text` to `key`: kept until his talk says it was delivered.
    function sent(key, text) {
        state = Drafts.sent(state, key, text, Date.now());
        write();
    }

    // What each talk has reached (Orchestrators' talks, by address); words
    // delivered go, and nothing is written when none did.
    function settle(talks) {
        const settled = Drafts.settled(state, talks ?? {});
        if (settled !== state) {
            state = settled;
            write();
        }
    }

    // The field put away: anything still waiting on the quarter second is
    // written now.
    function flush() {
        if (settling.running) {
            settling.stop();
            write();
        }
    }

    // Long enough that a word typed at his pace is one write, short enough
    // that a switch a moment after he stops typing still finds it.
    Timer {
        id: settling
        interval: 250
        onTriggered: root.write()
    }

    property bool writing: false
    property bool wanted: false
    function write() {
        if (writing) {
            wanted = true;
            return;
        }
        wanted = false;
        // What the file already holds is never written again: FileView says no saved for
        // it, which left every write after it waiting (Quickshell 0.3.2, tried 2026-11-12).
        const text = JSON.stringify(state);
        if (text === file.text())
            return;
        writing = true;
        file.setText(text);
    }
    function written() {
        writing = false;
        if (wanted)
            write();
    }

    // Said once a shell, since every change would say it again.
    property bool failSaid: false

    FileView {
        id: file
        path: Quickshell.statePath("launcher-drafts.json")
        blockLoading: true
        blockWrites: false
        printErrors: false
        onSaved: root.written()
        onSaveFailed: error => {
            if (!root.failSaid) {
                root.failSaid = true;
                console.warn(`launcher: couldn't keep what's typed to talk: ${FileViewError.toString(error)}`);
            }
            root.written();
        }
    }

    Component.onCompleted: state = Drafts.read(file.text(), Date.now())

    // As the shell stops, what's still waiting is written before it goes.
    Component.onDestruction: {
        if (settling.running || wanted) {
            settling.stop();
            file.blockWrites = true;
            file.setText(JSON.stringify(state));
        }
    }
}
