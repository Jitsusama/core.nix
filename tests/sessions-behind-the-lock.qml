pragma ComponentBehavior: Bound
// Drives a session handed over while the screen is locked, beside its
// sender in the sessions' kitty (modules/quickshell/Sessions.qml), with
// niri and kitty's kitten stood in over the record and process
// tests/sessions-behind-the-lock-state.sh lays out. Its window opens in
// that kitty, which makes what kitty said of its windows stale, and the
// lock is let go before kitty has said them again, so its sender's window
// isn't known yet. It's held until kitty has, then goes to its sender's
// workspace without the focus, rather than staying where it opened. It
// exits 0 once that holds, and otherwise 1, saying what didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string state: Quickshell.env("XDG_STATE_HOME")
    property string stage: "listed"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function sender() {
        return Sessions.running.find(session => session.address === "SENDER");
    }

    // A move of the handed-over window, as niri was asked for it.
    function moves() {
        return Niri.acts.filter(act => act.MoveWindowToWorkspace?.window_id === 22);
    }

    Connections {
        target: Sessions
        function onRunningChanged() {
            Qt.callLater(root.check);
        }
        function onLockLetGoChanged() {
            Qt.callLater(root.check);
        }
    }
    Connections {
        target: Niri
        function onSpawnsChanged() {
            Qt.callLater(root.check);
        }
        function onActsChanged() {
            Qt.callLater(root.check);
        }
    }

    Component.onCompleted: Qt.callLater(check)

    function check() {
        try {
            if (stage === "listed")
                listed();
            else if (stage === "opening")
                opening();
            else if (stage === "letGo")
                letGo();
            else if (stage === "told")
                told();
        } catch (error) {
            fail(error.message);
        }
    }

    // The sender's window is known through kitty's first word on its two;
    // the lock is taken, and a session is handed over beside the sender,
    // as a quest's spawn hands one over.
    function listed() {
        if (sender()?.window?.id !== 21)
            return;
        Sessions.behindTheLock = true;
        const handed = { sessions: [{ id: "s-handed", cwd: "/tmp", from: "s-sender" }] };
        const answer = Sessions.handedOver(JSON.stringify(handed));
        expect("handed over", answer === "taken", answer);
        stage = "opening";
    }

    // Its window opens in the sessions' kitty, on the workspace Joel was
    // on, so kitty is asked of its windows again; the lock is let go
    // before it answers.
    function opening() {
        if (!Niri.spawns.some(command => command.includes("pi --session s-handed")))
            return;
        const window = { id: 22, pid: 600, workspace_id: 1, session: true };
        Niri.windows = Object.assign({}, Niri.windows, { "22": window });
        Niri.revision += 1;
        Niri.windowOpenedOrChanged(window);
        stage = "letGo";
        Sessions.behindTheLock = false;
    }

    // Once the lock is gone, with its sender's window not known yet, it
    // waits where it opened; then kitty says its three windows.
    function letGo() {
        if (!Sessions.lockLetGo)
            return;
        expect("the sender's window not known until kitty says it again",
            sender().window === null, JSON.stringify(sender().window));
        expect("not moved before its sender's window is known", moves().length === 0,
            JSON.stringify(moves()));
        stage = "told";
        telling.running = true;
    }
    Process {
        id: telling
        command: ["sh", "-c", `echo '${JSON.stringify([
            { id: 1, tabs: [{ windows: [{ id: 1 }] }] },
            { id: 2, tabs: [{ windows: [{ id: 4 }] }] },
            { id: 3, tabs: [{ windows: [{ id: 5 }] }] }
        ])}' > "$0"`, `${root.state}/kitty-600-later`]
    }

    // Then it goes to its sender's workspace, without the focus.
    function told() {
        if (moves().length === 0)
            return;
        expect("moved to its sender's workspace, without the focus",
            JSON.stringify(moves()) === JSON.stringify([{ MoveWindowToWorkspace: {
                window_id: 22, reference: { Id: 2 }, focus: false } }]),
            JSON.stringify(moves()));
        expect("nothing focused", !Niri.acts.some(act => act.FocusWindow
            || act.FocusWorkspace || act.FocusMonitor), JSON.stringify(Niri.acts));
        Qt.exit(0);
    }

    Timer {
        running: true
        interval: 50000
        onTriggered: root.fail(`still at ${root.stage}: ${JSON.stringify(Niri.acts)}`)
    }
}
