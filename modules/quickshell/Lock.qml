// The lock screen: the time and a password field on every screen, checked by
// PAM through the quickshell service the system defines. Lock it with
// `qs ipc call lock lock`, as swayidle does when Joel steps away or the lid
// closes, and ask with `qs ipc call lock isLocked`. `qs ipc call lock release`
// lets it go without the password, as swayidle does when logind says to
// unlock: after a resume from hibernation, the disk's own prompt has already
// asked for it. Anything running as Joel could do as much through loginctl,
// so this opens nothing new. If Quickshell dies while
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
pragma ComponentBehavior: Bound
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

    // Whether the lock is asked for or up: from the moment it's asked for,
    // since niri refuses moves while it's locking too, until it's let go.
    // It's read back from the lock each time rather than bound to it, since
    // Quickshell's lock says its locked changed only when it's let go, and
    // never when it's taken.
    property bool held: false

    function lock() {
        sessionLock.locked = true;
        held = sessionLock.locked;
    }

    // A password half typed, or a question PAM is waiting on, goes with it,
    // so the next lock starts with an empty field.
    function release() {
        if (!sessionLock.locked)
            return;
        if (pam.active)
            pam.abort();
        password = "";
        question = "";
        said = "";
        failed = false;
        sessionLock.locked = false;
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

        function onLockedChanged() {
            root.held = sessionLock.locked;
        }

        function onSecureChanged() {
            root.held = sessionLock.locked;
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

        function release(): void {
            root.release();
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

            // Low on the screen, under the picture's sky, but never nearer its
            // foot than a floating frame stands from an edge, so a short
            // screen keeps the time on it, measured to the said line's
            // baseline. The field is placed first, on a whole device pixel
            // so its frame's hairline falls clean at 1.5, and the time and
            // what PAM says stand from it.
            Item {
                id: column
                anchors.fill: parent

                FontMetrics {
                    id: voiceFace
                    readonly property real capitals: tightBoundingRect("H").height
                    font.family: Theme.level.voice.family
                    font.pointSize: Theme.level.voice.pointSize
                    font.italic: Theme.level.voice.italic
                    font.variableAxes: Theme.level.voice.variableAxes
                }

                Hairline {
                    id: snap
                    visible: false
                }

                // The time and the field are two regions with no rule
                // between them, so the day's baseline stands three beats
                // over the field's frame.
                PictureTime {
                    id: time
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: field.y - 3 * Theme.space.beat - dayBaseline
                    date: clock.date
                }

                Floating {
                    id: field
                    x: snap.whole((column.width - width) / 2)
                    // What PAM says is about the field, so its capitals
                    // stand a beat under the frame, nearer it than the time
                    // is.
                    y: snap.whole(column.height - Math.max(column.height / 12,
                        Theme.space.rung.screen) - voiceFace.capitals - Theme.space.beat - height)
                    width: root.width
                    keys: true
                    urgent: root.failed

                    // A lone field: one row, its mark caps-centred, its
                    // sides the inset from the frame to the ink.
                    Item {
                        width: parent.width
                        height: prompt.height

                        Prompt {
                            id: prompt
                            x: Theme.space.rung.inset - markAt
                            width: parent.width - 2 * x
                            glyph: Theme.glyph.secret
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
                    y: field.y + field.height + Theme.space.beat + voiceFace.capitals
                        - saying.baselineOffset
                    width: root.width

                    // In the voice, since it's the lock speaking, in text
                    // as it does.
                    Text {
                        id: saying
                        width: root.width
                        // A line's room whether it says anything or not.
                        height: voiceFace.height
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: root.said
                        color: Theme.text
                        font.family: Theme.level.voice.family
                        font.pointSize: Theme.level.voice.pointSize
                        font.weight: Theme.level.voice.weight
                        font.italic: Theme.level.voice.italic
                        font.variableAxes: Theme.level.voice.variableAxes
                    }
                }
            }
        }
    }
}
