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
    // Each bar of the wave is a step of time, this long. A bar and the room
    // after it are each two hairlines wide, a texture kin to the frame's
    // border rather than a space, and together make the wave's pitch.
    readonly property int step: 80
    readonly property int stroke: 2 * Theme.border
    readonly property int pitch: 2 * stroke
    // As many as the wave holds, its last bar's edge on the wave's own.
    readonly property int bars: Math.max(0, Math.floor((wave.width + stroke) / pitch))

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
    // A floating frame's distance from the screen's edge, below it.
    implicitHeight: card.height + Theme.space.rung.screen + Theme.floating.reach
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

    // A meter that shows while this does stands above it, the space
    // between frames that belong together, rather than over it.
    Binding {
        target: Meters
        property: "under"
        value: root.visible ? card.height + Theme.space.rung.frames : 0
    }

    // The line's face, to measure its ink by: the glyph's, the words', the
    // time's and a capital's, since the room round the line is measured to
    // the ink. Each is a TextMetrics, whose bounds follow the face, where a
    // call on FontMetrics measures once, before the face has loaded, and a
    // binding never asks again.
    FontMetrics {
        id: ink
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.features: { "tnum": 1 }
    }
    TextMetrics {
        id: capitalInk
        font: ink.font
        text: "H"
    }
    TextMetrics {
        id: glyphInk
        font: ink.font
        text: mark.text
    }
    TextMetrics {
        id: wordsInk
        font: ink.font
        text: root.words
    }
    TextMetrics {
        id: clockInk
        font: ink.font
        text: "0:00"
    }
    TextMetrics {
        id: countdownInk
        font: ink.font
        text: "5 s"
    }

    Floating {
        id: card
        x: Theme.floating.reach
        y: Theme.floating.reach
        width: root.length

        // A meter's frame, as Meter.qml's: one line at Note, the frame's
        // inset from its ink on every side, above the capitals, below the
        // baseline, before the glyph and after the widest time; the glyph a
        // mark from its word, and the word, the wave and the time each
        // apart, the time's figures tabular so it never shifts.
        Item {
            id: line
            readonly property rect capital: capitalInk.tightBoundingRect
            readonly property rect glyph: glyphInk.tightBoundingRect
            readonly property rect words: wordsInk.tightBoundingRect
            // The widest the time gets, a clock's or the seconds left's, so
            // the wave's length never moves.
            readonly property rect clock: clockInk.tightBoundingRect
            readonly property rect countdown: countdownInk.tightBoundingRect
            readonly property real timeEnds: Math.max(clock.x + clock.width,
                countdown.x + countdown.width)
            // Where the line's baseline rests: its capitals centred in it,
            // so what rounding its height leaves is shared above and below.
            readonly property real rests: (height + capital.height) / 2

            width: parent.width
            // A whole number of pixels tall, so the frame's bottom is set.
            height: 2 * Theme.space.rung.inset + Math.round(capital.height)

            Text {
                id: mark
                x: Theme.space.rung.inset - line.glyph.x
                y: line.rests - baselineOffset
                text: root.face ? Theme.glyph[root.face.glyph] : ""
                color: root.face ? Theme[root.face.hue] : Theme.muted
                font: ink.font
            }

            Text {
                id: said
                x: mark.x + line.glyph.x + line.glyph.width + Theme.space.rung.mark - line.words.x
                y: line.rests - baselineOffset
                text: root.words
                color: root.face && root.face.quiet ? Theme.muted : Theme.text
                font: ink.font
            }

            // The newest bar at the right, each the loudest the microphone
            // heard in its step, a listen's first steps filling in from
            // there. It's as tall as the capitals and stands on the line's
            // baseline, as a mark beside the words does.
            Item {
                id: wave
                visible: root.face !== null && root.face.wave
                // On whole pixels, so its strokes are drawn sharp.
                x: Math.round(said.x + line.words.x + line.words.width + Theme.space.rung.apart)
                width: Math.round(time.x + line.clock.x - Theme.space.rung.apart) - x
                y: Math.round(line.rests - height)
                height: line.capital.height

                Repeater {
                    model: root.bars

                    Rectangle {
                        required property int index
                        readonly property var bar: root.levels[root.levels.length - root.bars
                            + index]

                        visible: bar !== undefined
                        x: wave.width - (root.bars - index) * root.pitch + root.stroke
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.stroke
                        height: bar ? Speaking.height(bar.level, root.stroke, wave.height) : 0
                        color: Theme.text
                    }
                }
            }

            // Set from the left of the room the widest time takes, so a
            // shorter one leaves its room after it, at the line's far end,
            // and the wave's gap to it never grows.
            Text {
                id: time
                x: line.width - Theme.space.rung.inset - line.timeEnds
                y: line.rests - baselineOffset
                text: root.face ? root.face.time : ""
                color: Theme.text
                font: ink.font
            }
        }
    }
}
