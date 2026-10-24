// Dictation's own look, at the bottom centre in a meter's frame while it's
// anything but idle, so what's heard is never heard unseen and it never
// reads as the microphone's volume (docs/design.md, Dictation). It shows a
// person speaking in the voice colour, its word, a wave of what the
// microphone has heard and the time; from 25 s the time counts down what's
// left of Whisper's 30 and the glyph turns to the warning colour. Typing
// shows once it lasts long enough to see. Then, for as long as a meter
// stays, what the service said of a listen that typed nothing, or of a
// press while it was still writing. Speaking.js works out what it says.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Services.Pipewire
import Quickshell.Wayland
import QtQuick
import "Speaking.js" as Speaking

PanelWindow {
    id: root

    // When this listen began, and the time the clock last read.
    property real began: 0
    property real now: 0
    // Whether typing has lasted long enough to show.
    property bool typingSeen: false
    // A moment's answer from the service, shown while it's held.
    property string after: ""
    // The wave's bars, a step of time each, the newest last.
    property var levels: []

    readonly property var face: Speaking.said(Dictation.state, Math.max(0, now - began), after,
        typingSeen)
    // As wide as a meter, so its frame is the meter's.
    readonly property int length: 320
    // Each bar of the wave is a step of time, this long, and its width and
    // the room after it make a pitch of the smallest space.
    readonly property int step: 80
    readonly property int pitch: Theme.space.xs
    readonly property int bars: Math.max(0, Math.floor(wave.width / pitch))

    readonly property string words: face ? face.words : ""

    // What it says, to the desktop test while reports are on.
    onWordsChanged: {
        if (Options.reports && root.words !== "")
            Report.say(`says dictation ${root.words}`);
    }

    // A screen coming or going (a dock, the lid, a monitor) can leave the
    // surface nowhere: with no screen left, Qt moves the window onto a
    // stand-in that can hold no surface, then onto the screen that comes
    // back by itself, so Quickshell never makes the surface again
    // (Launcher.qml says more). A listen under way then would go unseen to
    // its end, so once the screens change, a surface that's up is let go
    // for a turn and made again, as the notices' is.
    property bool remaking: false

    Connections {
        target: Quickshell

        function onScreensChanged() {
            root.remaking = true;
            Qt.callLater(() => {
                root.remaking = false;
            });
        }
    }

    visible: face !== null && !remaking
    anchors.bottom: true
    implicitWidth: length + 2 * Theme.floating.reach
    implicitHeight: card.height + Theme.space.xxxl + Theme.floating.reach
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    mask: card.area
    BackgroundEffect.blurRegion: card.area
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "dictation"

    Connections {
        target: Dictation

        function onStateChanged() {
            if (Dictation.listening) {
                root.began = Date.now();
                root.now = root.began;
                root.levels = [];
                root.after = "";
            }
            root.typingSeen = false;
            if (Dictation.transcribing)
                seen.restart();
        }

        function onAnswered(word) {
            root.after = word;
            hold.restart();
        }
    }

    // The clock reads the time each second of the listen, from its start,
    // so each reading falls just past a whole second.
    Timer {
        interval: 1000
        repeat: true
        running: Dictation.listening
        onTriggered: root.now = Date.now()
    }

    // Typing is told once it has lasted the theme's longest movement, so a
    // quick one never flickers.
    Timer {
        id: seen
        interval: Theme.motion.long
        onTriggered: root.typingSeen = Dictation.transcribing
    }

    // An answer stays as long as a meter does after a change.
    Timer {
        id: hold
        interval: 1500
        onTriggered: root.after = ""
    }

    // The microphone's peaks while it listens, each as it comes, so nothing
    // is asked every so often.
    PwNodePeakMonitor {
        node: Pipewire.defaultAudioSource
        enabled: Dictation.listening
        onPeakChanged: root.levels = Speaking.waved(root.levels, peak,
            Math.floor((Date.now() - root.began) / root.step), root.bars)
    }

    Reported {
        name: "dictation"
    }

    Floating {
        id: card
        x: Theme.floating.reach
        y: Theme.floating.reach
        width: root.length

        Item {
            width: parent.width
            height: Theme.space.xxxl - 2 * Theme.border

            Text {
                id: mark
                x: Theme.space.l - Theme.border
                anchors.verticalCenter: parent.verticalCenter
                width: Theme.space.xl
                text: root.face ? Theme.glyph[root.face.glyph] : ""
                color: root.face ? Theme[root.face.hue] : Theme.muted
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.body
            }

            Text {
                id: said
                anchors.left: mark.right
                anchors.verticalCenter: parent.verticalCenter
                text: root.words
                color: root.face && root.face.quiet ? Theme.muted : Theme.text
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.body
            }

            // The newest bar at the right, each the loudest the microphone
            // heard in its step, a listen's first steps filling in from there.
            Item {
                id: wave
                visible: root.face !== null && root.face.wave
                anchors.left: said.right
                anchors.leftMargin: Theme.space.m
                anchors.right: time.left
                anchors.rightMargin: Theme.space.l
                anchors.verticalCenter: parent.verticalCenter
                height: Theme.space.l

                Repeater {
                    model: root.bars

                    Rectangle {
                        required property int index
                        readonly property var bar: root.levels[root.levels.length - root.bars
                            + index]

                        visible: bar !== undefined
                        x: wave.width - (root.bars - index) * root.pitch
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.pitch / 2
                        height: bar ? Speaking.height(bar.level, 2 * Theme.border, wave.height) : 0
                        color: Theme.text
                    }
                }
            }

            // The widest the time gets, so the wave's length never moves.
            TextMetrics {
                id: widest
                font: time.font
                text: "0:00"
            }

            Text {
                id: time
                width: Math.ceil(widest.advanceWidth)
                horizontalAlignment: Text.AlignRight
                anchors.right: parent.right
                anchors.rightMargin: Theme.space.l - Theme.border
                anchors.verticalCenter: parent.verticalCenter
                text: root.face ? root.face.time : ""
                color: Theme.text
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.body
                font.features: { "tnum": 1 }
            }
        }
    }
}
