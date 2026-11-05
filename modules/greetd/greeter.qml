// The greeter: who's signing in, the time, and a password field, in the lock
// screen's look, on every screen. greetd asks it after a logout, or at boot
// on a machine that doesn't sign in by itself. It runs from a copy of the
// account's own Quickshell files, so Theme, Prompt, Floating and the rest
// are the desktop's, and greetd checks the password through PAM.
//
// The greeting changes with the hour and names the person, the one bit of
// whimsy, at the threshold. Tab moves to the next account when there's more
// than one, and the footer says so; Return signs in, into the session the
// footer names. A question PAM asks after the password is asked in the
// field and what it says besides is said under the card, as on the lock
// screen. How long the boot took sits along the bottom, small, for the
// curious.
//
// Before it hands the session to greetd it writes "launching" to
// $XDG_RUNTIME_DIR/greeter-launching, so whatever started it knows a
// greeter that stopped without writing it failed, and can ask another way.
//
// Nix fills in the words between at signs: the accounts, each with what
// they're called, the first being who's asked first; the session's command;
// and systemd-analyze.
import Quickshell
import Quickshell.Io
import Quickshell.Services.Greetd
import Quickshell.Wayland
import QtQuick

ShellRoot {
    id: root

    // As wide as polkit's prompt, the same kind of thing.
    readonly property int width: 560

    readonly property var accounts: @accounts@
    readonly property var session: @session@
    // The session by its program's name: niri for niri-session.
    readonly property string into: session[0].split("/").pop()
        .replace(/^[0-9a-z]{32}-/, "").replace(/-session$/, "")
    property int chosen: 0
    readonly property var account: accounts[chosen]
    readonly property var next: accounts.length > 1
        ? accounts[(chosen + 1) % accounts.length] : null

    property string password
    property bool failed: false
    // A question PAM asks after the password, with whether its answer shows.
    property string question
    property bool answerShows: false
    // What PAM said that wasn't a question.
    property string said
    // How long the boot took, once systemd has said, in parts.
    property var booted: []
    // Whether greetd has just refused the password. Quickshell then cancels
    // the session greetd has already ended, and greetd's error about that
    // says nothing about the password.
    property bool refused: false

    function greeting(hour) {
        if (hour < 5)
            return "Still up";
        if (hour < 12)
            return "Good morning";
        if (hour < 18)
            return "Good afternoon";
        return "Good evening";
    }

    function choose(index) {
        if (Greetd.state !== GreetdState.Inactive)
            return;
        chosen = index;
        failed = false;
        said = "";
    }

    function enter(text) {
        if (Greetd.state === GreetdState.Authenticating) {
            if (question !== "") {
                question = "";
                Greetd.respond(text);
            }
            return;
        }
        if (Greetd.state !== GreetdState.Inactive)
            return;
        password = text;
        said = "";
        refused = false;
        Greetd.createSession(account.name);
    }

    // "Startup finished in 4.9s (firmware) + 5.5s (loader) + 512ms (kernel)
    // + 11.6s (initrd) + 10.9s (userspace) = 33.5s" as the parts "booted in
    // 33.5 s", "firmware 4.9" and on, every part in seconds.
    function timings(said) {
        const seconds = time => {
            const parts = time.match(/(\d+(?:\.\d+)?)(min|ms|s)/g) || [];
            let total = 0;
            for (const part of parts) {
                const [, figure, unit] = part.match(/(\d+(?:\.\d+)?)(min|ms|s)/);
                total += Number(figure) * (unit === "min" ? 60 : unit === "ms" ? 0.001 : 1);
            }
            return total.toFixed(1);
        };
        const finished = said.match(/Startup finished in (.*) = (.*)$/m);
        if (!finished)
            return [];
        const steps = finished[1].split(" + ").map(step => {
            const [, time, name] = step.match(/^(.*) \((.*)\)$/) || [null, step, ""];
            return `${name} ${seconds(time)}`;
        });
        return [`booted in ${seconds(finished[2])} s`, ...steps];
    }

    FileView {
        id: marker
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/greeter-launching`
        printErrors: false
        // Written before greetd is asked, so it's there once the greeter
        // stops.
        blockWrites: true
    }

    Process {
        command: ["@analyze@", "time"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: root.booted = root.timings(text)
        }
    }

    Connections {
        target: Greetd

        function onAuthMessage(message, error, responseRequired, echoResponse) {
            const words = message.trim();
            if (!responseRequired) {
                if (words !== "")
                    root.said = words;
            } else if (root.password !== "") {
                Greetd.respond(root.password);
                root.password = "";
            } else {
                root.question = words.replace(/:$/, "");
                root.answerShows = echoResponse;
            }
        }

        function onAuthFailure(message) {
            root.password = "";
            root.question = "";
            root.failed = true;
            root.refused = true;
            Report.say("refused greeter");
        }

        function onError(error) {
            if (root.refused)
                return;
            root.password = "";
            root.question = "";
            root.failed = true;
            root.said = "The password couldn't be checked.";
            Greetd.cancelSession();
            Report.say("refused greeter");
        }

        function onReadyToLaunch() {
            root.said = "";
            marker.setText("launching\n");
            Report.say(`signs in ${root.account.name}`);
            Greetd.launch(root.session);
        }
    }

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: surface

            required property var modelData

            screen: modelData
            anchors.top: true
            anchors.bottom: true
            anchors.left: true
            anchors.right: true
            exclusionMode: ExclusionMode.Ignore
            color: Theme.deepest
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            WlrLayershell.namespace: "greeter"

            Reported {
                name: "greeter"
            }

            Image {
                anchors.fill: parent
                visible: Theme.wallpaper.lock !== ""
                source: visible ? "file://" + Theme.wallpaper.lock : ""
                fillMode: Image.PreserveAspectCrop
                readonly property real ratio: surface.screen ? surface.screen.devicePixelRatio : 1
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

            // As the lock's: low on the screen, under the picture's sky, but
            // never nearer its foot than a floating frame stands from an
            // edge, so a short screen keeps the time on it, measured to the
            // said line's baseline. The card is placed first, on a whole
            // device pixel so its hairlines fall clean at 1.5, and the time
            // and what PAM says stand from it.
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

                // The time and the card are two regions with no rule
                // between them, so the day's baseline stands three beats
                // over the card's frame.
                PictureTime {
                    id: time
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: card.y - 3 * Theme.space.beat - dayBaseline
                    date: clock.date
                }

                Floating {
                    id: card
                    x: snap.whole((column.width - width) / 2)
                    // What PAM says is about the field, so its capitals
                    // stand a beat under the card, nearer it than the time
                    // is.
                    y: snap.whole(column.height - Math.max(column.height / 12,
                        Theme.space.rung.screen) - voiceFace.capitals - Theme.space.beat - height)
                    width: root.width
                    keys: true
                    urgent: root.failed

                    Column {
                        width: parent.width

                        // The greeting at Words, in text, their name a
                        // strong span, on one line.
                        PromptHead {
                            width: parent.width
                            readonly property string greeting:
                                root.greeting(clock.date.getHours())
                            readonly property string called: "<span style=\""
                                + `color:${Theme.strong};`
                                + `font-weight:${Theme.level.words.spans.strong}">`
                                + `${root.account.called}</span>`
                            asked: `${greeting}, ${called}`
                            askedOpens: greeting.charAt(0)
                            oneLine: true
                        }

                        Hairline {
                            width: parent.width
                        }

                        // The field, one row, its mark on the greeting's
                        // column.
                        Item {
                            width: parent.width
                            height: prompt.height

                            Prompt {
                                id: prompt
                                x: Theme.space.rung.inset - markAt
                                width: parent.width - 2 * x
                                // A layer surface's keys go nowhere until
                                // something in it asks for them.
                                Component.onCompleted: take()
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
                                onCompleted: if (root.next !== null) {
                                    clear();
                                    root.choose((root.chosen + 1) % root.accounts.length);
                                }
                                onCancelled: clear()
                            }
                        }

                        Hairline {
                            width: parent.width
                        }

                        // Where Return leads, the session's program, on
                        // the greeting's column, and what the keys do.
                        PromptFoot {
                            width: parent.width
                            leads: `into <font color="${Theme.text}">${root.into}</font>`
                            keys: root.next === null ? [["\u21b5", "sign in"]]
                                : [["\u21b5", "sign in"], ["\u21e5", root.next.called]]
                        }
                    }
                }

                // Room for one line of what PAM says, kept so the card never
                // moves when it speaks.
                Lifted {
                    anchors.horizontalCenter: parent.horizontalCenter
                    y: card.y + card.height + Theme.space.beat + voiceFace.capitals
                        - saying.baselineOffset
                    width: root.width

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

            // How long the boot took, along the bottom, its baseline the
            // inset over the screen's foot. It's as wide as the card, its
            // first and last ink on the card's edges and its parts an even
            // space apart, a dot in the middle of each, so it lines up with
            // the card rather than nearly does. At Note, but in text, since
            // muted is too faint on the wallpaper.
            Lifted {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: Theme.space.rung.inset - (booted.height - noteFace.ascent)
                visible: root.booted.length > 0

                Item {
                    id: booted
                    width: root.width
                    height: noteFace.height

                    FontMetrics {
                        id: noteFace
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                        font.weight: Theme.level.note.weight
                    }

                    // Each part's ink, from its box's left.
                    readonly property var inks: root.booted.map(part => {
                        const ink = noteFace.tightBoundingRect(part);
                        return { from: ink.x, wide: ink.width };
                    })
                    readonly property real gap: inks.length < 2 ? 0
                        : (width - inks.reduce((sum, ink) => sum + ink.wide, 0))
                            / (inks.length - 1)
                    // Where each part's ink starts.
                    readonly property var starts: inks.map((ink, at) => inks.slice(0, at)
                        .reduce((sum, before) => sum + before.wide + gap, 0))

                    Repeater {
                        model: root.booted

                        Item {
                            id: part
                            required property string modelData
                            required property int index

                            Text {
                                x: booted.starts[part.index] - booted.inks[part.index].from
                                text: part.modelData
                                color: Theme.text
                                font: noteFace.font
                            }

                            // The dot in the middle of the space after it.
                            Text {
                                visible: part.index < root.booted.length - 1
                                x: booted.starts[part.index] + booted.inks[part.index].wide
                                    + (booted.gap - implicitWidth) / 2
                                text: "\u00b7"
                                color: Theme.text
                                font: noteFace.font
                            }
                        }
                    }
                }
            }
        }
    }
}
