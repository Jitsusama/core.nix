// The shell's Dictation (modules/dictation/Dictation.qml), run by Quickshell
// with nothing drawn: it says each state it comes to over a socket of the
// test's, so tests/dictation-watch.py, standing in for the service, can
// tell what the shell heard the moment it heard it. Quickshell's own log
// waits in a buffer, so it can't say that.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    Socket {
        id: report
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/report.sock`
        connected: true
    }

    Connections {
        target: Dictation

        function onStateChanged() {
            report.write(`state ${Dictation.state}\n`);
            report.flush();
        }
    }
}
