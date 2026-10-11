// The notification server: each notification is a notice in the top right
// corner, newest at the top. A click dismisses one; it leaves by itself when
// its time is up, unless it's critical.
//
// Each is kept in the session's history as it arrives (Notices.qml), for
// the launcher to find later.
//
// It's the notice kind (docs/design.md, Shape and Kinds): the edge frame,
// since it never takes the keys, or the alert's when critical; who or what
// first, in words, then the message; and a footer in muted monospace saying
// where it came from, its glyph a step apart from its name, and when.
import Quickshell
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    // Wide enough for a short message on one or two lines.
    readonly property int width: 380
    // The body step's line, to a whole pixel on the space grid.
    readonly property int line: Theme.space.xl - Theme.space.xs / 2

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
            if (!notification.transient)
                Notices.arrived(notification, urgent);
            const id = notification.id;
            notification.closed.connect(reason => Notices.left(id,
                reason === NotificationCloseReason.Expired));
            notification.tracked = true;
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
            const found = [];
            for (let i = 0; i < cards.count; i++) {
                const card = cards.itemAt(i);
                if (card)
                    found.push(card.area);
            }
            areas = found;
        }

        visible: server.trackedNotifications.values.length > 0
        anchors.top: true
        anchors.right: true
        implicitWidth: root.width + Theme.space.m + Theme.floating.reach
        implicitHeight: stack.height + Theme.space.m + Theme.floating.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: notices
        BackgroundEffect.blurRegion: notices
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "notifications"

        Column {
            id: stack
            x: Theme.floating.reach
            y: Theme.space.m
            width: root.width
            spacing: Theme.space.m

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

                    // Five seconds when the sender doesn't say.
                    Timer {
                        running: !card.urgent
                        interval: card.modelData.expireTimeout > 0 ? card.modelData.expireTimeout * 1000 : 5000
                        onTriggered: card.modelData.expire()
                    }
                }
            }
        }
    }
}
