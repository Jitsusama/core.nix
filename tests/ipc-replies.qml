// Whether a long IPC reply reaches its caller whole, in the Quickshell the
// machine runs (modules/quickshell/ipc-replies-whole.patch). The
// `an-ipc-reply-arrives-whole` check runs this as the shell, with QS
// naming its qs: the shell asks itself, through `qs ipc call`, for replies
// of a few lengths, the longest past what a socket's buffer takes at once,
// then for a function it hasn't got, and exits 1 for a reply that comes
// back short or a failed call that qs says succeeded.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    // Characters asked for: under the buffer, then past it twice over, as
    // the launcher's catalogue on a busy machine is.
    readonly property var lengths: [1000, 400000, 1000000]
    property int at: 0

    IpcHandler {
        target: "replies"

        function of(length: int): string {
            return "x".repeat(length);
        }
    }

    Process {
        id: caller
        command: ["sh", "-c", `"$QS" ipc -p shell.qml call replies of ${root.lengths[root.at]}`
            + " | tr -d '\\n' | wc -c"]
        stdout: StdioCollector {
            id: said
        }
        onExited: Qt.callLater(() => root.heard(Number(said.text.trim())))
    }

    function heard(length) {
        const wanted = lengths[at];
        if (length !== wanted) {
            console.warn(`FAIL a reply of ${wanted} characters came back as ${length}`);
            Qt.exit(1);
            return;
        }
        at += 1;
        if (at === lengths.length)
            missing.running = true;
        else
            caller.running = true;
    }

    // A call that fails ends qs with a failure, so a script can tell.
    Process {
        id: missing
        command: ["sh", "-c", "\"$QS\" ipc -p shell.qml call replies nothing"]
        onExited: exitCode => {
            if (exitCode === 0)
                console.warn("FAIL a call to a function that isn't there exited 0");
            Qt.callLater(() => Qt.exit(exitCode === 0 ? 1 : 0));
        }
    }

    // A check that never hears back fails with a reason rather than timing out.
    Timer {
        interval: 30000
        running: true
        onTriggered: {
            console.warn("FAIL the shell never heard its own replies");
            Qt.exit(1);
        }
    }

    Component.onCompleted: Qt.callLater(() => {
        caller.running = true;
    })
}
