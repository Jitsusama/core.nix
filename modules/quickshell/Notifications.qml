// The notification server: each notification is a card in the top right
// corner, newest at the top. A click dismisses one; it leaves by itself when
// its time is up, unless it's critical.
import Quickshell
import Quickshell.Services.Notifications
import Quickshell.Wayland
import QtQuick

Scope {
    NotificationServer {
        id: server
        bodySupported: true
        actionsSupported: true
        keepOnReload: true
        onNotification: notification => notification.tracked = true
    }

    PanelWindow {
        visible: server.trackedNotifications.values.length > 0
        anchors.top: true
        anchors.right: true
        margins.top: Theme.space.m
        margins.right: Theme.space.m
        implicitWidth: 400
        implicitHeight: cards.implicitHeight
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "notifications"

        Column {
            id: cards
            width: parent.width
            spacing: Theme.space.m

            Repeater {
                model: server.trackedNotifications

                delegate: Rectangle {
                    id: card

                    required property Notification modelData
                    readonly property bool critical: modelData.urgency === NotificationUrgency.Critical

                    width: cards.width
                    implicitHeight: text.implicitHeight + Theme.space.xl
                    color: Theme.background
                    border.color: critical ? Theme.alert : Theme.accent
                    border.width: Theme.border
                    radius: Theme.radius

                    Column {
                        id: text
                        anchors.fill: parent
                        anchors.margins: Theme.space.m
                        spacing: Theme.space.xs

                        Text {
                            width: parent.width
                            text: card.modelData.appName
                            visible: text !== ""
                            color: Theme.muted
                            elide: Text.ElideRight
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }
                        Text {
                            width: parent.width
                            text: card.modelData.summary
                            color: Theme.strong
                            wrapMode: Text.Wrap
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                            font.bold: true
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
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: card.modelData.dismiss()
                    }

                    // Five seconds when the sender doesn't say.
                    Timer {
                        running: !card.critical
                        interval: card.modelData.expireTimeout > 0 ? card.modelData.expireTimeout * 1000 : 5000
                        onTriggered: card.modelData.expire()
                    }
                }
            }
        }
    }
}
