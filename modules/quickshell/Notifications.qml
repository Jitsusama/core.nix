// The notification server: each notification is a notice in the top right
// corner, newest at the top. A click dismisses one; it leaves by itself when
// its time is up, unless it's critical. While notices are held (Quiet.qml),
// only a critical one shows.
//
// The corner's right edge sits a step inside the windows' (Margin.qml), and
// the notices make way below the margin's drawer while it's out.
//
// Each is kept in the session's history as it arrives (Notices.qml), for
// the launcher to find later. Super+N opens them there, by the keys, where
// the ones still here can be answered or let go, and while it's open the
// corner leaves them to it.
//
// It's the notice kind (docs/design.md, Shape and Kinds): the edge frame,
// since it never takes the keys, or the alert's when critical; who or what
// first, in words, then the message; and a footer in muted monospace saying
// where it came from, its glyph a step apart from its name, and when.
//
// What's playing (Music.qml) is a notice too, for a moment when a track
// changes or a media key is pressed, at the top of the same column so the
// two never cover each other: the cover beside the title and the artist,
// and the player and where in the track in the footer. It's no message, so
// it isn't kept; a click lets it go.
import Quickshell
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick
import QtQuick.Effects

Scope {
    id: root

    // Wide enough for a short message on one or two lines.
    readonly property int width: 380
    // The body step's line, to a whole pixel on the space grid.
    readonly property int line: Theme.space.xl - Theme.space.xs / 2

    // Whether what's playing is called for. The card's own visibility can't
    // say, since an item in a hidden window counts as hidden.
    readonly property bool playing: Music.shown && Music.card !== null
    // What the card says, or nothing while there's no player to say it of.
    readonly property var song: Music.card ?? {
        title: "", artist: "", art: "", from: "", mark: "", where: ""
    }

    // When each notification arrived, by its id, for the footer's time.
    property var arrived: ({})

    function since(then, now) {
        const minutes = Math.floor((now - then) / 60000);
        if (minutes < 1)
            return "now";
        if (minutes < 60)
            return `${minutes} min`;
        return Qt.formatDateTime(new Date(then), "HH:mm");
    }

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    NotificationServer {
        id: server
        bodySupported: true
        actionsSupported: true
        keepOnReload: true
        onNotification: notification => {
            root.arrived[notification.id] = Date.now();
            // One its sender marks transient asks not to be kept, as niri's
            // screenshot notice does.
            const urgent = notification.urgency === NotificationUrgency.Critical;
            const held = Quiet.on && !urgent;
            if (!notification.transient)
                Notices.arrived(notification, urgent, held);
            const id = notification.id;
            notification.closed.connect(reason => Notices.left(id,
                reason === NotificationCloseReason.Expired));
            // One held is let go at once, so it never shows.
            notification.tracked = !held;
        }
    }

    PanelWindow {
        id: window

        // Where the pointer reaches and what niri blurs: the notices alone,
        // not the gaps between them.
        property var areas: []
        readonly property Region notices: Region {
            regions: window.areas
        }

        function gather() {
            const found = root.playing ? [track.area] : [];
            for (let i = 0; i < cards.count; i++) {
                const card = cards.itemAt(i);
                if (card)
                    found.push(card.area);
            }
            areas = found;
        }

        visible: (server.trackedNotifications.values.length > 0 || root.playing)
            && !Notices.gathered
        anchors.top: true
        anchors.right: true
        implicitWidth: root.width + Theme.space.m + Theme.space.s + Theme.floating.reach
        implicitHeight: stack.height + stack.y + Theme.floating.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: notices
        BackgroundEffect.blurRegion: notices
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "notifications"

        Column {
            id: stack
            x: Theme.floating.reach
            y: Theme.space.m + Notices.below

            Behavior on y {
                NumberAnimation {
                    duration: Notices.below > 0 ? Theme.motion.medium : Theme.motion.short
                    easing.type: Easing.BezierSpline
                    easing.bezierCurve: Theme.motion.curve
                }
            }
            width: root.width
            spacing: Theme.space.m

            Floating {
                id: track

                width: stack.width
                visible: root.playing
                onVisibleChanged: Qt.callLater(window.gather)
                onHeightChanged: Qt.callLater(window.gather)

                Column {
                    id: said
                    x: Theme.space.l
                    width: parent.width - 2 * x
                    topPadding: Theme.space.m - Theme.border
                    bottomPadding: topPadding + 2

                    Item {
                        width: parent.width
                        height: Math.max(cover.height, lines.implicitHeight)

                        // Two lines tall, its corners the theme's inner
                        // radius, and the raised colour until it arrives.
                        Rectangle {
                            id: cover
                            width: 2 * root.line
                            height: width
                            radius: Theme.inner
                            color: Theme.raised
                            visible: root.song.art !== ""

                            Image {
                                anchors.fill: parent
                                source: root.song.art
                                asynchronous: true
                                fillMode: Image.PreserveAspectCrop
                                sourceSize.width: 2 * width
                                sourceSize.height: 2 * height
                                layer.enabled: true
                                layer.effect: MultiEffect {
                                    maskEnabled: true
                                    maskSource: corners
                                }
                            }

                            Rectangle {
                                id: corners
                                anchors.fill: parent
                                radius: Theme.inner
                                visible: false
                                layer.enabled: true
                            }
                        }

                        Column {
                            id: lines
                            anchors.left: cover.visible ? cover.right : parent.left
                            anchors.leftMargin: cover.visible ? Theme.space.m : 0
                            anchors.right: parent.right

                            Text {
                                width: parent.width
                                text: root.song.title
                                color: Theme.strong
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                                lineHeight: root.line
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.sans.family
                                font.pointSize: Theme.sans.body
                                font.weight: Font.Medium
                            }

                            Text {
                                width: parent.width
                                text: root.song.artist
                                visible: text !== ""
                                color: Theme.text
                                elide: Text.ElideRight
                                lineHeight: root.line
                                lineHeightMode: Text.FixedHeight
                                font.family: Theme.sans.family
                                font.pointSize: Theme.sans.body
                            }
                        }
                    }

                    // The player and where in the track, a step below.
                    Item {
                        width: parent.width
                        height: root.line + Theme.space.xs

                        Text {
                            id: player
                            anchors.bottom: parent.bottom
                            height: root.line
                            verticalAlignment: Text.AlignVCenter
                            text: root.song.mark
                            color: Theme.program
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }

                        Text {
                            anchors.left: player.right
                            anchors.leftMargin: Theme.space.s
                            anchors.right: place.left
                            anchors.rightMargin: Theme.space.l
                            anchors.bottom: parent.bottom
                            height: root.line
                            verticalAlignment: Text.AlignVCenter
                            text: root.song.from
                            elide: Text.ElideRight
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }

                        Text {
                            id: place
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom
                            height: root.line
                            verticalAlignment: Text.AlignVCenter
                            text: root.song.where
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                            font.features: { "tnum": 1 }
                        }
                    }
                }

                MouseArea {
                    width: parent.width
                    height: said.height
                    onClicked: Music.shown = false
                }
            }

            Repeater {
                id: cards
                model: server.trackedNotifications
                onItemAdded: Qt.callLater(window.gather)
                onItemRemoved: Qt.callLater(window.gather)

                delegate: Floating {
                    id: card

                    required property Notification modelData
                    readonly property var from: Notices.sender(modelData.appName)

                    width: stack.width
                    urgent: modelData.urgency === NotificationUrgency.Critical

                    Column {
                        id: words
                        x: Theme.space.l
                        width: parent.width - 2 * x
                        // The letters sit a step from the frame on every
                        // side, measured to the ink: the footer's glyph
                        // reaches lower than a line's letters, so the
                        // bottom keeps a little more.
                        topPadding: Theme.space.m - Theme.border
                        bottomPadding: topPadding + 2

                        Text {
                            width: parent.width
                            text: card.modelData.summary
                            color: Theme.strong
                            wrapMode: Text.Wrap
                            lineHeight: root.line
                            lineHeightMode: Text.FixedHeight
                            font.family: Theme.sans.family
                            font.pointSize: Theme.sans.body
                            font.weight: Font.Medium
                        }

                        Text {
                            width: parent.width
                            text: card.modelData.body
                            visible: text !== ""
                            color: Theme.text
                            wrapMode: Text.Wrap
                            maximumLineCount: 6
                            elide: Text.ElideRight
                            textFormat: Text.StyledText
                            lineHeight: root.line
                            lineHeightMode: Text.FixedHeight
                            font.family: Theme.sans.family
                            font.pointSize: Theme.sans.body
                        }

                        // Where it came from and when, a step below the message.
                        Item {
                            width: parent.width
                            height: root.line + Theme.space.xs

                            Text {
                                id: mark
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                text: String.fromCodePoint(card.from.glyph)
                                color: Theme[card.from.hue]
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                            }

                            Text {
                                anchors.left: mark.right
                                anchors.leftMargin: Theme.space.s
                                anchors.right: time.left
                                anchors.rightMargin: Theme.space.l
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                text: card.modelData.appName
                                elide: Text.ElideRight
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                            }

                            Text {
                                id: time
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: root.line
                                verticalAlignment: Text.AlignVCenter
                                readonly property real now: clock.date.getTime()
                                text: root.since(root.arrived[card.modelData.id] ?? now, now)
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.body
                                font.features: { "tnum": 1 }
                            }
                        }
                    }

                    MouseArea {
                        width: parent.width
                        height: words.height
                        onClicked: card.modelData.dismiss()
                    }

                    // Five seconds when the sender doesn't say. Quickshell's
                    // documents call its timeout seconds, but it passes on
                    // what the sender said, which is milliseconds.
                    Timer {
                        running: !card.urgent
                        interval: card.modelData.expireTimeout > 0
                            ? card.modelData.expireTimeout : 5000
                        onTriggered: card.modelData.expire()
                    }
                }
            }
        }
    }
}
