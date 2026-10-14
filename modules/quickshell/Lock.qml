// The lock screen: the time and a password field on every screen, checked by
// PAM through the quickshell service the system defines. Lock it with
// `qs ipc call lock lock`, as swayidle does when Joel steps away or the lid
// closes, and ask with `qs ipc call lock isLocked`. If Quickshell dies while
// locked, niri keeps the screens locked, and the shell started again takes
// the lock back, so the password still has somewhere to go: a switch
// restarts the shell, and can come while Joel is away. The field is the
// prompt kind (docs/design.md, Shape and Kinds): the padlock beside the
// dots, the accent frame, and the alert frame after a wrong password.
// Behind them is the theme's lock picture, shown whole: the time is lifted
// off it by the theme's text shadows, and both sit low, under the picture's
// sky. It fades in once read, so locking never waits on it.
//
// The password answers PAM's first question. A question after it, such as a
// security key's PIN, is asked in the field, and what PAM says besides, such
// as an account locked after too many tries, is said under it, in PAM's own
// words, since "try again" would be wrong.
//
// Once niri says every screen shows the lock, the lock writes "locked" to
// $XDG_RUNTIME_DIR/quickshell-locked, and "unlocked" once it's let go, so
// what locks before the machine sleeps can wait until nothing else is on
// screen (modules/swayidle). It writes niri's socket, which names niri's
// process, to $XDG_RUNTIME_DIR/quickshell-lock-holder first, and empties it
// first when let go: that's how a new shell knows the lock is its niri's,
// since the directory outlives a session while the account lingers, and a
// lock a crashed niri held mustn't lock the next one.
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
    // A question PAM asks after the password, with whether its answer shows.
    property string question
    property bool answerShows: false
    // What PAM said that wasn't a question.
    property string said

    function lock() {
        sessionLock.locked = true;
    }

    function enter(text) {
        if (pam.active) {
            if (pam.responseRequired && question !== "") {
                question = "";
                pam.respond(text);
            }
            return;
        }
        password = text;
        said = "";
        pam.start();
    }

    FileView {
        id: marker
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quickshell-locked`
        printErrors: false
        // A word in memory, so written before the shell says it's locked.
        blockWrites: true
    }

    FileView {
        id: holder
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quickshell-lock-holder`
        printErrors: false
        // Read before anything is drawn, to know whether to lock.
        blockLoading: true
        blockWrites: true
    }

    readonly property string niri: Quickshell.env("NIRI_SOCKET") ?? ""

    // A shell started while its niri holds the lock locks again, and niri
    // hands it the screens at once. Otherwise it says it's unlocked, since a
    // marker left by a shell that died says nothing about this one.
    Component.onCompleted: {
        if (niri !== "" && holder.text().trim() === niri)
            lock();
        else
            marker.setText("unlocked\n");
    }

    Connections {
        target: sessionLock

        function onSecureChanged() {
            if (sessionLock.secure) {
                holder.setText(`${root.niri}\n`);
                marker.setText("locked\n");
            } else {
                holder.setText("");
                marker.setText("unlocked\n");
            }
            Report.say(`${sessionLock.secure ? "secured" : "released"} lock`);
        }
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
        onPamMessage: {
            const words = message.trim();
            if (!responseRequired) {
                if (words !== "")
                    root.said = words;
            } else if (root.password !== "") {
                respond(root.password);
                root.password = "";
            } else {
                root.question = words.replace(/:$/, "");
                root.answerShows = responseVisible;
            }
        }
        onCompleted: result => {
            root.password = "";
            root.question = "";
            root.failed = result !== PamResult.Success;
            if (root.failed) {
                Report.say("refused lock");
            } else {
                root.said = "";
                sessionLock.locked = false;
            }
        }
        onError: {
            root.password = "";
            root.question = "";
            root.failed = true;
            root.said = "The password couldn't be checked.";
            Report.say("refused lock");
        }
    }

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    WlSessionLock {
        id: sessionLock

        WlSessionLockSurface {
            id: surface

            color: Theme.deepest

            Reported {
                name: "lock"
            }

            Image {
                anchors.fill: parent
                visible: Theme.wallpaper.lock !== ""
                source: visible ? "file://" + Theme.wallpaper.lock : ""
                fillMode: Image.PreserveAspectCrop
                readonly property real ratio: surface.screen?.devicePixelRatio ?? 1
                // Read at the screen's own pixels, not the file's.
                sourceSize: Qt.size(surface.width * ratio, surface.height * ratio)
                asynchronous: true
                opacity: status === Image.Ready ? 1 : 0

                Behavior on opacity {
                    NumberAnimation {
                        duration: Theme.motion.long
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: Theme.motion.curve
                    }
                }
            }

            Column {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: parent.height / 12
                spacing: Theme.space.xl

                Lifted {
                    anchors.horizontalCenter: parent.horizontalCenter

                    Column {
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: Qt.formatDateTime(clock.date, "HH:mm")
                            color: Theme.text
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.hero
                            font.features: { "tnum": 1 }
                        }

                        // Nothing else is on screen to say the day.
                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: Qt.formatDateTime(clock.date, "dddd d MMMM")
                            color: Theme.text
                            font.family: Theme.sans.family
                            font.pointSize: Theme.sans.large
                        }
                    }
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
                            secret: root.question === "" || !root.answerShows
                            hint: root.question !== "" ? root.question.toLowerCase()
                                : root.failed && root.said === "" ? "try again" : ""
                            onTextChanged: if (text !== "")
                                root.failed = false
                            onAccepted: {
                                const typed = text;
                                clear();
                                root.enter(typed);
                            }
                            onCancelled: clear()
                        }
                    }
                }

                // Room for one line of what PAM says, kept so the field never
                // moves when it speaks.
                Lifted {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: root.width

                    Text {
                        width: root.width
                        height: Theme.space.xl
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: root.said
                        color: Theme.text
                        font.family: Theme.sans.family
                        font.pointSize: Theme.sans.large
                    }
                }
            }
        }
    }
}
