// The one text box every part of the shell asks through: the launcher's
// search, the lock screen's password and polkit's. A label sits in front of
// what's typed, and a secret shows as dots.
import QtQuick

Rectangle {
    id: prompt

    property string label
    property bool secret: false
    property alias text: input.text

    signal accepted
    signal cancelled

    function clear() {
        input.text = "";
    }

    implicitWidth: 480
    implicitHeight: input.implicitHeight + Theme.gap * 2
    color: Theme.background
    border.color: Theme.accent
    border.width: Theme.border
    radius: Theme.radius

    Text {
        id: label
        anchors.left: parent.left
        anchors.leftMargin: Theme.gap
        anchors.verticalCenter: parent.verticalCenter
        text: prompt.label
        color: Theme.accent
        font.family: Theme.fontFamily
        font.pointSize: Theme.fontSize
    }

    TextInput {
        id: input
        anchors.left: label.right
        anchors.leftMargin: prompt.label === "" ? 0 : Theme.gap
        anchors.right: parent.right
        anchors.rightMargin: Theme.gap
        anchors.verticalCenter: parent.verticalCenter
        focus: true
        clip: true
        echoMode: prompt.secret ? TextInput.Password : TextInput.Normal
        color: Theme.foreground
        selectionColor: Theme.selection
        selectedTextColor: Theme.bright_foreground
        font.family: Theme.fontFamily
        font.pointSize: Theme.fontSize
        onAccepted: prompt.accepted()
        Keys.onEscapePressed: prompt.cancelled()
    }
}
