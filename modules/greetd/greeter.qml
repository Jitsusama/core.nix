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
    // A line of the title: the large step plus half the smallest.
    readonly property int line: Theme.space.xl + Theme.space.xs / 2

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
    // How long the boot took, once systemd has said.
    property string booted
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
    // + 11.6s (initrd) + 10.9s (userspace) = 33.5s" as "booted in 33.5 s ·
    // firmware 4.9 · …", every part in seconds.
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
            return "";
        const steps = finished[1].split(" + ").map(step => {
            const [, time, name] = step.match(/^(.*) \((.*)\)$/) || [null, step, ""];
            return `${name} ${seconds(time)}`;
        });
        return [`booted in ${seconds(finished[2])} s`, ...steps].join("  \u00b7  ");
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

                    Column {
                        width: parent.width

                        Item {
                            width: parent.width
                            // The letters sit as far from the frame as from
                            // its sides, as on polkit's prompt.
                            height: title.height + 2 * (Theme.inset + Theme.space.l)
                                - Theme.border - 7

                            Text {
                                id: title
                                x: Theme.inset + Theme.space.l
                                y: Theme.inset + Theme.space.l - Theme.border - 5
                                width: parent.width - 2 * x
                                textFormat: Text.RichText
                                readonly property string called: "<span style=\""
                                    + `color:${Theme.strong};font-weight:500">`
                                    + `${root.account.called}</span>`
                                text: `${root.greeting(clock.date.getHours())}, ${called}`
                                color: Theme.text
                                elide: Text.ElideRight
                                lineHeight: root.line
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.sans.family
                                font.pointSize: Theme.sans.large
                            }
                        }

                        Rectangle {
                            width: parent.width
                            height: Theme.border
                            color: Theme.edge
                        }

                        Item {
                            width: parent.width
                            height: prompt.height + 2 * Theme.inset

                            Prompt {
                                id: prompt
                                x: Theme.inset
                                y: Theme.inset
                                width: parent.width - 2 * Theme.inset
                                // A layer surface's keys go nowhere until
                                // something in it asks for them.
                                Component.onCompleted: take()
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
                                onCompleted: if (root.next !== null) {
                                    clear();
                                    root.choose((root.chosen + 1) % root.accounts.length);
                                }
                                onCancelled: clear()
                            }
                        }

                        Rectangle {
                            width: parent.width
                            height: Theme.border
                            color: Theme.edge
                        }

                        Item {
                            width: parent.width
                            height: Theme.space.xxl + Theme.space.s

                            // Where Return leads: the session's program.
                            Text {
                                anchors.left: parent.left
                                anchors.leftMargin: Theme.space.l
                                anchors.right: keys.left
                                anchors.rightMargin: Theme.space.xl
                                anchors.verticalCenter: parent.verticalCenter
                                textFormat: Text.StyledText
                                text: `into <font color="${Theme.text}">${root.into}</font>`
                                color: Theme.muted
                                elide: Text.ElideRight
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
                            }

                            KeyHints {
                                id: keys
                                anchors.right: parent.right
                                anchors.rightMargin: Theme.space.l
                                anchors.verticalCenter: parent.verticalCenter
                                keys: root.next === null ? [["\u21b5", "sign in"]]
                                    : [["\u21b5", "sign in"], ["\u21e5", root.next.called]]
                            }
                        }
                    }
                }

                // Room for one line of what PAM says, kept so the card never
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

            Lifted {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.bottom
                anchors.bottomMargin: Theme.space.xl

                Text {
                    text: root.booted
                    color: Theme.text
                    font.family: Theme.mono.family
                    font.pointSize: Theme.mono.small
                    font.features: { "tnum": 1 }
                }
            }
        }
    }
}
