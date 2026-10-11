// The lock screen: the time and a password field on every screen, checked by
// PAM through the quickshell service the system defines. Lock it with
// `qs ipc call lock lock`, as swayidle does when Joel steps away or the lid
// closes, and ask with `qs ipc call lock isLocked`. If Quickshell dies while
// locked, niri keeps the screens locked. The field is the prompt kind
// (docs/design.md, Shape and Kinds): the padlock beside the dots, the accent
// frame, and the alert frame after a wrong password.
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pam
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    // As wide as a long password's dots.
    readonly property int width: 400

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
            color: Theme.deepest

            Column {
                anchors.centerIn: parent
                spacing: Theme.space.xl

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Qt.formatDateTime(clock.date, "HH:mm")
                    color: Theme.text
                    font.family: Theme.mono.family
                    font.pointSize: Theme.mono.hero
                    font.features: { "tnum": 1 }
                }

                Floating {
                    width: root.width
                    keys: true
                    urgent: root.failed

                    Item {
                        width: parent.width
                        height: prompt.height + 2 * Theme.inset

                        Prompt {
                            id: prompt
                            x: Theme.inset
                            y: Theme.inset
                            width: parent.width - 2 * Theme.inset
                            glyph: String.fromCodePoint(0xf023)
                            hue: Theme.machine
                            secret: true
                            hint: root.failed ? "try again" : ""
                            onTextChanged: if (text !== "")
                                root.failed = false
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
    }
}
