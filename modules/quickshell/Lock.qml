// The lock screen: the time and a password box on every screen, checked by
// PAM through the quickshell service the system defines. Lock it with
// `qs ipc call lock lock`, as swayidle does when Joel steps away or the lid
// closes, and ask with `qs ipc call lock isLocked`. If Quickshell dies while
// locked, niri keeps the screens locked.
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pam
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    property string password
    property bool failed: false

    function lock() {
        sessionLock.locked = true;
    }

    IpcHandler {
        target: "lock"

        function lock(): void {
            root.lock();
        }

        function isLocked(): bool {
            return sessionLock.locked;
        }
    }

    PamContext {
        id: pam
        config: "quickshell"
        onResponseRequiredChanged: {
            if (responseRequired)
                respond(root.password);
        }
        onCompleted: result => {
            root.password = "";
            root.failed = result !== PamResult.Success;
            if (!root.failed)
                sessionLock.locked = false;
        }
    }

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    WlSessionLock {
        id: sessionLock

        WlSessionLockSurface {
            color: Theme.darker_background

            Column {
                anchors.centerIn: parent
                spacing: Theme.gap * 2

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Qt.formatDateTime(clock.date, "HH:mm")
                    color: Theme.foreground
                    font.family: Theme.fontFamily
                    font.pointSize: Theme.fontSize * 6
                }

                Prompt {
                    id: prompt
                    label: root.failed ? "Try again" : "Password"
                    secret: true
                    onAccepted: {
                        if (pam.active)
                            return;
                        root.password = text;
                        clear();
                        pam.start();
                    }
                    onCancelled: clear()
                }
            }
        }
    }
}
