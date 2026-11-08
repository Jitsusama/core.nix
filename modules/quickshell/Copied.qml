pragma Singleton
// What was copied last, as text on one line, for what the shell offers to
// do with a copy: Tuple's Join the Call in the Clipboard (Tuple.qml) and
// mpv's Play the Copied Link (modules/mpv/MpvProgram.qml), which share this
// one watch rather than each starting its own. It's made only once one of
// them asks, so a machine with neither starts nothing.
//
// Each copy is one line: its lines run together with spaces. A copy with no
// text, such as a screenshot, or one marked as a secret, reads as an empty
// line, so nothing copied before it is offered after it's gone. wl-paste
// skips a copy that has no type asked for, so it asks for none and gives
// the command whatever there is, and the command looks at the copy's types
// to know whether that's text.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // The last copy's text, or "" for one with none.
    property string text: ""

    Process {
        running: true
        command: ["wl-paste", "--no-newline", "--watch", "sh", "-c",
            "if [ \"$CLIPBOARD_STATE\" = data ] && wl-paste --list-types"
            + " | grep -q -x -E 'text/.*|TEXT|STRING|UTF8_STRING'"
            + "; then tr '\\n' ' '; else cat > /dev/null; fi; echo"]
        stdout: SplitParser {
            onRead: line => root.text = line
        }
    }
}
