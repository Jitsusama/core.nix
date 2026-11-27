// Whether a long IPC reply reaches its caller whole, in the Quickshell the
// machine runs (modules/quickshell/ipc-replies-whole.patch). The
// `an-ipc-reply-arrives-whole` check runs this as the shell, with QS
// naming its qs: the shell asks itself, through `qs ipc call`, for replies
// of a few lengths, the longest past what a socket's buffer takes at once,
// then for a function it hasn't got, then for one that answers nothing, and
// exits 1 for a reply that comes back short, a failed call that qs says
// succeeded, or a call answering nothing that qs says failed.
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

        // Answers nothing, as the sessions handler's untakenFor does.
        function told(named: string, why: string): void {
            root.toldWith = `${named} ${why}`;
        }
    }

    property string toldWith: ""

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
            if (exitCode === 0) {
                console.warn("FAIL a call to a function that isn't there exited 0");
                Qt.callLater(() => Qt.exit(1));
            } else {
                voided.running = true;
            }
        }
    }

    // A call that answers nothing ends qs with success, which is how
    // opens-in-sessions.sh tells a shell that took untakenFor from one
    // that has none, falling back to untaken only for the second.
    Process {
        id: voided
        command: ["sh", "-c", "\"$QS\" ipc -p shell.qml call replies told s-1 refused"]
        onExited: exitCode => {
            if (exitCode !== 0)
                console.warn(`FAIL a call that answers nothing exited ${exitCode}`);
            else if (root.toldWith !== "s-1 refused")
                console.warn(`FAIL a call that answers nothing was given "${root.toldWith}"`);
            const passed = exitCode === 0 && root.toldWith === "s-1 refused";
            Qt.callLater(() => Qt.exit(passed ? 0 : 1));
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
